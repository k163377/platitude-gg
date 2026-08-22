//! What is standing part-way through, and the four ways out of it.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::opstate::{self, OpState};
use crate::process::{GitCommand, GitExecutor};

/// How to leave an operation that stopped part-way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Continuation {
    /// Carry on with what is staged now.
    Continue,
    /// Undo everything and return to where the operation started.
    Abort,
    /// Drop the current commit and move to the next one.
    Skip,
    /// Stop stepping but keep the tree as it is (sequencer ops only).
    Quit,
}

impl Continuation {
    fn flag(self) -> &'static str {
        match self {
            Continuation::Continue => "--continue",
            Continuation::Abort => "--abort",
            Continuation::Skip => "--skip",
            Continuation::Quit => "--quit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InProgress {
    Rebase,
    Merge,
    CherryPick,
    Revert,
}

impl InProgress {
    /// Widened from private to the module only because the file split
    /// put [`pick`](super::pick) on the other side of it; nothing outside
    /// `integrate` can see it.
    pub(super) fn command(self) -> &'static str {
        match self {
            InProgress::Rebase => "rebase",
            InProgress::Merge => "merge",
            InProgress::CherryPick => "cherry-pick",
            InProgress::Revert => "revert",
        }
    }

    fn supports(self, continuation: Continuation) -> bool {
        match self {
            // `git merge` steps through nothing, so there is nothing to
            // skip or quit.
            InProgress::Merge => {
                matches!(continuation, Continuation::Continue | Continuation::Abort)
            }
            _ => true,
        }
    }

    /// The operation an [`OpState`] describes, if any.
    ///
    /// Order matters: a rebase that stops on a conflicting pick also writes
    /// `CHERRY_PICK_HEAD`, and the rebase is what the user must continue.
    pub fn from_state(state: &OpState) -> Option<Self> {
        if state.rebasing {
            Some(InProgress::Rebase)
        } else if state.merging {
            Some(InProgress::Merge)
        } else if state.cherry_picking {
            Some(InProgress::CherryPick)
        } else if state.reverting {
            Some(InProgress::Revert)
        } else {
            None
        }
    }
}

/// Continues, aborts or skips whatever is currently in progress.
///
/// Returns `Ok(false)` when nothing is in progress, so a UI button can be a
/// no-op instead of an error.
pub async fn resolve_current(
    executor: &GitExecutor,
    workdir: &Path,
    continuation: Continuation,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let state = opstate::detect(executor, workdir, cancel).await?;
    let Some(op) = InProgress::from_state(&state) else {
        return Ok(false);
    };
    resolve(executor, workdir, op, continuation, cancel).await?;
    Ok(true)
}

pub async fn resolve(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    continuation: Continuation,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if !op.supports(continuation) {
        return Err(GitError::UnexpectedOutput {
            command: format!("git {} {}", op.command(), continuation.flag()),
            message: format!("{} does not support {}", op.command(), continuation.flag()),
        });
    }
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args([op.command(), continuation.flag()]);
    // Continuing a sequencer op would otherwise open an editor for the
    // commit message it is about to write. `git merge --continue` and
    // `git rebase --continue` accept no arguments at all — they rely on
    // GIT_EDITOR, which the process layer pins to `true`.
    if continuation == Continuation::Continue
        && matches!(op, InProgress::CherryPick | InProgress::Revert)
    {
        cmd = cmd.arg("--no-edit");
    }
    executor.run(cmd, cancel).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_supports_only_continue_and_abort() {
        assert!(InProgress::Merge.supports(Continuation::Continue));
        assert!(InProgress::Merge.supports(Continuation::Abort));
        assert!(!InProgress::Merge.supports(Continuation::Skip));
        assert!(InProgress::CherryPick.supports(Continuation::Skip));
    }

    #[test]
    fn rebase_wins_over_the_cherry_pick_head_it_leaves_behind() {
        let state = OpState {
            rebasing: true,
            cherry_picking: true,
            ..Default::default()
        };
        assert_eq!(InProgress::from_state(&state), Some(InProgress::Rebase));
        assert_eq!(InProgress::from_state(&OpState::default()), None);
    }
}
