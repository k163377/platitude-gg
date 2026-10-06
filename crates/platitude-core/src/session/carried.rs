//! What the other working copies are carrying: one `status` each, read one
//! copy at a time.
//!
//! The cost is the reads: a `status -uall` per copy, the dominant term of
//! a tree's read (ci/baseline/poll-cost-windows-x64.md). Each copy is read
//! when the page's pace says (`session::pace` — a copy that weighs more is
//! read less often, and never under this tree's read when it can be
//! helped), on the background handle, which the slots serve after anything
//! somebody waits on and keep out of the click's reserve
//! (`process::Slots`). Which copies there are rides this tree's listing.
//!
//! An opening, a reading behind the listing while nothing paces, and the
//! tests read every copy in one pass instead
//! ([`RepoSession::refresh_carried`]), one at a time all the same.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitExecutor, Meter};
use crate::status::Kinds;
use crate::worktrees::WorktreeEntry;

use super::StashRead;
use super::pace::CopiesPace;

/// One other working copy's uncommitted work, as a row draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    /// The row's chip name: the path's last segment, as the WORKTREES row
    /// shows it (`joins::shown_name`).
    pub name: crate::Name,
    /// Where that copy is, as git printed it — what the row opens, in a
    /// tab of its own (the only place that can stage and commit there).
    pub path: String,
    /// That copy's HEAD, where the row is drawn; a HEAD the walk never
    /// reached draws nothing.
    pub head: Oid,
    /// The six tallies the row shows — that copy's, not this window's.
    pub kinds: Kinds,
}

/// The interval a fixed pace starts at, before anyone sets one.
pub const COPIES_INTERVAL_DEFAULT_SECS: u32 = 30;

/// The shortest fixed interval: faster spends the machine on rows nobody
/// reads.
pub const COPIES_INTERVAL_MIN_SECS: u32 = 5;

/// The longest — an hour, the way the auto-fetch ceiling is one.
pub const COPIES_INTERVAL_MAX_SECS: u32 = 3600;

/// The fixed interval that will run for the one asked, clamped to the
/// range. The one place the range is applied, as with
/// `auto_fetch_minutes`: the settings screen and a hand-written
/// `settings.toml` write the same field.
#[must_use]
pub fn copies_interval_secs(asked: u32) -> u32 {
    asked.clamp(COPIES_INTERVAL_MIN_SECS, COPIES_INTERVAL_MAX_SECS)
}

/// One other copy as a listing named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Listed {
    /// What its path is compared on (`joins::same_path_key`).
    pub(super) key: String,
    /// As git printed it.
    pub(super) path: String,
    pub(super) head: Option<Oid>,
}

/// The copies a listing names that can be read: every one but this
/// window's own, a bare entry and a prunable one (the directory is gone).
pub(super) fn listed(worktrees: &[WorktreeEntry], here: &Path) -> Vec<Listed> {
    // git prints its own separators and case: compare keys
    // (`joins::same_path_key`).
    let here = crate::session::joins::same_path_key(&here.to_string_lossy());
    worktrees
        .iter()
        .filter(|w| !w.bare && !w.prunable)
        .map(|w| Listed {
            key: crate::session::joins::same_path_key(&w.path),
            path: w.path.clone(),
            head: w
                .head_hex
                .as_deref()
                .and_then(|hex| Oid::from_hex_str(hex.trim()).ok()),
        })
        .filter(|copy| copy.key != here)
        .collect()
}

/// Reads one copy's status: its row, or `None` where it has nothing to
/// show (or its HEAD is unknown, which draws nothing).
///
/// `pane` is the copy the read-only pane stands on, if one does: when it
/// is this copy, the `status` is the pane's file list too, and goes there
/// the moment it is read — before a clean copy is left out, a copy gone
/// clean being a list the pane has to hear.
///
/// Safe to aim at somebody else's tree only because every invocation
/// carries `--no-optional-locks` / `GIT_OPTIONAL_LOCKS=0`
/// (`process::executor`): without them `status` writes that tree's index,
/// and its owner's `git add` fails on `index.lock`.
async fn read_one(
    executor: &GitExecutor,
    copy: &Listed,
    cancel: &CancellationToken,
    pane: Option<&PaneRead>,
) -> Result<Option<Carried>, GitError> {
    let status = crate::status::load(executor, Path::new(&copy.path), cancel).await?;
    let dirty = status.is_dirty();
    // Counted once per path, because the pane the row leads to lists
    // paths (`Kinds::folded`).
    let kinds = Kinds::folded(&status);
    if let Some(pane) = pane.filter(|pane| pane.key == copy.key) {
        (pane.hand)(status);
    }
    let Some(head) = copy.head else {
        return Ok(None);
    };
    if !dirty {
        return Ok(None);
    }
    Ok(Some(Carried {
        name: crate::session::joins::shown_name(&copy.path),
        path: copy.path.clone(),
        head,
        kinds,
    }))
}

/// The pane's copy as a read of the copies meets it ([`read_one`]): which
/// copy, and where its `status` goes.
#[derive(Clone)]
pub(super) struct PaneRead {
    /// `joins::same_path_key` of the copy.
    key: String,
    hand: Arc<dyn Fn(crate::status::WorkingTreeStatus) + Send + Sync>,
}

/// Which other copy the read-only pane stands on, and which reading of it
/// the pane was last handed — what the reads of the copies and the pane's
/// own reads both answer to.
///
/// Every read of the copy takes a number when it is asked, and only one
/// numbered above the reading handed last is handed: a copy's read waits
/// behind the background queue, and landing after a read the pane asked
/// later it would put the older list back. Stepping onto a copy counts as
/// a reading of it, so nothing asked before the step is handed — a read
/// asked before the pane stepped off a copy and back included.
#[derive(Default)]
pub(super) struct Pane(std::sync::Mutex<PaneState>);

#[derive(Default)]
struct PaneState {
    asks: u64,
    standing: Option<Standing>,
}

struct Standing {
    key: String,
    path: String,
    name: String,
    /// The number of the reading handed last — before the first, the one
    /// below the read the pane stepped on with.
    handed: u64,
}

impl Pane {
    /// Stands the pane on the copy at `path` (shown as `name`), answering
    /// its key and the number of the read asked with it. Another copy
    /// starts at this read: what was asked of it before is not handed.
    fn stand(&self, path: &str, name: &str) -> (String, u64) {
        let key = crate::session::joins::same_path_key(path);
        let mut state = super::relock(&self.0);
        state.asks += 1;
        let number = state.asks;
        match &mut state.standing {
            Some(standing) if standing.key == key => {
                standing.path = path.to_string();
                standing.name = name.to_string();
            }
            standing => {
                *standing = Some(Standing {
                    key: key.clone(),
                    path: path.to_string(),
                    name: name.to_string(),
                    handed: number - 1,
                });
            }
        }
        (key, number)
    }

    /// The copy the pane stands on and a number for a copy read's reading
    /// of it; `None` while it stands on none.
    fn ask(&self) -> Option<(String, u64)> {
        let mut state = super::relock(&self.0);
        state.asks += 1;
        let number = state.asks;
        state
            .standing
            .as_ref()
            .map(|standing| (standing.key.clone(), number))
    }

    /// Takes a reading of the copy `key` asked as `number` for the pane,
    /// answering the path and name it is shown under — `None` where the
    /// pane stands elsewhere now, or holds a reading asked later.
    fn take(&self, key: &str, number: u64) -> Option<(String, String)> {
        let mut state = super::relock(&self.0);
        let standing = state.standing.as_mut().filter(|s| s.key == key)?;
        if number <= standing.handed {
            return None;
        }
        standing.handed = number;
        Some((standing.path.clone(), standing.name.clone()))
    }

    /// The key of the copy the pane stands on, if any.
    pub(super) fn standing(&self) -> Option<String> {
        super::relock(&self.0)
            .standing
            .as_ref()
            .map(|standing| standing.key.clone())
    }

    /// The pane is about this window's own tree again.
    fn leave(&self) {
        super::relock(&self.0).standing = None;
    }
}

/// Whether the other copies are read, which there are, what their reads
/// left and the read in flight — under one lock, so a read begun before
/// the copies were turned off cannot land after it. With a flag beside
/// the rows, that read put its row back up with the reading stopped, and
/// nothing took it down.
#[derive(Default)]
pub(super) struct Copies {
    state: std::sync::Mutex<CopiesState>,
}

struct CopiesState {
    /// The settings' switch (`settings::CopiesReading::Off` is off).
    read: bool,
    /// How many times the copies have been turned off; a read lands only
    /// on the number it began with.
    turned: u64,
    /// The copies the last listing named, in its order.
    listed: Arc<Vec<Listed>>,
    /// In the listing's order.
    rows: Arc<Vec<Carried>>,
    /// The read in flight, cancelled when the copies are turned off (a
    /// read still waiting for a slot then spawns nothing).
    pass: Option<CancellationToken>,
}

impl Default for CopiesState {
    fn default() -> Self {
        Self {
            read: true,
            turned: 0,
            listed: Arc::new(Vec::new()),
            rows: Arc::new(Vec::new()),
            pass: None,
        }
    }
}

/// What a read begins with: the count it lands on, and the token that
/// stops it.
pub(super) struct PassTicket {
    turned: u64,
    cancel: CancellationToken,
}

/// What a landing came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Landing {
    /// The row is as the read found it, and that moved it.
    Moved,
    /// The row is as the read found it, which is as it was.
    Same,
    /// Nothing was written: the copies were turned off since the read
    /// began.
    Refused,
}

impl Copies {
    /// Begins a read, or none while the copies are off. The token is a
    /// child of `parent`, so the session's close ends it.
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

    /// Puts one copy's row as its read found it — up, replaced, or down
    /// where it has nothing to show.
    pub(super) fn land_one(&self, ticket: &PassTicket, key: &str, row: Option<Carried>) -> Landing {
        let mut state = super::relock(&self.state);
        if state.turned != ticket.turned {
            return Landing::Refused;
        }
        let at = state
            .rows
            .iter()
            .position(|r| super::joins::same_path_key(&r.path) == key);
        let held = at.map(|at| &state.rows[at]);
        if held == row.as_ref() {
            return Landing::Same;
        }
        let order = |path: &str| {
            let key = super::joins::same_path_key(path);
            state.listed.iter().position(|c| c.key == key)
        };
        let mut rows: Vec<Carried> = state
            .rows
            .iter()
            .filter(|r| super::joins::same_path_key(&r.path) != key)
            .cloned()
            .collect();
        if let Some(row) = row {
            rows.push(row);
        }
        rows.sort_by_key(|r| order(&r.path).unwrap_or(usize::MAX));
        state.rows = Arc::new(rows);
        Landing::Moved
    }

    /// Takes the copies a listing named, dropping the rows of any it no
    /// longer names. Those rows were drawn by nothing already: the walk
    /// draws only copies the listing says stand where they did
    /// (`relay::carried_current`).
    pub(super) fn list(&self, listed: Vec<Listed>) {
        let mut state = super::relock(&self.state);
        if state.rows.iter().any(|r| {
            let key = super::joins::same_path_key(&r.path);
            !listed.iter().any(|c| c.key == key)
        }) {
            let kept: Vec<Carried> = state
                .rows
                .iter()
                .filter(|r| {
                    let key = super::joins::same_path_key(&r.path);
                    listed.iter().any(|c| c.key == key)
                })
                .cloned()
                .collect();
            state.rows = Arc::new(kept);
        }
        state.listed = Arc::new(listed);
    }

    pub(super) fn listed(&self) -> Arc<Vec<Listed>> {
        Arc::clone(&super::relock(&self.state).listed)
    }

    /// Turns the copies on or off; off takes the rows down and stops the
    /// read in flight. Says whether a row moved.
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
        state.rows = Arc::new(Vec::new());
        true
    }

    /// Whether the copies are read at all.
    pub(super) fn reads(&self) -> bool {
        super::relock(&self.state).read
    }

    pub(super) fn rows(&self) -> Arc<Vec<Carried>> {
        Arc::clone(&super::relock(&self.state).rows)
    }
}

/// A pass over the other copies, for its starter to wait on
/// ([`super::RepoSession::refresh_carried`]).
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
    /// Nothing more landed: the session closed, or the copies were turned
    /// off, under the pass.
    Dropped,
}

/// What reading one copy came to.
enum CopyRead {
    Landed(Landing),
    /// git failed or was stopped: the row stays as it was.
    Unread,
}

/// Says the pass is over to the pace, however the pass task ends.
struct PassOut(Arc<super::RepoSession>);

impl Drop for PassOut {
    fn drop(&mut self) {
        self.0.pacing.change(|rules, _| rules.set_passing(false));
    }
}

impl super::RepoSession {
    /// Reads what every other copy is carrying, one after another, and
    /// asks for the rebuild if any came back different. Answers with the
    /// pass; `None` where none begins — the repository is not open, a read
    /// of the copies is still out, or the copies are off.
    ///
    /// For an opening, a reading behind the listing while nothing paces,
    /// and the tests; a page on screen reads its copies at its pace
    /// (`session::pacer`). Each read here is reported to that pace all the
    /// same, so a copy just read is not read again at once.
    pub fn refresh_carried(self: &Arc<Self>) -> Option<CarriedPass> {
        let workdir = self.workdir()?;
        let Ok(permit) = Arc::clone(&self.carried_slot).try_acquire_owned() else {
            tracing::trace!("carried read skipped: the previous one has not finished");
            return None;
        };
        // After the permit, so at most one ticket is ever out: the pass
        // the turning-off stops is the one running.
        let ticket = self.copies.begin(&self.root_cancel)?;
        self.pacing.change(|rules, _| rules.set_passing(true));
        let out = PassOut(Arc::clone(self));
        // The pane's copy, numbered at the ask like the pane's own reads.
        let pane = self.pane_read();
        let (tell, told) = tokio::sync::oneshot::channel();
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            let _out = out;
            let outcome = s.pass_over_copies(&workdir, &ticket, pane).await;
            if tell.send(outcome).is_err() {
                tracing::trace!("nobody was waiting on the pass over the other copies");
            }
        });
        Some(CarriedPass { told })
    }

    /// Waits for any read of the other copies to end — a pass, or one
    /// copy's — the boundary a test waits on before counting their reads or
    /// turning them off.
    pub async fn wait_for_carried_pass(&self) {
        if let Ok(permit) = self.carried_slot.acquire().await {
            drop(permit);
        }
    }

    /// One pass: the listing, then each copy's `status` (the pane's copy
    /// first, handed to the pane as it lands) and its landing.
    async fn pass_over_copies(
        self: &Arc<Self>,
        workdir: &Path,
        ticket: &PassTicket,
        pane: Option<PaneRead>,
    ) -> CarriedOutcome {
        // Its own listing: one process, instead of keeping a copy of the
        // worktree pass's in step.
        let Ok(worktrees) =
            crate::worktrees::load(&self.exec_background, workdir, &ticket.cancel).await
        else {
            return CarriedOutcome::Dropped;
        };
        // Recorded before the reads, so a reading is never newer than the
        // listing it is drawn against (`RepoSession::note_copy_heads`).
        self.note_copy_heads(&worktrees, workdir);
        let listed = self.copies.listed();
        // The pane's copy first; the rows keep the listing's order
        // (`Copies::land_one`).
        let mut order: Vec<&Listed> = listed.iter().collect();
        if let Some(pane) = &pane {
            order.sort_by_key(|copy| copy.key != pane.key);
        }
        let started = std::time::Instant::now();
        let mut moved = false;
        for copy in order {
            if ticket.cancel.is_cancelled() {
                return CarriedOutcome::Dropped;
            }
            let began = self.pacing.at();
            let (read, weight) = self.read_and_land(copy, ticket, pane.as_ref()).await;
            self.pacing
                .change(|rules, _| rules.copy_ended(&copy.key, began, weight));
            match read {
                CopyRead::Landed(Landing::Refused) => return CarriedOutcome::Dropped,
                CopyRead::Landed(Landing::Moved) => moved = true,
                CopyRead::Landed(Landing::Same) | CopyRead::Unread => {}
            }
        }
        // Read by the measurement (ci/baseline/git-slots-windows-x64.md).
        tracing::info!(
            copies = listed.len(),
            carrying = self.copies.rows().len(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            slots = ?self.exec_background.slots().report(),
            "carried pass"
        );
        if ticket.cancel.is_cancelled() {
            return CarriedOutcome::Dropped;
        }
        // These rows are drawn by the walk alone; one walk for the pass.
        if moved {
            self.refresh_log();
        }
        CarriedOutcome::Landed { moved }
    }

    /// Reads one copy and lands its row, metered: what came of it, and its
    /// weight (`session::pace`) — a read that failed weighs what it spawned
    /// all the same.
    async fn read_and_land(
        self: &Arc<Self>,
        copy: &Listed,
        ticket: &PassTicket,
        pane: Option<&PaneRead>,
    ) -> (CopyRead, Option<Duration>) {
        let meter = Arc::new(Meter::default());
        let read = meter
            .over(async {
                match read_one(&self.exec_background, copy, &ticket.cancel, pane).await {
                    Ok(row) => CopyRead::Landed(self.copies.land_one(ticket, &copy.key, row)),
                    Err(_) => CopyRead::Unread,
                }
            })
            .await;
        (read, meter.weighed())
    }

    /// One copy, read because the pace said so: its row landed, and the
    /// walk that draws it run here when it moved, so the copy's weight
    /// holds what its change cost (a failed read's included, as in
    /// [`Self::read_and_land`]). `None` where no read began — the copy is
    /// not listed any more, a pass holds the copies, or they are off.
    pub(super) async fn read_paced_copy(
        self: &Arc<Self>,
        key: &str,
    ) -> Option<(String, Option<Duration>)> {
        let copy = self.copies.listed().iter().find(|c| c.key == key)?.clone();
        let permit = Arc::clone(&self.carried_slot).try_acquire_owned().ok()?;
        let ticket = self.copies.begin(&self.root_cancel)?;
        let pane = self.pane_read();
        let meter = Arc::new(Meter::default());
        meter
            .over(async {
                let read = read_one(&self.exec_background, &copy, &ticket.cancel, pane.as_ref());
                if let Ok(row) = read.await
                    && self.copies.land_one(&ticket, &copy.key, row) == Landing::Moved
                {
                    self.walk_here(StashRead::default()).await;
                }
            })
            .await;
        drop(permit);
        Some((copy.path, meter.weighed()))
    }

    /// Stands the read-only pane on one other working copy and reads its
    /// file list, fresh — for the pane about to show it, and for a window
    /// coming back. Read-only: the pane has every write control down on
    /// another copy.
    ///
    /// While it stands there, each read of that copy (its paced turn, or a
    /// pass) hands it the same list from its own `status` ([`read_one`]),
    /// so a turn reads the copy once; this read is for the moments that
    /// cannot wait for the copy's turn.
    ///
    /// The last ask wins (`carried_read` stops the read the pane stepped
    /// off), and the pane is handed only a reading of the copy it stands
    /// on, asked after the one it holds ([`Pane`]).
    pub fn read_carried_status(self: &Arc<Self>, path: String, name: String) {
        let (key, number) = self.carried_pane.stand(&path, &name);
        let s = Arc::clone(self);
        let cancel = self.carried_read.begin(&self.root_cancel);
        self.runtime.spawn(async move {
            let at = std::path::PathBuf::from(&path);
            let Ok(status) = crate::status::load(&s.executor, &at, &cancel).await else {
                return;
            };
            // Checked again: cancellation is cooperative, so a finished read
            // can arrive behind the ask that replaced it.
            if cancel.is_cancelled() {
                return;
            }
            s.hand_carried(&key, number, status);
        });
    }

    /// The pane is about this window's own tree again: no read of the
    /// copies hands it another copy's list from here on.
    pub fn leave_carried_status(&self) {
        self.carried_pane.leave();
    }

    /// The copy the pane stands on, numbered for a read asked now, and
    /// where that read hands its `status`; `None` while it stands on none.
    fn pane_read(self: &Arc<Self>) -> Option<PaneRead> {
        self.carried_pane.ask().map(|(key, number)| {
            let s = Arc::clone(self);
            let hand_key = key.clone();
            PaneRead {
                key,
                hand: Arc::new(move |status| s.hand_carried(&hand_key, number, status)),
            }
        })
    }

    /// Hands the pane a reading of the copy `key`, asked as `number`, if it
    /// still stands there and holds nothing asked later.
    fn hand_carried(&self, key: &str, number: u64, status: crate::status::WorkingTreeStatus) {
        if let Some((path, name)) = self.carried_pane.take(key, number) {
            self.sink
                .event(crate::session::SessionEvent::CarriedStatusLoaded { path, name, status });
        }
    }

    /// How the other copies are read — the settings' choice
    /// (`settings::CopiesReading`). Off holds for an opening's pass too,
    /// and takes down the rows drawn and the read in flight (`Copies`).
    pub fn set_copies_pace(self: &Arc<Self>, pace: CopiesPace) {
        let read = pace != CopiesPace::Off;
        let back_on = read && !self.copies.reads();
        if self.copies.turn(read) {
            self.refresh_log();
        }
        // Turned back on, the rows come back in one pass and one walk: read
        // copy by copy at the pace, each row would walk the history again.
        // Begun before the pace hears, so it starts no copy's read of its
        // own beside the pass.
        if back_on {
            drop(self.refresh_carried());
        }
        self.pacing.change(|rules, at| rules.set_pace(pace, at));
    }

    /// The set as the reads left it, for the walk that draws it. Whole
    /// records compare: the tallies are drawn on the row, so a copy that
    /// only staged another file has moved a row.
    pub(super) fn carried(&self) -> Arc<Vec<Carried>> {
        self.copies.rows()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixed_interval_one_number_stands_for() {
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

    fn listed_as(names: &[&str]) -> Vec<Listed> {
        names
            .iter()
            .map(|name| Listed {
                key: crate::session::joins::same_path_key(&format!("/copies/{name}")),
                path: format!("/copies/{name}"),
                head: None,
            })
            .collect()
    }

    fn row(name: &str) -> Carried {
        Carried {
            name: name.into(),
            path: format!("/copies/{name}"),
            head: Oid::from_hex_str(&"a".repeat(40)).expect("an oid"),
            kinds: Kinds::default(),
        }
    }

    fn key(name: &str) -> String {
        crate::session::joins::same_path_key(&format!("/copies/{name}"))
    }

    #[test]
    fn a_read_begun_before_the_copies_were_turned_off_lands_nothing() {
        let copies = Copies::default();
        copies.list(listed_as(&["a"]));
        let root = CancellationToken::new();
        let first = copies.begin(&root).expect("the copies are read");
        assert_eq!(
            copies.land_one(&first, &key("a"), Some(row("a"))),
            Landing::Moved
        );
        let second = copies.begin(&root).expect("still read");
        assert!(copies.turn(false), "the rows come down with the switch");
        assert!(
            second.cancel.is_cancelled(),
            "and the read in flight is stopped"
        );
        assert!(!root.is_cancelled(), "the session's own token is not");
        assert_eq!(
            copies.land_one(&second, &key("a"), Some(row("a"))),
            Landing::Refused,
            "what was read for the switch before cannot come back up"
        );
        assert!(copies.rows().is_empty());
        assert!(
            copies.begin(&root).is_none(),
            "and no read begins while the copies are off"
        );
    }

    #[test]
    fn the_pane_takes_only_the_newest_reading_of_the_copy_it_stands_on() {
        let pane = Pane::default();
        assert_eq!(
            pane.ask(),
            None,
            "standing on no copy, a read hands nothing"
        );
        let (a, first) = pane.stand("/copies/a", "a");
        let (_, pass) = pane.ask().expect("standing on a");
        let (_, again) = pane.stand("/copies/a", "a");
        assert!(pane.take(&a, again).is_some(), "the newest read is handed");
        assert_eq!(
            pane.take(&a, pass),
            None,
            "a read asked before it, landing after, would put an older list back"
        );
        assert_eq!(pane.take(&a, first), None);

        let (b, on_b) = pane.stand("/copies/b", "b");
        assert_eq!(
            pane.take(&a, pass),
            None,
            "the copy stepped off is nobody's to show"
        );
        let (_, back) = pane.stand("/copies/a", "a");
        assert_eq!(
            pane.take(&a, pass),
            None,
            "a read asked before the step back loses to the step back's read"
        );
        assert_eq!(pane.take(&b, on_b), None, "nor is b, stepped off");
        assert!(pane.take(&a, back).is_some());
        pane.leave();
        assert_eq!(
            pane.take(&a, back + 1),
            None,
            "a pane put away is handed nothing"
        );
    }

    #[test]
    fn the_copies_turned_back_on_land_a_new_read_and_still_refuse_an_old_one() {
        let copies = Copies::default();
        copies.list(listed_as(&["old", "new"]));
        let root = CancellationToken::new();
        let stale = copies.begin(&root).expect("read");
        copies.turn(false);
        assert!(!copies.turn(true), "turning on moves no row by itself");
        let fresh = copies.begin(&root).expect("read again");
        assert_eq!(
            copies.land_one(&stale, &key("old"), Some(row("old"))),
            Landing::Refused
        );
        assert_eq!(
            copies.land_one(&fresh, &key("new"), Some(row("new"))),
            Landing::Moved
        );
        assert_eq!(
            copies.land_one(&fresh, &key("new"), Some(row("new"))),
            Landing::Same
        );
        assert_eq!(copies.rows().len(), 1);
    }

    #[test]
    fn rows_land_one_by_one_in_the_listing_s_order_and_go_down_clean() {
        let copies = Copies::default();
        copies.list(listed_as(&["a", "b", "c"]));
        let root = CancellationToken::new();
        let ticket = copies.begin(&root).expect("read");
        copies.land_one(&ticket, &key("c"), Some(row("c")));
        copies.land_one(&ticket, &key("a"), Some(row("a")));
        let names: Vec<String> = copies.rows().iter().map(|r| r.name.to_string()).collect();
        assert_eq!(
            names,
            ["a", "c"],
            "in the listing's order, whatever order they land in"
        );
        assert_eq!(copies.land_one(&ticket, &key("a"), None), Landing::Moved);
        let names: Vec<String> = copies.rows().iter().map(|r| r.name.to_string()).collect();
        assert_eq!(names, ["c"], "a copy with nothing to show has no row");
        assert_eq!(copies.land_one(&ticket, &key("b"), None), Landing::Same);
    }

    #[test]
    fn a_copy_the_listing_no_longer_names_loses_its_row() {
        let copies = Copies::default();
        copies.list(listed_as(&["a", "b"]));
        let root = CancellationToken::new();
        let ticket = copies.begin(&root).expect("read");
        copies.land_one(&ticket, &key("a"), Some(row("a")));
        copies.land_one(&ticket, &key("b"), Some(row("b")));
        copies.list(listed_as(&["b"]));
        let names: Vec<String> = copies.rows().iter().map(|r| r.name.to_string()).collect();
        assert_eq!(names, ["b"]);
    }
}
