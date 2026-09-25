//! What the suite waits on a session with: budgets that diagnose a
//! stuck causal wait.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use platitude_core::session::SessionEvent;

/// How long a wait puts up with the session saying nothing; every event
/// renews it.
pub const QUIET_BUDGET: Duration = Duration::from_secs(120);

/// The whole of a wait, as a backstop under [`QUIET_BUDGET`]: a session
/// talking without ever getting to the answer renews the silence budget
/// forever, and only a livelock reaches this one.
pub const OVERALL_BUDGET: Duration = Duration::from_secs(900);

/// Bounds an await the suite has no other backstop for — a session
/// boundary like `wait_for_snapshot_reads` sends no event [`Patience`]
/// could count, so a wedged git would hang the binary until the CI kill
/// with no test named. [`OVERALL_BUDGET`]: nothing correct takes this long.
pub async fn bounded<T>(what: &str, wait: impl Future<Output = T>) -> T {
    match tokio::time::timeout(OVERALL_BUDGET, wait).await {
        Ok(answer) => answer,
        Err(_) => panic!("{what}: no answer within the overall budget ({OVERALL_BUDGET:?})"),
    }
}

/// Polls `future` once, by hand, so it stops at its first wait point
/// (rules/core.md「競合の相手は手で目的の地点まで進める」). `Pending` means it
/// now waits there; awaiting it afterwards carries it on. The twin of the
/// crate's own (`platitude_core::wait`), which this binary cannot reach.
pub fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    Pin::new(future).poll(&mut cx)
}

/// What a wait spends while it waits: [`QUIET_BUDGET`] under
/// [`OVERALL_BUDGET`] (rules/core.md「待ちの上限は失敗検出の backstop」).
///
/// A budget counted from the first poll cannot tell a stopped session from
/// a slow one — under load one git round trip grows by an order of
/// magnitude (ci/baseline/code-costs-windows-x64.md §テストとハーネス) —
/// and raising it until that fits delays every wait's hang verdict alike.
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

    /// Time until the next diagnostic backstop.
    pub fn remaining(&self) -> Duration {
        let quiet = QUIET_BUDGET.saturating_sub(self.quiet_since.elapsed());
        let whole = OVERALL_BUDGET.saturating_sub(self.started.elapsed());
        quiet.min(whole)
    }

    /// Fails the test once the budget is spent. Call it outside any
    /// lock — the dump it prints takes one.
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
