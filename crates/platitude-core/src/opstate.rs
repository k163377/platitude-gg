//! Detection of in-progress repository operations (rebase / merge /
//! cherry-pick / revert / bisect).
//!
//! Uses `git rev-parse --git-path <name>` to resolve marker paths (correct
//! for worktrees and split git dirs) and checks their existence.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Marker names resolved via `--git-path`, in query order.
const MARKERS: [&str; 6] = [
    "rebase-merge",
    "rebase-apply",
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "BISECT_LOG",
];

/// Which multi-step operations are currently in progress.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpState {
    pub rebasing: bool,
    pub merging: bool,
    pub cherry_picking: bool,
    pub reverting: bool,
    pub bisecting: bool,
}

impl OpState {
    pub fn any(&self) -> bool {
        self.rebasing || self.merging || self.cherry_picking || self.reverting || self.bisecting
    }
}

/// Whether a cherry-pick / revert sequence still has commits left to
/// replay.
///
/// A different question from [`detect`]: the markers there say "an
/// operation is standing here for someone to finish", while the todo
/// list says "git has more commits to get through". They usually arrive
/// together — but a revert that turns out to record nothing leaves the
/// second without the first, because git refuses the commit it was
/// about to write before any `REVERT_HEAD` exists, and the sequence
/// stands there with its remaining steps (実測 2.55,
/// `integrate_integration`).
pub async fn sequence_pending(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--git-path", "sequencer/todo"]);
    let out = executor.run(cmd, cancel).await?;
    Ok(workdir.join(out.stdout_utf8().trim_end()).exists())
}

/// Detects in-progress operations for the repository at `workdir`.
pub async fn detect(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<OpState, GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).arg("rev-parse");
    for marker in MARKERS {
        cmd = cmd.args(["--git-path", marker]);
    }
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let mut exists = [false; MARKERS.len()];
    for (i, line) in text.lines().take(MARKERS.len()).enumerate() {
        // --git-path prints paths relative to the cwd (the workdir) or
        // absolute ones; joining handles both.
        exists[i] = workdir.join(line.trim_end()).exists();
    }
    Ok(OpState {
        rebasing: exists[0] || exists[1],
        merging: exists[2],
        cherry_picking: exists[3],
        reverting: exists[4],
        bisecting: exists[5],
    })
}
