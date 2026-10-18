//! `git pull`, and the pull of a branch the worktree is not on.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// `git pull --ff-only --no-rebase`: the branch the worktree is on, brought
/// up to its upstream by a fast-forward.
///
/// Where it pulls from is git's config (`branch.<name>.merge`); how to
/// reconcile is not asked, since the row is offered only where the branch
/// is behind (デザイン規約 §取り込んで合流させる). `--no-rebase` matters
/// even then: with `pull.rebase` set, git runs the rebase path's clean-tree
/// check first and turns a fast-forward down over any unstaged change
/// (`cannot pull with rebase: You have unstaged changes`, 2.55). A branch
/// that diverged since the last read is refused whole (`Not possible to
/// fast-forward`, 128) and nothing is left standing.
///
/// Every non-zero exit is a failure the log raises itself over (no
/// `answers_by_code`): 1 is a fetch that did not reach the remote, a branch
/// with no tracking information, or a tree the fast-forward would write
/// over — the last names its files, as [`crate::integrate::merge`] does,
/// and is the refusal the session carries work across
/// (`session::build::integrate_carrying`).
pub async fn pull(
    executor: &GitExecutor,
    workdir: &Path,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // `--no-autostash` for the merge underneath, as `integrate::merge`
        // pins it.
        .args(["pull", "--ff-only", "--no-rebase", "--no-autostash"])
        .timeout(timeout)
        .paced_elsewhere();
    executor.run(cmd, cancel).await.map(|_| ())
}

/// The pull of a branch the worktree is not on: `git fetch <remote>
/// <merge>:refs/heads/<branch>`, the far side's branch fetched straight
/// into it.
///
/// No `+`: git moves the branch only onto a commit that descends from it
/// and turns anything else down (`[rejected] (non-fast-forward)`). Nothing
/// is checked out to merge or rebase into, so a fast-forward is the whole
/// of a pull here (デザイン規約 §取り込んで合流させる). The remote-tracking
/// ref comes along under the remote's own refspec, as a pull's fetch
/// brings it.
///
/// Where the branch tracks is read at the moment of asking, by the one
/// reader of the branch keys ([`super::push_marks`]); a branch tracking
/// nothing by then is refused here, before git.
pub async fn fast_forward(
    executor: &GitExecutor,
    workdir: &Path,
    branch: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let marks = super::push_marks(executor, workdir, branch, cancel).await?;
    let (Some(remote), Some(merge)) = (marks.tracks, marks.merge) else {
        return Err(GitError::Rejected {
            message: format!("{branch} tracks no branch to pull from"),
        });
    };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["fetch", "--"])
        .arg(remote)
        .arg(format!("{merge}:refs/heads/{branch}"))
        .timeout(timeout)
        .paced_elsewhere();
    executor.run(cmd, cancel).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use super::pull;

    /// A pull only fast-forwards, never rebases, and pins `--no-autostash`
    /// as `integrate::merge` does: the carry is the session's. Git alone
    /// reads the flags, so the command line is where they are pinned.
    #[tokio::test]
    async fn a_pull_only_fast_forwards_and_pins_autostash_off() {
        let (exec, asked) = crate::refusing::git();
        let cancel = CancellationToken::new();
        let timeout = Duration::from_secs(1);
        pull(&exec, &crate::refusing::nowhere(), timeout, &cancel)
            .await
            .expect_err("there is no git here to pull with");
        assert_eq!(
            asked.displays(),
            ["git pull --ff-only --no-rebase --no-autostash"]
        );
    }
}
