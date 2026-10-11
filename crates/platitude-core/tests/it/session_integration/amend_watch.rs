//! What an amend would make of the index, asked again while one is in view
//! (`RepoSession::watch_amend`): on the status that shows the index or HEAD
//! moved, whoever moved it — the counts alone stay put when another git
//! stages other bytes of a file already staged.

use crate::support::TestRepo;
use crate::support::session::opened;
use crate::support::wait::bounded;
use platitude_core::session::SessionEvent;

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
    session.close();
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
    session.close();
}
