//! The other copies' rows against a real second working copy: the pass
//! that reads them, and what turning the copies off does to a pass in
//! flight.
//!
//! A slot the test holds keeps the pass on the queue, so the turning-off
//! lands on a pass that has begun and not yet read
//! (rules/core.md §非同期・並行テストの実装方針).

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened_with, scenario};
use crate::support::wait::bounded;
use platitude_core::process::{Limits, Pace, Priority, Slots};
use platitude_core::session::{CarriedOutcome, CopiesPace, Recording, SessionEvent};

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
    session.set_copies_pace(CopiesPace::Off);
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

/// What the pane was handed, by copy name, and whether each list was dirty.
fn handed_in(events: &[SessionEvent]) -> Vec<(String, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CarriedStatusLoaded { name, status, .. } => {
                Some((name.clone(), status.is_dirty()))
            }
            _ => None,
        })
        .collect()
}

fn handed(sink: &CaptureSink, from: usize) -> Vec<(String, bool)> {
    handed_in(&sink.events.lock().unwrap()[from..])
}

/// The commands started from event `from` on.
fn ran_since(sink: &CaptureSink, from: usize) -> Vec<String> {
    sink.events.lock().unwrap()[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .collect()
}

/// A copy's path as `git worktree list` prints it — the spelling the app
/// stands the pane on (`Carried::path`), which a temp path spelled by
/// hand need not match (a short 8.3 name on Windows).
fn listed_path(repo: &mut TestRepo, name: &str) -> String {
    repo.git(&["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|path| path.ends_with(&format!("/{name}")))
        .expect("the copy is listed")
        .to_string()
}

/// Opened beside one copy carrying a file, with the opening's pass over,
/// and the pane standing on that copy with its own read in.
async fn standing_on_the_copy(
    repo: &mut TestRepo,
    exec: platitude_core::process::GitExecutor,
) -> (Arc<CaptureSink>, Arc<platitude_core::session::RepoSession>) {
    let (sink, session) = opened_with(repo, exec).await;
    sink.opening_settled(&session).await;
    bounded("the opening's pass", session.wait_for_carried_pass()).await;
    let at = listed_path(repo, "copy");
    session.read_carried_status(at, "copy".into());
    sink.wait_for("the pane's own read", |events| {
        (!handed_in(events).is_empty()).then_some(())
    })
    .await;
    (sink, session)
}

/// The pass reads the copy the pane stands on with the rest — the same
/// `status` — so it hands the pane its list, and the copies' tick reads
/// that copy once, not once for the rows and again for the pane.
#[tokio::test(flavor = "multi_thread")]
async fn a_pass_hands_the_pane_the_copy_it_stands_on() {
    let (mut repo, _head) = scenario();
    a_copy_beside(&mut repo);
    let (sink, session) = standing_on_the_copy(&mut repo, crate::support::exec::isolated()).await;

    session.set_recording(Recording::WithBackground);
    let from = sink.events.lock().unwrap().len();
    let pass = session.refresh_carried().expect("a pass begins");
    assert!(
        matches!(
            bounded("the pass", pass.outcome()).await,
            CarriedOutcome::Landed { .. }
        ),
        "the pass landed"
    );
    assert_eq!(handed(&sink, from), [("copy".to_string(), true)]);
    let ran = ran_since(&sink, from);
    assert_eq!(
        ran.iter()
            .filter(|c| c.contains("status --porcelain"))
            .count(),
        1,
        "{ran:#?}"
    );
    session.close();
}

/// A copy gone clean drops its row, yet the pane standing on it still
/// hears so: an empty list, handed before the pass leaves clean copies out.
#[tokio::test(flavor = "multi_thread")]
async fn a_copy_gone_clean_is_handed_its_empty_list() {
    let (mut repo, _head) = scenario();
    a_copy_beside(&mut repo);
    let (sink, session) = standing_on_the_copy(&mut repo, crate::support::exec::isolated()).await;
    let copy = repo.path.with_file_name("copy");
    std::fs::remove_file(copy.join("carried.txt")).expect("the copy goes clean");

    let from = sink.events.lock().unwrap().len();
    let pass = session.refresh_carried().expect("a pass begins");
    bounded("the pass", pass.outcome()).await;
    assert_eq!(handed(&sink, from), [("copy".to_string(), false)]);
    session.close();
}

/// A pass asked while the pane stood on one copy, reading after it moved
/// to another, hands neither: not the copy left — its files would show
/// under the other's name — and not the other, which the pane's own read
/// answers.
#[tokio::test(flavor = "multi_thread")]
async fn a_pass_hands_nothing_to_a_pane_that_moved_on() {
    let (mut repo, _head) = scenario();
    a_copy_beside(&mut repo);
    let other = repo.path.with_file_name("other");
    let other_at = other.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "other", &other_at]);
    std::fs::write(other.join("other.txt"), "o\n").expect("a file in the other copy");
    let slots = Arc::new(Slots::new(Limits {
        total: 1,
        reserve: 0,
    }));
    let exec = crate::support::exec::isolated().scheduled(Arc::clone(&slots));
    let (sink, session) = standing_on_the_copy(&mut repo, exec).await;

    let from = sink.events.lock().unwrap().len();
    // The pass and the pane's next read queue behind the held slot, so the
    // pane has moved on before the pass reads anything.
    let held = bounded(
        "the test's own slot",
        slots.acquire(Priority::Interactive, Pace::Here),
    )
    .await
    .expect("the pool is free");
    let pass = session.refresh_carried().expect("a pass begins");
    session.read_carried_status(listed_path(&mut repo, "other"), "other".into());
    drop(held);
    bounded("the pass", pass.outcome()).await;
    sink.wait_for("the pane's read of the other copy", |events| {
        handed_in(&events[from..])
            .iter()
            .any(|(name, _)| name == "other")
            .then_some(())
    })
    .await;
    assert_eq!(handed(&sink, from), [("other".to_string(), true)]);
    session.close();
}
