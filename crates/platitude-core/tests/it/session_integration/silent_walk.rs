//! A graph walk git says nothing in for its silence budget
//! (`session::walk_silence`): answered as not landed to whoever waits on it,
//! said on the band, and left running to land.
//!
//! The budget is mocked to nothing, so every walk is told silent on the
//! watch's first look, before git can have answered (`process::child`'s
//! `watch_silence`) — a stand-in for a git that answers late, through the real
//! walk, the real executor's watch and a real git. mry's mocks are the
//! thread's own, so these run on the default current-thread runtime
//! (rules-refs/core.md「テストの差し替えは mry」).

use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{opened, pass_of, write_result};
use platitude_core::OperationKind;
use platitude_core::commit::CommitOptions;
use platitude_core::session::{self, FollowUp, SessionEvent};

/// The commit's settling does not wait on the walk gone silent behind it:
/// it settles naming the graph as not landed. The walk lands after all with
/// the commit's row, under the band's mark, and that landing takes the mark
/// down.
#[tokio::test]
#[mry::lock(session::walk_silence)]
async fn a_silent_walk_lets_the_write_settle_and_lands_after() {
    session::mock_walk_silence().returns(Duration::ZERO);
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);

    let (sink, session) = opened(&repo).await;
    let from = sink.events.lock().unwrap().len();
    session.commit("after a silent walk".into(), CommitOptions::default());
    assert_eq!(write_result(&sink, OperationKind::Commit).await, None);

    let failed = sink
        .wait_for("the commit settled", |evs| {
            evs[from..].iter().find_map(|e| match e {
                SessionEvent::WriteSettled {
                    kind: OperationKind::Commit,
                    failed,
                    ..
                } => Some(failed.clone()),
                _ => None,
            })
        })
        .await;
    assert!(
        failed.contains(&FollowUp::Graph),
        "settled without the graph landing under it: {failed:?}"
    );

    // The walk behind the commit lands with its row (two commits) while
    // the mark is up, and that landing takes it down. Other walks go up and
    // down beside it: every walk is silent at once here.
    let (marks, landed) = sink
        .wait_for("the silent walk landed and its mark came down", |evs| {
            let tail = &evs[from..];
            let landed = tail
                .iter()
                .position(|e| pass_of(e).is_some_and(|pass| pass.total == 2))?;
            let marks: Vec<(usize, bool)> = tail
                .iter()
                .enumerate()
                .filter_map(|(at, e)| match e {
                    SessionEvent::LogSilent { silent } => Some((at, *silent)),
                    _ => None,
                })
                .collect();
            let lowered = marks.iter().any(|&(at, silent)| !silent && at > landed);
            lowered.then_some((marks, landed))
        })
        .await;
    let before = marks.iter().rev().find(|&&(at, _)| at < landed);
    assert!(
        matches!(before, Some(&(_, true))),
        "the mark was up as the walk landed: {marks:?}, landed at {landed}"
    );
    let after = marks.iter().find(|&&(at, _)| at > landed);
    assert!(
        matches!(after, Some(&(_, false))),
        "and the landing took it down: {marks:?}, landed at {landed}"
    );
    session.close_and_wait().await;
}
