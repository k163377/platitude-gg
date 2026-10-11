//! Running the rebase: the helper that installs the todo list, and the
//! message files the reword `exec` lines read while git replays.

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::{RebaseOptions, RebaseOutcome, rebase_command};
use crate::process::{GitExecutor, Trace};
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
    // 1 that is a refusal ([`not_a_stop`]). Traced: a commit hook's no and
    // an ssh signing program's are in git's record, not its words
    // ([`refused_by`]).
    let cmd = rebase_command(&repo.workdir, upstream, options, Some(&editor))
        .answers_by_code(1)
        .except_when(not_a_stop)
        .traced();
    let command = cmd.describe();
    let ran = executor.run_unchecked(cmd, cancel).await;
    // The todo is installed (or refused) by now; only the message files
    // may still have a reader.
    drop(plan);
    // A non-zero end is a failure for `landed` to read, as `GitExecutor::run`
    // answers it; the trace that names a refusal is read here first.
    let (result, refused) = match ran {
        Ok(out) if out.code == 0 => (Ok(()), None),
        Ok(out) => {
            let stderr = out.failure_message();
            let refused = refused_by(&stderr, &out.trace)
                .map(|kind| (kind, command.clone(), out.code, stderr.clone()));
            let failed = GitError::Failed {
                command,
                code: out.code,
                stderr,
            };
            (Err(failed), refused)
        }
        Err(error) => (Err(error), None),
    };
    let mut outcome = crate::integrate::landed(executor, &repo.workdir, result, cancel).await;
    if let Some((kind, command, code, stderr)) = refused {
        if left_standing(executor, repo, &outcome, cancel).await {
            outcome = Err(match put_back(executor, repo, cancel).await {
                Ok(()) => report::rewrite_refused(kind, command, code, stderr),
                Err(left_standing) => left_standing,
            });
        } else if outcome.is_err() {
            // Nothing found standing to put back: turned down at the first
            // checkout, or a read that could not tell, where the band shows
            // whatever stands.
            outcome = Err(report::rewrite_refused(kind, command, code, stderr));
        }
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

/// What git says when a pick's `prepare-commit-msg` hook said no, made in
/// the rebase's own process (sequencer.c, the minimum to 2.55).
const PREPARE_HOOK_FAILED: &str = "'prepare-commit-msg' hook failed";

/// Whether the rebase's exit 1 is a refusal rather than a stop, for its
/// row in the log ([`crate::process::GitCommand::except_when`]): what turns
/// a plain rebase down ([`crate::integrate::turned_down`]) and what
/// [`refused_by`] names.
fn not_a_stop(said: &str, trace: &Trace) -> bool {
    crate::integrate::turned_down(said) || refused_by(said, trace).is_some()
}

/// Which refusal a replay that ended non-zero was, for one stopped over a
/// commit turned down rather than to be settled: a plan is one press, so it
/// is put back and reported (デザイン規約 §答えの要らない報せ), where a
/// conflict or an `edit` stop is the reader's to carry on. Only what git
/// said or recorded names it. A commit hook that ended non-zero first — on
/// a pick, in git's words; on a reword's `exec` or a squash's last commit,
/// a `git commit` of its own, whose no git adds no words to
/// (builtin/commit.c), in its trace: a hook's own output lands among git's
/// words, a lock's or a signer's words with it, and git makes no more of
/// that commit after its hook said no. Then a commit git could not sign
/// ([`crate::commit::signing_failed`]), a lock it named, untracked files
/// its first checkout would overwrite. A stop git gives no cause for is
/// left as git left it, for the band to carry.
///
/// The last two stop the replay before it begins, with nothing to put back.
fn refused_by(said: &str, trace: &Trace) -> Option<report::ReportKind> {
    if said.contains(PREPARE_HOOK_FAILED) || trace.hook_failed(&crate::commit::COMMIT_HOOKS) {
        Some(report::ReportKind::RewriteCommitHook)
    } else if crate::commit::signing_failed(said, trace) {
        Some(report::ReportKind::RewriteUnsigned)
    } else if crate::report::lock_taken(said) {
        Some(report::ReportKind::RewriteLocked)
    } else if said.contains(crate::integrate::UNTRACKED_IN_THE_WAY) {
        Some(report::ReportKind::RewriteUntrackedInTheWay)
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

    /// Which refusal each set of words is — and the stops that are none.
    /// What only git's trace tells (a hook on a squash's last commit, an ssh
    /// signer) is pinned against real git in `integrate_integration::refused`.
    #[test]
    fn a_refused_replay_is_told_by_what_said_no() {
        let untraced = Trace::default();
        let kind = |said: &str| refused_by(said, &untraced);
        assert_eq!(
            kind("error: 'prepare-commit-msg' hook failed\nCould not apply 5ecd2e3... c"),
            Some(ReportKind::RewriteCommitHook)
        );
        assert_eq!(
            kind(
                "error: gpg failed to sign the data:\n[GNUPG:] NO_SECKEY\nerror: failed to write commit object"
            ),
            Some(ReportKind::RewriteUnsigned)
        );
        assert_eq!(
            kind("prep says no\nCould not apply 246838c... # c\n"),
            None,
            "a squash's last commit turned down says nothing of who: its trace does"
        );
        assert_eq!(
            kind("error: failed to write commit object\nCould not apply 246838c... c"),
            None,
            "the object store's failure says the same"
        );
        assert_eq!(
            kind(
                "error: update_ref failed for ref 'CHERRY_PICK_HEAD': cannot lock ref \
                 'CHERRY_PICK_HEAD': Unable to create 'C:/t/.git/CHERRY_PICK_HEAD.lock': File exists.\n\
                 error: could not apply 5ecd2e3... c"
            ),
            Some(ReportKind::RewriteLocked)
        );
        assert_eq!(
            kind(
                "error: The following untracked working tree files would be overwritten by \
                 checkout:\n\told.txt\nPlease move or remove them before you switch branches.\n\
                 Aborting\nerror: could not detach HEAD"
            ),
            Some(ReportKind::RewriteUntrackedInTheWay)
        );
        assert_eq!(
            kind("error: could not apply 5ecd2e3... c\nCould not apply 5ecd2e3... c"),
            None
        );
        assert!(not_a_stop(
            "Unable to create 'C:/t/.git/index.lock': File exists.",
            &untraced
        ));
        assert!(!not_a_stop(
            "error: could not apply 5ecd2e3... c\nCould not apply 5ecd2e3... c",
            &untraced
        ));
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
