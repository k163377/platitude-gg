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

/// Parses a todo file (git's own, or one we wrote). Comments, blank lines
/// and commands this application does not model are skipped. Production
/// only writes todos (render_todo → apply_plan overwrites git's file
/// whole), so the read half exists for the tests that pin the format.
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
