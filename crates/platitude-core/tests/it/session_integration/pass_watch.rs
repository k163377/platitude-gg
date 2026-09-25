//! What a graph pass says when it falls over. A panicking pass is
//! swallowed at the task boundary, leaving a graph turning on an empty
//! column with nothing reported; `PassWatch` breaks that silence, and
//! these prove it still does. The fault is raised from inside the pass
//! (`PassHooks`, here `PassDoors`), the only place it can come from.
//!
//! Also held here: the mark a fallen rebuild leaves (same arm), and
//! `PassDoors::fail_every_pass`, which the `STALE GRAPH` screenshots need
//! to keep reaching the walk.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, PassDoors, is_stream_event, open_with_doors, scenario};
use crate::support::wait::bounded;
use platitude_core::session::{PassStep, RefreshOutcome, RepoSession, SessionEvent};

/// `scenario()` opened with the doors in hand and the opening's passes
/// over, so the pass each test takes down is the one it asked for.
async fn settled() -> (TestRepo, Arc<CaptureSink>, Arc<RepoSession>, Arc<PassDoors>) {
    let (repo, _) = scenario();
    let (sink, session, doors) = open_with_doors(&repo);
    sink.opened_graph(&session, 5).await;
    (repo, sink, session, doors)
}

fn announcements(sink: &CaptureSink) -> usize {
    sink.count(|event| matches!(event, SessionEvent::LogStarted { .. }))
}

/// It emptied the column on its way in and nobody else can speak for that
/// generation: a `LogFailed` under any other number leaves the graph turning.
#[tokio::test(flavor = "multi_thread")]
async fn a_stream_that_falls_over_answers_its_own_generation() {
    let (_repo, sink, session, doors) = settled().await;

    doors.run_inside_next_pass(PassStep::Streaming, || panic!("the walk fell over"));
    session.restart_log();

    let (announced, failed, error) = sink
        .wait_for("the fallen stream's report", |events| {
            let failed = events.iter().rev().find_map(|event| match event {
                SessionEvent::LogFailed { generation, error } => Some((*generation, error.clone())),
                _ => None,
            })?;
            let announced = events.iter().rev().find_map(|event| match event {
                SessionEvent::LogStarted { generation } => Some(*generation),
                _ => None,
            })?;
            Some((announced, failed.0, failed.1))
        })
        .await;

    assert_eq!(
        failed, announced,
        "the answer belongs to the stream that emptied the column"
    );
    // The band's `STALE GRAPH` carries the sentence
    // (rules-refs/app-ui.md「Rust に文言を置かない」).
    assert!(
        error.is_empty(),
        "no words were put in the graph: {error:?}"
    );
    session.close();
}

/// Nothing on screen waits on it and a real (older) graph is standing, so
/// the tab carries the failure, the rows are left alone, and the graph
/// only hears it has fallen behind ([`SessionEvent::LogStale`]).
#[tokio::test(flavor = "multi_thread")]
async fn an_offscreen_pass_that_falls_over_reports_the_operation() {
    let (_repo, sink, session, doors) = settled().await;
    let standing = sink.count(is_stream_event);

    doors.run_inside_next_pass(PassStep::Swapping, || panic!("the walk fell over"));
    session.refresh_log();

    let message = sink
        .wait_for("the fallen rebuild's report", |events| {
            events.iter().find_map(|event| match event {
                SessionEvent::OpFailed { op, error } if *op == "log" => Some(error.to_string()),
                _ => None,
            })
        })
        .await;
    assert!(
        message.contains("the graph walk ended without an answer"),
        "the failure says what happened: {message}"
    );

    // The pass has stopped, so what the graph has heard is all of it.
    bounded("the pass that fell over", session.wait_for_graph_passes()).await;
    assert_eq!(
        sink.count(is_stream_event),
        standing,
        "the graph on screen was left standing: {:?}",
        sink.events.lock().unwrap()
    );
    assert_eq!(
        sink.count(|event| matches!(event, SessionEvent::LogStale { stale: true })),
        1,
        "and was marked as no longer this repository's: {:?}",
        sink.events.lock().unwrap()
    );
    session.close();
}

/// The mark comes down when a pass reads the repository again — even an
/// `Unchanged` rebuild, which otherwise stays silent; without that, the
/// band would carry `STALE GRAPH` over a graph read since.
#[tokio::test(flavor = "multi_thread")]
async fn a_graph_read_again_is_no_longer_behind() {
    let (_repo, sink, session, doors) = settled().await;

    doors.run_inside_next_pass(PassStep::Swapping, || panic!("the walk fell over"));
    session.refresh_log();
    sink.wait_for("the fallen rebuild's mark", |events| {
        events
            .iter()
            .find(|event| matches!(event, SessionEvent::LogStale { stale: true }))
            .map(|_| ())
    })
    .await;

    // Nothing has moved: `Unchanged`, with only the mark to take down.
    let outcome = bounded("the quiet rebuild", session.refresh_log_tracked().outcome()).await;
    assert_eq!(
        outcome,
        RefreshOutcome::Unchanged,
        "the quiet rebuild is the one that carries this"
    );
    let taken = sink
        .wait_for("the mark coming back down", |events| {
            events
                .iter()
                .rev()
                .find_map(|event| match event {
                    SessionEvent::LogStale { stale } => Some(*stale),
                    _ => None,
                })
                .filter(|stale| !stale)
        })
        .await;
    assert!(!taken, "the graph is this repository's again");
    session.close();
}

/// The fault the screen is driven with reaches the pass's ordinary
/// reporting arm. An injection that stopped reaching the walk would leave
/// the `STALE GRAPH` verbs photographing a healthy graph and passing
/// (rules-refs/app-ui.md「`STALE GRAPH` の動確は `graph-stale` / `graph-stopped`」),
/// so the outcome, the words, and the mark are held here.
#[tokio::test(flavor = "multi_thread")]
async fn the_fault_the_screen_is_driven_with_fails_the_walk() {
    let (_repo, sink, session, doors) = settled().await;

    doors.fail_every_pass(PassStep::Swapping);
    let outcome = bounded("the made failure", session.refresh_log_tracked().outcome()).await;

    assert_eq!(
        outcome,
        RefreshOutcome::Failed,
        "the rebuild came back as a failure"
    );
    let message = sink
        .wait_for("the made failure's report", |events| {
            events.iter().find_map(|event| match event {
                SessionEvent::OpFailed { op, error } if *op == "log" => Some(error.to_string()),
                _ => None,
            })
        })
        .await;
    assert!(
        message.contains("the graph walk was made to fail"),
        "the words are the fault's own: {message}"
    );
    assert_eq!(
        sink.count(|event| matches!(event, SessionEvent::LogStale { stale: true })),
        1,
        "and the graph left standing was marked: {:?}",
        sink.events.lock().unwrap()
    );
    session.close();
}

/// A closing session has nobody left to read the report, and a closing tab
/// should not paint "history could not be read". The pass still owns the
/// stream, so this is the guard's own silence.
#[tokio::test(flavor = "multi_thread")]
async fn a_closing_session_is_told_nothing() {
    let (_repo, sink, session, doors) = settled().await;
    let announced = announcements(&sink);

    // Closed from inside the pass, after the announcement: closing first
    // would stop it at its own cancellation check, before the guard.
    let closing = Arc::downgrade(&session);
    doors.run_inside_next_pass(PassStep::Streaming, move || {
        if let Some(session) = closing.upgrade() {
            session.close();
        }
        panic!("the walk fell over as the window closed");
    });
    session.restart_log();

    // The pass is gone, guard and all: anything it was going to say has
    // been said.
    bounded(
        "the pass that fell over as the window closed",
        session.wait_for_graph_passes(),
    )
    .await;

    assert!(
        announcements(&sink) > announced,
        "the pass did announce its stream before it fell over"
    );
    let events = sink.events.lock().unwrap();
    assert!(
        !events.iter().any(|event| matches!(
            event,
            SessionEvent::LogFailed { .. } | SessionEvent::OpFailed { op: "log", .. }
        )),
        "a closing window heard nothing: {events:?}"
    );
}
