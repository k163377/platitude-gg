//! Running the rebase: the helper that installs the todo list, and the
//! message files the reword `exec` lines read while git replays.

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::{RebaseOptions, RebaseOutcome, rebase_command};
use crate::process::GitExecutor;
use crate::repo::RepoInfo;
use crate::report;
use crate::scratch::ScratchFile;

use super::todo::{RebaseStep, TodoAction, TodoLine, render_todo};

/// Argument that puts the helper into todo-editor mode. It takes the plan
/// path, and git appends the todo path.
pub const TODO_EDITOR_FLAG: &str = "--todo-editor";

/// Name of the helper executable, which ships beside the application.
pub const HELPER_NAME: &str = "pgg-todo-editor";

/// Scratch tag of the message files reword `exec` lines read.
const REWORD_MSG_TAG: &str = "REWORD_MSG";

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
) -> Result<RebaseOutcome, GitError> {
    if steps.is_empty() {
        return Ok(RebaseOutcome::Done);
    }
    // A plan of nothing but drops is ordinary as long as it has ground to
    // land on: git moves the branch to the upstream and says so. `--root`
    // is the one with none — it replays onto a placeholder commit git
    // makes up, and with every line dropped that placeholder is what the
    // branch is left pointing at: an empty tree with no message (measured).
    if options.root && steps.iter().all(|s| s.action == TodoAction::Drop) {
        return Err(report::drop_all_commits());
    }

    let io_error = |source| GitError::Io {
        command: "git rebase --interactive".to_string(),
        source,
    };

    // Message files must outlive the rebase: the `exec` lines read them
    // while git is replaying — and a replay that stops part-way keeps the
    // rest of the todo, so a later `--continue` reads them from another
    // process entirely. They are only dropped once the rebase is known
    // not to be standing.
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
        // No report of its own: the box a message is typed into will not
        // save an empty one (`DetailsPane.canSave`), so nothing on screen
        // can reach here to be told about. A backstop for a plan built by
        // hand, and the log is where a backstop belongs.
        let Some(message) = step.message.as_deref().filter(|m| !m.trim().is_empty()) else {
            return Err(GitError::Rejected {
                message: format!("reword of {} has no message", step.oid),
            });
        };
        let file = ScratchFile::create(
            &repo.git_dir,
            REWORD_MSG_TAG,
            normalized(message).as_bytes(),
        )
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
    // Exit 1 is this command answering rather than failing, and both
    // answers have a landing of their own on screen: "your work is in the
    // way" sends the caller round through a stash, and a replay that
    // stopped part-way raises the badge and the exit card. Only 0 and 1
    // are answers, so the 128 a name git does not know exits with still
    // reads as the failure it is (規約 §終了コードで答える問い合わせ).
    let cmd = rebase_command(&repo.workdir, upstream, options, Some(&editor)).answers_by_code(1);
    let result = executor.run(cmd, cancel).await;
    // The todo is installed (or refused) by now; only the message files
    // may still have a reader.
    drop(plan);
    let outcome = crate::integrate::landed(executor, &repo.workdir, result.map(drop), cancel).await;
    match &outcome {
        // The remaining todo still points at them; they wait for the
        // `--continue`. Whatever an abort strands is swept below, on the
        // next rebase that runs to the end — a moment when nothing can be
        // standing.
        //
        // **An error keeps them too**: `landed`'s probe can fail over a
        // rebase that is in fact standing, and files dropped there would
        // break the todo's own `exec` lines. A stray file costs the sweep
        // one more entry; a missing one breaks a `--continue`.
        Ok(RebaseOutcome::Stopped) | Err(_) => {
            for file in message_files {
                file.keep();
            }
        }
        Ok(RebaseOutcome::Done) => {
            drop(message_files);
            ScratchFile::sweep(&repo.git_dir, REWORD_MSG_TAG);
        }
        Ok(RebaseOutcome::Blocked(_)) => drop(message_files),
    }
    outcome
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

pub(crate) fn sh_quote(word: &str) -> String {
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
