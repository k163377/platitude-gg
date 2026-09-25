//! What the other working copies are carrying: one `status` each, on a
//! tick of their own.
//!
//! The cost is the reads: a `status -uall` per copy, already the
//! dominant term of the page's own tick
//! (ci/baseline/poll-cost-windows-x64.md). Hence a slower tick
//! (`settings::Defaults::copies_interval_secs`), a pass that drops the
//! next tick ([`RepoSession::refresh_carried`]), and the background
//! handle, which the slots serve after anything somebody waits on and
//! keep out of the click's reserve (`process::Slots`). The listing of
//! which copies there are is cheap and rides the page's tick.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::oid::Oid;
use crate::process::GitExecutor;
use crate::status::Kinds;
use crate::worktrees::WorktreeEntry;

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

/// Default seconds between passes over the other copies: a pass then runs
/// about a tenth of the time (ci/baseline/git-slots-windows-x64.md §既定値).
pub const COPIES_INTERVAL_DEFAULT_SECS: u32 = 30;

/// The shortest interval: faster spends the machine on rows nobody reads.
pub const COPIES_INTERVAL_MIN_SECS: u32 = 5;

/// The longest — an hour, the way the auto-fetch ceiling is one.
pub const COPIES_INTERVAL_MAX_SECS: u32 = 3600;

/// The interval that will run for the one asked: zero is off, anything
/// else is clamped to the range. The one place the range is applied, as
/// with `auto_fetch_minutes`: the settings screen and a hand-written
/// `settings.toml` write the same field.
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
/// Every read is asked for at once on the background handle; how many
/// run is the slots' to say (`process::Slots`). Waiting on this window's
/// own reads instead made a ring that held the opening pass on the
/// graph's spinner.
///
/// Safe to aim at somebody else's tree only because every invocation
/// carries `--no-optional-locks` / `GIT_OPTIONAL_LOCKS=0`
/// (`process::executor`): without them `status` writes that tree's index,
/// and its owner's `git add` fails on `index.lock`.
///
/// Skips this window's own copy, a bare entry and a prunable one (the
/// directory is gone).
pub(super) async fn read_all(
    executor: &GitExecutor,
    worktrees: &[WorktreeEntry],
    here: &Path,
    cancel: &CancellationToken,
) -> Vec<Carried> {
    // git prints its own separators and case: compare keys
    // (`joins::same_path_key`).
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
        // A read that panicked is one row missing.
        if let Ok(Some(wip)) = joined {
            found.push(wip);
        }
    }
    // Read by the measurement (ci/baseline/git-slots-windows-x64.md).
    tracing::info!(
        copies = mine.len(),
        carrying = found.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        slots = ?executor.slots().report(),
        "carried pass"
    );
    // In the listing's order: this list decides whether the graph is
    // walked again, and a shuffled set would cost a `git log` for nothing.
    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, wip)| wip).collect()
}

/// Whether the other copies are read, what the last pass left, and the
/// pass in flight — under one lock, so a pass begun before the copies
/// were turned off cannot land after it. With a flag beside the rows,
/// that pass put its rows back up with the tick stopped, and nothing took
/// them down.
#[derive(Default)]
pub(super) struct Copies {
    state: std::sync::Mutex<CopiesState>,
}

struct CopiesState {
    /// The settings' switch (`settings::Defaults::copies_interval_secs`
    /// above zero).
    read: bool,
    /// How many times the copies have been turned off; a pass lands only
    /// on the number it began with.
    turned: u64,
    rows: std::sync::Arc<Vec<Carried>>,
    /// The pass in flight, cancelled when the copies are turned off (a
    /// read still waiting for a slot then spawns nothing).
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

    /// Turns the copies on or off; off takes the rows down and stops the
    /// pass in flight. Says whether a row moved.
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

    pub(super) fn rows(&self) -> std::sync::Arc<Vec<Carried>> {
        std::sync::Arc::clone(&super::relock(&self.state).rows)
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
    /// Nothing landed: the session closed, or the copies were turned off,
    /// under the pass.
    Dropped,
}

impl super::RepoSession {
    /// Reads what the other copies are carrying, and asks for the rebuild
    /// if it came back different. Answers with the pass; `None` where
    /// none begins — the repository is not open, the pass before is still
    /// out, or the copies are off.
    ///
    /// Off the page's tick, which a `status` per copy is too dear for (the
    /// module doc), and apart from the worktree pass, which the write
    /// queue's settling waits on: a write here does not move what the
    /// other copies carry. One at a time — a tick that finds the last pass
    /// out is dropped.
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
        let carried = read_all(&self.exec_background, &worktrees, workdir, &ticket.cancel).await;
        // Reads stopped part-way are dropped: what they left out would
        // come down as copies with nothing to show.
        if ticket.cancel.is_cancelled() {
            return CarriedOutcome::Dropped;
        }
        match self.copies.land(ticket, carried) {
            Landing::Moved => {
                // These rows are drawn by the walk alone.
                self.refresh_log();
                CarriedOutcome::Landed { moved: true }
            }
            Landing::Same => CarriedOutcome::Landed { moved: false },
            Landing::Refused => CarriedOutcome::Dropped,
        }
    }

    /// Waits for any pass over the other copies to end — the boundary a
    /// test waits on before counting their reads or turning them off.
    pub async fn wait_for_carried_pass(&self) {
        if let Ok(permit) = self.carried_slot.acquire().await {
            drop(permit);
        }
    }

    /// Reads the file list of one other working copy, fresh, for the pane
    /// about to show it (the pass keeps only the rows' tallies). Read-only:
    /// the pane has every write control down on another copy.
    ///
    /// The last ask wins (`carried_read`): the reads are of different
    /// trees, and the screen must answer the row the reader is on.
    ///
    /// A copy whose pane is open is read twice per tick, pass and pane
    /// (ci/baseline/git-slots-windows-x64.md §ペインが立っているコピーの二重読み);
    /// whether the list can ride on the pass's answer is undecided.
    pub fn read_carried_status(self: &std::sync::Arc<Self>, path: String, name: String) {
        let s = std::sync::Arc::clone(self);
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
            s.sink
                .event(crate::session::SessionEvent::CarriedStatusLoaded { path, name, status });
        });
    }

    /// Whether the other copies are read at all — the settings' "never"
    /// (`settings::Defaults::copies_interval_secs` = 0), held here for the
    /// reads an opening and a focus fire too (`refresh_quick`). Off takes
    /// down the rows drawn and the pass in flight (`Copies`).
    pub fn set_copies_read(self: &std::sync::Arc<Self>, read: bool) {
        if self.copies.turn(read) {
            self.refresh_log();
        }
    }

    /// The set as the last pass left it, for the walk that draws it. Whole
    /// records compare: the tallies are drawn on the row, so a copy that
    /// only staged another file has moved a row.
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
