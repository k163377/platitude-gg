//! Dropping a commit out from under the history that follows it.

use crate::support::TestRepo;
use crate::support::integrate::{env, helper, info};
use platitude_core::process::GitExecutor;
use platitude_core::sequencer;
use tokio_util::sync::CancellationToken;

/// Dropping one commit out of the middle leaves everything after it in
/// place, rewritten onto the gap.
#[tokio::test]
async fn dropping_a_commit_keeps_the_ones_after_it() {
    let mut repo = TestRepo::init();
    let kept = repo.commit_file("a.txt", "one\n", "root");
    let doomed = repo.commit_file("b.txt", "two\n", "the one to go");
    repo.commit_file("c.txt", "three\n", "after it");
    let (exec, cancel) = env();

    let plan = sequencer::plan_edit(&exec, &repo.path, &doomed, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("drop");

    assert_eq!(
        repo.git(&["log", "--format=%s"])
            .lines()
            .collect::<Vec<_>>(),
        vec!["after it", "root"]
    );
    // The commit's own file goes with it; the later one stays.
    assert!(!repo.path.join("b.txt").exists());
    assert!(repo.path.join("c.txt").exists());
    // The plan starts at the dropped commit's parent, and that parent is
    // the upstream rather than a step in it: nothing below the gap is
    // replayed, so it keeps the object name it had.
    assert_eq!(repo.git(&["rev-parse", "HEAD~1"]), kept);
}

/// The two edges of the same operation, measured rather than assumed:
/// the newest commit (nothing after it to replay) and the very first one
/// (no parent to start the plan from).
#[tokio::test]
async fn dropping_at_either_end_of_the_history() {
    let (exec, cancel) = env();

    // The newest commit.
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let newest = repo.commit_file("b.txt", "two\n", "the newest");
    let plan = sequencer::plan_edit(&exec, &repo.path, &newest, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("drop the newest");
    assert_eq!(repo.git(&["log", "--format=%s"]), "root");

    // The first commit, which has no parent to be the plan's upstream —
    // the plan says `--root` instead.
    let mut repo = TestRepo::init();
    let first = repo.commit_file("a.txt", "one\n", "the first");
    repo.commit_file("b.txt", "two\n", "the second");
    let plan = sequencer::plan_edit(&exec, &repo.path, &first, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    assert!(plan.root, "no parent, so the plan reaches the root");
    sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("drop the root");
    assert_eq!(repo.git(&["log", "--format=%s"]), "the second");
    assert!(!repo.path.join("a.txt").exists());
}

/// Planning against the first commit asks for a parent that is not
/// there, and that "no" is an answer rather than a failed command. Left
/// unmarked it counts as a failure, and the command log throws its panel
/// open over a perfectly good drop (規約 §終了コードで答える問い合わせ).
#[tokio::test]
async fn reaching_past_the_first_commit_is_an_answer_not_a_failure() {
    use crate::support::Ends;
    use platitude_core::process::CommandEnd;
    use std::sync::Arc;

    let mut repo = TestRepo::init();
    let first = repo.commit_file("a.txt", "one\n", "the first");
    repo.commit_file("b.txt", "two\n", "the second");

    let ends = Arc::new(Ends::default());
    let executor = GitExecutor::new().observed(Arc::clone(&ends) as _, true);
    let cancel = CancellationToken::new();
    let plan = sequencer::plan_edit(
        &executor,
        &repo.path,
        &first,
        sequencer::Edit::Drop,
        &cancel,
    )
    .await
    .expect("plan");
    assert!(plan.root);

    let recorded = ends.0.lock().unwrap().clone();
    assert!(
        !recorded.is_empty(),
        "the observer saw the commands go past"
    );
    assert!(
        !recorded
            .iter()
            .any(|end| matches!(end, CommandEnd::Exited(code) if *code != 0)),
        "nothing here failed: {recorded:?}"
    );
}

/// The refusal sits before the edit is even looked at (`plan_edit` checks
/// the range first), so a drop and a reword hit the same wall: a plain
/// interactive rebase would flatten the merge rather than replay it.
#[tokio::test]
async fn a_range_holding_a_merge_is_refused_rather_than_flattened() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let target = repo.commit_file("b.txt", "two\n", "before the merge");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("s.txt", "side\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("m.txt", "main\n", "main work");
    repo.git(&["merge", "--no-ff", "--no-edit", "side"]);
    let before = repo.git(&["rev-parse", "HEAD"]);
    let (exec, cancel) = env();

    for edit in [
        sequencer::Edit::Drop,
        sequencer::Edit::Reword("nope\n".into()),
    ] {
        let err = sequencer::plan_edit(&exec, &repo.path, &target, edit, &cancel)
            .await
            .expect_err("a rebase would drop the merge");
        assert!(err.to_string().contains("merge commit"), "{err}");
        // The refusal is this application's own, and no rebase ever ran.
        // Blaming it on a command's output puts a command the person
        // never saw in front of them (規約 §git が言ったことを読む場所).
        assert!(!err.to_string().contains("unexpected output"), "{err}");
    }
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "nothing ran");
}

/// A merge *below* the commit is not in the way: the replay stands on it
/// rather than repeating it, so it keeps both its parents. Dropping the
/// newest commit is the edge here: a plan that reaches one commit too
/// far refuses over a merge it was never going to touch.
#[tokio::test]
async fn a_merge_under_the_dropped_commit_is_left_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("s.txt", "side\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("m.txt", "main\n", "main work");
    repo.git(&["merge", "--no-ff", "--no-edit", "side"]);
    let merge = repo.git(&["rev-parse", "HEAD"]);
    let (exec, cancel) = env();

    // The newest commit, sitting straight on the merge: nothing follows
    // it, so the whole plan is the one drop.
    let newest = repo.commit_file("b.txt", "two\n", "on top of the merge");
    let plan = sequencer::plan_edit(&exec, &repo.path, &newest, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    assert_eq!(plan.upstream, merge, "the merge is the ground, not a step");
    sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("drop the newest");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), merge);
    assert!(!repo.path.join("b.txt").exists());

    // And with a commit after it to replay.
    let doomed = repo.commit_file("c.txt", "three\n", "the one to go");
    repo.commit_file("d.txt", "four\n", "after it");
    let plan = sequencer::plan_edit(&exec, &repo.path, &doomed, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("drop the one under the newest");

    assert_eq!(
        repo.git(&["log", "--format=%s"])
            .lines()
            .collect::<Vec<_>>(),
        vec![
            "after it",
            "Merge branch 'side'",
            "main work",
            "side work",
            "root"
        ],
        "the merge is still there, with both sides under it"
    );
    assert_eq!(
        repo.git(&["rev-list", "--merges", "--count", "HEAD"]),
        "1",
        "and it is still a merge"
    );
    assert!(!repo.path.join("c.txt").exists());
    assert!(repo.path.join("d.txt").exists());
}

/// The one shape that has no ground to land on. `--root` replays onto a
/// placeholder git makes up, so dropping every line leaves that
/// placeholder behind as the branch tip: an empty tree with no message
/// (measured). Refusing says so before anything moves.
#[tokio::test]
async fn dropping_the_only_commit_is_refused() {
    let mut repo = TestRepo::init();
    let only = repo.commit_file("a.txt", "one\n", "the only one");
    let (exec, cancel) = env();

    let plan = sequencer::plan_edit(&exec, &repo.path, &only, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    assert!(plan.root, "there is no parent to start at");
    let err = sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect_err("nothing would be left to point at");
    assert!(err.to_string().contains("every commit"), "{err}");
    assert!(!err.to_string().contains("unexpected output"), "{err}");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), only, "nothing ran");
}
