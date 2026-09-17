//! Cherry-pick and revert, including the ones that land as nothing.

use crate::support::TestRepo;
use crate::support::exec::{env, observed_env};
use crate::support::integrate::{conflicting_branches, current_op};
use platitude_core::integrate::{self, Continuation, InProgress, Landing};
use platitude_core::opstate;
use platitude_core::process::Kept;

/// Both halves are gone from disk: the marker a badge reads, and the
/// sequence a `--skip` steps. The two come apart — a revert that
/// records nothing leaves the sequence without the marker, so
/// `current_op` on its own calls a half-walked sequence
/// clean.
async fn nothing_in_progress(repo: &TestRepo) {
    assert_eq!(current_op(repo).await, None, "no operation for a badge");
    let (exec, cancel) = env();
    assert!(
        !opstate::sequence_pending(&exec, &repo.path, &cancel)
            .await
            .expect("sequence state"),
        "no sequence left standing"
    );
}

#[tokio::test]
async fn cherry_pick_and_revert() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    let picked = repo.commit_file_id("b.txt", "two\n", "wanted elsewhere");
    repo.git(&["checkout", "main"]);
    let (exec, cancel) = env();

    integrate::cherry_pick(&exec, &repo.path, &[picked], &cancel)
        .await
        .expect("cherry-pick");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "wanted elsewhere");
    assert!(repo.path.join("b.txt").exists());

    integrate::revert(&exec, &repo.path, &["HEAD".into()], &cancel)
        .await
        .expect("revert");
    assert!(!repo.path.join("b.txt").exists());
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        r#"Revert "wanted elsewhere""#
    );
    assert_eq!(current_op(&repo).await, None);
}

/// A commit whose changes the branch already has records nothing, and
/// git stops there — exit 1 with the sequencer state left standing,
/// which is a badge on the toolbar and a panel over the log for
/// something nobody has to do anything about. The branch is left
/// exactly as it was, with no operation in progress
/// (デザイン規約 §履歴を合流させる).
#[tokio::test]
async fn a_cherry_pick_the_branch_already_has_leaves_nothing_behind() {
    use crate::support::Ends;
    use platitude_core::process::CommandEnd;
    use std::sync::Arc;

    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    let picked = repo.commit_file_id("f.txt", "base\nsame\n", "the change");
    repo.git(&["checkout", "main"]);
    // main arrives at the identical content under a commit of its own,
    // so replaying `picked` here has nothing left to write.
    repo.commit_file("f.txt", "base\nsame\n", "the same change, arrived at here");
    let before = repo.git(&["rev-parse", "HEAD"]);

    let ends = Arc::new(Ends::default());
    let (exec, cancel) = observed_env(ends.clone(), Kept::Asked);
    integrate::cherry_pick(&exec, &repo.path, &[picked], &cancel)
        .await
        .expect("a pick with nothing in it is not a failure");

    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        before,
        "no commit written"
    );
    nothing_in_progress(&repo).await;
    assert_eq!(repo.git(&["status", "--porcelain"]), "", "tree untouched");

    // The stop is an answer, so the log keeps the row without raising
    // itself over it (規約 §終了コードで答える問い合わせ).
    let recorded = ends.0.lock().unwrap().clone();
    assert!(
        recorded
            .iter()
            .any(|end| matches!(end, CommandEnd::Answered(1))),
        "the stop was recorded as an answer: {recorded:?}"
    );
    assert!(
        !recorded
            .iter()
            .any(|end| matches!(end, CommandEnd::Exited(code) if *code != 0)),
        "nothing here failed: {recorded:?}"
    );
}

/// A commit that was empty when it was made is what was asked for, so it
/// lands as it stands. That is what `--allow-empty` buys: without it git
/// stops on those too, in the very same words as the pick above — and
/// the two are not the same answer (measured, 2.55).
#[tokio::test]
async fn a_commit_that_was_always_empty_is_picked_as_it_stands() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.git(&["commit", "--allow-empty", "-m", "a marker of its own"]);
    let empty = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["checkout", "main"]);
    let (exec, cancel) = env();

    integrate::cherry_pick(&exec, &repo.path, &[empty], &cancel)
        .await
        .expect("cherry-pick");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "a marker of its own"
    );
    nothing_in_progress(&repo).await;
}

/// The commits either side of an empty one still land: `--skip` moves
/// the sequence on.
#[tokio::test]
async fn the_commits_around_an_empty_pick_still_land() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    let first = repo.commit_file_id("a.txt", "one\n", "before");
    let empty = repo.commit_file_id("f.txt", "base\nsame\n", "the change");
    let last = repo.commit_file_id("c.txt", "three\n", "after");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "base\nsame\n", "the same change, arrived at here");
    let (exec, cancel) = env();

    integrate::cherry_pick(&exec, &repo.path, &[first, empty, last], &cancel)
        .await
        .expect("cherry-pick");
    assert_eq!(
        repo.git(&["log", "-3", "--format=%s"]),
        "after\nbefore\nthe same change, arrived at here"
    );
    nothing_in_progress(&repo).await;
}

/// A revert with nothing left to undo stops before the sequencer: git
/// refuses the commit it was about to write and the operation is over
/// where it stands, so there is nothing to skip. The wording is
/// `git commit`'s own, on stdout with stderr empty (measured, 2.55).
#[tokio::test]
async fn a_revert_with_nothing_left_to_undo_lands_as_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    let added = repo.commit_file_id("f.txt", "one\ntwo\n", "adds the line");
    repo.commit_file("f.txt", "one\n", "takes it back by hand");
    let before = repo.git(&["rev-parse", "HEAD"]);
    let (exec, cancel) = env();

    integrate::revert(&exec, &repo.path, &[added], &cancel)
        .await
        .expect("a revert with nothing in it is not a failure");
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        before,
        "no commit written"
    );
    nothing_in_progress(&repo).await;
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

/// Several reverts do go through the sequencer, and an empty one there
/// stops it with the state standing — the same walk past as the pick,
/// reached by a different message.
#[tokio::test]
async fn the_commits_around_an_empty_revert_still_land() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    let added = repo.commit_file_id("f.txt", "one\ntwo\n", "adds the line");
    repo.commit_file("f.txt", "one\n", "takes it back by hand");
    let keeps = repo.commit_file_id("k.txt", "keep\n", "adds k");
    let (exec, cancel) = env();

    integrate::revert(&exec, &repo.path, &[added, keeps], &cancel)
        .await
        .expect("revert");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        r#"Revert "adds k""#
    );
    assert!(!repo.path.join("k.txt").exists());
    nothing_in_progress(&repo).await;
}

/// [`conflicting_branches`], and the commit a pick of `side` replays.
fn conflicting_sides() -> (TestRepo, String) {
    let mut repo = conflicting_branches();
    let picked = repo.git(&["rev-parse", "side"]);
    (repo, picked)
}

#[tokio::test]
async fn a_conflicting_cherry_pick_is_routed_to_the_right_command() {
    let (repo, picked) = conflicting_sides();
    let (exec, cancel) = env();

    // A landing of its own: git stopped and left the cherry-pick
    // standing — copying a commit onto a branch that has moved on
    // ends here as ordinarily as a merge does
    // (デザイン規約 §進行中の操作から出る).
    assert_eq!(
        integrate::cherry_pick(&exec, &repo.path, &[picked], &cancel)
            .await
            .expect("a conflict is an answer, not a failure"),
        Landing::Stopped
    );
    assert_eq!(current_op(&repo).await, Some(InProgress::CherryPick));

    integrate::resolve_current(&exec, &repo.path, Continuation::Abort, &cancel)
        .await
        .expect("abort");
    assert_eq!(current_op(&repo).await, None);
}

/// The revert half of the same answer, and the command log's side of it:
/// the stop is recorded as an answer, so nothing raises the panel over a
/// conflict there is a card for (規約 §終了コードで答える問い合わせ).
#[tokio::test]
async fn a_conflicting_revert_stops_rather_than_failing() {
    use crate::support::Ends;
    use platitude_core::process::CommandEnd;
    use std::sync::Arc;

    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    let added = repo.commit_file_id("f.txt", "one\ntwo\n", "adds the line");
    // The line the revert wants to take away is not the line that is
    // there any more, so undoing that commit collides.
    repo.commit_file("f.txt", "one\nrewritten\n", "says it another way");

    let ends = Arc::new(Ends::default());
    let (exec, cancel) = observed_env(ends.clone(), Kept::Asked);
    assert_eq!(
        integrate::revert(&exec, &repo.path, &[added], &cancel)
            .await
            .expect("a conflict is an answer, not a failure"),
        Landing::Stopped
    );
    assert_eq!(current_op(&repo).await, Some(InProgress::Revert));

    let recorded = ends.0.lock().unwrap().clone();
    assert!(
        !recorded
            .iter()
            .any(|end| matches!(end, CommandEnd::Exited(code) if *code != 0)),
        "nothing here failed: {recorded:?}"
    );

    integrate::resolve_current(&exec, &repo.path, Continuation::Abort, &cancel)
        .await
        .expect("abort");
    assert_eq!(current_op(&repo).await, None);
}

/// A stop is the one failure that turns into a landing. Everything else
/// travels on as the failure it looks like — git leaves nothing standing
/// for those (measured, 2.55: exit 128 and no marker of any kind), so the
/// screen gets git's own words.
#[tokio::test]
async fn a_failure_that_left_nothing_standing_is_still_a_failure() {
    let (repo, _) = conflicting_sides();
    let (exec, cancel) = env();

    integrate::cherry_pick(&exec, &repo.path, &["no-such-rev".into()], &cancel)
        .await
        .expect_err("a name git cannot read is a failure");
    nothing_in_progress(&repo).await;

    // A tree in the way is refused before anything is replayed, which is
    // not the operation standing there either.
    std::fs::write(repo.path.join("f.txt"), "still being typed\n").expect("dirty the tree");
    integrate::revert(&exec, &repo.path, &["HEAD".into()], &cancel)
        .await
        .expect_err("work in the way is a failure");
    nothing_in_progress(&repo).await;
}

/// The failure that leaves this operation's *own* marker standing: a
/// cherry-pick asked for while one is already in progress. git spends
/// 128 on it with `CHERRY_PICK_HEAD` right there (measured, 2.55), so reading
/// the marker without the code would call it a stop — and the sentence
/// saying what is really in the way would never reach the screen.
#[tokio::test]
async fn a_second_pick_over_one_already_standing_is_still_a_failure() {
    let (mut repo, picked) = conflicting_sides();
    let (exec, cancel) = env();

    assert_eq!(
        integrate::cherry_pick(&exec, &repo.path, std::slice::from_ref(&picked), &cancel)
            .await
            .expect("the first one stops"),
        Landing::Stopped
    );
    // Resolved and staged, so what refuses the second one is the
    // operation standing.
    std::fs::write(repo.path.join("f.txt"), "resolved\n").expect("resolve");
    repo.git(&["add", "--", "f.txt"]);

    integrate::cherry_pick(&exec, &repo.path, &[picked], &cancel)
        .await
        .expect_err("a pick over one already standing is a failure");
    assert_eq!(
        current_op(&repo).await,
        Some(InProgress::CherryPick),
        "and the one that was standing is still standing"
    );
}

#[tokio::test]
async fn resolving_with_nothing_in_progress_is_a_no_op() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();
    assert!(
        !integrate::resolve_current(&exec, &repo.path, Continuation::Abort, &cancel)
            .await
            .expect("no-op"),
        "nothing was in progress"
    );
}
