//! What the other working copies are carrying: one `status` each, on a
//! tick of their own.
//!
//! **The cost is the read, not the row.** One `status -uall` is most of a
//! second on a reference-sized tree and it is the page's own tick's
//! dominant term already (ci/baseline/poll-cost-windows-x64.md), so this
//! may not ride that tick — and a window kept open beside another copy
//! is what these rows are for, so it may not wait for somebody to click
//! the window either. Hence a slower tick of its own
//! (`settings::Defaults::copies_interval_secs`), a pass that drops the
//! next tick rather than stacking ([`RepoSession::refresh_carried`]),
//! and a cap on how many run at once that is not this module's: the
//! reads go out on the session's background handle, and the slots
//! serve them after anything somebody is waiting on and keep them out
//! of the click's reserve (`process::Slots`,
//! ci/baseline/git-slots-windows-x64.md).
//! The listing that says which copies there are is the cheap half and
//! rides the page's tick; this is the expensive one (CLAUDE.md §性能予算).

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
    /// Where that copy is, as git printed it. **What the row opens**: the
    /// changes themselves are read in the copy they belong to, by opening
    /// it in a tab of its own — the same door the WORKTREES row is, and
    /// the only one that can stage and commit in the tree it is about.
    pub path: String,
    /// The commit the row leashes down to: that copy's HEAD. The row is
    /// drawn where that commit lands, so a copy whose HEAD the walk never
    /// reached draws nothing at all.
    pub head: Oid,
    /// The six tallies the row names. **Off the row, not off the view** —
    /// the numbers belong to the copy the row is about, and the window
    /// has one set of its own beside them.
    pub kinds: Kinds,
}

/// Seconds between passes over the other copies, for a fresh settings
/// file: three of the page's own ticks. Provisional until the
/// measurement in ci/baseline/git-slots-windows-x64.md has been taken.
pub const COPIES_INTERVAL_DEFAULT_SECS: u32 = 30;

/// The shortest interval offered: a `status` per copy every few seconds
/// is already a machine spent on rows nobody is reading.
pub const COPIES_INTERVAL_MIN_SECS: u32 = 5;

/// The longest — an hour, the way the auto-fetch ceiling is one.
pub const COPIES_INTERVAL_MAX_SECS: u32 = 3600;

/// The interval that will actually run, for a number a person asked
/// for. Zero is off — the rows are not read at all — and anything else
/// is held between the floor and the ceiling. The one place the range
/// is applied, for the reason `auto_fetch_minutes` is the one place its
/// ceiling is: the settings screen and a hand-written `settings.toml`
/// write the same field.
#[must_use]
pub fn copies_interval_secs(asked: u32) -> u32 {
    if asked == 0 {
        0
    } else {
        asked.clamp(COPIES_INTERVAL_MIN_SECS, COPIES_INTERVAL_MAX_SECS)
    }
}

/// Reads every other working copy's status and keeps the ones with
/// something to show.
///
/// **How many run at once is the slots' to say**, not this function's:
/// every read is asked for at once on the background handle, and the
/// slots admit as many as the half outside the click's reserve allows,
/// after whatever this window's reader is waiting on (`process::Slots`).
/// Ordering them
/// behind this window's own reads by waiting on those instead is what
/// may not be done — that made a ring, and the opening pass sat on the
/// graph's loading spinner (observed); the cap keeps the machine for the
/// reader without a wait.
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
    let started = std::time::Instant::now();
    let mut set = tokio::task::JoinSet::new();
    for (at, entry) in mine.iter().enumerate() {
        let executor = executor.clone();
        let cancel = cancel.clone();
        let path = std::path::PathBuf::from(&entry.path);
        let name = crate::session::joins::shown_name(&entry.path);
        let shown_path = entry.path.clone();
        let head = entry
            .head_hex
            .as_deref()
            .and_then(|hex| Oid::from_hex_str(hex.trim()).ok());
        set.spawn(async move {
            let head = head?;
            let status = crate::status::load(&executor, &path, &cancel).await.ok()?;
            if !status.is_dirty() {
                return None;
            }
            Some((
                at,
                Carried {
                    name,
                    path: shown_path,
                    head,
                    // Counted once per path, because the pane the row
                    // leads to lists paths (`Kinds::folded`).
                    kinds: Kinds::folded(&status),
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
    // The pass as the measurement reads it: how many copies were read,
    // and the wall clock the slots let them through in
    // (ci/baseline/git-slots-windows-x64.md).
    tracing::info!(
        copies = mine.len(),
        carrying = found.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        slots = ?executor.slots().report(),
        "carried pass"
    );
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
    /// **On a cadence of its own** (`settings::Defaults::copies_interval_secs`),
    /// apart from the listing that rides the page's tick: the listing is
    /// one process and nothing else, while this is a whole `status` per
    /// copy. A window kept open beside another copy is what these rows
    /// are for, so they may not wait for somebody to click it — and they
    /// may not ride a ten-second tick either
    /// (ci/baseline/poll-cost-windows-x64.md).
    ///
    /// **Off the worktree pass as well.** What waits on that pass is the
    /// write queue's settling, and a commit here may not be called done
    /// on the strength of somebody else's `status` — a write over here
    /// does not move what they are carrying.
    ///
    /// **One at a time**: on a big tree with several copies a pass can
    /// outlast the interval, and the next tick is dropped rather than
    /// queued, the way the page's own poll drops its own. **Nobody is
    /// waiting on any of it**, so the whole pass goes out on the
    /// background handle: served after the reader's own commands, kept
    /// out of the click's reserve, and taken back out of the queue with
    /// the session if it closes first (`process::Slots`).
    pub fn refresh_carried(self: &std::sync::Arc<Self>) {
        if !self.copies_read.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
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
            let Ok(worktrees) = crate::worktrees::load(&s.exec_background, &workdir, &cancel).await
            else {
                return;
            };
            let carried = read_all(&s.exec_background, &worktrees, &workdir, &cancel).await;
            if s.note_carried(carried) {
                // The rebuild a status that moved this tree asks for:
                // these rows are drawn by the walk and by nothing else.
                s.refresh_log();
            }
        });
    }

    /// Reads what one other working copy is holding, for the pane that is
    /// about to show it.
    ///
    /// **A read of its own, aimed at the copy somebody is looking at.**
    /// The tick above keeps the rows' tallies current for every copy;
    /// this is the file list behind one row, asked for when that row is
    /// selected — so a pane opened on a copy shows what it holds now
    /// rather than what it held at the last tick.
    ///
    /// Nothing here can write: the pane it feeds has every write control
    /// down while it is showing another copy, and the only door into that
    /// copy's own writes is its own tab.
    ///
    /// **One at a time, and the last ask wins** (`carried_read`): these
    /// are reads of different trees, so two of them are not a race the
    /// clock settles fairly — the answer about the row the reader has
    /// stepped off must not be the one left on screen.
    ///
    /// **Outside [`AT_ONCE`]**, so a tick where a copy's pane is open
    /// stands one read more than the cap: the copy being read pays twice,
    /// once for its row's tallies and once for the pane's file list.
    /// Whether the two can be one read is part of the measurement the cap
    /// itself is waiting on (ci/baseline/perf-windows-x64.md §未取得).
    pub fn read_carried_status(self: &std::sync::Arc<Self>, path: String, name: String) {
        let s = std::sync::Arc::clone(self);
        let cancel = self.carried_read.begin(&self.root_cancel);
        self.runtime.spawn(async move {
            let at = std::path::PathBuf::from(&path);
            let Ok(status) = crate::status::load(&s.executor, &at, &cancel).await else {
                return;
            };
            // Asked again after the read: cancelling is cooperative, so a
            // read that had already worked its answer out can arrive here
            // behind the ask that passed it.
            if cancel.is_cancelled() {
                return;
            }
            s.sink
                .event(crate::session::SessionEvent::CarriedStatusLoaded { path, name, status });
        });
    }

    /// Whether the other copies are read at all — the settings' "never"
    /// (`settings::Defaults::copies_interval_secs` = 0), which the
    /// page's tick honours on its own and which this makes hold for the
    /// reads an opening and a focus fire as well (`refresh_quick`).
    /// Turned off, the rows already drawn come down with it: what they
    /// said is a reading nobody will take again.
    pub fn set_copies_read(self: &std::sync::Arc<Self>, read: bool) {
        self.copies_read
            .store(read, std::sync::atomic::Ordering::SeqCst);
        if !read && self.note_carried(Vec::new()) {
            self.refresh_log();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interval_one_number_stands_for() {
        assert_eq!(copies_interval_secs(0), 0, "zero is off");
        assert_eq!(
            copies_interval_secs(1),
            COPIES_INTERVAL_MIN_SECS,
            "under the floor is the floor"
        );
        assert_eq!(copies_interval_secs(30), 30);
        assert_eq!(
            copies_interval_secs(COPIES_INTERVAL_MAX_SECS + 1),
            COPIES_INTERVAL_MAX_SECS,
            "past the ceiling is the ceiling"
        );
    }
}
