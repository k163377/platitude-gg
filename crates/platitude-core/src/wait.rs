//! What a unit test waits with: the budget that names a hung await, and
//! the hand that drives a future to the point where it has to wait.
//!
//! The integration suite has its own (`tests/it/support/wait.rs`); a test
//! inside the crate cannot reach that binary, so this is its twin, under
//! the same budget.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// The whole of a wait, as a backstop: nothing correct takes this long,
/// and a run that does is reported as the failure it is, under the
/// caller's name for it. The integration suite's `OVERALL_BUDGET`.
pub(crate) const OVERALL_BUDGET: Duration = Duration::from_secs(900);

/// Bounds an await that no event answers — a hand-stepped tick, a
/// tracked completion, a spawned task — so one nobody schedules fails by
/// name instead of hanging the binary.
pub(crate) async fn bounded<T>(what: &str, wait: impl Future<Output = T>) -> T {
    match tokio::time::timeout(OVERALL_BUDGET, wait).await {
        Ok(answer) => answer,
        Err(_) => panic!("{what}: no answer within the overall budget ({OVERALL_BUDGET:?})"),
    }
}

/// Polls `future` once, by hand: it runs to the first point where it has
/// to wait and stops there. This is how a test puts a competitor exactly
/// where a race is — parked on a gate, inside a read — without a
/// `yield_now` and a guess about what the scheduler did with it. `Ready`
/// is the answer; `Pending` says the future now waits on whatever it
/// reached, and awaiting it afterwards carries it on from there.
pub(crate) fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    Pin::new(future).poll(&mut cx)
}
