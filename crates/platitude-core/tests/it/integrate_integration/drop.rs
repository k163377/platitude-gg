//! Dropping a commit out from under the history that follows it.

use crate::support::TestRepo;
use crate::support::exec::{env, observed_env};
use crate::support::integrate::apply;
use platitude_core::process::Kept;
use platitude_core::sequencer;

#[tokio::test]
async fn dropping_a_commit_keeps_the_ones_after_it() {
    let mut repo = TestRepo::init();
    let kept = repo.commit_file_id("a.txt", "one\n", "root");
    let doomed = repo.commit_file_id("b.txt", "two\n", "the one to go");
    repo.commit_file("c.txt", "three\n", "after it");
    let (exec, cancel) = env();

    let plan = sequencer::plan_edit(&exec, &repo.path, &doomed, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    apply(&repo, &plan).await;

    assert_eq!(
        repo.git(&["log", "--format=%s"])
            .lines()
            .collect::<Vec<_>>(),
        vec!["after it", "root"]
    );
    assert!(!repo.path.join("b.txt").exists());
    assert!(repo.path.join("c.txt").exists());
    // The dropped commit's parent is the upstream, so nothing below the
    // gap is replayed.
    assert_eq!(repo.git(&["rev-parse", "HEAD~1"]), kept);
}

/// The newest commit (nothing after it to replay) and the first (no parent
/// to start the plan from).
#[tokio::test]
async fn dropping_at_either_end_of_the_history() {
    let (exec, cancel) = env();

    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let newest = repo.commit_file_id("b.txt", "two\n", "the newest");
    let plan = sequencer::plan_edit(&exec, &repo.path, &newest, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    apply(&repo, &plan).await;
    assert_eq!(repo.git(&["log", "--format=%s"]), "root");

    // The first commit.
    let mut repo = TestRepo::init();
    let first = repo.commit_file_id("a.txt", "one\n", "the first");
    repo.commit_file("b.txt", "two\n", "the second");
    let plan = sequencer::plan_edit(&exec, &repo.path, &first, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    assert!(plan.root, "no parent, so the plan reaches the root");
    apply(&repo, &plan).await;
    assert_eq!(repo.git(&["log", "--format=%s"]), "the second");
    assert!(!repo.path.join("a.txt").exists());
}

/// Planning against the first commit asks for a parent that is not there;
/// unmarked, that "no" opens the panel over a good drop
/// (core.md「終了コードで答える問い合わせは」).
#[tokio::test]
async fn reaching_past_the_first_commit_is_an_answer_not_a_failure() {
    use crate::support::Ends;
    use platitude_core::process::CommandEnd;
    use std::sync::Arc;

    let mut repo = TestRepo::init();
    let first = repo.commit_file_id("a.txt", "one\n", "the first");
    repo.commit_file("b.txt", "two\n", "the second");

    let ends = Arc::new(Ends::default());
    let (executor, cancel) = observed_env(ends.clone(), Kept::Asked);
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

/// The replay stands on a merge below the commit, so it keeps both
/// parents. Dropping the newest commit is the edge: a plan that reaches
/// one commit too far refuses over a merge it never touches.
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

    // The newest commit, straight on the merge.
    let newest = repo.commit_file_id("b.txt", "two\n", "on top of the merge");
    let plan = sequencer::plan_edit(&exec, &repo.path, &newest, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    assert_eq!(plan.upstream, merge, "the merge is the ground, not a step");
    apply(&repo, &plan).await;
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), merge);
    assert!(!repo.path.join("b.txt").exists());

    // And with a commit after it to replay.
    let doomed = repo.commit_file_id("c.txt", "three\n", "the one to go");
    repo.commit_file("d.txt", "four\n", "after it");
    let plan = sequencer::plan_edit(&exec, &repo.path, &doomed, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    apply(&repo, &plan).await;

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
