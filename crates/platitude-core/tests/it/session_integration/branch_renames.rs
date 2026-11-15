//! A local branch renamed outside the window, told apart from one deleted
//! and another made on its commit by the new name's reflog: the snapshot
//! says which name an old one answers to now (`RefsSnapshot::renamed_to`),
//! so a menu standing on the old name can follow it.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, poll_landed};
use platitude_core::session::{RefsSnapshot, SessionEvent};

/// What `answer` reads off the last snapshot `listing` holds for.
async fn read_off<T>(
    sink: &Arc<CaptureSink>,
    listing: impl Fn(&RefsSnapshot) -> bool,
    answer: impl Fn(&RefsSnapshot) -> T,
) -> T {
    sink.wait_for("a snapshot listing the branches", move |evs| {
        evs.iter().rev().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot, .. } if listing(snapshot) => {
                Some(answer(snapshot))
            }
            _ => None,
        })
    })
    .await
}

/// The name `old` answers to in the last snapshot that lists `new` and
/// not `old`.
async fn answer_after(
    sink: &Arc<CaptureSink>,
    old: &'static str,
    new: &'static str,
) -> Option<String> {
    read_off(
        sink,
        move |s| s.local_named(new).is_some() && s.local_named(old).is_none(),
        move |s| s.renamed_to(old).map(str::to_string),
    )
    .await
}

fn with_topic() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["branch", "topic"]);
    repo
}

#[tokio::test(flavor = "multi_thread")]
async fn a_branch_renamed_outside_answers_to_its_new_name() {
    let mut repo = with_topic();
    let (sink, session) = opened(&repo).await;
    let before = read_off(
        &sink,
        |s| s.local_named("topic").is_some(),
        |s| s.renamed_to("topic").map(str::to_string),
    )
    .await;
    assert_eq!(before, None, "nothing renamed yet");

    repo.git(&["branch", "-m", "topic", "topic-two"]);
    poll_landed(&session).await;
    assert_eq!(
        answer_after(&sink, "topic", "topic-two").await.as_deref(),
        Some("topic-two"),
        "git's own reflog line says it was renamed"
    );
    session.close_and_wait().await;
}

/// The branch the worktree is on renamed the same way: HEAD goes along, and
/// the old name answers to the new one as well.
#[tokio::test(flavor = "multi_thread")]
async fn the_branch_the_tree_is_on_renamed_answers_too() {
    let mut repo = with_topic();
    repo.git(&["switch", "-q", "topic"]);
    let (sink, session) = opened(&repo).await;
    read_off(&sink, |s| s.local_named("topic").is_some(), |_| ()).await;
    repo.git(&["branch", "-m", "topic", "topic-two"]);
    poll_landed(&session).await;
    assert_eq!(
        answer_after(&sink, "topic", "topic-two").await.as_deref(),
        Some("topic-two")
    );
    session.close_and_wait().await;
}

/// Deleted and another made on the same commit between two reads: the same
/// listing as a rename, but the new name's reflog says it was made, not
/// renamed — the old name answers to nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_branch_deleted_and_another_made_on_its_commit_is_no_rename() {
    let mut repo = with_topic();
    let (sink, session) = opened(&repo).await;
    read_off(&sink, |s| s.local_named("topic").is_some(), |_| ()).await;

    repo.git(&["branch", "-D", "topic"]);
    repo.git(&["branch", "topic-two", "HEAD"]);
    poll_landed(&session).await;
    assert_eq!(answer_after(&sink, "topic", "topic-two").await, None);
    session.close_and_wait().await;
}

/// Renamed twice, a read between: both old names answer to the last one.
#[tokio::test(flavor = "multi_thread")]
async fn a_rename_of_a_rename_answers_with_the_last_name() {
    let mut repo = with_topic();
    let (sink, session) = opened(&repo).await;
    read_off(&sink, |s| s.local_named("topic").is_some(), |_| ()).await;

    repo.git(&["branch", "-m", "topic", "topic-two"]);
    poll_landed(&session).await;
    answer_after(&sink, "topic", "topic-two").await;
    repo.git(&["branch", "-m", "topic-two", "topic-three"]);
    poll_landed(&session).await;
    let last = read_off(
        &sink,
        |s| s.local_named("topic-three").is_some(),
        |s| {
            (
                s.renamed_to("topic").map(str::to_string),
                s.renamed_to("topic-two").map(str::to_string),
            )
        },
    )
    .await;
    assert_eq!(
        last,
        (
            Some("topic-three".to_string()),
            Some("topic-three".to_string())
        )
    );
    session.close_and_wait().await;
}

/// The old name made again is a branch of its own: it answers to itself,
/// not to the name it was renamed to.
#[tokio::test(flavor = "multi_thread")]
async fn an_old_name_made_again_is_a_branch_of_its_own() {
    let mut repo = with_topic();
    let (sink, session) = opened(&repo).await;
    read_off(&sink, |s| s.local_named("topic").is_some(), |_| ()).await;

    repo.git(&["branch", "-m", "topic", "topic-two"]);
    poll_landed(&session).await;
    answer_after(&sink, "topic", "topic-two").await;
    repo.git(&["branch", "topic", "HEAD"]);
    poll_landed(&session).await;
    let again = read_off(
        &sink,
        |s| s.local_named("topic").is_some() && s.local_named("topic-two").is_some(),
        |s| s.renamed_to("topic").map(str::to_string),
    )
    .await;
    assert_eq!(again, None);
    session.close_and_wait().await;
}
