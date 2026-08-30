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
//!
//! The mark a fallen rebuild leaves on the graph is held here too — it
//! goes up in the same arm — along with the other door into a pass,
//! `fail_every_pass`, which is what lets the band's `STALE GRAPH` be
//! photographed and so has to keep reaching the walk.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, is_stream_event, open_unawaited, scenario};
use crate::support::wait::bounded;
use platitude_core::session::{PassStep, RefreshOutcome, RepoSession, SessionEvent};

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
    // The band's `STALE GRAPH` badge carries the sentence; core has
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
/// real graph — only older than it should be — so the tab carries the
/// failure and the rows are left alone. What the graph does hear is that
/// it has fallen behind ([`SessionEvent::LogStale`]), which is the state
/// and not a stream's answer: the band raises `STALE GRAPH` over the
/// picture that is still standing.
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
    assert_eq!(
        sink.count(|event| matches!(event, SessionEvent::LogStale { stale: true })),
        1,
        "and was marked as no longer this repository's: {:?}",
        sink.events.lock().unwrap()
    );
    session.close();
}

/// The mark goes back down when a pass reads the repository again — and
/// **the pass that says so is the one that sends nothing else**.
///
/// A rebuild over a repository nothing has touched comes back
/// `Unchanged` and stays silent, so that a quiet auto-fetch tick costs
/// the consumer nothing. That silence is what makes this worth holding:
/// with the mark only ever going up, the ordinary answer to "is it
/// current again" would never arrive, and the band would carry
/// `STALE GRAPH` over a graph that had been read since.
#[tokio::test(flavor = "multi_thread")]
async fn a_graph_read_again_is_no_longer_behind() {
    let (_repo, sink, session) = settled().await;

    session.run_inside_next_pass(PassStep::Swapping, || panic!("the walk fell over"));
    session.refresh_log();
    sink.wait_for("the fallen rebuild's mark", |events| {
        events
            .iter()
            .find(|event| matches!(event, SessionEvent::LogStale { stale: true }))
            .map(|_| ())
    })
    .await;

    // Nothing has moved, so this one walks the same commits and finds
    // the picture on screen to be them: `Unchanged`, and the only thing
    // it has to say is that the mark can come down.
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
/// reporting arm.
///
/// **The one injection whose failure would look like a pass.**
/// `fail_every_pass` exists so `STALE GRAPH` can be photographed, and a
/// verb photographs whatever window it is given: an injection that
/// stopped reaching the walk would leave every run of those verbs
/// looking at a healthy graph and calling it green (verify-ui スキル
/// 「仕込みが PATH に届かなかった run は普通の窓を撮る」). So the arm it
/// leaves by is held here — the outcome, the words, and the mark that
/// tells the band a whole graph has gone out of date.
#[tokio::test(flavor = "multi_thread")]
async fn the_fault_the_screen_is_driven_with_fails_the_walk() {
    let (_repo, sink, session) = settled().await;

    session.fail_every_pass(PassStep::Swapping);
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
