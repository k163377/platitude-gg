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

/// Locates the helper next to the running executable (packaging must keep
/// the two together).
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
    // All drops is fine onto an upstream; under `--root` git would leave the
    // branch on its made-up placeholder (empty tree, no message).
    if options.root && steps.iter().all(|s| s.action == TodoAction::Drop) {
        return Err(report::drop_all_commits());
    }

    let io_error = |source| GitError::Io {
        command: "git rebase --interactive".to_string(),
        source,
    };

    // Message files must outlive a stopped replay (a later `--continue`
    // runs the remaining `exec` lines); they go only once the rebase is
    // known not to be standing.
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
        // A backstop for the log alone: the message box will not save an
        // empty one (`MessageActionsRow.canSave`).
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
        // `--allow-empty`: a marker commit made empty on purpose is reworded
        // like any other, as git's own `reword` does (`ALLOW_EMPTY`). The
        // index is the picked commit's here, so nothing else can come of it.
        lines.push(TodoLine::Exec {
            command: format!(
                "git commit --amend --allow-empty --cleanup=whitespace --file {}",
                sh_quote(&shell_path(file.path()))
            ),
        });
        message_files.push(file);
    }

    let plan = ScratchFile::create(&repo.git_dir, "rebase-todo", render_todo(&lines).as_bytes())
        .map_err(io_error)?;
    let editor = sequence_editor_command(helper, plan.path());
    // Exit 1 answers (work in the way, or a stopped replay), each with its
    // own landing on screen; a 128 stays a failure
    // (.claude/rules/core.md「終了コードで答える問い合わせ」) — and so does a
    // 1 that is a commit turned down, which the replay is put back for.
    let cmd = rebase_command(&repo.workdir, upstream, options, Some(&editor))
        .answers_by_code(1)
        .except_when_it_says(&NOT_A_STOP);
    let result = executor.run(cmd, cancel).await;
    // The todo is installed (or refused) by now; only the message files
    // may still have a reader.
    drop(plan);
    let refused = match &result {
        Err(GitError::Failed {
            command,
            code,
            stderr,
        }) => commit_refused(stderr).map(|kind| (kind, command.clone(), *code, stderr.clone())),
        _ => None,
    };
    let mut outcome =
        crate::integrate::landed(executor, &repo.workdir, result.map(drop), cancel).await;
    if let Some((kind, command, code, stderr)) = refused
        && left_standing(executor, repo, &outcome, cancel).await
    {
        outcome = Err(match put_back(executor, repo, cancel).await {
            Ok(()) => report::rewrite_refused(kind, command, code, stderr),
            Err(left_standing) => left_standing,
        });
    }
    match &outcome {
        // Kept for the `--continue`; what an abort strands is swept on the
        // next rebase that runs to the end. An error keeps them too:
        // `landed`'s probe can fail over a rebase that is in fact standing,
        // and a missing file breaks a `--continue`.
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

/// What git says, part-way through a replay, when a commit it remade was
/// turned down — the replay then stops as over a conflict, with exit 1, or
/// dies with it standing (128: no signing key named at all, which git
/// dies over in its own process) (sequencer.c, the same words at the
/// minimum and 2.55). Signing: [`crate::commit::UNSIGNED`]. A hook:
/// `prepare-commit-msg` on a pick, or any commit hook on a reword's `exec`
/// (the one `exec` a plan writes, whose `git commit` runs them all). And
/// the `pre-rebase` hook's no, which stops nothing.
const NOT_A_STOP: [&str; 5] = [
    crate::commit::UNSIGNED[0],
    crate::commit::UNSIGNED[1],
    PREPARE_HOOK_FAILED,
    REWORD_EXEC_FAILED,
    crate::integrate::PRE_REBASE_REFUSED,
];
const PREPARE_HOOK_FAILED: &str = "'prepare-commit-msg' hook failed";
const REWORD_EXEC_FAILED: &str = "execution failed: git commit --amend";

/// Which refusal a stopped replay's words are, for one that stopped over
/// a commit turned down rather than to be settled: a plan is one press, so
/// it is put back and reported (デザイン規約 §答えの要らない報せ), where a
/// conflict or an `edit` stop is the reader's to carry on. A lock is left a
/// stop: the next `--continue` gets past one that has gone.
fn commit_refused(stderr: &str) -> Option<report::ReportKind> {
    if crate::commit::UNSIGNED
        .iter()
        .any(|said| stderr.contains(said))
    {
        Some(report::ReportKind::RewriteUnsigned)
    } else if crate::report::lock_taken(stderr) {
        None
    } else if stderr.contains(PREPARE_HOOK_FAILED) || stderr.contains(REWORD_EXEC_FAILED) {
        Some(report::ReportKind::RewriteCommitHook)
    } else {
        None
    }
}

/// Whether the replay a commit was refused in is still standing: stopped
/// (exit 1), or died with it standing — a signing key git cannot even name
/// dies (128) inside the rebase's own process when it remakes a pick,
/// where a reword's `exec` answers 1.
async fn left_standing(
    executor: &GitExecutor,
    repo: &RepoInfo,
    outcome: &Result<RebaseOutcome, GitError>,
    cancel: &CancellationToken,
) -> bool {
    match outcome {
        Ok(RebaseOutcome::Stopped) => true,
        Ok(_) => false,
        Err(_) => crate::opstate::detect(executor, &repo.workdir, cancel)
            .await
            .is_ok_and(|state| state.rebasing),
    }
}

/// Puts a replay that stopped over a refused commit back where it began.
/// A failed abort leaves the rebase standing, which the band then shows.
async fn put_back(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    crate::integrate::resolve(
        executor,
        &repo.workdir,
        crate::integrate::InProgress::Rebase,
        crate::integrate::Continuation::Abort,
        cancel,
    )
    .await
}

/// The `GIT_SEQUENCE_EDITOR` value that installs `plan` as the todo list.
///
/// git runs this through a shell, so the words are quoted and separators
/// made forward — backslashes would read as escapes.
pub fn sequence_editor_command(helper: &Path, plan: &Path) -> String {
    format!(
        "{} {TODO_EDITOR_FLAG} {}",
        sh_quote(&shell_path(helper)),
        sh_quote(&shell_path(plan))
    )
}

/// Todo-editor mode: writes the plan over git's todo file, keeping the
/// lines git put in it ([`super::merge_todo`]). Returns what had to be
/// left out, for the helper to name on stderr.
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

    /// Whitespace alone counts as no message.
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
