//! Merge / rebase / cherry-pick / revert, the conflict flow they share, and
//! interactive rebase driven by the todo-editor helper.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use crate::support::TestRepo;
use platitude_core::conflict::{self, ConflictKind, Side};
use platitude_core::integrate::{self, Continuation, InProgress, MergeOptions, RebaseOptions};
use platitude_core::process::GitExecutor;
use platitude_core::repo::RepoInfo;
use platitude_core::sequencer::{self, RebaseStep, TodoAction};
use platitude_core::{opstate, publish, status};
use tokio_util::sync::CancellationToken;

fn env() -> (GitExecutor, CancellationToken) {
    (GitExecutor::new(), CancellationToken::new())
}

async fn info(repo: &TestRepo) -> RepoInfo {
    let (exec, cancel) = env();
    platitude_core::repo::open(&exec, &repo.path, &cancel)
        .await
        .expect("open repo")
}

/// The helper Cargo built for this test run.
fn helper() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"))
}

/// main and side both change the same line of `f.txt`.
fn conflicting_branches() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("f.txt", "side\n", "side change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo
}

async fn current_op(repo: &TestRepo) -> Option<InProgress> {
    let (exec, cancel) = env();
    let state = opstate::detect(&exec, &repo.path, &cancel)
        .await
        .expect("op state");
    InProgress::from_state(&state)
}

#[tokio::test]
async fn merge_fast_forward_and_no_ff() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("b.txt", "two\n", "side work");
    repo.git(&["checkout", "main"]);
    let (exec, cancel) = env();

    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect("fast-forward merge");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "side work");
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");

    repo.git(&["checkout", "-b", "other", "HEAD~1"]);
    repo.commit_file("c.txt", "three\n", "other work");
    integrate::merge(
        &exec,
        &repo.path,
        "main",
        &MergeOptions {
            no_ff: true,
            message: Some("explicit merge".into()),
            ..Default::default()
        },
        &cancel,
    )
    .await
    .expect("no-ff merge");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "explicit merge");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%P"]).split(' ').count(),
        2,
        "a merge commit was recorded"
    );
}

#[tokio::test]
async fn ff_only_merge_refuses_a_real_merge() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();
    let err = integrate::merge(
        &exec,
        &repo.path,
        "side",
        &MergeOptions {
            ff_only: true,
            ..Default::default()
        },
        &cancel,
    )
    .await
    .expect_err("not a fast-forward");
    assert!(err.to_string().contains("fast-forward"), "{err}");
}

#[tokio::test]
async fn a_conflicting_merge_is_reported_then_aborted() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();

    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect_err("conflict stops the merge");

    assert_eq!(current_op(&repo).await, Some(InProgress::Merge));
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    let files = conflict::conflicted(&s);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "f.txt");
    assert_eq!(files[0].kind, ConflictKind::BothModified);
    assert!(files[0].kind.is_content_conflict());

    assert!(
        integrate::resolve_current(&exec, &repo.path, Continuation::Abort, &cancel)
            .await
            .expect("abort"),
        "something was in progress"
    );
    assert_eq!(current_op(&repo).await, None);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "main\n"
    );
}

#[tokio::test]
async fn merge_does_not_offer_skip() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();
    let err = integrate::resolve(
        &exec,
        &repo.path,
        InProgress::Merge,
        Continuation::Skip,
        &cancel,
    )
    .await
    .expect_err("merge cannot skip");
    assert!(err.to_string().contains("does not support"), "{err}");
}

#[tokio::test]
async fn resolving_a_conflict_by_taking_one_side_lets_the_merge_continue() {
    let mut repo = conflicting_branches();
    let (exec, cancel) = env();
    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect_err("conflict");

    conflict::take_side(&exec, &repo.path, &["f.txt".into()], Side::Theirs, &cancel)
        .await
        .expect("take theirs");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "side\n"
    );

    integrate::resolve_current(&exec, &repo.path, Continuation::Continue, &cancel)
        .await
        .expect("continue");
    assert_eq!(current_op(&repo).await, None);
    assert_eq!(
        repo.git(&["log", "-1", "--format=%P"]).split(' ').count(),
        2
    );
}

#[tokio::test]
async fn rebase_replays_commits_onto_the_upstream() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("b.txt", "two\n", "topic one");
    repo.commit_file("c.txt", "three\n", "topic two");
    repo.git(&["checkout", "main"]);
    repo.commit_file("d.txt", "four\n", "main moved");
    repo.git(&["checkout", "topic"]);
    let (exec, cancel) = env();

    integrate::rebase(
        &exec,
        &repo.path,
        "main",
        &RebaseOptions::default(),
        &cancel,
    )
    .await
    .expect("rebase");

    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "4");
    let subjects = repo.git(&["log", "--format=%s"]);
    assert_eq!(
        subjects.lines().collect::<Vec<_>>(),
        vec!["topic two", "topic one", "main moved", "root"]
    );
}

#[tokio::test]
async fn a_conflicting_rebase_reports_progress_and_can_be_aborted() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("f.txt", "topic one\n", "topic one");
    repo.commit_file("g.txt", "extra\n", "topic two");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo.git(&["checkout", "topic"]);
    let (exec, cancel) = env();

    integrate::rebase(
        &exec,
        &repo.path,
        "main",
        &RebaseOptions::default(),
        &cancel,
    )
    .await
    .expect_err("conflict stops the rebase");

    assert_eq!(current_op(&repo).await, Some(InProgress::Rebase));
    let progress = conflict::rebase_progress(&exec, &repo.path, &cancel)
        .await
        .expect("progress")
        .expect("a rebase is running");
    assert_eq!(
        (progress.current, progress.total),
        (1, 2),
        "stopped on the first of two commits"
    );

    integrate::resolve_current(&exec, &repo.path, Continuation::Abort, &cancel)
        .await
        .expect("abort");
    assert_eq!(current_op(&repo).await, None);
    assert!(
        conflict::rebase_progress(&exec, &repo.path, &cancel)
            .await
            .expect("progress")
            .is_none()
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "topic two");
}

#[tokio::test]
async fn a_conflicting_rebase_can_be_skipped() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("f.txt", "topic\n", "doomed commit");
    repo.commit_file("g.txt", "extra\n", "keeper");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo.git(&["checkout", "topic"]);
    let (exec, cancel) = env();

    integrate::rebase(
        &exec,
        &repo.path,
        "main",
        &RebaseOptions::default(),
        &cancel,
    )
    .await
    .expect_err("conflict");
    integrate::resolve_current(&exec, &repo.path, Continuation::Skip, &cancel)
        .await
        .expect("skip the conflicting commit");

    assert_eq!(current_op(&repo).await, None);
    let subjects = repo.git(&["log", "--format=%s"]);
    assert!(!subjects.contains("doomed commit"), "got: {subjects}");
    assert!(subjects.contains("keeper"));
}

#[tokio::test]
async fn cherry_pick_and_revert() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    let picked = repo.commit_file("b.txt", "two\n", "wanted elsewhere");
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

#[tokio::test]
async fn a_conflicting_cherry_pick_is_routed_to_the_right_command() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    let picked = repo.commit_file("f.txt", "side\n", "side change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    let (exec, cancel) = env();

    integrate::cherry_pick(&exec, &repo.path, &[picked], &cancel)
        .await
        .expect_err("conflict");
    assert_eq!(current_op(&repo).await, Some(InProgress::CherryPick));

    integrate::resolve_current(&exec, &repo.path, Continuation::Abort, &cancel)
        .await
        .expect("abort");
    assert_eq!(current_op(&repo).await, None);
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

// --- interactive rebase -------------------------------------------------

#[tokio::test]
async fn interactive_rebase_reorders_and_drops_commits() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    repo.commit_file("c.txt", "three\n", "third");
    repo.commit_file("d.txt", "four\n", "fourth");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let plan = sequencer::plan_for(&exec, &repo.path, "HEAD~3", &cancel)
        .await
        .expect("plan");
    assert_eq!(
        plan.iter().map(|s| s.subject.as_str()).collect::<Vec<_>>(),
        vec!["second", "third", "fourth"],
        "the todo list is oldest first"
    );

    // Keep fourth, then second; drop third.
    let steps = vec![
        plan[2].clone(),
        plan[0].clone(),
        RebaseStep {
            action: TodoAction::Drop,
            ..plan[1].clone()
        },
    ];
    sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "HEAD~3",
        &steps,
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("interactive rebase");

    let subjects = repo.git(&["log", "--format=%s"]);
    assert_eq!(
        subjects.lines().collect::<Vec<_>>(),
        vec!["second", "fourth", "root"],
        "reordered, with third gone"
    );
    assert!(!repo.path.join("c.txt").exists());
}

#[tokio::test]
async fn interactive_rebase_squashes_and_rewords() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "keep me");
    repo.commit_file("c.txt", "three\n", "fold me in");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let plan = sequencer::plan_for(&exec, &repo.path, "HEAD~2", &cancel)
        .await
        .expect("plan");
    let steps = vec![
        RebaseStep {
            action: TodoAction::Reword,
            message: Some("reworded subject\n\nwith a body\n".into()),
            ..plan[0].clone()
        },
        RebaseStep {
            action: TodoAction::Fixup,
            ..plan[1].clone()
        },
    ];
    sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "HEAD~2",
        &steps,
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("interactive rebase");

    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B"]),
        "reworded subject\n\nwith a body"
    );
    assert!(
        repo.path.join("b.txt").exists() && repo.path.join("c.txt").exists(),
        "the folded commit's content survived"
    );
}

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

    let err = sequencer::plan_edit(
        &exec,
        &repo.path,
        &target,
        sequencer::Edit::Reword("nope\n".into()),
        &cancel,
    )
    .await
    .expect_err("a rebase would drop the merge");
    assert!(err.to_string().contains("merge commit"), "{err}");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "nothing ran");
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

#[tokio::test]
async fn interactive_rebase_stops_at_an_edit_step() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    repo.commit_file("c.txt", "three\n", "third");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let plan = sequencer::plan_for(&exec, &repo.path, "HEAD~2", &cancel)
        .await
        .expect("plan");
    let steps = vec![
        RebaseStep {
            action: TodoAction::Edit,
            ..plan[0].clone()
        },
        plan[1].clone(),
    ];
    // Stopping at an `edit` step is git's normal behaviour, reported as a
    // non-zero exit; the session surfaces it and refreshes.
    let _ = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "HEAD~2",
        &steps,
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await;

    assert_eq!(current_op(&repo).await, Some(InProgress::Rebase));
    let progress = conflict::rebase_progress(&exec, &repo.path, &cancel)
        .await
        .expect("progress")
        .expect("running");
    assert_eq!(progress.total, 2);

    integrate::resolve_current(&exec, &repo.path, Continuation::Continue, &cancel)
        .await
        .expect("continue");
    assert_eq!(current_op(&repo).await, None);
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "3");
}

#[tokio::test]
async fn the_helper_refuses_a_malformed_invocation() {
    let out = std::process::Command::new(helper())
        .arg("--wrong-flag")
        .arg("a")
        .arg("b")
        .output()
        .expect("run helper");
    assert!(
        !out.status.success(),
        "an unknown flag must fail the rebase"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown argument"));
}

/// The lookup the application uses at runtime, pointed at the directory
/// Cargo puts binaries in — the same arrangement packaging must keep.
#[tokio::test]
async fn the_helper_is_found_beside_the_other_binaries() {
    let built = helper();
    let dir = built.parent().expect("binary directory");
    assert_eq!(sequencer::helper_in(dir).expect("found"), built);
    assert!(Path::new(&built).is_file());
}

// --- published-history warning ------------------------------------------

#[tokio::test]
async fn publish_state_distinguishes_pushed_commits() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["config", "core.bare", "true"]);

    let mut work = TestRepo::init();
    work.git(&["remote", "add", "origin", &origin.file_url()]);
    work.git(&["fetch", "origin"]);
    work.git(&["checkout", "-b", "main", "origin/main"]);
    work.commit_file("b.txt", "two\n", "local one");
    work.commit_file("c.txt", "three\n", "local two");
    let (exec, cancel) = env();

    // Nothing pushed yet: the whole range is local.
    let state = publish::state_of(&exec, &work.path, "origin/main..HEAD", &cancel)
        .await
        .expect("state");
    assert_eq!((state.total, state.unpublished), (2, 2));
    assert!(!state.rewrites_published());

    work.git(&["push", "origin", "main"]);
    work.commit_file("d.txt", "four\n", "local three");

    // One commit past the remote; rewriting the last three touches two
    // commits the remote already has.
    let state = publish::state_of(&exec, &work.path, "HEAD~3..HEAD", &cancel)
        .await
        .expect("state");
    assert_eq!(state.total, 3);
    assert_eq!(state.unpublished, 1);
    assert_eq!(state.published(), 2);
    assert!(state.rewrites_published());

    // Amending the tip alone is safe; amending its parent is not.
    let tip = publish::state_of(&exec, &work.path, &publish::only("HEAD"), &cancel)
        .await
        .expect("state");
    assert!(!tip.rewrites_published());
    let parent = publish::state_of(&exec, &work.path, &publish::only("HEAD~1"), &cancel)
        .await
        .expect("state");
    assert!(parent.rewrites_published());
}

#[tokio::test]
async fn a_repository_without_remotes_has_nothing_published() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let (exec, cancel) = env();

    let state = publish::state_of(&exec, &repo.path, "HEAD~1..HEAD", &cancel)
        .await
        .expect("state");
    assert_eq!((state.total, state.unpublished), (1, 1));
    assert!(!state.rewrites_published());
}
