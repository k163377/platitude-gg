//! What the other working copies are carrying: one `status` each, on a
//! tick of their own.
//!
//! **The cost is the read, not the row.** One `status -uall` is most of a
//! second on a reference-sized tree and it is the page's own tick's
//! dominant term already (ci/baseline/poll-cost-windows-x64.md), so this
//! may not ride that tick — and a window kept open beside another copy
//! is what these rows are for, so it may not wait for somebody to click
//! the window either. Hence a slower tick of its own
//! (`Metrics.copiesIntervalMs`), a cap on how many run at once, and a
//! pass that drops the next tick rather than stacking
//! ([`RepoSession::refresh_carried`]). The listing that says which copies
//! there are is the cheap half and rides the page's tick; this is the
//! expensive one (CLAUDE.md §性能予算).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::oid::Oid;
use crate::process::GitExecutor;
use crate::status::Kinds;
use crate::worktrees::WorktreeEntry;

/// One other working copy's uncommitted work, as a row draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    /// The name the row's chip shows — the last segment of the path, the
    /// same one the WORKTREES row uses (`joins::shown_name`).
    pub name: crate::Name,
    /// The commit the row leashes down to: that copy's HEAD. The row is
    /// drawn where that commit lands, so a copy whose HEAD the walk never
    /// reached draws nothing at all.
    pub head: Oid,
    /// The six tallies the row names. **Off the row, not off the view** —
    /// the numbers belong to the copy the row is about, and the window
    /// has one set of its own beside them.
    pub kinds: Kinds,
}

/// How many of those reads run at once.
///
/// **The cap's job is to keep this window's own reads from waiting**, not
/// to get the other rows up quickly: a focus fires this window's status
/// and refs beside these, and those are what somebody is looking at.
/// **Ordering them instead is what may not be done** — letting this
/// window's status go first by waiting on its flight made a ring, and the
/// opening pass sat on the graph's loading spinner (observed). What keeps
/// the machine for the reader is this cap and the separate tick, not a
/// wait. Four is the number to start from — the wall clock behind it is
/// still to be measured (ci/baseline/perf-windows-x64.md §未取得).
const AT_ONCE: usize = 4;

/// Reads every other working copy's status, capped at [`AT_ONCE`] at a
/// time, and keeps the ones with something to show.
///
/// **What makes this safe to point at somebody else's tree is already
/// standing**: every invocation carries `--no-optional-locks` and
/// `GIT_OPTIONAL_LOCKS=0` (`process::executor`). Without them a plain
/// `status` writes the index of the tree it runs in, and while this
/// window holds that lock the person working in that copy gets
/// `fatal: Unable to create … index.lock` from their own `git add`
/// (measured). Nothing here re-states it; the read is the window's
/// ordinary one, aimed elsewhere.
///
/// Skipped without a read: this window's own copy, a bare entry (no
/// working tree to be dirty), and a prunable one (git says the directory
/// is gone, so the read could only fail).
pub(super) async fn read_all(
    executor: &GitExecutor,
    worktrees: &[WorktreeEntry],
    here: &Path,
    cancel: &CancellationToken,
) -> Vec<Carried> {
    // Spelled once: git prints its own separators and case, so telling
    // this window's copy from the rest is a comparison of keys rather
    // than of paths (`joins::same_path_key`).
    let here = crate::session::joins::same_path_key(&here.to_string_lossy());
    let mine: Vec<&WorktreeEntry> = worktrees
        .iter()
        .filter(|w| carries_a_tree(w) && crate::session::joins::same_path_key(&w.path) != here)
        .collect();
    if mine.is_empty() {
        return Vec::new();
    }
    let permits = std::sync::Arc::new(tokio::sync::Semaphore::new(AT_ONCE));
    let mut set = tokio::task::JoinSet::new();
    for (at, entry) in mine.iter().enumerate() {
        let permits = std::sync::Arc::clone(&permits);
        let executor = executor.clone();
        let cancel = cancel.clone();
        let path = std::path::PathBuf::from(&entry.path);
        let name = crate::session::joins::shown_name(&entry.path);
        let head = entry
            .head_hex
            .as_deref()
            .and_then(|hex| Oid::from_hex_str(hex.trim()).ok());
        set.spawn(async move {
            // A closed semaphore is the runtime going down; nothing to read.
            let _permit = permits.acquire_owned().await.ok()?;
            let head = head?;
            let status = crate::status::load(&executor, &path, &cancel).await.ok()?;
            if !status.is_dirty() {
                return None;
            }
            Some((
                at,
                Carried {
                    name,
                    head,
                    kinds: Kinds::of(&status),
                },
            ))
        });
    }
    let mut found: Vec<(usize, Carried)> = Vec::new();
    while let Some(joined) = set.join_next().await {
        // A read that panicked is one row missing, not a session lost.
        if let Ok(Some(wip)) = joined {
            found.push(wip);
        }
    }
    // **In the listing's order, not the order they answered.** What the
    // rows are compared against to decide whether the graph is walked
    // again is this list, and a set that merely came back shuffled would
    // spend a whole `git log` saying nothing (`note_worktree_holders`
    // sorts its own for the same reason).
    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, wip)| wip).collect()
}

impl super::RepoSession {
    /// Reads what the other copies are carrying, and asks for the rebuild
    /// if it came back different.
    ///
    /// **On a cadence of its own** (`Metrics.copiesIntervalMs`), apart
    /// from the listing that rides the page's tick: the listing is one
    /// process and nothing else, while this is a whole `status` per copy.
    /// A window kept open beside another copy is what these rows are for,
    /// so they may not wait for somebody to click it — and they may not
    /// ride a ten-second tick either
    /// (ci/baseline/poll-cost-windows-x64.md).
    ///
    /// **Off the worktree pass as well.** What waits on that pass is the
    /// write queue's settling, and a commit here may not be called done
    /// on the strength of somebody else's `status` — a write over here
    /// does not move what they are carrying.
    ///
    /// **One at a time**: on a big tree with several copies a pass can
    /// outlast the interval, and the next tick is dropped rather than
    /// queued, the way the page's own poll drops its own.
    pub fn refresh_carried(self: &std::sync::Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let Ok(permit) = std::sync::Arc::clone(&self.carried_slot).try_acquire_owned() else {
            tracing::trace!("carried read skipped: the previous one has not finished");
            return;
        };
        let s = std::sync::Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            let cancel = s.root_cancel.clone();
            // The listing again rather than the one the worktree pass
            // read: one process and 23ms of it (measured), against keeping
            // a second copy of the listing in step with that pass.
            let Ok(worktrees) = crate::worktrees::load(&s.executor, &workdir, &cancel).await else {
                return;
            };
            let carried = read_all(&s.executor, &worktrees, &workdir, &cancel).await;
            if s.note_carried(carried) {
                // The rebuild a status that moved this tree asks for:
                // these rows are drawn by the walk and by nothing else.
                s.refresh_log();
            }
        });
    }

    /// The set as the last read left it, for the walk that draws it.
    pub(super) fn carried(&self) -> std::sync::Arc<Vec<Carried>> {
        match self.carried.lock() {
            Ok(held) => std::sync::Arc::clone(&held),
            Err(poisoned) => std::sync::Arc::clone(&poisoned.into_inner()),
        }
    }

    /// Files away what the other copies are carrying, and says whether the
    /// graph has to be walked again for it.
    ///
    /// **A row stands or falls on the whole record, not on the dirt
    /// alone**: the tallies are drawn on the row, so a copy that only
    /// staged another file has moved a row the walk has to rebuild.
    pub(super) fn note_carried(&self, fresh: Vec<Carried>) -> bool {
        // A poisoned lock is taken rather than given up on, the way the
        // holders' is: what is behind it is a snapshot, and dropping it
        // would take every row off the graph for the rest of the session.
        let mut held = match self.carried.lock() {
            Ok(held) => held,
            Err(poisoned) => poisoned.into_inner(),
        };
        if ***held == fresh {
            return false;
        }
        *held = std::sync::Arc::new(fresh);
        true
    }
}

/// Whether the entry has a working tree that can be dirty at all.
fn carries_a_tree(entry: &WorktreeEntry) -> bool {
    !entry.bare && !entry.prunable
}
