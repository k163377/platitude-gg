//! The execution slots against real git: what the cap does to a session,
//! what a token does to a command still waiting, and what two identical
//! background reads spend.
//!
//! The competitors are driven by hand onto the queue
//! (`support::wait::poll_once`) while a slot the test holds keeps them
//! there, so every race below is asked for at the exact point it
//! could happen (core.md §非同期・並行テスト).

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::exec::{Log, observed_env};
use crate::support::session::{opened_with, scenario};
use crate::support::wait::{bounded, poll_once};
use platitude_core::process::{Limits, Pace, Priority, Slots};
use platitude_core::session::{Recording, SessionEvent};
use platitude_core::{CommandEnd, GitCommand, GitExecutor, Kept, Oid};

fn one_slot() -> Arc<Slots> {
    Arc::new(Slots::new(Limits {
        total: 1,
        reserve: 0,
    }))
}

/// An executor watched by a log, on `slots`, and a token nothing cancels.
fn logged_on(slots: &Arc<Slots>) -> (GitExecutor, Arc<Log>, tokio_util::sync::CancellationToken) {
    let log = Arc::new(Log::default());
    let (exec, cancel) = observed_env(log.clone(), Kept::Asked);
    (exec.scheduled(Arc::clone(slots)), log, cancel)
}

fn status_of(repo: &TestRepo) -> GitCommand {
    GitCommand::new()
        .cwd(&repo.path)
        .args(["status", "--porcelain=v2", "-z"])
}

/// Two identical background reads queued behind a held slot spend one
/// process between them, and both get its answer.
#[tokio::test]
async fn an_identical_background_read_shares_the_run_still_queued() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.write_file("f.txt", "dirty\n");
    let slots = one_slot();
    let (exec, log, cancel) = logged_on(&slots);
    let exec = exec.background();

    // The pool is one slot, and this holds it: everything asked for
    // below waits in the queue until it is given back.
    let held = bounded(
        "the test's own slot",
        slots.acquire(Priority::Interactive, Pace::Here),
    )
    .await
    .expect("the pool is free");

    let mut first = Box::pin(exec.run(status_of(&repo), &cancel));
    let mut second = Box::pin(exec.run(status_of(&repo), &cancel));
    assert!(poll_once(&mut first).is_pending(), "the first is queued");
    assert!(
        poll_once(&mut second).is_pending(),
        "the second follows the first rather than queueing beside it"
    );
    let report = slots.report();
    assert_eq!(
        report.queued_background, 1,
        "one of the two is in the queue"
    );
    assert_eq!(report.shared, 1, "and the other joined it");

    drop(held);
    let first = bounded("the leader's run", first)
        .await
        .expect("status ran");
    let second = bounded("the follower's answer", second)
        .await
        .expect("the follower was answered");
    assert_eq!(first.stdout, second.stdout, "one answer, handed to both");
    assert!(
        first.stdout_utf8().contains("f.txt"),
        "and it is the repository's: {}",
        first.stdout_utf8()
    );
    assert_eq!(
        log.ends_of(&["status"]).len(),
        1,
        "one process between the two reads"
    );
    assert_eq!(slots.report().running, 0, "and its slot came back");
}

/// A command whose token fires while it waits for a slot never spawns:
/// the row the log was given at the ask ends cancelled, the queue is
/// left as if nothing had asked, and the next ask runs.
#[tokio::test]
async fn a_read_cancelled_while_it_waits_is_never_spawned() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let slots = one_slot();
    let (exec, log, _never) = logged_on(&slots);
    let held = bounded(
        "the test's own slot",
        slots.acquire(Priority::Interactive, Pace::Here),
    )
    .await
    .expect("the pool is free");

    let cancel = tokio_util::sync::CancellationToken::new();
    let mut waiting = Box::pin(exec.run(status_of(&repo), &cancel));
    assert!(poll_once(&mut waiting).is_pending(), "the read is queued");
    assert_eq!(slots.report().queued_interactive, 1);

    cancel.cancel();
    let refused = bounded("the cancelled wait", waiting)
        .await
        .expect_err("a read cancelled while waiting does not run");
    assert!(refused.is_cancelled(), "{refused}");
    assert_eq!(
        log.ends_of(&["status"]),
        vec![CommandEnd::Cancelled],
        "the row the ask was given ends cancelled, and no process ran"
    );
    let report = slots.report();
    assert_eq!(
        report.queued_interactive, 0,
        "the queue is as if nothing had asked"
    );
    assert_eq!(report.left_waiting, 1);

    drop(held);
    let fresh = tokio_util::sync::CancellationToken::new();
    bounded(
        "a read after the slot came back",
        exec.run(status_of(&repo), &fresh),
    )
    .await
    .expect("the next ask runs");
    assert_eq!(
        log.ends_of(&["status"]),
        vec![CommandEnd::Cancelled, CommandEnd::Exited(0)]
    );
}

/// A session opened on two slots opens whole — the refs, the status and
/// the walk it starts together are more than two, and every one of them
/// lands — and gives every slot back once it has.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_opens_whole_through_two_slots_and_gives_them_back() {
    let (repo, _head) = scenario();
    let slots = Arc::new(Slots::new(Limits::of(2)));
    let exec = crate::support::exec::isolated().scheduled(Arc::clone(&slots));
    let (sink, session) = opened_with(&repo, exec).await;
    sink.opened_graph(&session, 5).await;
    let report = slots.report();
    assert!(
        report.admitted >= 3,
        "refs, status and the walk all ran: {report:?}"
    );
    assert_eq!(report.running, 0, "and left no slot held: {report:?}");
    assert_eq!(report.queued_interactive, 0, "{report:?}");
    session.close();
}

/// A read queued for a page that then closes goes with the page: the
/// session's close takes the queued command out, and the slot it was
/// waiting for spawns nothing on its behalf.
///
/// The session watches its own commands (`CommandFeed`), so what the
/// queued read did is read off the sink — with the background reads
/// switched on, since a details read is one the session makes on
/// its own.
#[tokio::test(flavor = "multi_thread")]
async fn a_closed_session_takes_its_queued_reads_with_it() {
    let (repo, head) = scenario();
    let slots = one_slot();
    let exec = crate::support::exec::isolated().scheduled(Arc::clone(&slots));
    let (sink, session) = opened_with(&repo, exec).await;
    sink.opened_graph(&session, 5).await;
    session.set_recording(Recording::WithBackground);
    let held = bounded(
        "the test's own slot",
        slots.acquire(Priority::Interactive, Pace::Here),
    )
    .await
    .expect("the opening has given its slots back");

    let oid = Oid::from_hex_str(head.trim()).expect("HEAD is an oid");
    let task = session.load_details(oid).expect("the repository is open");
    let id = sink
        .wait_for("the details read to queue", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandStarted { id, display, .. } if display.contains(" show ") => {
                    Some(*id)
                }
                _ => None,
            })
        })
        .await;
    // **`CommandStarted` is the ask being announced, not its place in the
    // queue.** `GitExecutor::execute` tells the observer and only then
    // goes to `Slots::acquire`, so between the two the count is still 0
    // for as long as it takes that task to reach the lock — which under a
    // full gate is long enough to be seen on both machines
    // (P3-確認事項 §core). The queue is what this is about, so the queue
    // is what is waited for; the event is kept for the id.
    bounded(
        "the details read to reach the queue",
        slots.settled(|report| report.queued_interactive == 1),
    )
    .await;
    assert_eq!(
        slots.report().queued_interactive,
        1,
        "queued behind the held slot"
    );

    session.close();
    let outcome = bounded("the details read's outcome", task.outcome()).await;
    assert!(
        matches!(outcome, platitude_core::session::DetailsOutcome::Cancelled),
        "the close ended it: {outcome:?}"
    );
    let end = sink
        .wait_for("the queued read's end", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandFinished { id: got, end, .. } if *got == id => Some(*end),
                _ => None,
            })
        })
        .await;
    assert_eq!(
        end,
        CommandEnd::Cancelled,
        "the closed page's read was never spawned"
    );
    assert_eq!(slots.report().queued_interactive, 0);
    drop(held);
}
