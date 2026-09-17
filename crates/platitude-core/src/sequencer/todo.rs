//! The todo file's own format: the verbs git understands, the lines it
//! holds, and the step the screen composes into one of them.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoAction {
    Pick,
    /// Keep the commit, replace its message (needs [`RebaseStep::message`]).
    Reword,
    /// Stop after applying so the user can change the commit.
    Edit,
    /// Fold into the previous commit, combining the messages.
    Squash,
    /// Fold into the previous commit, keeping only its message.
    Fixup,
    /// Leave the commit out.
    Drop,
}

impl TodoAction {
    fn keyword(self) -> &'static str {
        match self {
            TodoAction::Pick | TodoAction::Reword => "pick",
            TodoAction::Edit => "edit",
            TodoAction::Squash => "squash",
            TodoAction::Fixup => "fixup",
            TodoAction::Drop => "drop",
        }
    }

    #[cfg(test)]
    pub(crate) fn from_keyword(word: &str) -> Option<Self> {
        match word {
            "p" | "pick" => Some(TodoAction::Pick),
            "r" | "reword" => Some(TodoAction::Reword),
            "e" | "edit" => Some(TodoAction::Edit),
            "s" | "squash" => Some(TodoAction::Squash),
            "f" | "fixup" => Some(TodoAction::Fixup),
            "d" | "drop" => Some(TodoAction::Drop),
            _ => None,
        }
    }
}

/// One commit in the plan, oldest first (git's todo order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseStep {
    pub action: TodoAction,
    pub oid: String,
    pub subject: String,
    /// Replacement message; required for [`TodoAction::Reword`].
    pub message: Option<String>,
}

impl RebaseStep {
    pub fn pick(oid: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            action: TodoAction::Pick,
            oid: oid.into(),
            subject: subject.into(),
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TodoLine {
    Command {
        action: TodoAction,
        oid: String,
        subject: String,
    },
    /// `exec <command>`: run a shell command at this point in the replay.
    Exec { command: String },
}

pub fn render_todo(lines: &[TodoLine]) -> String {
    let mut out = String::new();
    for line in lines {
        match line {
            TodoLine::Command {
                action,
                oid,
                subject,
            } => {
                out.push_str(action.keyword());
                out.push(' ');
                out.push_str(oid);
                if !subject.is_empty() {
                    out.push(' ');
                    // A todo subject is a trailing comment to git; newlines
                    // in it would forge extra commands.
                    out.push_str(&subject.replace(['\n', '\r'], " "));
                }
                out.push('\n');
            }
            TodoLine::Exec { command } => {
                out.push_str("exec ");
                out.push_str(&command.replace(['\n', '\r'], " "));
                out.push('\n');
            }
        }
    }
    out
}

/// What a merge left behind: the todo to write, and the lines of git's
/// own that had no commit left to hang off ([`merge_todo`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedTodo {
    pub text: String,
    /// git's lines that were dropped, for the helper to name on stderr —
    /// where git shows what its editor said.
    pub orphaned: Vec<String>,
}

/// The plan, with the lines git wrote into its own todo carried over.
///
/// **git decides which refs follow a rewrite, and this keeps its answer.**
/// Under `--update-refs` git writes an `update-ref <ref>` line after the
/// commit each local branch inside the range stands on — and writes none
/// for a tag, none for a branch outside the range, and **none for a branch
/// another working copy has checked out**, which it says so about in a
/// comment instead (measured, 2.51: `# Ref refs/heads/held checked out at
/// '<path>'`). Replacing the file wholesale threw all of that away, so the
/// flag was passed and nothing followed; working the set out again here
/// would be a second implementation of a rule git already applies, and the
/// two would disagree exactly where it matters (P3-確認事項 §A).
///
/// So the plan's own command lines are what is written, and every other
/// line git put in travels with the commit it came after: a step that was
/// moved takes its ref with it, which is what a hand editing the file
/// would leave behind. Comments go — git reads none of them.
///
/// Matched on the id, which git abbreviates and the plan spells whole.
///
/// **Both files are walked once.** The plan is as long as the range, and
/// the range can be the whole history: a scan of git's lines per plan line
/// is quadratic, and so is closing the gap a taken line leaves behind —
/// seconds against milliseconds by the tens of thousands of rows, and
/// worst where the plan moved rows furthest (ci/baseline の
/// code-costs-windows-x64.md §todo の突き合わせ).
pub fn merge_todo(plan: &str, generated: &str) -> MergedTodo {
    let mut trailers = Trailers::default();
    for line in generated.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match command_oid(line) {
            Some(oid) => trailers.open(oid),
            // Before the first command there is nothing for a line to
            // hang off, and git writes none there.
            None => trailers.hold(line),
        }
    }

    let mut text = String::new();
    // Held to the next command line: a reword's own `exec` follows its
    // `pick` in the plan, and it is that exec which leaves HEAD where the
    // ref should land.
    let mut pending: Vec<String> = Vec::new();
    for line in plan.lines() {
        if let Some(oid) = command_oid(line.trim()) {
            for held in pending.drain(..) {
                text.push_str(&held);
                text.push('\n');
            }
            pending = trailers.take(oid).unwrap_or_default();
        }
        text.push_str(line);
        text.push('\n');
    }
    for held in pending {
        text.push_str(&held);
        text.push('\n');
    }
    MergedTodo {
        text,
        orphaned: trailers.left_over(),
    }
}

/// git's own lines, kept under the commit each followed and looked up by
/// the whole id the plan spells.
///
/// **git abbreviates to one length per file**, so the index is on that
/// length and the lookup is a hash of the plan id's first `len` bytes. A
/// file that somehow carried two lengths is answered from the earliest
/// line that matches, which is the order a scan would have found.
#[derive(Default)]
struct Trailers {
    /// One entry per command line git wrote, in file order; `None` once
    /// the plan has taken it.
    held: Vec<Option<Vec<String>>>,
    /// The abbreviated id of each of those, to its place above.
    at: std::collections::HashMap<String, usize>,
    /// The lengths those ids come in, shortest first — one, in every file
    /// git writes.
    lengths: Vec<usize>,
}

impl Trailers {
    fn open(&mut self, oid: &str) {
        self.at.insert(oid.to_string(), self.held.len());
        self.held.push(Some(Vec::new()));
        if !self.lengths.contains(&oid.len()) {
            self.lengths.push(oid.len());
            self.lengths.sort_unstable();
        }
    }

    fn hold(&mut self, line: &str) {
        if let Some(Some(held)) = self.held.last_mut() {
            held.push(line.to_string());
        }
    }

    /// The lines git hung off `oid`, taken out so a second plan line
    /// naming the same commit cannot have them too.
    fn take(&mut self, oid: &str) -> Option<Vec<String>> {
        let at = self
            .lengths
            .iter()
            .filter_map(|len| self.at.get(oid.get(..*len)?))
            .min()?;
        self.held.get_mut(*at)?.take()
    }

    /// What the plan had no line for: a commit git wrote a ref after and
    /// the plan does not carry, which is a range that moved under it.
    fn left_over(self) -> Vec<String> {
        self.held.into_iter().flatten().flatten().collect()
    }
}

/// The id of a todo command line, or `None` where the line is not one —
/// `update-ref`, `label`, `reset` and the rest carry no commit.
fn command_oid(line: &str) -> Option<&str> {
    let (word, rest) = line.split_once(char::is_whitespace)?;
    let names_a_commit = matches!(
        word,
        "p" | "pick"
            | "r"
            | "reword"
            | "e"
            | "edit"
            | "s"
            | "squash"
            | "f"
            | "fixup"
            | "d"
            | "drop"
    );
    if !names_a_commit {
        return None;
    }
    let oid = rest.split_whitespace().next()?;
    (!oid.is_empty()).then_some(oid)
}

/// Parses a todo file (git's own, or one we wrote). Comments, blank lines
/// and commands this application does not model are skipped. Production
/// only writes todos (render_todo → apply_plan), so the read half exists
/// for the tests that pin the format.
#[cfg(test)]
pub fn parse_todo(text: &str) -> Vec<TodoLine> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (word, rest) = match line.split_once(char::is_whitespace) {
            Some((w, r)) => (w, r.trim_start()),
            None => (line, ""),
        };
        if word == "x" || word == "exec" {
            out.push(TodoLine::Exec {
                command: rest.to_string(),
            });
            continue;
        }
        let Some(action) = TodoAction::from_keyword(word) else {
            continue;
        };
        let (oid, subject) = match rest.split_once(char::is_whitespace) {
            Some((o, s)) => (o, s.trim_start()),
            None => (rest, ""),
        };
        if oid.is_empty() {
            continue;
        }
        out.push(TodoLine::Command {
            action,
            oid: oid.to_string(),
            subject: subject.to_string(),
        });
    }
    out
}
