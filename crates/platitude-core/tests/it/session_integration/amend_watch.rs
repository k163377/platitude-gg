//! What an amend would make of the index, asked again while one is in view
//! (`RepoSession::watch_amend`): on the status that shows the index or HEAD
//! moved, whoever moved it — the counts alone stay put when another git
//! stages other bytes of a file already staged.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, write_answer, write_settled};
use crate::support::wait::bounded;
use platitude_core::commit::CommitOptions;
use platitude_core::session::{Recording, RepoSession, SessionEvent};

/// The answers so far, in the order they came.
fn answers(events: &[SessionEvent]) -> Vec<bool> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AmendChecked { empties } => Some(*empties),
            _ => None,
        })
        .collect()
}

/// HEAD changes `a.txt`, and what is staged takes that back: amending would
/// leave HEAD changing nothing.
fn undone() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("a.txt", "two\n", "head");
    repo.write_file("a.txt", "one\n");
    repo.git(&["add", "a.txt"]);
    repo
}

#[tokio::test]
async fn an_index_another_git_changes_under_the_same_counts_is_asked_about_again() {
    let mut repo = undone();
    let (sink, session) = opened(&repo).await;

    session.watch_amend(true);
    sink.wait_for("the first answer", |evs| {
        answers(evs).first().copied().filter(|empties| *empties)
    })
    .await;

    // Another git stages other bytes of the one file staged: one file staged
    // before and after.
    repo.write_file("a.txt", "three\n");
    repo.git(&["add", "a.txt"]);
    bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
    sink.wait_for("the answer about the index now", |evs| {
        (answers(evs).last() == Some(&false)).then_some(())
    })
    .await;
    session.close_and_wait().await;
}

/// A status with nothing staged answers that an amend empties nothing — an
/// amend of the message alone never empties a commit. Whether a status asks
/// at all (out of view, an index as last asked) is `session::author`'s
/// own test.
#[tokio::test]
async fn an_index_with_nothing_staged_is_answered_as_no_empty_amend() {
    let mut repo = undone();
    let (sink, session) = opened(&repo).await;

    session.watch_amend(true);
    sink.wait_for("the first answer", |evs| {
        answers(evs).first().copied().filter(|empties| *empties)
    })
    .await;
    repo.git(&["reset", "-q"]);
    bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
    sink.wait_for("the answer with nothing staged", |evs| {
        (answers(evs).last() == Some(&false)).then_some(())
    })
    .await;
    session.close_and_wait().await;
}

fn is_status_spawn(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. } if display.starts_with("git status"))
}

/// A status read asked for once the opening's are over, parked at its
/// spawn: it keeps the status flight, so the read behind the next write
/// waits behind it. Answers the release.
async fn status_held(
    sink: &Arc<CaptureSink>,
    session: &Arc<RepoSession>,
) -> std::sync::mpsc::Sender<()> {
    sink.opening_settled(session).await;
    bounded("the opening's reads", session.wait_for_snapshot_reads()).await;
    session.set_recording(Recording::WithBackground);
    let release = sink.park_once("the parked status", is_status_spawn);
    session.refresh_status();
    sink.wait_for("the status reached its spawn", |evs| {
        evs.iter().any(is_status_spawn).then_some(())
    })
    .await;
    release
}

/// A commit tells the status it left as git answers (`session::foretold`),
/// and the amend in view is asked about that index there — not once the
/// read behind the commit lands, which is held off here: with nothing left
/// staged, it is answered at once.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_index_a_commit_leaves_is_asked_about_at_its_answer() {
    let repo = undone();
    let (sink, session) = opened(&repo).await;
    session.watch_amend(true);
    sink.wait_for("the first answer", |evs| {
        answers(evs).first().copied().filter(|empties| *empties)
    })
    .await;

    let release = status_held(&sink, &session).await;
    let id = session
        .commit("one again".into(), CommitOptions::default())
        .expect("the commit was accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    let from = sink
        .events
        .lock()
        .unwrap()
        .iter()
        .position(|e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id))
        .expect("the commit answered");
    sink.wait_for("the answer about the index the commit left", move |evs| {
        (answers(&evs[from..]).first() == Some(&false)).then_some(())
    })
    .await;
    release.send(()).expect("the parked status is waiting");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    session.close_and_wait().await;
}
