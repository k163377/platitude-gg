//! What an ask is, and the gate each kind goes through.
//!
//! An ask is one reason to spawn git. Every ask is of one kind; **the kind
//! decides the gate** — what a second ask does while one is out — and
//! **who waits decides the priority**: somebody (a click, the front page)
//! runs on `executor` / `exec_user`, nobody on `exec_background`
//! (`process::Slots`; a command paced elsewhere is a separate axis).
//!
//! | kind | answers | gate | a second ask while one is out | on failure |
//! |---|---|---|---|---|
//! | run | a change somebody asked for | the write queue (`session::write`) | queues; an unasked fetch yields to the writes behind it | git's words |
//! | follow, folded | the repository as it stands, one answer for every asker | [`Coalesce`]; `ReadFlight` where the askers wait | one more run after the one under way, on the latest inputs | read again at the next chance |
//! | follow, taken over | the same, by a long pass whose partial answer may go | [`Takeover`] | the newer stops the older, [`TAKEOVER_STREAK`] times in a row at most | read again at the next chance |
//! | latest | what the screen is looking at | `Latest`, `DiffEpoch` | the newer stops the older | answered too where the screen shows a wait |
//! | once | a fact read out of the configuration | `Derived`; the merge-tool candidates' slot, whose second ask the first's answer serves | shares the read under way | not kept |
//! | chance | what a clock fires | [`Coalesce::try_turn`]; a bare `try_acquire` (the auto fetch, the remote tags' catch-up) | not fired: the next tick does it | the next tick |
//!
//! **A follow ask is never dropped**: intermediate states may be skipped,
//! the last one is always read. Only a chance ask may be dropped, and only
//! where another tick is sure to come.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use super::relock;

/// A follow read nobody awaits: one run at a time, and an ask that finds
/// one running is owed to it — the run takes it as it ends
/// ([`Coalesce::end`]) and reads once more on the latest inputs. A burst
/// of asks costs at most one run after the one under way.
pub(super) struct Coalesce {
    slot: Arc<tokio::sync::Semaphore>,
    owed: AtomicBool,
    /// Holds waiting for the turn ([`Coalesce::hold`]).
    holding: AtomicUsize,
}

/// One hold waiting, counted for as long as its future lives — a hold
/// dropped while it waits stops counting.
struct Waiting<'a>(&'a AtomicUsize);

impl<'a> Waiting<'a> {
    fn on(count: &'a AtomicUsize) -> Self {
        count.fetch_add(1, Ordering::SeqCst);
        Self(count)
    }
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The right to run, held from before the read looks until its answer is
/// out — so an older answer cannot land after a newer one.
#[must_use = "dropping a turn ends the run without taking what it is owed"]
pub(super) struct Turn {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Default for Coalesce {
    fn default() -> Self {
        Self {
            slot: Arc::new(tokio::sync::Semaphore::new(1)),
            owed: AtomicBool::new(false),
            holding: AtomicUsize::new(0),
        }
    }
}

impl Coalesce {
    /// A turn now, or the ask owed to the run under way.
    pub(super) fn ask(&self) -> Option<Turn> {
        self.owe();
        self.try_turn()
    }

    /// Owes a run without taking a turn: the run under way takes it as it
    /// ends, or whoever next looks at [`Self::owed`].
    pub(super) fn owe(&self) {
        self.owed.store(true, Ordering::SeqCst);
    }

    /// A turn only where nothing runs, owing nothing otherwise — for a
    /// chance ask, which the next tick stands in for.
    pub(super) fn try_turn(&self) -> Option<Turn> {
        let permit = Arc::clone(&self.slot).try_acquire_owned().ok()?;
        Some(Turn { _permit: permit })
    }

    /// Takes into the run that is starting what was owed so far: true
    /// where an ask came in since the last run started. Called before the
    /// run looks, so an ask after that is owed to the next one.
    pub(super) fn take_owed(&self) -> bool {
        self.owed.swap(false, Ordering::SeqCst)
    }

    /// Whether a run is owed.
    pub(super) fn owed(&self) -> bool {
        self.owed.load(Ordering::SeqCst)
    }

    /// Whether a run is under way.
    pub(super) fn busy(&self) -> bool {
        self.slot.available_permits() == 0
    }

    /// Waits out the run under way and keeps the next one out until the
    /// turn is ended — for a write the read must not stand beside. It goes
    /// before the runs owed meanwhile ([`Self::end`]); the caller ends the
    /// turn, which starts them.
    pub(super) async fn hold(&self) -> Option<Turn> {
        let _waiting = Waiting::on(&self.holding);
        let permit = Arc::clone(&self.slot).acquire_owned().await.ok()?;
        Some(Turn { _permit: permit })
    }

    /// Ends a run: the next turn where an ask came in meanwhile, which the
    /// caller runs; `None` where nothing is owed, or where another caller
    /// took the turn first (that caller's run takes the ask).
    ///
    /// An owed ask keeps the turn as it is, so the gate never reads idle
    /// between the two runs (`busy`) — unless a hold waits: the turn goes
    /// to it, or a stream of asks would keep its write out for good. One
    /// landing between the look and the drop is taken by the look after
    /// it.
    pub(super) fn end(&self, turn: Turn) -> Option<Turn> {
        if self.owed() && self.holding.load(Ordering::SeqCst) == 0 {
            return Some(turn);
        }
        drop(turn);
        if self.owed() { self.try_turn() } else { None }
    }
}

/// Running passes a newer ask may stop in a row before it waits for the
/// one under way to land instead. Two: a stream of asks (refs moving on
/// every tick of a large repository on a slow machine) still lands one
/// pass in three, and an ask once the passes have settled — a Reload, a
/// toggle — stops the pass under way at once. One just after a burst (an
/// opening asks for several) may wait for the pass that burst left.
pub(super) const TAKEOVER_STREAK: u32 = 2;

/// A follow read by a long pass whose partial answer may go (the graph):
/// the newer ask stops the older, as long as that does not leave the
/// screen with nothing — after [`TAKEOVER_STREAK`] running passes stopped
/// in a row, the next waits for the one under way to end on its own.
///
/// A pass not started yet is stopped freely: it has cost nothing.
#[derive(Default)]
pub(super) struct Takeover {
    state: Mutex<TakeoverState>,
    /// Woken when a running pass ends.
    ended: tokio::sync::Notify,
}

#[derive(Default)]
struct TakeoverState {
    /// Numbers the passes, so they can be told apart.
    taken: u64,
    /// The latest ask's pass: the one the screen is waiting on.
    owner: Option<Pass>,
    /// The pass under way.
    running: Option<Pass>,
    /// Running passes stopped since one last ended on its own.
    streak: u32,
}

#[derive(Clone)]
struct Pass {
    number: u64,
    token: CancellationToken,
}

/// One pass's place: started by [`Lap::start`].
pub(super) struct Lap {
    gate: Arc<Takeover>,
    pass: Pass,
    /// Waits for the pass under way to end before starting.
    behind: bool,
}

/// A started pass; dropping it ends it.
pub(super) struct Running {
    gate: Arc<Takeover>,
    pass: Pass,
}

impl Takeover {
    /// Takes the screen over for a new pass: the place it starts from,
    /// which carries its token ([`Lap::token`]). `with` runs under the
    /// lock, so whatever it numbers follows the order the passes were
    /// taken in.
    pub(super) fn take<R>(
        self: &Arc<Self>,
        parent: &CancellationToken,
        with: impl FnOnce() -> R,
    ) -> (Lap, R) {
        let (pass, numbered, displaced, behind) = {
            let mut state = relock(&self.state);
            let numbered = with();
            state.taken += 1;
            let pass = Pass {
                number: state.taken,
                token: parent.child_token(),
            };
            let previous = state.owner.replace(pass.clone());
            let live = state
                .running
                .as_ref()
                .filter(|run| !run.token.is_cancelled())
                .map(|run| run.number);
            let (displaced, behind) = match (previous, live) {
                // The screen's pass is under way: stopped, unless that
                // would make one stopped too many in a row.
                (Some(prev), Some(run)) if prev.number == run => {
                    if state.streak >= TAKEOVER_STREAK {
                        (None, true)
                    } else {
                        state.streak += 1;
                        (Some(prev.token), false)
                    }
                }
                // A pass left to land is under way, and the previous ask
                // was waiting behind it: this one takes its place there.
                (previous, Some(_)) => (previous.map(|prev| prev.token), true),
                (previous, None) => (previous.map(|prev| prev.token), false),
            };
            (pass, numbered, displaced, behind)
        };
        // Outside the lock: it wakes somebody else's task.
        if let Some(prev) = displaced {
            prev.cancel();
        }
        let lap = Lap {
            gate: Arc::clone(self),
            pass,
            behind,
        };
        (lap, numbered)
    }
}

impl Lap {
    /// The pass's token: cancelled when a newer ask stops it, or the
    /// session closes.
    pub(super) fn token(&self) -> CancellationToken {
        self.pass.token.clone()
    }

    /// Starts the pass: at once, or once the pass it was left behind has
    /// ended. `None` where a newer ask stopped this one first.
    pub(super) async fn start(self) -> Option<Running> {
        loop {
            let ended = self.gate.ended.notified();
            {
                let mut state = relock(&self.gate.state);
                if self.pass.token.is_cancelled() {
                    return None;
                }
                let blocked = self.behind
                    && state
                        .running
                        .as_ref()
                        .is_some_and(|run| !run.token.is_cancelled());
                if !blocked {
                    state.running = Some(self.pass.clone());
                    return Some(Running {
                        gate: Arc::clone(&self.gate),
                        pass: self.pass.clone(),
                    });
                }
            }
            tokio::select! {
                () = ended => {}
                () = self.pass.token.cancelled() => return None,
            }
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let mut state = relock(&self.gate.state);
        if state
            .running
            .as_ref()
            .is_some_and(|run| run.number == self.pass.number)
        {
            state.running = None;
        }
        // Ended on its own: the screen has an answer, so asks may stop
        // passes again.
        if !self.pass.token.is_cancelled() {
            state.streak = 0;
        }
        drop(state);
        self.gate.ended.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wait::{bounded, poll_once};

    #[test]
    fn an_ask_while_a_run_is_out_is_owed_to_it_and_folded() {
        let gate = Coalesce::default();
        let turn = gate.ask().expect("nothing runs");
        assert!(gate.take_owed(), "the run takes the ask that started it");
        assert!(gate.ask().is_none(), "a run is under way");
        assert!(
            gate.ask().is_none(),
            "and a third ask folds into the same debt"
        );
        let again = gate.end(turn).expect("the run owes one more");
        assert!(gate.busy(), "kept through to the next run");
        assert!(gate.take_owed());
        assert!(gate.end(again).is_none(), "nothing came in meanwhile");
        assert!(!gate.busy());
    }

    /// A hold goes before the runs owed beside it — a write behind a
    /// stream of asks would never run — and ending its turn starts them.
    #[test]
    fn a_waiting_hold_goes_before_the_runs_owed() {
        let gate = Coalesce::default();
        let turn = gate.ask().expect("nothing runs");
        assert!(gate.take_owed());
        let mut held = Box::pin(gate.hold());
        assert!(
            poll_once(&mut held).is_pending(),
            "behind the run under way"
        );
        assert!(gate.ask().is_none(), "owed to the run under way");
        assert!(gate.end(turn).is_none(), "the turn goes to the hold");
        let std::task::Poll::Ready(Some(write)) = poll_once(&mut held) else {
            panic!("the hold has the turn");
        };
        assert!(gate.ask().is_none(), "the reads stay out while it holds");
        assert!(gate.end(write).is_some(), "and run as it ends");
    }

    #[test]
    fn a_chance_ask_owes_nothing() {
        let gate = Coalesce::default();
        let turn = gate.try_turn().expect("nothing runs");
        assert!(gate.try_turn().is_none());
        assert!(gate.end(turn).is_none(), "the dropped tick is not owed");
    }

    /// The floor holds while the stopped passes wind down beside the one
    /// left to land: their ends are not that pass landing.
    #[tokio::test]
    async fn a_stopped_pass_winding_down_does_not_lift_the_floor() {
        let root = CancellationToken::new();
        let gate = Arc::new(Takeover::default());
        let mut stopped = Vec::new();
        for _ in 0..TAKEOVER_STREAK {
            let (_, lap) = took(&gate, &root);
            stopped.push(bounded("a pass", lap.start()).await.expect("at once"));
        }
        let (landing, lap) = took(&gate, &root);
        let left = bounded("a pass", lap.start()).await.expect("at once");

        let (_, lap) = took(&gate, &root);
        let mut start = Box::pin(lap.start());
        assert!(
            poll_once(&mut start).is_pending(),
            "behind the pass under way"
        );
        // The stopped passes end while it waits: each end wakes it, and it
        // looks again.
        stopped.clear();
        assert!(
            poll_once(&mut start).is_pending(),
            "still behind the pass under way"
        );
        assert!(!landing.is_cancelled());
        drop(left);
        assert!(bounded("the pass behind", start).await.is_some());
    }

    fn took(gate: &Arc<Takeover>, root: &CancellationToken) -> (CancellationToken, Lap) {
        let (lap, ()) = gate.take(root, || ());
        let token = lap.token();
        (token, lap)
    }

    #[tokio::test]
    async fn a_newer_pass_stops_the_older_until_the_streak_is_spent() {
        let root = CancellationToken::new();
        let gate = Arc::new(Takeover::default());
        let mut running = Vec::new();
        let mut tokens = Vec::new();
        for _ in 0..=TAKEOVER_STREAK {
            let (token, lap) = took(&gate, &root);
            tokens.push(token);
            running.push(
                bounded("a pass starting", lap.start())
                    .await
                    .expect("at once"),
            );
        }
        for stopped in &tokens[..TAKEOVER_STREAK as usize] {
            assert!(
                stopped.is_cancelled(),
                "each ask stopped the pass under way"
            );
        }
        let landing = tokens.last().expect("a pass").clone();

        // The streak is spent: the next ask leaves the pass under way to land.
        let (waiting, lap) = took(&gate, &root);
        assert!(
            !landing.is_cancelled(),
            "the pass under way is left to land"
        );
        let mut start = Box::pin(lap.start());
        assert!(
            poll_once(&mut start).is_pending(),
            "the new pass waits for it"
        );

        // A newer ask stops the waiting pass freely, and waits as well.
        let (newest, lap) = took(&gate, &root);
        assert!(waiting.is_cancelled());
        assert!(bounded("the stopped pass", start).await.is_none());
        assert!(!landing.is_cancelled());
        let mut start = Box::pin(lap.start());
        assert!(poll_once(&mut start).is_pending());

        running.clear();
        let after = bounded("the pass behind", start)
            .await
            .expect("started once the pass under way landed");
        assert!(!newest.is_cancelled());
        drop(after);

        // That pass ended on its own: an ask stops the next pass again.
        let (first, lap) = took(&gate, &root);
        let _first = bounded("a pass", lap.start()).await.expect("at once");
        let (_, lap) = took(&gate, &root);
        let _second = bounded("a pass", lap.start()).await.expect("at once");
        assert!(first.is_cancelled());
    }
}
