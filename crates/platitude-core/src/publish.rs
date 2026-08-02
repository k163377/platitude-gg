//! Whether commits about to be rewritten are already on a remote.
//!
//! Rewriting published history is legal and sometimes right, so this only
//! answers the question — the warning and the decision belong to the UI
//! (実装計画 §6).
//!
//! "Published" means reachable from some remote-tracking ref, which is only
//! as fresh as the last fetch. A repository with no remotes has nothing
//! published, so nothing to warn about.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// How much of a range is already on a remote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PublishState {
    /// Commits in the range.
    pub total: u32,
    /// Of those, the ones no remote-tracking ref reaches.
    pub unpublished: u32,
}

impl PublishState {
    /// Commits in the range that a remote already has.
    pub fn published(&self) -> u32 {
        self.total.saturating_sub(self.unpublished)
    }

    /// True when rewriting the range would rewrite published history.
    pub fn rewrites_published(&self) -> bool {
        self.published() > 0
    }
}

/// The range naming exactly one commit (`<rev>^!`), for an amend warning.
pub fn only(rev: &str) -> String {
    format!("{rev}^!")
}

/// Counts a revision range and the part of it no remote has.
///
/// `range` is anything `git rev-list` accepts — `origin/main..HEAD` for a
/// rebase, [`only`] for an amend.
pub async fn state_of(
    executor: &GitExecutor,
    workdir: &Path,
    range: &str,
    cancel: &CancellationToken,
) -> Result<PublishState, GitError> {
    let total = count(executor, workdir, range, false, cancel).await?;
    if total == 0 {
        return Ok(PublishState::default());
    }
    let unpublished = count(executor, workdir, range, true, cancel).await?;
    Ok(PublishState { total, unpublished })
}

async fn count(
    executor: &GitExecutor,
    workdir: &Path,
    range: &str,
    exclude_remotes: bool,
    cancel: &CancellationToken,
) -> Result<u32, GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-list", "--count", range]);
    if exclude_remotes {
        cmd = cmd.args(["--not", "--remotes"]);
    }
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    text.trim().parse().map_err(|_| GitError::UnexpectedOutput {
        command: format!("git rev-list --count {range}"),
        message: text.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_is_the_remainder() {
        let state = PublishState {
            total: 5,
            unpublished: 2,
        };
        assert_eq!(state.published(), 3);
        assert!(state.rewrites_published());
    }

    #[test]
    fn a_wholly_local_range_needs_no_warning() {
        let state = PublishState {
            total: 3,
            unpublished: 3,
        };
        assert_eq!(state.published(), 0);
        assert!(!state.rewrites_published());
        assert!(!PublishState::default().rewrites_published());
    }

    #[test]
    fn single_commit_range_uses_the_commit_only_notation() {
        assert_eq!(only("HEAD"), "HEAD^!");
    }
}
