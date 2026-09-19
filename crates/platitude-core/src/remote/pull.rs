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
/// **git does the whole of it**, and it is handed no arguments at all.
/// Which way the far side is brought in — a merge, a rebase, a
/// fast-forward only — is `pull.rebase` / `pull.ff`, and where it goes is
/// what `branch.<name>.remote` and `branch.<name>.merge` say. Reading
/// those here to compose a fetch and a merge would be a second
/// implementation of the command, answering differently from the terminal
/// in the same repository (CLAUDE.md §絶対制約). **The menu's two rows
/// are the two ends of that one comparison** (デザイン規約 §取り込んで
/// 合流させる), so neither has anything to name.
///
/// **The exit code cannot sort the two landings on its own**, the same
/// way [`crate::integrate::merge`]'s cannot: a conflict, a tree whose
/// changes would be overwritten, and a branch with no tracking
/// information all exit 1 (measured, 2.55). So the answer is asked of the
/// repository — an operation left standing is the stop, and anything else
/// that exited non-zero is the failure it looks like.
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
        // Exit 1 is this command answering as often as failing — the same
        // reason the merge's row is kept without raising the panel over it
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
/// **Both halves of the command can stop**: the merge leaves
/// `MERGE_HEAD`, and the rebase `pull.rebase` asks for leaves
/// `rebase-merge` (measured, 2.55) — so the question is about the
/// repository, not about which way git was configured to integrate.
/// The refusals exit 1 with neither (a dirty tree the merge would write
/// over, a branch with no tracking information), and stay the failures
/// they read as.
///
/// A read that fails answers "no", so git's own words are what reaches
/// the screen: this is a question *about* that failure, and letting it
/// replace the answer would report a `rev-parse` where git said why it
/// would not pull.
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
