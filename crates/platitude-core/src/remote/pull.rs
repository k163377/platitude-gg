//! `git pull`.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::Landing;
use crate::opstate;
use crate::process::{GitCommand, GitExecutor};

/// `git pull`: the branch the working tree is on, brought in line with
/// its upstream.
///
/// Handed no arguments: git's config decides how and from where
/// (デザイン規約 §取り込んで合流させる). Composing a fetch and a merge
/// here would answer differently from a terminal (CLAUDE.md §絶対制約).
///
/// As with [`crate::integrate::merge`], exit 1 covers a conflict, a tree
/// that would be overwritten, and a branch with no tracking information,
/// so the repository is asked: an operation left standing is the stop, any
/// other non-zero exit a failure.
pub async fn pull(
    executor: &GitExecutor,
    workdir: &Path,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Landing, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["pull", "--no-edit"])
        .timeout(timeout)
        .paced_elsewhere()
        // Exit 1 answers as often as it fails
        // (デザイン規約 §git が言ったことを読む場所).
        .answers_by_code(1);
    match executor.run(cmd, cancel).await {
        Ok(_) => Ok(Landing::Done),
        Err(error) if stopped_partway(executor, workdir, &error, cancel).await => {
            Ok(Landing::Stopped)
        }
        Err(error) => Err(error),
    }
}

/// Whether a pull that exited non-zero left something standing to be
/// finished.
///
/// Either way of integrating can stop (merge leaves `MERGE_HEAD`, rebase
/// `rebase-merge`), so the question is about the repository, not the
/// configured integration.
///
/// A read that fails answers "no", so git's own words about the pull
/// reach the screen rather than a `rev-parse` error.
async fn stopped_partway(
    executor: &GitExecutor,
    workdir: &Path,
    error: &GitError,
    cancel: &CancellationToken,
) -> bool {
    if !matches!(error, GitError::Failed { code: 1, .. }) {
        return false;
    }
    opstate::detect(executor, workdir, cancel)
        .await
        .is_ok_and(|state| state.merging || state.rebasing)
}
