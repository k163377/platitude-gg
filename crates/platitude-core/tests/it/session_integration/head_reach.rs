//! Whether anything besides HEAD's branch still holds its tip once a
//! rewrite moves the branch off it — the answer the drop's hold mark reads
//! (`session::head_reach`, `reachable`) — and the person's own
//! `rebase.updateRefs`, under which git takes the branches on the tip
//! along.

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, TestSession, opened};
use crate::support::wait::bounded;
use platitude_core::session::SessionEvent;

/// A session's first answer about the tip; the session is closed after.
/// The opening's refs read asks once, and nothing ahead of it answers.
async fn first_reach(repo: &TestRepo) -> bool {
    let (sink, session) = opened(repo).await;
    let reached = sink
        .wait_for("the answer about the tip", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::HeadReachChecked { reached_elsewhere } => Some(*reached_elsewhere),
                _ => None,
            })
        })
        .await;
    session.close_and_wait().await;
    reached
}

/// The answer the session last sent about the tip.
fn last_reach(events: &[SessionEvent]) -> Option<bool> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::HeadReachChecked { reached_elsewhere } => Some(*reached_elsewhere),
        _ => None,
    })
}

/// Waits until the last answer about the tip is `held`.
async fn answered(sink: &CaptureSink, what: &str, held: bool) {
    sink.wait_for(what, |evs| (last_reach(evs) == Some(held)).then_some(()))
        .await;
}

/// Puts a branch on the root and has a poll read it: the refs moved, so the
/// reach is asked again, though the graph draws the same commits.
async fn moved_a_ref_elsewhere(repo: &mut TestRepo, session: &TestSession) {
    repo.git(&["branch", "other", "HEAD~1"]);
    let outcome = bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
    assert!(outcome.landed(), "the poll read the moved ref: {outcome:?}");
}

/// The root and the tip, with `keep` on the tip beside `main`.
fn a_branch_on_the_tip() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    repo.commit_file("b.txt", "2\n", "the tip");
    repo.git(&["branch", "keep"]);
    repo
}

/// A branch on the tip holds it where a rewrite leaves it behind (git's
/// default), and holds nothing where the setting takes it along. What the
/// setting does not move still holds: a branch further along, a tag.
#[tokio::test(flavor = "multi_thread")]
async fn a_branch_on_the_tip_holds_it_unless_the_setting_takes_it_along() {
    let mut repo = a_branch_on_the_tip();
    assert!(
        first_reach(&repo).await,
        "without the setting the branch stays on the old tip"
    );

    repo.git(&["config", "rebase.updateRefs", "true"]);
    assert!(
        !first_reach(&repo).await,
        "with it the branch moves with the rewrite"
    );

    repo.git(&["switch", "--quiet", "keep"]);
    repo.commit_file("c.txt", "3\n", "beyond");
    repo.git(&["switch", "--quiet", "main"]);
    repo.git(&["branch", "also-on-the-tip"]);
    assert!(
        first_reach(&repo).await,
        "a branch further along is not one the rewrite moves"
    );

    repo.git(&["branch", "--delete", "--force", "keep"]);
    repo.git(&["tag", "v1"]);
    assert!(first_reach(&repo).await, "nor is a tag on the tip");
}

/// git leaves a branch another worktree has out where it is, so under the
/// setting it still holds the tip. The worktree listing can land after the
/// opening's refs read; a ref moved once it has asks again with it.
#[tokio::test(flavor = "multi_thread")]
async fn a_branch_another_worktree_has_out_holds_the_tip_under_the_setting() {
    let mut repo = a_branch_on_the_tip();
    let elsewhere = repo.path.with_file_name("held-worktree");
    let at = elsewhere.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", &at, "keep"]);
    repo.git(&["config", "rebase.updateRefs", "true"]);

    let (sink, session) = opened(&repo).await;
    sink.wait_for("the listing naming the other worktree", |evs| {
        evs.iter()
            .any(|e| {
                matches!(e, SessionEvent::WorktreesLoaded { worktrees, .. } if worktrees.len() == 2)
            })
            .then_some(())
    })
    .await;
    sink.opening_settled(&session).await;
    moved_a_ref_elsewhere(&mut repo, &session).await;
    answered(&sink, "the tip held by the other worktree's branch", true).await;
    session.close_and_wait().await;
}

/// The setting is read again where the person may have changed it — the
/// settings screen closing — and the answer follows with no ref moving,
/// both ways.
#[tokio::test(flavor = "multi_thread")]
async fn the_setting_read_again_moves_the_answer_with_no_ref_moving() {
    let mut repo = a_branch_on_the_tip();
    let (sink, session) = opened(&repo).await;
    answered(&sink, "the opening's answer", true).await;

    repo.git(&["config", "rebase.updateRefs", "true"]);
    session.reread_update_refs();
    answered(&sink, "the branch moving with the rewrite", false).await;

    repo.git(&["config", "--unset", "rebase.updateRefs"]);
    session.reread_update_refs();
    answered(&sink, "the branch staying again", true).await;
    session.close_and_wait().await;
}

/// With the setting already read (on), a ref moving still asks the walk —
/// the setting kept is not an answer: the branch further along that held
/// the tip goes, and the answer turns to "not held".
#[tokio::test(flavor = "multi_thread")]
async fn a_known_setting_still_walks_when_the_refs_move() {
    let mut repo = a_branch_on_the_tip();
    repo.git(&["config", "rebase.updateRefs", "true"]);
    repo.git(&["switch", "--quiet", "--create", "ahead"]);
    repo.commit_file("c.txt", "3\n", "beyond");
    repo.git(&["switch", "--quiet", "main"]);
    let (sink, session) = opened(&repo).await;
    answered(&sink, "the opening's answer, the setting read", true).await;
    sink.opening_settled(&session).await;

    repo.git(&["branch", "--delete", "--force", "ahead"]);
    let outcome = bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
    assert!(outcome.landed(), "the poll read the moved ref: {outcome:?}");
    answered(&sink, "the branch further along gone", false).await;
    session.close_and_wait().await;
}

/// git's own side of the rule: what it moves under the setting
/// (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;
    use crate::support::session::{install_todo_editor, write_answer};

    /// What the rule stands on: under the setting, a drop through the
    /// session's own replay takes a branch sitting on the tip to the new tip
    /// — a commit dropped from the middle and the tip itself dropped alike —
    /// so nothing is left holding the old chain but the reflog.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "git moving a branch on the tip under rebase.updateRefs: not worth the pre-merge run"]
    async fn under_the_setting_a_drop_takes_a_branch_on_the_tip_along() {
        install_todo_editor();
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "1\n", "root");
        let middle = repo.commit_file_id("b.txt", "2\n", "drop me");
        let tip = repo.commit_file_id("c.txt", "3\n", "the tip");
        repo.git(&["branch", "keep"]);
        repo.git(&["config", "rebase.updateRefs", "true"]);

        let (sink, session) = opened(&repo).await;
        let id = session.drop_commit(middle).expect("the drop is taken");
        assert_eq!(write_answer(&sink, id).await, None);
        let rewritten = repo.git(&["rev-parse", "HEAD"]);
        assert_ne!(rewritten, tip, "the tip was rewritten");
        assert_eq!(
            repo.git(&["rev-parse", "keep"]),
            rewritten,
            "the branch on the tip went along"
        );
        assert_eq!(
            repo.git(&["branch", "--contains", &tip]),
            "",
            "no branch holds the old tip"
        );

        let id = session
            .drop_commit(rewritten.clone())
            .expect("the drop is taken");
        assert_eq!(write_answer(&sink, id).await, None);
        assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "root");
        assert_eq!(
            repo.git(&["rev-parse", "keep"]),
            repo.git(&["rev-parse", "HEAD"]),
            "dropping the tip itself takes it along too"
        );
        assert_eq!(repo.git(&["branch", "--contains", &rewritten]), "");
        session.close_and_wait().await;
    }
}
