//! What a unit test waits with: the budget that names a hung await, and
//! the hand that drives a future to the point where it has to wait. Twin
//! of `tests/it/support/wait.rs`, which a unit test cannot reach.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// Backstop for a whole wait: nothing correct takes this long. Same as
/// the integration suite's `OVERALL_BUDGET`.
pub(crate) const OVERALL_BUDGET: Duration = Duration::from_secs(900);

/// Bounds an await that no event answers — a hand-stepped tick, a
/// tracked completion, a spawned task — so one nobody schedules fails
/// by name.
pub(crate) async fn bounded<T>(what: &str, wait: impl Future<Output = T>) -> T {
    match tokio::time::timeout(OVERALL_BUDGET, wait).await {
        Ok(answer) => answer,
        Err(_) => panic!("{what}: no answer within the overall budget ({OVERALL_BUDGET:?})"),
    }
}

/// Polls `future` once, so it runs to its first wait point and stops
/// there (core.md §非同期・並行テストの実装方針). After `Pending`, awaiting it
/// carries on from that point.
pub(crate) fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    Pin::new(future).poll(&mut cx)
}
