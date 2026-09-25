//! The other copies' rows against a real second working copy: the pass
//! that reads them, and what turning the copies off does to a pass in
//! flight.
//!
//! A slot the test holds keeps the pass on the queue, so the turning-off
//! lands on a pass that has begun and not yet read
//! (rules/core.md §非同期・並行テストの実装方針).

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{opened_with, scenario};
use crate::support::wait::bounded;
use platitude_core::process::{Limits, Pace, Priority, Slots};
use platitude_core::session::{CarriedOutcome, SessionEvent};

/// A second working copy of `repo` beside it, carrying an untracked
/// file — a row for the graph to draw, leashed to the commit both
/// copies stand on.
fn a_copy_beside(repo: &mut TestRepo) {
    let copy = repo.path.with_file_name("copy");
    let at = copy.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "copy", &at]);
    std::fs::write(copy.join("carried.txt"), "u\n").expect("a file in the copy");
}

/// A graph pass's generation, and whether any of its rows is another copy's.
fn copy_drawn(event: &SessionEvent) -> Option<(u64, bool)> {
    let (generation, rows) = match event {
        SessionEvent::LogChunk { generation, rows }
        | SessionEvent::LogReplaced {
            generation, rows, ..
        } => (*generation, rows),
        _ => return None,
    };
    Some((generation, rows.iter().any(|r| r.carried.is_some())))
}

/// The generation of the first graph pass whose rows draw another copy.
fn draws_a_copy(events: &[SessionEvent]) -> Option<u64> {
    events
        .iter()
        .filter_map(copy_drawn)
        .find_map(|(generation, drawn)| drawn.then_some(generation))
}

fn pass_draws_a_copy(events: &[SessionEvent], generation: u64) -> bool {
    events
        .iter()
        .filter_map(copy_drawn)
        .any(|(at, drawn)| at == generation && drawn)
}

/// A pass that landed after the switch would put the copy's row back up,
/// with the tick stopped and nothing left to take it down again.
#[tokio::test(flavor = "multi_thread")]
async fn a_pass_in_flight_when_the_copies_are_turned_off_lands_nothing() {
    let (mut repo, _head) = scenario();
    a_copy_beside(&mut repo);
    let slots = Arc::new(Slots::new(Limits {
        total: 1,
        reserve: 0,
    }));
    let exec = crate::support::exec::isolated().scheduled(Arc::clone(&slots));
    let (sink, session) = opened_with(&repo, exec).await;
    let with_copy = sink
        .wait_for("a graph drawing the copy's row", draws_a_copy)
        .await;
    // Let the opening's pass and the walk it asked for land, so what
    // follows is the test's own subject.
    bounded(
        "the opening's pass over the copies",
        session.wait_for_carried_pass(),
    )
    .await;
    sink.pass_after(
        "the pass drawing the copy's row",
        with_copy.saturating_sub(1),
    )
    .await;

    let held = bounded(
        "the test's own slot",
        slots.acquire(Priority::Interactive, Pace::Here),
    )
    .await
    .expect("the pool is free");
    let pass = session
        .refresh_carried()
        .expect("a pass begins: the copies are read and none is out");
    // Both queued behind the held slot: the rows come down, the walk is
    // asked for again.
    session.set_copies_read(false);
    drop(held);
    assert_eq!(
        bounded("the pass", pass.outcome()).await,
        CarriedOutcome::Dropped,
        "the pass begun before the turning lands nothing"
    );
    let after = sink
        .pass_after("the graph without the copy's row", with_copy)
        .await;
    {
        let events = sink.events.lock().unwrap();
        assert!(
            !pass_draws_a_copy(&events, after.generation),
            "the rows stayed down"
        );
    }
    bounded("the pass's end", session.wait_for_carried_pass()).await;
    assert!(
        session.refresh_carried().is_none(),
        "and no pass begins while the copies are off"
    );
    session.close();
}
