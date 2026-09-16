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

/// Whether the other copies are read at all, what the last pass left,
/// and the pass in flight — **one owner under one lock**, so a pass that
/// began before the copies were turned off cannot land after it: the
/// switch and the rows move together, and a landing is checked against
/// the switch's count in the same breath as it is written. Held apart
/// (a flag beside the rows), the pass that was out when the switch went
/// off landed its reading afterwards, and with the tick stopped nothing
/// ever took those rows down again.
#[derive(Default)]
pub(super) struct Copies {
    state: std::sync::Mutex<CopiesState>,
}

struct CopiesState {
    /// The settings' switch (`settings::Defaults::copies_interval_secs`
    /// above zero).
    read: bool,
    /// How many times the copies have been turned off. A pass takes the
    /// number as it begins and lands on the same number only — rows read
    /// for a switch since turned are a reading nobody asked for, and
    /// would put back up what the turning took down.
    turned: u64,
    rows: std::sync::Arc<Vec<Carried>>,
    /// The pass in flight, stopped when the copies are turned off: a
    /// read still waiting for a slot then spawns nothing, and one
    /// running is not left to answer a question nobody is asking.
    pass: Option<CancellationToken>,
}

impl Default for CopiesState {
    fn default() -> Self {
        Self {
            read: true,
            turned: 0,
            rows: std::sync::Arc::new(Vec::new()),
            pass: None,
        }
    }
}

/// What a pass begins with: the count it lands on, and the token that
/// stops it.
pub(super) struct PassTicket {
    turned: u64,
    cancel: CancellationToken,
}

/// What a landing came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Landing {
    /// The rows are as the pass read them, and that moved one.
    Moved,
    /// The rows are as the pass read them, which is as they were.
    Same,
    /// Nothing was written: the copies were turned off since the pass
    /// began.
    Refused,
}

impl Copies {
    /// Begins a pass, or none while the copies are off. The token is a
    /// child of `parent`, so the session's close ends the pass the way
    /// it ends everything else.
    pub(super) fn begin(&self, parent: &CancellationToken) -> Option<PassTicket> {
        let mut state = super::relock(&self.state);
        if !state.read {
            return None;
        }
        let cancel = parent.child_token();
        state.pass = Some(cancel.clone());
        Some(PassTicket {
            turned: state.turned,
            cancel,
        })
    }

    /// Lands what a pass read, and says what that came to.
    pub(super) fn land(&self, ticket: &PassTicket, fresh: Vec<Carried>) -> Landing {
        let mut state = super::relock(&self.state);
        if state.turned != ticket.turned {
            return Landing::Refused;
        }
        if *state.rows == fresh {
            return Landing::Same;
        }
        state.rows = std::sync::Arc::new(fresh);
        Landing::Moved
    }

    /// Turns the copies on or off. Off takes the rows down — what they
    /// said is a reading nobody will take again — and stops the pass in
    /// flight. Says whether a row moved.
    pub(super) fn turn(&self, read: bool) -> bool {
        let mut state = super::relock(&self.state);
        state.read = read;
        if read {
            return false;
        }
        state.turned += 1;
        if let Some(pass) = state.pass.take() {
            pass.cancel();
        }
        if state.rows.is_empty() {
            return false;
        }
        state.rows = std::sync::Arc::new(Vec::new());
        true
    }

    /// The rows as the last landing left them, for the walk that draws
    /// them.
    pub(super) fn rows(&self) -> std::sync::Arc<Vec<Carried>> {
        std::sync::Arc::clone(&super::relock(&self.state).rows)
    }
}

/// A pass over the other copies, for whoever started it to wait on
/// ([`super::RepoSession::refresh_carried`]): the reads are the
/// session's, and this is where they are over.
#[derive(Debug)]
pub struct CarriedPass {
    told: tokio::sync::oneshot::Receiver<CarriedOutcome>,
}

impl CarriedPass {
    /// Waits for the pass to end, and says how.
    pub async fn outcome(self) -> CarriedOutcome {
        self.told.await.unwrap_or(CarriedOutcome::Dropped)
    }
}

/// How a pass over the other copies ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarriedOutcome {
    /// The rows are as the pass read them; `moved` says whether that
    /// changed one, and asked for the walk again.
    Landed { moved: bool },
    /// Nothing landed: the reads were stopped — by the session's close,
    /// or by the copies being turned off under the pass — or the copies
    /// were turned off between the reads and the landing.
    Dropped,
}

impl super::RepoSession {
    /// Reads what the other copies are carrying, and asks for the rebuild
    /// if it came back different. Answers with the pass, for a caller
    /// that waits on it; `None` where none begins — the repository is
    /// not open, the pass before is still out, or the copies are off.
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
    pub fn refresh_carried(self: &std::sync::Arc<Self>) -> Option<CarriedPass> {
        let workdir = self.workdir()?;
        let Ok(permit) = std::sync::Arc::clone(&self.carried_slot).try_acquire_owned() else {
            tracing::trace!("carried read skipped: the previous one has not finished");
            return None;
        };
        // After the permit, so at most one ticket is ever out: the pass
        // the turning-off stops is the one running.
        let ticket = self.copies.begin(&self.root_cancel)?;
        let (tell, told) = tokio::sync::oneshot::channel();
        let s = std::sync::Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            let outcome = s.pass_over_copies(&workdir, &ticket).await;
            if tell.send(outcome).is_err() {
                tracing::trace!("nobody was waiting on the pass over the other copies");
            }
        });
        Some(CarriedPass { told })
    }

    /// One pass: the listing, a `status` per copy, and the landing.
    async fn pass_over_copies(
        self: &std::sync::Arc<Self>,
        workdir: &Path,
        ticket: &PassTicket,
    ) -> CarriedOutcome {
        // The listing again rather than the one the worktree pass read:
        // one process, against keeping a second copy of the listing in
        // step with that pass.
        let Ok(worktrees) =
            crate::worktrees::load(&self.exec_background, workdir, &ticket.cancel).await
        else {
            return CarriedOutcome::Dropped;
        };
        // Where the listing above found each copy, into the record the
        // rows are drawn against. **Before the reads rather than after**:
        // it describes the same moment they were started from, and a
        // reading is only ever behind a listing taken after it
        // (`RepoSession::note_copy_heads`).
        self.note_copy_heads(&worktrees, workdir);
        let carried = read_all(&self.exec_background, &worktrees, workdir, &ticket.cancel).await;
        // Reads stopped part-way are not a reading: what they left out
        // would come down as copies with nothing to show.
        if ticket.cancel.is_cancelled() {
            return CarriedOutcome::Dropped;
        }
        match self.copies.land(ticket, carried) {
            Landing::Moved => {
                // The rebuild a status that moved this tree asks for:
                // these rows are drawn by the walk and by nothing else.
                self.refresh_log();
                CarriedOutcome::Landed { moved: true }
            }
            Landing::Same => CarriedOutcome::Landed { moved: false },
            Landing::Refused => CarriedOutcome::Dropped,
        }
    }

    /// Waits until no pass over the other copies is in flight — the
    /// boundary a test closes on before it counts their reads or turns
    /// them off, the way `wait_for_snapshot_reads` closes the opening's.
    pub async fn wait_for_carried_pass(&self) {
        if let Ok(permit) = self.carried_slot.acquire().await {
            drop(permit);
        }
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
    /// **One read more than the tick's**, so a copy whose pane is open
    /// pays twice: once for its row's tallies and once for the pane's
    /// file list — each through the execution slots like every other git
    /// (`process::Slots`), with nothing of its own to cap the pair. What
    /// the second read costs is measured — one whole `status` of that
    /// tree (ci/baseline/git-slots-windows-x64.md, the section on this
    /// pair); whether the list can ride on the pass's own answer instead
    /// is a decision nobody has taken.
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
    /// Turned off, the rows already drawn come down with it — what they
    /// said is a reading nobody will take again — and so does the pass
    /// in flight, whose reading would put them back up (`Copies`).
    pub fn set_copies_read(self: &std::sync::Arc<Self>, read: bool) {
        if self.copies.turn(read) {
            self.refresh_log();
        }
    }

    /// The set as the last pass left it, for the walk that draws it.
    ///
    /// **A row stands or falls on the whole record, not on the dirt
    /// alone**: the tallies are drawn on the row, so a copy that only
    /// staged another file has moved a row the walk has to rebuild.
    pub(super) fn carried(&self) -> std::sync::Arc<Vec<Carried>> {
        self.copies.rows()
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

    fn row(name: &str) -> Carried {
        Carried {
            name: name.into(),
            path: format!("/copies/{name}"),
            head: Oid::from_hex_str(&"a".repeat(40)).expect("an oid"),
            kinds: Kinds::default(),
        }
    }

    // Turned off under a pass in flight, the copies stop it and refuse
    // what it read: the rows the turning took down do not come back up.
    #[test]
    fn a_pass_begun_before_the_copies_were_turned_off_lands_nothing() {
        let copies = Copies::default();
        let root = CancellationToken::new();
        let first = copies.begin(&root).expect("the copies are read");
        assert_eq!(copies.land(&first, vec![row("a")]), Landing::Moved);
        let second = copies.begin(&root).expect("still read");
        assert!(copies.turn(false), "the rows come down with the switch");
        assert!(
            second.cancel.is_cancelled(),
            "and the pass in flight is stopped"
        );
        assert!(!root.is_cancelled(), "the session's own token is not");
        assert_eq!(
            copies.land(&second, vec![row("a")]),
            Landing::Refused,
            "what was read for the switch before cannot come back up"
        );
        assert!(copies.rows().is_empty());
        assert!(
            copies.begin(&root).is_none(),
            "and no pass begins while the copies are off"
        );
    }

    // Turned back on, the copies read again on a count of their own: a
    // pass from before the turning is still refused, a new one lands.
    #[test]
    fn the_copies_turned_back_on_land_a_new_pass_and_still_refuse_an_old_one() {
        let copies = Copies::default();
        let root = CancellationToken::new();
        let stale = copies.begin(&root).expect("read");
        copies.turn(false);
        assert!(!copies.turn(true), "turning on moves no row by itself");
        let fresh = copies.begin(&root).expect("read again");
        assert_eq!(copies.land(&stale, vec![row("old")]), Landing::Refused);
        assert_eq!(copies.land(&fresh, vec![row("new")]), Landing::Moved);
        assert_eq!(copies.land(&fresh, vec![row("new")]), Landing::Same);
        assert_eq!(copies.rows().len(), 1);
    }
}
