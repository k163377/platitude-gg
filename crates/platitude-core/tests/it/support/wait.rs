//! What the suite waits on a session with: budgets that diagnose a stuck
//! causal wait rather than establish correctness by elapsed time.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use platitude_core::session::SessionEvent;

/// How long a wait puts up with the session saying nothing. Every event
/// renews it, so what spends it is silence — not the wait taking a while.
pub const QUIET_BUDGET: Duration = Duration::from_secs(120);

/// The whole of a wait, as a backstop under [`QUIET_BUDGET`]: a session
/// talking without ever getting to the answer renews the silence budget
/// forever, and only a livelock reaches this one.
pub const OVERALL_BUDGET: Duration = Duration::from_secs(900);

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

    /// Time until the next diagnostic backstop. Correctness never depends
    /// on spending this duration; it only wakes an event-driven waiter when
    /// the event source has stopped altogether.
    pub fn remaining(&self) -> Duration {
        let quiet = QUIET_BUDGET.saturating_sub(self.quiet_since.elapsed());
        let whole = OVERALL_BUDGET.saturating_sub(self.started.elapsed());
        quiet.min(whole)
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
