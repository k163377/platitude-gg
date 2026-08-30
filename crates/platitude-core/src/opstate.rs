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
/// stands there with its remaining steps (measured 2.55,
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

/// The commits a standing merge is bringing in, as `MERGE_HEAD` lists
/// them — one id per line, so an octopus comes back with all of its
/// sides.
///
/// `None` when the file offered nothing to read: gone, unreadable, or
/// holding no id this could parse. **Not the same answer as "the merge is
/// over"**, which is what an empty list would say — the read that failed
/// once would drop the graph's dotted edges, and the next one put them
/// back, walking the whole history twice over a file that never changed.
/// A caller that cannot tell keeps what it had.
///
/// Only the ids: what the sides are *called* is [`crate::conflict::sides`],
/// and the two are wanted in different places (a name goes in a sentence,
/// an id joins the graph).
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

/// Detects in-progress operations, without a process, for a caller that
/// already knows where the git directory is.
///
/// [`detect`] spends a `rev-parse` on resolving the marker names, and
/// then does exactly this — the markers are files, and whether one exists
/// is the whole of the question. The resolving is what `repo::open`
/// already answered, linked worktrees included, so a caller holding a
/// [`crate::repo::RepoInfo`] can ask as often as it likes
/// (`RepoSession::refresh_op_progress` asks several times a second while
/// a replay is running).
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
