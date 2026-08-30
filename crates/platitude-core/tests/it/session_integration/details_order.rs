//! Request order, cancellation, and completion are independent of timing.
use crate::support::TestRepo;
use crate::support::session::opened;
use crate::support::wait::{OVERALL_BUDGET, bounded};
use platitude_core::Oid;
use platitude_core::session::{DetailsOutcome, Recording, SessionEvent};

#[tokio::test]
async fn a_b_a_submitted_before_workers_run_only_reads_the_last_attempt() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one", "A");
    let a = Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    repo.commit_file("b.txt", "two", "B");
    let b = Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 2).await;
    session.set_recording(Recording::WithBackground);
    // No await between requests on this current-thread runtime: the old
    // tasks have not entered the executor when their tokens are cancelled.
    let first = session.load_details(a).unwrap();
    let middle = session.load_details(b).unwrap();
    let last = session.load_details(a).unwrap();
    let generation = last.generation();
    assert!(first.generation() < middle.generation() && middle.generation() < generation);
    assert_eq!(
        bounded("first request", first.outcome()).await,
        DetailsOutcome::Cancelled
    );
    assert_eq!(
        bounded("middle request", middle.outcome()).await,
        DetailsOutcome::Cancelled
    );
    assert_eq!(
        bounded("last request", last.outcome()).await,
        DetailsOutcome::Sent
    );
    assert_eq!(
        sink.count(
            |e| matches!(e, SessionEvent::DetailsLoaded { generation: g, .. } if *g == generation)
        ),
        1
    );
    assert_eq!(
        sink.count(|e| matches!(
            e,
            SessionEvent::DetailsLoaded { .. } | SessionEvent::DetailsFailed { .. }
        )),
        1
    );
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::CommandStarted { display, .. }
        if display.contains("--format=%H%x00%P%x00%aN"))),
        1,
        "superseded reads never spawned git"
    );
    session.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_replaced_read_stops_after_entering_the_executor() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one", "valid");
    let good = Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    let bad = Oid::from_hex_str(&"f".repeat(40)).unwrap();
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 1).await;
    session.set_recording(Recording::WithBackground);
    let (entered, entering) = tokio::sync::oneshot::channel();
    let (release, parked) = std::sync::mpsc::channel();
    sink.hook_once(
        |e| {
            matches!(e, SessionEvent::CommandStarted { display, .. }
            if display.contains("--format=%H%x00%P%x00%aN"))
        },
        move || {
            entered.send(()).unwrap();
            parked
                .recv_timeout(OVERALL_BUDGET)
                .expect("release parked details command");
        },
    );
    let old = session.load_details(bad).unwrap();
    bounded("details entered executor", entering).await.unwrap();
    let current = session.load_details(good).unwrap();
    let generation = current.generation();
    assert_eq!(
        bounded("current details", current.outcome()).await,
        DetailsOutcome::Sent
    );
    release.send(()).unwrap();
    assert_eq!(
        bounded("cancelled details task", old.outcome()).await,
        DetailsOutcome::Cancelled
    );
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::DetailsFailed { .. })),
        0
    );
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::OpFailed { op: "details", .. })),
        0
    );
    assert_eq!(
        sink.count(
            |e| matches!(e, SessionEvent::DetailsLoaded { generation: g, .. } if *g == generation)
        ),
        1
    );
    session.close();
}

#[tokio::test]
async fn a_current_failure_and_a_reopened_session_keep_their_request_identity() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one", "valid");
    let bad = Oid::from_hex_str(&"f".repeat(40)).unwrap();
    let (sink, first_session) = opened(&repo).await;
    let first = first_session.load_details(bad).unwrap();
    let first_generation = first.generation();
    assert_eq!(
        bounded("failed details", first.outcome()).await,
        DetailsOutcome::Failed
    );
    assert_eq!(
        sink.count(
            |e| matches!(e, SessionEvent::DetailsFailed { generation, oid, .. }
        if *generation == first_generation && *oid == bad)
        ),
        1
    );
    first_session.close();
    let (_, second_session) = opened(&repo).await;
    let second = second_session.load_details(bad).unwrap();
    assert!(
        second.generation() > first_generation,
        "a feed may outlive the old session"
    );
    assert_eq!(
        bounded("reopened details", second.outcome()).await,
        DetailsOutcome::Failed
    );
    second_session.close();
}
