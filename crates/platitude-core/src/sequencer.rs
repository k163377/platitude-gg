//! Interactive rebase driven from the GUI.
//!
//! git asks an editor to write the todo list; this module supplies that
//! editor. The plan the user assembled is written to a file, and
//! `GIT_SEQUENCE_EDITOR` is pointed at the `pg-todo-editor` helper that
//! ships beside the application, which copies the plan over git's todo
//! file and exits (実装計画 §6, P3-確認事項 §残っている実装).
//!
//! Rewording is expressed as `pick` plus an `exec git commit --amend
//! --file`, not as a `reword` line. A `reword` would open `GIT_EDITOR`,
//! which the process layer pins to `true` so nothing hangs — and the
//! message would silently stay as it was. The `exec` form states the new
//! message outright, so it either applies or fails loudly.

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::{RebaseOptions, rebase_command};
use crate::process::GitExecutor;
use crate::repo::RepoInfo;
use crate::scratch::ScratchFile;

/// Argument that puts the helper into todo-editor mode. It takes the plan
/// path, and git appends the todo path.
pub const TODO_EDITOR_FLAG: &str = "--todo-editor";

/// Name of the helper executable, which ships beside the application.
pub const HELPER_NAME: &str = "pg-todo-editor";

/// Locates the helper next to the running executable.
///
/// Packaging must keep the two together; without the helper, interactive
/// rebase is the one operation that cannot work.
pub fn helper_path() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "the running executable has no directory",
        )
    })?;
    helper_in(dir)
}

/// Looks for the helper in one directory.
pub fn helper_in(dir: &Path) -> std::io::Result<PathBuf> {
    let helper = dir.join(format!("{HELPER_NAME}{}", std::env::consts::EXE_SUFFIX));
    if !helper.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{HELPER_NAME} is missing from {}", dir.display()),
        ));
    }
    Ok(helper)
}

/// What to do with one commit of the range.
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

    fn from_keyword(word: &str) -> Option<Self> {
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

/// One line of a todo file.
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

/// Renders a todo file git will consume.
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
/// and commands this application does not model are skipped.
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

/// A plan plus the range it rewrites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPlan {
    /// Revision the rebase treats as upstream; empty when `root` is set.
    pub upstream: String,
    /// The plan reaches the first commit, so the rebase needs `--root`.
    pub root: bool,
    pub steps: Vec<RebaseStep>,
}

impl EditPlan {
    /// Rebase options that replay exactly this plan's range.
    pub fn options(&self) -> RebaseOptions {
        RebaseOptions {
            root: self.root,
            ..Default::default()
        }
    }
}

/// A single-commit change to an existing history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Fold the commit into the one before it, combining the messages.
    SquashIntoParent,
    /// Replace the commit's message.
    Reword(String),
    /// Leave the commit out of the history entirely.
    Drop,
}

impl Edit {
    /// How many commits before the target the plan has to start at.
    ///
    /// `squash` folds into the line above it, so the parent must be in the
    /// plan as well; a reword only needs the commit itself.
    ///
    /// A drop takes the parent in too, for a different reason: dropping
    /// the newest commit would otherwise leave a plan whose every line is
    /// a drop, which [`rebase_interactive`] refuses. The parent rides
    /// along as an ordinary `pick` and keeps its own object name
    /// (measured — `dropping_at_either_end_of_the_history`).
    fn depth(&self) -> u32 {
        match self {
            Edit::SquashIntoParent | Edit::Drop => 2,
            Edit::Reword(_) => 1,
        }
    }
}

/// Builds the plan that applies `edit` to `oid`.
///
/// Refuses a range containing a merge: a plain interactive rebase drops
/// merge commits, so carrying on would silently flatten the history the
/// user is looking at. `--rebase-merges` is a different operation, and the
/// UI does not offer it here.
pub async fn plan_edit(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &str,
    edit: Edit,
    cancel: &CancellationToken,
) -> Result<EditPlan, GitError> {
    let fail = |message: String| GitError::UnexpectedOutput {
        command: "git rebase --interactive".to_string(),
        message,
    };

    // History shorter than the plan needs means the range starts at the
    // very first commit, which has no parent to name as upstream.
    let start = format!("{oid}~{}", edit.depth());
    let upstream: String = resolve(executor, workdir, &start, cancel)
        .await?
        .unwrap_or_default();
    let root = upstream.is_empty();

    if has_merges(executor, workdir, &upstream, root, cancel).await? {
        return Err(fail(
            "this range contains a merge commit, which a rebase would drop".to_string(),
        ));
    }

    let mut steps = plan_for_range(executor, workdir, &upstream, root, cancel).await?;
    let Some(index) = steps.iter().position(|s| s.oid == oid) else {
        return Err(fail(format!(
            "{} is not in the history of the current branch",
            short(oid)
        )));
    };
    match edit {
        Edit::SquashIntoParent => {
            if index == 0 {
                return Err(fail(format!(
                    "{} is the first commit, so it has nothing to fold into",
                    short(oid)
                )));
            }
            steps[index].action = TodoAction::Squash;
        }
        Edit::Reword(message) => {
            steps[index].action = TodoAction::Reword;
            steps[index].message = Some(message);
        }
        Edit::Drop => steps[index].action = TodoAction::Drop,
    }
    Ok(EditPlan {
        upstream,
        root,
        steps,
    })
}

fn short(oid: &str) -> &str {
    oid.get(..8).unwrap_or(oid)
}

/// Resolves a revision, returning `None` when git does not know it.
async fn resolve(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    let cmd = crate::process::GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
        .arg(format!("{rev}^{{commit}}"))
        // "there is no such commit" is the answer, not a failure: a plan
        // that reaches the very first commit asks for its parent and is
        // told there is none. Left unmarked it counts as a failed command
        // and the command log throws its panel open over a perfectly good
        // squash or drop near the root (.claude/rules/core.md).
        .answers_by_code();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Ok(None);
    }
    let text = out.stdout_utf8().trim().to_string();
    Ok((!text.is_empty()).then_some(text))
}

/// Whether the range holds any commit with more than one parent.
async fn has_merges(
    executor: &GitExecutor,
    workdir: &Path,
    upstream: &str,
    root: bool,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = crate::process::GitCommand::new()
        .cwd(workdir)
        .args(["rev-list", "--merges", "--count"])
        .arg(range_arg(upstream, root));
    let out = executor.run(cmd, cancel).await?;
    Ok(out.stdout_utf8().trim() != "0")
}

/// The commits `git rebase -i <upstream>` would offer, oldest first.
pub async fn plan_for(
    executor: &GitExecutor,
    workdir: &Path,
    upstream: &str,
    cancel: &CancellationToken,
) -> Result<Vec<RebaseStep>, GitError> {
    plan_for_range(executor, workdir, upstream, false, cancel).await
}

/// `<upstream>..HEAD`, or all of `HEAD` when the range starts at the root.
fn range_arg(upstream: &str, root: bool) -> String {
    if root {
        "HEAD".to_string()
    } else {
        format!("{upstream}..HEAD")
    }
}

async fn plan_for_range(
    executor: &GitExecutor,
    workdir: &Path,
    upstream: &str,
    root: bool,
    cancel: &CancellationToken,
) -> Result<Vec<RebaseStep>, GitError> {
    let cmd = crate::process::GitCommand::new()
        .cwd(workdir)
        .args(["log", "--reverse", "--format=%H%x00%s", "-z"])
        .arg(range_arg(upstream, root));
    let out = executor.run(cmd, cancel).await?;
    let mut steps = Vec::new();
    for record in out.stdout.split(|b| *b == 0).collect::<Vec<_>>().chunks(2) {
        let [oid, subject] = record else { continue };
        let oid = String::from_utf8_lossy(oid);
        let oid = oid.trim();
        if oid.is_empty() {
            continue;
        }
        steps.push(RebaseStep::pick(
            oid,
            String::from_utf8_lossy(subject).trim_end().to_string(),
        ));
    }
    Ok(steps)
}

/// Runs `git rebase --interactive` with `steps` as the todo list.
///
/// `helper` is the executable that understands [`TODO_EDITOR_FLAG`] —
/// normally [`HELPER_NAME`] found by [`helper_path`] beside the app.
pub async fn rebase_interactive(
    executor: &GitExecutor,
    repo: &RepoInfo,
    upstream: &str,
    steps: &[RebaseStep],
    options: &RebaseOptions,
    helper: &Path,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if steps.is_empty() {
        return Ok(());
    }
    if steps.iter().all(|s| s.action == TodoAction::Drop) {
        return Err(GitError::UnexpectedOutput {
            command: "git rebase --interactive".to_string(),
            message: "refusing a plan that drops every commit".to_string(),
        });
    }

    let io_error = |source| GitError::Io {
        command: "git rebase --interactive".to_string(),
        source,
    };

    // Message files must outlive the rebase: the `exec` lines read them
    // while git is replaying.
    let mut message_files: Vec<ScratchFile> = Vec::new();
    let mut lines: Vec<TodoLine> = Vec::new();
    for step in steps {
        lines.push(TodoLine::Command {
            action: step.action,
            oid: step.oid.clone(),
            subject: step.subject.clone(),
        });
        if step.action != TodoAction::Reword {
            continue;
        }
        let Some(message) = step.message.as_deref().filter(|m| !m.trim().is_empty()) else {
            return Err(GitError::UnexpectedOutput {
                command: "git rebase --interactive".to_string(),
                message: format!("reword of {} has no message", step.oid),
            });
        };
        let file = ScratchFile::create(&repo.git_dir, "REWORD_MSG", normalized(message).as_bytes())
            .map_err(io_error)?;
        lines.push(TodoLine::Exec {
            command: format!(
                "git commit --amend --cleanup=whitespace --file {}",
                sh_quote(&shell_path(file.path()))
            ),
        });
        message_files.push(file);
    }

    let plan = ScratchFile::create(&repo.git_dir, "rebase-todo", render_todo(&lines).as_bytes())
        .map_err(io_error)?;
    let editor = sequence_editor_command(helper, plan.path());
    let cmd = rebase_command(&repo.workdir, upstream, options, Some(&editor));
    let result = executor.run(cmd, cancel).await.map(drop);
    // Keep both sets of scratch files alive until git is done with them.
    drop(plan);
    drop(message_files);
    result
}

/// The `GIT_SEQUENCE_EDITOR` value that installs `plan` as the todo list.
///
/// git runs this through a shell, so the words are shell-quoted and the
/// path separators normalized — a Windows path full of backslashes would
/// otherwise be read as escape sequences.
pub fn sequence_editor_command(helper: &Path, plan: &Path) -> String {
    format!(
        "{} {TODO_EDITOR_FLAG} {}",
        sh_quote(&shell_path(helper)),
        sh_quote(&shell_path(plan))
    )
}

/// Todo-editor mode: replace git's todo file with the prepared plan.
pub fn apply_plan(plan_path: &Path, todo_path: &Path) -> std::io::Result<()> {
    let plan = std::fs::read(plan_path)?;
    std::fs::write(todo_path, plan)
}

/// Path as a shell sees it (git's shell on Windows takes forward slashes).
fn shell_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Single-quotes a word for `sh`.
fn sh_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// CRLF to LF with exactly one trailing newline, matching [`crate::commit`].
fn normalized(message: &str) -> String {
    let mut text = message.replace("\r\n", "\n");
    while text.ends_with('\n') {
        text.pop();
    }
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_actions_git_understands() {
        let lines = vec![
            TodoLine::Command {
                action: TodoAction::Pick,
                oid: "aaa".into(),
                subject: "first".into(),
            },
            TodoLine::Command {
                action: TodoAction::Fixup,
                oid: "bbb".into(),
                subject: "second".into(),
            },
            TodoLine::Exec {
                command: "git commit --amend".into(),
            },
        ];
        assert_eq!(
            render_todo(&lines),
            "pick aaa first\nfixup bbb second\nexec git commit --amend\n"
        );
    }

    #[test]
    fn reword_renders_as_pick_because_the_exec_carries_the_message() {
        let lines = vec![TodoLine::Command {
            action: TodoAction::Reword,
            oid: "aaa".into(),
            subject: "s".into(),
        }];
        assert_eq!(render_todo(&lines), "pick aaa s\n");
    }

    #[test]
    fn newlines_in_a_subject_cannot_forge_commands() {
        let lines = vec![TodoLine::Command {
            action: TodoAction::Pick,
            oid: "aaa".into(),
            subject: "innocent\ndrop bbb".into(),
        }];
        assert_eq!(render_todo(&lines), "pick aaa innocent drop bbb\n");
    }

    #[test]
    fn parses_git_own_todo_including_short_forms() {
        let text = "\
# This is a combination of 2 commits.
pick 1111111 first subject
f 2222222 second subject

x echo hi
drop 3333333 gone
noop-command 4444444 ignored
";
        let lines = parse_todo(text);
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines[0],
            TodoLine::Command {
                action: TodoAction::Pick,
                oid: "1111111".into(),
                subject: "first subject".into()
            }
        );
        assert_eq!(
            lines[1],
            TodoLine::Command {
                action: TodoAction::Fixup,
                oid: "2222222".into(),
                subject: "second subject".into()
            }
        );
        assert_eq!(
            lines[2],
            TodoLine::Exec {
                command: "echo hi".into()
            }
        );
        assert_eq!(
            lines[3],
            TodoLine::Command {
                action: TodoAction::Drop,
                oid: "3333333".into(),
                subject: "gone".into()
            }
        );
    }

    #[test]
    fn render_and_parse_round_trip() {
        let lines = parse_todo("pick aaa one\nsquash bbb two\n");
        assert_eq!(render_todo(&lines), "pick aaa one\nsquash bbb two\n");
    }

    #[test]
    fn editor_command_is_shell_quoted_with_forward_slashes() {
        let cmd = sequence_editor_command(
            Path::new(r"C:\Program Files\pg\platitude-gg.exe"),
            Path::new(r"C:\repo\.git\platitude\rebase-todo-1"),
        );
        assert_eq!(
            cmd,
            "'C:/Program Files/pg/platitude-gg.exe' --todo-editor \
             'C:/repo/.git/platitude/rebase-todo-1'"
        );
    }

    #[test]
    fn single_quotes_in_a_path_are_escaped() {
        assert_eq!(sh_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn the_helper_must_sit_in_the_directory_it_is_looked_for_in() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = helper_in(dir.path()).expect_err("nothing there yet");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(
            error.to_string().contains(HELPER_NAME),
            "the message names what packaging must ship: {error}"
        );

        let placed = dir
            .path()
            .join(format!("{HELPER_NAME}{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&placed, b"").expect("place helper");
        assert_eq!(helper_in(dir.path()).expect("found"), placed);
    }

    #[test]
    fn apply_plan_overwrites_the_todo_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plan = dir.path().join("plan");
        let todo = dir.path().join("todo");
        std::fs::write(&plan, "pick aaa one\n").expect("write plan");
        std::fs::write(&todo, "pick aaa one\npick bbb two\n").expect("write todo");
        apply_plan(&plan, &todo).expect("apply");
        assert_eq!(
            std::fs::read_to_string(&todo).expect("read"),
            "pick aaa one\n"
        );
    }
}
