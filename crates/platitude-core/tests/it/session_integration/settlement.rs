//! The page's read boundary before unchanged snapshot workers start: neither an already finished graph nor its
//! matching generation discharges a read registered by the caller.

use std::sync::Arc;

use crate::support::session::{opened_with, scenario};
use crate::support::wait::bounded;
use platitude_core::process::{Limits, Pace, Priority, Slots};

async fn a_snapshot_owed(refs: bool) {
    let (repo, _) = scenario();
    let slots = Arc::new(Slots::new(Limits {
        total: 1,
        reserve: 0,
    }));
    let exec = crate::support::exec::isolated().scheduled(Arc::clone(&slots));
    let (sink, session) = opened_with(&repo, exec).await;
    let before = sink.opened_graph(&session, 5).await;
    assert!(session.graph_settled_at(before.generation));
    let held = bounded(
        "the test's slot",
        slots.acquire(Priority::Interactive, Pace::Here),
    )
    .await
    .expect("a free slot");
    if refs {
        session.refresh_refs();
    } else {
        session.refresh_status();
    }
    assert!(
        !session.graph_settled_at(before.generation),
        "the unchanged snapshot is owed before spawn"
    );
    drop(held);
    bounded("the snapshot ending", session.wait_for_snapshot_reads()).await;
    bounded("any walk it asked for", session.wait_for_graph_passes()).await;
    assert!(session.graph_settled_at(before.generation));
    session.close();
}

#[tokio::test]
async fn a_settled_graph_waits_for_an_unchanged_refs_read_registered_before_spawn() {
    a_snapshot_owed(true).await;
}

#[tokio::test]
async fn a_settled_graph_waits_for_an_unchanged_status_read_registered_before_spawn() {
    a_snapshot_owed(false).await;
}
