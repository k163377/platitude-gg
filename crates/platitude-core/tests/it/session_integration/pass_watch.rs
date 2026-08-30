//! What a graph pass says when it falls over.
//!
//! The one ending with no `match` arm behind it: a pass that panics dies
//! where it stands and the runtime swallows it at the task boundary, so
//! nothing here would go red on its own — the symptom is a graph left
//! turning on an empty column with nothing in the error surface and
//! nothing in the command log. `PassWatch` is what breaks that silence,
//! and these are what prove it still does.
//!
//! The fault is raised from inside the pass (`run_inside_next_pass`),
//! because that is the only place it can come from.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, is_stream_event, open_unawaited, scenario};
use crate::support::wait::bounded;
use platitude_core::session::{PassStep, RepoSession, SessionEvent};

/// `scenario()` opened, with everything the opening starts closed out:
/// the pass each test takes down is the one it asked for and no other.
async fn settled() -> (TestRepo, Arc<CaptureSink>, Arc<RepoSession>) {
    let (repo, _) = scenario();
    let (sink, session) = open_unawaited(&repo);
    sink.opened_graph(&session, 5).await;
    (repo, sink, session)
}

fn announcements(sink: &CaptureSink) -> usize {
    sink.count(|event| matches!(event, SessionEvent::LogStarted { .. }))
}

/// A streaming pass that falls over answers the stream it announced.
///
/// It emptied the column on its way in and nobody else can speak for
/// that generation, so the report has to carry the same number the
/// `LogStarted` did — a `LogFailed` under any other number leaves the
/// graph turning exactly as it was.
#[tokio::test(flavor = "multi_thread")]
async fn a_stream_that_falls_over_answers_its_own_generation() {
    let (_repo, sink, session) = settled().await;

    session.run_inside_next_pass(PassStep::Streaming, || panic!("the walk fell over"));
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
    // The band's `PARTIAL HISTORY` badge carries the sentence; core has
    // no words of its own here, and nobody said any (app-ui.md
    // 「Rust に文言を置かない」).
    assert!(
        error.is_empty(),
        "no words were put in the graph: {error:?}"
    );
    session.close();
}

/// An off-screen pass that falls over reports as the operation it was.
///
/// Nothing on screen is waiting on it and what is standing there is a
/// real graph — only older than it should be — so the graph hears
/// nothing at all and the tab carries the failure.
#[tokio::test(flavor = "multi_thread")]
async fn an_offscreen_pass_that_falls_over_reports_the_operation() {
    let (_repo, sink, session) = settled().await;
    let standing = sink.count(is_stream_event);

    session.run_inside_next_pass(PassStep::Swapping, || panic!("the walk fell over"));
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
    session.close();
}

/// A window that is closing is told nothing.
///
/// The report exists for a graph left waiting on a pass, and a session
/// whose runtime is going away has nobody left to read one — being told
/// its history could not be read is the last thing a closing tab should
/// paint. The pass still gets far enough to own the stream, so this is
/// the silence of the guard and not of a pass that never announced.
#[tokio::test(flavor = "multi_thread")]
async fn a_closing_session_is_told_nothing() {
    let (_repo, sink, session) = settled().await;
    let announced = announcements(&sink);

    // Closed from inside the pass, after the announcement: closing first
    // would stop the pass at its own cancellation check, well before the
    // guard this is about.
    let closing = Arc::downgrade(&session);
    session.run_inside_next_pass(PassStep::Streaming, move || {
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
