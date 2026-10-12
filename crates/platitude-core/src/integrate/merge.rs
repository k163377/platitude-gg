//! `git merge`.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::{Landing, opstate};
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

/// `git merge <rev>`.
///
/// Exit 1 is both a conflict and a name git cannot merge
/// (rules-refs/core.md「`git merge` の終了コードは 1 が両義」), so a merge
/// left standing is the stop and any other non-zero exit is the failure it
/// looks like.
pub async fn merge(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    options: &MergeOptions,
    cancel: &CancellationToken,
) -> Result<Landing, GitError> {
    // `--no-autostash` pins off `merge.autoStash`: the stash a dirty tree
    // goes round through is this end's, which puts staged work back staged
    // (`session::build::carry_across`); git's own restores with a plain
    // `apply` and parks a stopped merge's work in `MERGE_AUTOSTASH`, out
    // of the stash list.
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["merge", "--no-edit", "--no-autostash"]);
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
    // Exit 1 is as often an answer as a failure: the log keeps the row
    // without raising itself (デザイン規約 §git が言ったことを読む場所) —
    // except where git names work in the way, which is a refusal whose
    // words list the files, so the row keeps them (a fast-forward refused
    // exits 1; a real merge refused exits 2, a failure as it stands).
    let cmd = cmd
        .args(["--", rev])
        .answers_by_code(1)
        .except_when_it_says(super::REFUSED_OVER_WORK);
    match executor.run(cmd, cancel).await {
        Ok(_) => Ok(Landing::Done),
        // Nothing stands behind this one, so nothing is read.
        Err(error) if refused_over_work(&error) => Err(error),
        Err(error) if stopped_on_a_conflict(executor, workdir, &error, cancel).await => {
            Ok(Landing::Stopped)
        }
        Err(error) => Err(error),
    }
}

/// Whether git turned a merge — or a pull's — down over work in the way:
/// tracked changes or untracked files its fast-forward or merge would write
/// over (exit 1 for the fast-forward, 2 for a real merge; nothing left
/// standing either way). The refusal a stash gets past
/// (`session::build::integrate_carrying`).
#[must_use]
pub fn refused_over_work(error: &GitError) -> bool {
    matches!(error, GitError::Failed { stderr, .. } if stderr.contains(super::OVERWRITES_WORK))
}

/// The message a stopped merge is going to record, with git's own
/// comment lines taken out. Empty where there is nothing to read.
///
/// `git merge --continue` and a plain `git commit` record the identical
/// commit from this file (rules-refs/core.md「止まった merge を `git commit`
/// で終わらせたもの」), so the commit box can show it before it is written
/// (デザイン規約 §進行中の操作から出る). Comment lines go because
/// `commit::commit` uses `--cleanup=whitespace`, which keeps them; a moved
/// `core.commentChar` leaves them in the box, where they can be deleted.
/// `git_dir` is the worktree's own (`RepoInfo::git_dir`).
#[must_use]
pub fn stopped_message(git_dir: &Path) -> String {
    let raw = crate::conflict::git_file(git_dir, "MERGE_MSG");
    let kept: Vec<&str> = raw.lines().filter(|l| !l.starts_with('#')).collect();
    kept.join("\n").trim().to_string()
}

/// Whether a merge that exited non-zero left itself standing to be
/// finished.
///
/// `--squash` is not seen: it writes `SQUASH_MSG` and no `MERGE_HEAD`, so
/// its conflicts arrive as a failure (nothing in the application asks for
/// one).
///
/// A read that fails answers "no", so git's reason for not merging reaches
/// the screen instead of a `rev-parse` error.
async fn stopped_on_a_conflict(
    executor: &GitExecutor,
    workdir: &Path,
    error: &GitError,
    cancel: &CancellationToken,
) -> bool {
    // The code is read as well as the marker: a merge asked for while one
    // is already standing exits 128 with `MERGE_HEAD` there, and the marker
    // alone would call that a stop and swallow git's reason.
    if !matches!(error, GitError::Failed { code: 1, .. }) {
        return false;
    }
    opstate::detect(executor, workdir, cancel)
        .await
        .is_ok_and(|state| state.merging)
}
