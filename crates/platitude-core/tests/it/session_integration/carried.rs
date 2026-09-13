//! The other copies' rows against a real second working copy: the pass
//! that reads them, and what turning the copies off does to a pass in
//! flight.
//!
//! The pass is driven onto the queue while a slot the test holds keeps
//! it there, so the turning-off lands on a pass that has begun and has
//! not read — asked for at the exact point it could happen rather than
//! left to a scheduler (core.md §非同期・並行テスト).

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

/// The rows a graph pass sent — streamed in a chunk, or swapped in whole
/// — as its generation and whether any of them is another copy's.
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

/// Whether the pass at `generation` drew a row for another copy.
fn pass_draws_a_copy(events: &[SessionEvent], generation: u64) -> bool {
    events
        .iter()
        .filter_map(copy_drawn)
        .any(|(at, drawn)| at == generation && drawn)
}

/// The copies turned off under a pass that has begun: the pass lands
/// nothing, the rows come down with the switch, and the graph drawn
/// after it has no row for the copy. A pass that landed would have put
/// the row back up, with the tick stopped and nothing left to take it
/// down again.
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
    // The opening's own pass is over — the row it read is on screen —
    // and the walk it asked for has landed, so what follows is the
    // test's own subject.
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
    // Under it: the rows come down and the walk is asked for again,
    // both queued behind the held slot.
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
