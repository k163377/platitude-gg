//! `git merge`.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::opstate;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeOptions {
    /// Always record a merge commit (`--no-ff`).
    pub no_ff: bool,
    /// Refuse anything but a fast-forward (`--ff-only`).
    pub ff_only: bool,
    /// Bring the changes in without committing (`--squash`).
    pub squash: bool,
    /// Replaces the generated merge message.
    pub message: Option<String>,
}

/// What a merge did.
///
/// Reading the two apart is the whole point: a merge that stops on a
/// conflict is where merging a branch that moved on normally ends up, and
/// calling that a failure puts a red line over an ordinary afternoon
/// (2026-08-22 ユーザー報告).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeOutcome {
    /// git took the merge to the end: a merge commit, a fast-forward, or
    /// a branch that was already in.
    Done,
    /// git stopped and left the merge standing — `MERGE_HEAD`, the
    /// markers in the tree, the conflicted rows. Nothing failed; the way
    /// on is the exit card (デザイン規約 §進行中の操作から出る).
    Stopped,
}

/// `git merge <rev>`.
///
/// **The exit code cannot tell these apart on its own.** `git merge`
/// spends 1 on both a conflict and a name it cannot merge, keeps 128 for
/// a `--ff-only` it must refuse and 2 for a tree whose changes would be
/// overwritten (実測 2.55). So the answer is asked of the repository
/// instead: a merge left standing is a stop, and anything else that
/// exited non-zero is the failure it looks like.
pub async fn merge(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    options: &MergeOptions,
    cancel: &CancellationToken,
) -> Result<MergeOutcome, GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args(["merge", "--no-edit"]);
    if options.no_ff {
        cmd = cmd.arg("--no-ff");
    }
    if options.ff_only {
        cmd = cmd.arg("--ff-only");
    }
    if options.squash {
        cmd = cmd.arg("--squash");
    }
    if let Some(message) = &options.message {
        cmd = cmd.args(["-m", message]);
    }
    // Exit 1 is this command answering as often as failing, so the log
    // keeps the row without raising itself over it; whether the operation
    // failed is settled below and reported by the write's own answer
    // (デザイン規約 §git が言ったことを読む場所).
    let cmd = cmd.args(["--", rev]).answers_by_code(1);
    match executor.run(cmd, cancel).await {
        Ok(_) => Ok(MergeOutcome::Done),
        Err(error) if stopped_on_a_conflict(executor, workdir, &error, cancel).await => {
            Ok(MergeOutcome::Stopped)
        }
        Err(error) => Err(error),
    }
}

/// The message a stopped merge is going to record, with git's own
/// comment lines taken out. Empty where there is nothing to read.
///
/// **Both ways of finishing that merge write from this same file.**
/// `git merge --continue` and a plain `git commit` record the identical
/// commit — same tree, same two parents, same message, and the same four
/// hooks (`pre-commit`, `prepare-commit-msg`, `commit-msg`,
/// `post-commit`); `post-merge` and `pre-merge-commit` belong to a merge
/// that never stopped and fire for neither. That holds even where the
/// resolution records nothing at all: git writes the empty merge commit
/// either way (実測 2.55). So the application can put this in the box
/// the commit will be made from, and the person sees the message before
/// it is written rather than after
/// (デザイン規約 §進行中の操作から出る).
///
/// The comment lines go because the box shows what will be recorded: git
/// strips them on the way through an editor, and this application
/// commits with `--cleanup=whitespace` (`commit::commit`), which would
/// not. `#` is git's own default; a repository that has moved
/// `core.commentChar` keeps its comment lines here, where they are at
/// least visible and can be deleted.
pub async fn stopped_message(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> String {
    let raw = crate::conflict::git_file(executor, workdir, "MERGE_MSG", cancel).await;
    let kept: Vec<&str> = raw.lines().filter(|l| !l.starts_with('#')).collect();
    kept.join("\n").trim().to_string()
}

/// Whether a merge that exited non-zero left itself standing to be
/// finished, rather than refusing before it began.
///
/// `--squash` is the one stop this does not see: it writes `SQUASH_MSG`
/// and no `MERGE_HEAD` (実測 2.55), so git leaves no operation to
/// continue and its conflicts arrive as the failure they read as. Nothing
/// in the application asks for one — the option is here for completeness
/// of the command, not for a screen.
///
/// A read that fails answers "no", so the merge's own words are what
/// reaches the screen: this is a question *about* that failure, and
/// letting it replace the answer would report a `rev-parse` where git
/// said why it would not merge.
async fn stopped_on_a_conflict(
    executor: &GitExecutor,
    workdir: &Path,
    error: &GitError,
    cancel: &CancellationToken,
) -> bool {
    if !matches!(error, GitError::Failed { .. }) {
        return false;
    }
    opstate::detect(executor, workdir, cancel)
        .await
        .is_ok_and(|state| state.merging)
}
