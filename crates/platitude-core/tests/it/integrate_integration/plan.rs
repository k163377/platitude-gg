//! The interactive-rebase screen's preview, and the stop whose reason
//! only git's own markers can tell.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::helper;
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

// --- the preview ---------------------------------------------------------

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

// --- why the rebase stopped ----------------------------------------------

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

    let (progress, stop) = integrate::rebase_standing(&exec, &repo.path, &cancel)
        .await
        .expect("stop");
    assert!(stop.editing, "the amend marker is what says edit");
    assert!(!stop.oid.is_empty(), "git names the commit it stopped on");
    assert_eq!(
        progress.map(|p| (p.current, p.total)),
        Some((1, 2)),
        "the same read carries the step count"
    );
    let named = repo.git(&["rev-parse", &format!("{}^{{commit}}", stop.oid)]);
    assert_eq!(
        named.trim(),
        ids[2],
        "stopped-sha resolves to the edited commit"
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]).trim(),
        ids[2],
        "HEAD sits on the commit, which is what makes the amend box the tool"
    );

    // The way on is the exit card's own `--continue`.
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

    let (_, stop) = integrate::rebase_standing(&exec, &repo.path, &cancel)
        .await
        .expect("stop");
    assert!(!stop.editing, "a conflicted stop carries no amend marker");

    integrate::resolve_current(&exec, &repo.path, integrate::Continuation::Abort, &cancel)
        .await
        .expect("abort");
}
