//! What the suite waits on a session with: budgets counted against
//! silence rather than the clock, and the settling a baseline needs.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use platitude_core::session::SessionEvent;

/// How long a wait puts up with the session saying nothing. Every event
/// renews it, so what spends it is silence — not the wait taking a while.
pub const QUIET_BUDGET: Duration = Duration::from_secs(20);

/// The whole of a wait, as a backstop under [`QUIET_BUDGET`]: a session
/// talking without ever getting to the answer renews the silence budget
/// forever, and only a livelock reaches this one.
pub const OVERALL_BUDGET: Duration = Duration::from_secs(300);

/// What a wait spends while it waits.
///
/// A wait is here to catch a session that stopped, and a budget counted
/// from the first poll cannot tell that from one that is merely slow.
/// `session_integration::concurrent_writes_are_serialized` puts 12 writes
/// through the queue one at a time, and under `cargo test --workspace` —
/// 264 integration tests, a thread per core, all spawning git — one round
/// trip takes ~2.5s: 実測, 20 seconds bought 8 of them, where the test on
/// its own finished all 12 in 4.7s. Raising the number until that fits
/// would hand every other wait in the suite the same head start before it
/// notices a hang.
///
/// Progress is what tells the two apart, so that is what the budget is
/// counted against: every event renews it, and only a session gone quiet
/// spends it. Same reading as 規約 §「もう起きない」を sleep で確かめない —
/// a stretch of clock is not a state. A hang still needs the same
/// [`QUIET_BUDGET`] of nothing to be called one; it is only the waits that
/// are demonstrably being answered that no longer pay for it.
pub struct Patience {
    started: Instant,
    quiet_since: Instant,
    seen: usize,
}

impl Patience {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            quiet_since: now,
            seen: 0,
        }
    }

    /// Renews the budget if the session has said anything since the last
    /// look.
    pub fn note(&mut self, events: usize) {
        if events != self.seen {
            self.seen = events;
            self.quiet_since = Instant::now();
        }
    }

    /// How long the session has been saying nothing.
    pub fn quiet_for(&self) -> Duration {
        self.quiet_since.elapsed()
    }

    /// Fails the test once the budget is spent. Call it with no lock held
    /// — the dump it prints takes one.
    pub fn check(&self, what: &str, events: &Mutex<Vec<SessionEvent>>) {
        let (quiet, whole) = (self.quiet_since.elapsed(), self.started.elapsed());
        assert!(
            quiet < QUIET_BUDGET && whole < OVERALL_BUDGET,
            "timed out waiting for {what} after {whole:?}, the last \
             {quiet:?} of it in silence; events so far: {:?}",
            events.lock().unwrap()
        );
    }
}

impl Default for Patience {
    fn default() -> Self {
        Self::new()
    }
}

/// How long nothing may happen before [`settled`] calls the session done.
///
/// Only the gap between one git command ending and the next one starting
/// has to fit in here — a command that is *running* is not silence but an
/// outstanding start (see [`settled`]) — so this is scheduling time, not
/// git time. 実測 under five copies of the suite at 32 threads each: the
/// opening's tag-inclusive pass took 639ms to get its `log -z` out of the
/// door after the tag-less one finished, on a machine where a single git
/// spawn was taking upwards of a second. Three times that, and still an
/// order under [`QUIET_BUDGET`], so a session that has genuinely hung is
/// still called one on the same terms as everywhere else.
const SETTLE_WINDOW: Duration = Duration::from_secs(2);

/// Whether every git command the session has announced has also reported
/// back. A command that is still running says nothing while it runs, and
/// on a loaded machine it says nothing for seconds (実測: 5830ms for a
/// two-commit walk) — long enough for a wait that only counts silence to
/// call the session finished in the middle of its opening.
///
/// Commands that are not being recorded report neither end, so the two
/// counts stay in step whatever `set_record_background` is set to; what
/// they cost is visibility, which is why [`settled`] is only as good as
/// how early recording was switched on.
fn nothing_running(events: &[SessionEvent]) -> bool {
    let (mut started, mut finished) = (0usize, 0usize);
    for event in events {
        match event {
            SessionEvent::CommandStarted { .. } => started += 1,
            SessionEvent::CommandFinished { .. } => finished += 1,
            _ => {}
        }
    }
    started == finished
}

/// Waits until the session has stopped doing things: nothing running, and
/// nothing said for a [`SETTLE_WINDOW`].
///
/// For taking a *baseline* — a count of what has happened so far, against
/// which whatever the test does next is measured. The trap that shape
/// walks into is that a session which has not been asked for anything for
/// a moment is not the same as one that has finished what it was already
/// doing: work set in motion by the opening lands whenever it lands, and
/// under load that is after a test on an idle machine would have finished
/// reading. Anything still in flight then gets counted as the reaction to
/// what the test did next.
///
/// Counted against silence rather than a fixed number of looks, for the
/// same reason [`Patience`] is (規約 §「もう起きない」を sleep で確かめない),
/// and it borrows `Patience` for the giving up: a command that never
/// reports back is a wait that never goes quiet, and gets the same
/// [`QUIET_BUDGET`] and dump as any other stuck wait.
pub async fn settled(events: &Mutex<Vec<SessionEvent>>) {
    let mut patience = Patience::new();
    loop {
        let (running, quiet) = {
            let evs = events.lock().unwrap();
            patience.note(evs.len());
            (!nothing_running(&evs), patience.quiet_for())
        };
        if !running && quiet >= SETTLE_WINDOW {
            return;
        }
        patience.check("the session to settle", events);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
