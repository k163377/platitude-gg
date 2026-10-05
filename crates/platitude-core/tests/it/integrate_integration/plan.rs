//! The interactive-rebase screen's preview, and the stop whose reason
//! only git's own markers can tell.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::helper;
use crate::support::remote::shallow_clone;
use platitude_core::error::GitError;
use platitude_core::integrate::{self, RebaseOutcome};
use platitude_core::rebase_plan::{self, PlanAnswer, PlanRefusal};
use platitude_core::sequencer::{self, RebaseStep, TodoAction};

fn four_commits() -> (TestRepo, Vec<String>) {
    let mut repo = TestRepo::init();
    let ids = vec![
        repo.commit_file_id("a.txt", "one\n", "root"),
        repo.commit_file_id("b.txt", "two\n", "second"),
        repo.commit_file_id("c.txt", "three\n", "third"),
        repo.commit_file_id("d.txt", "four\n", "fourth"),
    ];
    (repo, ids)
}

#[tokio::test]
async fn a_preview_lists_the_range_oldest_first_with_its_onto_row() {
    let (mut repo, ids) = four_commits();
    repo.git(&["branch", "base", &ids[0]]);
    let (exec, cancel) = env();

    let answer = rebase_plan::preview(&exec, &repo.path, &ids[1], &cancel)
        .await
        .expect("preview");
    let PlanAnswer::Plan(plan) = answer else {
        panic!("refused: {answer:?}");
    };
    assert_eq!(plan.from, ids[1]);
    assert_eq!(plan.upstream, ids[0], "the parent of the clicked commit");
    assert!(!plan.root);
    assert_eq!(
        plan.rows
            .iter()
            .map(|r| r.subject.as_str())
            .collect::<Vec<_>>(),
        vec!["second", "third", "fourth"],
        "oldest first — git's own todo order"
    );
    assert_eq!(plan.rows[0].oid, ids[1]);
    assert!(
        !plan.rows[0].author_name.is_empty() && plan.rows[0].author_email.contains('@'),
        "rows carry what a screen row draws: {:?}",
        plan.rows[0]
    );
    let onto = plan.onto.expect("an onto row");
    assert_eq!(onto.oid, ids[0]);
    assert_eq!(onto.subject, "root");
    assert_eq!(
        plan.onto_ref, "base",
        "a branch standing on the base is the name said first"
    );
}

#[tokio::test]
async fn a_preview_from_the_first_commit_needs_root_and_offers_no_onto() {
    let (repo, ids) = four_commits();
    let (exec, cancel) = env();

    let answer = rebase_plan::preview(&exec, &repo.path, &ids[0], &cancel)
        .await
        .expect("preview");
    let PlanAnswer::Plan(plan) = answer else {
        panic!("refused: {answer:?}");
    };
    assert!(plan.root);
    assert!(plan.upstream.is_empty());
    assert!(plan.onto.is_none());
    assert_eq!(plan.rows.len(), 4, "the whole history is the range");
}

#[tokio::test]
async fn a_range_holding_a_merge_is_refused() {
    let mut repo = TestRepo::init();
    let base = repo.commit_file_id("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("s.txt", "side\n", "side change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("m.txt", "main\n", "main change");
    repo.git(&["merge", "--no-ff", "-m", "merge side", "side"]);
    repo.commit_file("g.txt", "after\n", "after the merge");
    let (exec, cancel) = env();

    let answer = rebase_plan::preview(&exec, &repo.path, &base, &cancel)
        .await
        .expect("preview");
    assert_eq!(answer, PlanAnswer::Refused(PlanRefusal::AcrossMerge));
}

#[tokio::test]
async fn a_shallow_clones_edge_is_refused_rather_than_read_as_the_first_commit() {
    let (_source, clone, held) = shallow_clone(6, 3);
    assert_eq!(held.len(), 3, "the clone holds only what --depth asked for");
    let (exec, cancel) = env();

    let answer = rebase_plan::preview(&exec, &clone, &held[0], &cancel)
        .await
        .expect("preview");
    assert_eq!(
        answer,
        PlanAnswer::Refused(PlanRefusal::UnfetchedBase),
        "offered as --root instead, the replay rewrites the edge into a first commit and \
         cuts the branch off from the history this clone never fetched"
    );
}

#[tokio::test]
async fn a_commit_above_the_shallow_edge_still_plans_onto_it() {
    let (_source, clone, held) = shallow_clone(6, 3);
    let (exec, cancel) = env();

    let answer = rebase_plan::preview(&exec, &clone, &held[1], &cancel)
        .await
        .expect("preview");
    let PlanAnswer::Plan(plan) = answer else {
        panic!("refused: {answer:?}");
    };
    assert!(!plan.root, "the edge is a perfectly good upstream to name");
    assert_eq!(plan.upstream, held[0]);
    assert_eq!(plan.rows.len(), 2);
}

#[tokio::test]
async fn a_folder_that_is_not_a_repository_fails_instead_of_planning_from_the_root() {
    let (exec, cancel) = env();
    let dir = tempfile::tempdir().expect("tempdir");

    // git exits 128 here; only 1 (a revision looked for and not found) is an answer.
    let error = rebase_plan::preview(&exec, dir.path(), &"0".repeat(40), &cancel)
        .await
        .expect_err("nothing here to read");
    assert!(
        matches!(error, GitError::Failed { code: 128, .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_onto_name_is_the_one_the_graph_would_lead_with() {
    let (mut repo, ids) = four_commits();
    // Created out of order, one nested: the graph sorts chips by name after
    // kind and HEAD, and the base cannot be HEAD.
    for name in ["zulu", "feature/x", "alpha", "main-ish"] {
        repo.git(&["branch", name, &ids[0]]);
    }
    let (exec, cancel) = env();

    let answer = rebase_plan::preview(&exec, &repo.path, &ids[1], &cancel)
        .await
        .expect("preview");
    let PlanAnswer::Plan(plan) = answer else {
        panic!("refused: {answer:?}");
    };
    assert_eq!(plan.onto_ref, "alpha");

    // No branch there: the name is empty and the screen writes the short id
    // (デザイン規約 §フル interactive rebase).
    let answer = rebase_plan::preview(&exec, &repo.path, &ids[2], &cancel)
        .await
        .expect("preview");
    let PlanAnswer::Plan(plan) = answer else {
        panic!("refused: {answer:?}");
    };
    assert_eq!(plan.upstream, ids[1]);
    assert!(plan.onto_ref.is_empty());
    assert!(plan.onto.is_some(), "the row is still drawn, named or not");
}

#[tokio::test]
async fn a_click_on_another_branches_commit_is_refused() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    let side = repo.commit_file_id("s.txt", "side\n", "side change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("m.txt", "main\n", "main change");
    let (exec, cancel) = env();

    let answer = rebase_plan::preview(&exec, &repo.path, &side, &cancel)
        .await
        .expect("preview");
    assert_eq!(answer, PlanAnswer::Refused(PlanRefusal::OffBranch));
}

#[tokio::test]
async fn an_edit_stop_says_so_and_names_the_commit_it_sits_on() {
    let (mut repo, ids) = four_commits();
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let mut steps = sequencer::plan_for(&exec, &repo.path, &ids[1], &cancel)
        .await
        .expect("plan");
    assert_eq!(steps.len(), 2);
    steps[0].action = TodoAction::Edit; // stop on "third"
    let outcome = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        &ids[1],
        &steps,
        &Default::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("rebase");
    assert!(matches!(outcome, RebaseOutcome::Stopped), "{outcome:?}");

    let (progress, stop) = integrate::rebase_standing(&repo.path.join(".git"));
    assert!(stop.editing, "the amend marker is what says edit");
    assert!(!stop.oid.is_empty(), "git names the commit it stopped on");
    assert_eq!(
        progress.map(|p| (p.current, p.total)),
        Some((1, 2)),
        "the same read carries the step count"
    );
    let named = repo.git(&["rev-parse", &format!("{}^{{commit}}", stop.oid)]);
    assert_eq!(named.trim(), ids[2], "the marker names the edited commit");
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]).trim(),
        ids[2],
        "HEAD sits on the commit, which is what makes the amend box the tool"
    );

    integrate::resolve_current(
        &exec,
        &repo.path,
        integrate::Continuation::Continue,
        &cancel,
    )
    .await
    .expect("continue");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "fourth");
}

/// `rebase-merge/stopped-sha` keeps the todo's own id after an earlier step
/// rewrote it, naming a commit the graph is not drawing. The amend marker
/// holds the stop's HEAD, which the card names and the boxes amend.
#[tokio::test]
async fn an_edit_stop_after_a_reword_names_the_replayed_commit_not_the_todos() {
    let (mut repo, ids) = four_commits();
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let mut steps = sequencer::plan_for(&exec, &repo.path, &ids[1], &cancel)
        .await
        .expect("plan");
    assert_eq!(steps.len(), 2);
    steps[0].action = TodoAction::Reword; // "third" gets a new id...
    steps[0].message = Some("third reworded".to_string());
    steps[1].action = TodoAction::Edit; // ...so "fourth" is replayed onto it
    let outcome = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        &ids[1],
        &steps,
        &Default::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("rebase");
    assert!(matches!(outcome, RebaseOutcome::Stopped), "{outcome:?}");

    let head = repo.git(&["rev-parse", "HEAD"]).trim().to_string();
    assert_ne!(head, ids[3], "the replay gave the edited commit a new id");
    let (_, stop) = integrate::rebase_standing(&repo.path.join(".git"));
    assert!(stop.editing);
    assert_eq!(
        stop.oid, head,
        "the stop names the commit HEAD sits on, not the todo's own id"
    );
    let stopped_sha = std::fs::read_to_string(repo.path.join(".git/rebase-merge/stopped-sha"))
        .expect("stopped-sha");
    assert_eq!(stopped_sha.trim(), ids[3]);

    // A second amend must work too: git moves HEAD off the stopped id and
    // updates none of its markers, so nothing may key on HEAD being that id.
    repo.git(&["commit", "--amend", "-m", "fourth amended"]);
    let amended = repo.git(&["rev-parse", "HEAD"]).trim().to_string();
    assert_ne!(amended, head);
    let (_, after) = integrate::rebase_standing(&repo.path.join(".git"));
    assert!(after.editing, "the stop still stands");
    assert_eq!(after.oid, head, "and still names where it stopped");
    repo.git(&["commit", "--amend", "-m", "fourth amended twice"]);

    integrate::resolve_current(
        &exec,
        &repo.path,
        integrate::Continuation::Continue,
        &cancel,
    )
    .await
    .expect("continue takes both amends");
    assert_eq!(
        repo.git(&["log", "--format=%s", "-2"])
            .lines()
            .collect::<Vec<_>>(),
        ["fourth amended twice", "third reworded"]
    );
}

#[tokio::test]
async fn a_conflict_stop_is_not_an_edit_stop() {
    let mut repo = TestRepo::init();
    let first = repo.commit_file_id("f.txt", "one\n", "first");
    repo.commit_file("f.txt", "two\n", "second");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    // Dropping the commit the next one builds on forces the conflict.
    let steps = vec![
        RebaseStep {
            action: TodoAction::Drop,
            ..RebaseStep::pick(&first, "first")
        },
        sequencer::plan_for(&exec, &repo.path, &format!("{first}~0"), &cancel)
            .await
            .expect("plan")
            .pop()
            .expect("the newest step"),
    ];
    let outcome = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "",
        &steps,
        &platitude_core::integrate::RebaseOptions {
            root: true,
            ..Default::default()
        },
        &helper(),
        &cancel,
    )
    .await
    .expect("rebase");
    assert!(matches!(outcome, RebaseOutcome::Stopped), "{outcome:?}");

    let (_, stop) = integrate::rebase_standing(&repo.path.join(".git"));
    assert!(!stop.editing, "a conflicted stop carries no amend marker");

    integrate::resolve_current(&exec, &repo.path, integrate::Continuation::Abort, &cancel)
        .await
        .expect("abort");
}
