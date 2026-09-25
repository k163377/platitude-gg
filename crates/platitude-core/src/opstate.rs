//! Detection of in-progress repository operations (rebase / merge /
//! cherry-pick / revert / bisect) by marker files, resolved with
//! `rev-parse --git-path` (correct for worktrees and split git dirs).

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
/// Not [`detect`]'s question: a revert that records nothing stops with
/// its remaining steps before any `REVERT_HEAD` exists
/// (`integrate_integration`).
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

/// The commits a standing merge is bringing in, one per `MERGE_HEAD` line
/// (an octopus has several).
///
/// `None` when the file offered nothing to read (gone, unreadable, no
/// parseable id) — not an empty list, which would say "the merge is over"
/// and make one failed read rebuild the graph twice. A caller that cannot
/// tell keeps what it had. What the sides are *called* is
/// [`crate::conflict::sides`].
pub async fn merge_heads(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Option<Vec<crate::oid::Oid>> {
    let heads: Vec<crate::oid::Oid> =
        crate::conflict::git_file(executor, workdir, "MERGE_HEAD", cancel)
            .await
            .lines()
            .filter_map(|line| crate::oid::Oid::from_hex_str(line.trim()).ok())
            .collect();
    (!heads.is_empty()).then_some(heads)
}

/// Detects in-progress operations without a process, for a caller holding
/// the git directory `repo::open` resolved ([`crate::repo::RepoInfo`],
/// linked worktrees included) — cheap enough for
/// `RepoSession::refresh_op_progress` to ask several times a second.
#[must_use]
pub fn detect_at(git_dir: &Path) -> OpState {
    let has = |marker: &str| git_dir.join(marker).exists();
    OpState {
        rebasing: has(MARKERS[0]) || has(MARKERS[1]),
        merging: has(MARKERS[2]),
        cherry_picking: has(MARKERS[3]),
        reverting: has(MARKERS[4]),
        bisecting: has(MARKERS[5]),
    }
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
