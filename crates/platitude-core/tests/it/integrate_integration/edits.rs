//! Squashing and rewording a commit already down in the history.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::helper;
use platitude_core::integrate::{RebaseOptions, RebaseOutcome};
use platitude_core::sequencer::{self, RebaseStep, TodoAction};

// --- one-commit edits (squash into parent / reword) ----------------------

/// Runs whatever `plan_edit` produced, the way the session does.
async fn apply(repo: &TestRepo, plan: &sequencer::EditPlan) {
    let (exec, cancel) = env();
    let repo_info = info(repo).await;
    sequencer::rebase_interactive(
        &exec,
        &repo_info,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("run the plan");
}

#[tokio::test]
async fn squash_into_parent_folds_one_commit_and_keeps_the_rest() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "keep me");
    let target = repo.commit_file("c.txt", "three\n", "fold me in");
    repo.commit_file("d.txt", "four\n", "after");
    let (exec, cancel) = env();

    let plan = sequencer::plan_edit(
        &exec,
        &repo.path,
        &target,
        sequencer::Edit::SquashIntoParent,
        &cancel,
    )
    .await
    .expect("plan");
    assert!(!plan.root, "history is deep enough to name an upstream");
    apply(&repo, &plan).await;

    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "3");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "after");
    // The squashed pair kept both messages and both files.
    let folded = repo.git(&["log", "-1", "--format=%B", "HEAD~1"]);
    assert!(
        folded.contains("keep me") && folded.contains("fold me in"),
        "{folded}"
    );
    assert!(repo.path.join("c.txt").exists() && repo.path.join("d.txt").exists());
}

#[tokio::test]
async fn squashing_the_second_commit_reaches_back_to_the_root() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let target = repo.commit_file("b.txt", "two\n", "second");
    let (exec, cancel) = env();

    let plan = sequencer::plan_edit(
        &exec,
        &repo.path,
        &target,
        sequencer::Edit::SquashIntoParent,
        &cancel,
    )
    .await
    .expect("plan");
    assert!(plan.root, "the root has no parent to name as upstream");
    assert!(plan.upstream.is_empty());
    apply(&repo, &plan).await;

    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "1");
    assert!(repo.path.join("a.txt").exists() && repo.path.join("b.txt").exists());
}

#[tokio::test]
async fn the_first_commit_has_nothing_to_fold_into() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let (exec, cancel) = env();

    let err = sequencer::plan_edit(
        &exec,
        &repo.path,
        &root,
        sequencer::Edit::SquashIntoParent,
        &cancel,
    )
    .await
    .expect_err("nothing before the root");
    assert!(err.to_string().contains("first commit"), "{err}");
}

#[tokio::test]
async fn rewording_an_older_commit_replaces_only_its_message() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let target = repo.commit_file("b.txt", "two\n", "old subject");
    repo.commit_file("c.txt", "three\n", "after");
    let (exec, cancel) = env();

    let plan = sequencer::plan_edit(
        &exec,
        &repo.path,
        &target,
        sequencer::Edit::Reword("new subject\n\nwith a body\n".into()),
        &cancel,
    )
    .await
    .expect("plan");
    apply(&repo, &plan).await;

    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "3");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B", "HEAD~1"]),
        "new subject\n\nwith a body"
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "after");
}

#[tokio::test]
async fn rewording_the_root_commit_works_through_root_mode() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let (exec, cancel) = env();

    let plan = sequencer::plan_edit(
        &exec,
        &repo.path,
        &root,
        sequencer::Edit::Reword("renamed root\n".into()),
        &cancel,
    )
    .await
    .expect("plan");
    assert!(plan.root);
    apply(&repo, &plan).await;

    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", "HEAD~1"]),
        "renamed root"
    );
}

#[tokio::test]
async fn a_commit_outside_the_current_branch_is_refused() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    let elsewhere = repo.commit_file("s.txt", "side\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("m.txt", "main\n", "main work");
    let (exec, cancel) = env();

    let err = sequencer::plan_edit(
        &exec,
        &repo.path,
        &elsewhere,
        sequencer::Edit::Reword("nope\n".into()),
        &cancel,
    )
    .await
    .expect_err("not in this history");
    assert!(err.to_string().contains("not in the history"), "{err}");
}

#[tokio::test]
async fn a_reword_without_a_message_is_refused_before_git_runs() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let before = repo.git(&["rev-parse", "HEAD"]);

    let steps = vec![RebaseStep {
        action: TodoAction::Reword,
        oid: before.clone(),
        subject: "second".into(),
        message: None,
    }];
    let err = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "HEAD~1",
        &steps,
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect_err("a reword needs a message");
    assert!(err.to_string().contains("no message"), "{err}");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "nothing ran");
}

/// A reword standing *behind* a conflicting step survives the stop: the
/// remaining todo's `exec` line reads its message file from the later
/// `--continue`, a different process entirely, so the file must still be
/// there — and once a later rebase runs clean, the leftovers are gone.
#[tokio::test]
async fn a_reword_behind_a_conflict_survives_the_stop_and_continue() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "base");
    repo.git(&["switch", "-c", "side"]);
    repo.commit_file("f.txt", "side\n", "their line");
    repo.git(&["switch", "main"]);
    let conflicting = repo.commit_file("f.txt", "ours\n", "our line");
    let reworded = repo.commit_file("g.txt", "g\n", "old words");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let steps = vec![
        RebaseStep::pick(conflicting, "our line"),
        RebaseStep {
            action: TodoAction::Reword,
            oid: reworded,
            subject: "old words".into(),
            message: Some("new words".into()),
        },
    ];
    let outcome = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "side",
        &steps,
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("run the plan");
    assert!(
        matches!(outcome, RebaseOutcome::Stopped),
        "the pick conflicts: {outcome:?}"
    );

    repo.write_file("f.txt", "settled\n");
    repo.git(&["add", "--", "f.txt"]);
    repo.git(&["rebase", "--continue"]);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "new words");

    // A clean rebase over the settled history sweeps what the stop left.
    let head = repo.git(&["rev-parse", "HEAD"]);
    let outcome = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "HEAD~1",
        &[RebaseStep::pick(head, "new words")],
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("a clean run");
    assert!(matches!(outcome, RebaseOutcome::Done), "{outcome:?}");
    let leftovers: Vec<_> = std::fs::read_dir(repo.path.join(".git").join("platitude"))
        .map(|entries| entries.flatten().map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(leftovers.is_empty(), "swept: {leftovers:?}");
}
