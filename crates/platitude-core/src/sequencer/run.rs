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
        // The log alone: the box a message is typed into will not save
        // an empty one (`DetailsPane.canSave`), so nothing on screen can
        // reach here to be told about. A backstop for a plan built by
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
    // Exit 1 is this command answering, and both answers have a landing
    // of their own on screen: "your work is in the way" sends the caller
    // round through a stash, and a replay that stopped part-way raises
    // the badge and the exit card. Only 0 and 1 are answers, so the 128
    // a name git does not know exits with still reads as the failure it
    // is (規約 §終了コードで答える問い合わせ).
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

/// Todo-editor mode: write the prepared plan over git's todo file,
/// **keeping the lines git put in it** ([`super::merge_todo`]).
///
/// Answers what had to be left out, for the helper to say on stderr.
pub fn apply_plan(plan_path: &Path, todo_path: &Path) -> std::io::Result<Vec<String>> {
    let plan = std::fs::read_to_string(plan_path)?;
    let generated = std::fs::read_to_string(todo_path)?;
    let merged = super::merge_todo(&plan, &generated);
    std::fs::write(todo_path, merged.text)?;
    Ok(merged.orphaned)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;
    use crate::report::ReportKind;

    fn no_helper() -> &'static Path {
        Path::new("no-such-helper-for-this-test")
    }

    /// The one shape that has no ground to land on. `--root` replays onto
    /// a placeholder git makes up, so dropping every line leaves that
    /// placeholder behind as the branch tip: an empty tree with no message
    /// (measured). Refusing says so before anything moves.
    #[tokio::test]
    async fn a_root_plan_of_nothing_but_drops_is_refused_before_git_runs() {
        let (exec, asked) = refusing::git();
        let (repo, cancel) = (refusing::repo(), CancellationToken::new());
        let steps = [RebaseStep {
            action: TodoAction::Drop,
            ..RebaseStep::pick("aaa", "the only one")
        }];
        let options = RebaseOptions {
            root: true,
            ..Default::default()
        };
        let err = rebase_interactive(&exec, &repo, "", &steps, &options, no_helper(), &cancel)
            .await
            .expect_err("nothing would be left to point at");
        assert_eq!(
            err.report().map(|report| report.kind),
            Some(ReportKind::DropAllCommits),
            "{err}"
        );
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }

    /// A reword carries its message in the `exec` line, so a step with
    /// none — or with nothing but whitespace — is refused before the todo
    /// is written.
    #[tokio::test]
    async fn a_reword_without_a_message_is_refused_before_git_runs() {
        let (exec, asked) = refusing::git();
        let (repo, cancel) = (refusing::repo(), CancellationToken::new());
        for given in [None, Some(" \n".to_string())] {
            let steps = [RebaseStep {
                action: TodoAction::Reword,
                message: given,
                ..RebaseStep::pick("aaa", "second")
            }];
            let err = rebase_interactive(
                &exec,
                &repo,
                "HEAD~1",
                &steps,
                &RebaseOptions::default(),
                no_helper(),
                &cancel,
            )
            .await
            .expect_err("a reword needs a message");
            assert!(
                matches!(&err, GitError::Rejected { message } if message.contains("no message")),
                "{err}"
            );
        }
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }
}
