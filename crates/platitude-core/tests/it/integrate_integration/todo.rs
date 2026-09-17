//! Interactive rebase: the todo the helper writes, and the steps it
//! stops at.

use std::path::Path;

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::{current_op, helper};
use platitude_core::integrate::{self, Continuation, InProgress, RebaseOptions};
use platitude_core::sequencer::{self, RebaseStep, TodoAction};

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
    let (progress, _) = integrate::rebase_standing(&exec, &repo.path, &cancel)
        .await
        .expect("progress");
    assert_eq!(progress.expect("running").total, 2);

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

/// `rebase.rebaseMerges` is the person's own config, and it decides the
/// shape of the todo git writes: with it standing the list opens with
/// `label onto` / `reset onto` ahead of the picks (measured on 2.55).
/// The helper replaces that file, so the driven rebase says
/// `--no-rebase-merges` and the plan is the whole of what git is asked
/// for.
///
/// Two things are being held here, and neither is the history's shape —
/// the replaced todo makes the outcome the same either way today. First,
/// the flag has to work on the minimum git, which the run in that
/// container measures (`cargo xtask linux test -p platitude-core`).
/// Second, git has to take the config and the flag standing
/// together — the manual says the flag countermands the config, and this
/// is that sentence run.
#[tokio::test]
async fn a_plan_runs_whole_with_rebase_merges_set_in_the_config() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    repo.commit_file("c.txt", "three\n", "third");
    repo.git(&["config", "rebase.rebaseMerges", "true"]);
    assert_eq!(
        repo.git(&["config", "rebase.rebaseMerges"]),
        "true",
        "the config has to be standing for this to measure anything"
    );
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let plan = sequencer::plan_for(&exec, &repo.path, "HEAD~2", &cancel)
        .await
        .expect("plan");
    let steps = vec![
        RebaseStep {
            action: TodoAction::Drop,
            ..plan[0].clone()
        },
        plan[1].clone(),
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
    .expect("the plan runs with rebase.rebaseMerges standing");

    assert_eq!(current_op(&repo).await, None);
    assert_eq!(
        repo.git(&["log", "--format=%s"])
            .lines()
            .collect::<Vec<_>>(),
        vec!["third", "root"],
        "the plan replayed flat: the dropped commit is gone and nothing else"
    );
    assert!(!repo.path.join("b.txt").exists());
}

/// **`--update-refs` works because git's own todo is kept.**
///
/// git writes an `update-ref` line after the commit each local branch
/// inside the range stands on, and decides that set itself: no line for a
/// tag, none for a branch outside the range, and **none for a branch
/// another working copy has checked out** — that one it says so about in
/// a comment. The helper used to write the plan over the whole file, so
/// every one of those lines went and the flag moved nothing (measured,
/// 2.55; P3-確認事項 §A). Working the set out here instead would be a
/// second implementation of a rule git already applies, and the two would
/// disagree exactly where it costs most.
#[tokio::test]
async fn update_refs_moves_the_branches_git_named_and_no_others() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "c1");
    repo.commit_file("b.txt", "two\n", "c2");
    repo.commit_file("c.txt", "three\n", "c3");
    repo.commit_file("d.txt", "four\n", "c4");
    // Inside the range the rebase replays, outside it, on a tag, and on a
    // branch a second working copy is standing on.
    repo.git(&["branch", "inside", "HEAD~1"]);
    repo.git(&["branch", "outside", "HEAD~3"]);
    repo.git(&["tag", "t-inside", "HEAD~1"]);
    repo.git(&["branch", "held", "HEAD~2"]);
    let elsewhere = repo.path.with_file_name("held-copy");
    let at = elsewhere.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", &at, "held"]);
    let was_inside = repo.git(&["rev-parse", "inside"]);
    let was_held = repo.git(&["rev-parse", "held"]);
    let was_outside = repo.git(&["rev-parse", "outside"]);
    let was_tag = repo.git(&["rev-parse", "t-inside"]);

    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let mut steps = sequencer::plan_for(&exec, &repo.path, "HEAD~3", &cancel)
        .await
        .expect("plan");
    // A reword of the oldest commit in the range, so every id after it
    // has to change — which is the whole of what a ref has to follow.
    steps[0].action = TodoAction::Reword;
    steps[0].message = Some("c2 said again".into());
    let options = RebaseOptions {
        update_refs: true,
        ..RebaseOptions::default()
    };
    let outcome = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "HEAD~3",
        &steps,
        &options,
        &helper(),
        &cancel,
    )
    .await
    .expect("the replay runs");
    assert!(
        matches!(outcome, integrate::RebaseOutcome::Done),
        "{outcome:?}"
    );

    assert_eq!(
        repo.git(&["log", "--format=%s"])
            .lines()
            .collect::<Vec<_>>(),
        vec!["c4", "c3", "c2 said again", "c1"],
        "the reword landed, so every id above it moved"
    );
    assert_ne!(
        repo.git(&["rev-parse", "inside"]),
        was_inside,
        "the branch inside the range followed the rewrite"
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", "inside"]),
        "c3",
        "and it is on the commit it was on, rewritten"
    );
    assert_eq!(
        repo.git(&["rev-parse", "held"]),
        was_held,
        "the branch another working copy has checked out is left where it was — \
         git writes no line for one, and this end does not write one either"
    );
    assert_eq!(repo.git(&["rev-parse", "outside"]), was_outside);
    assert_eq!(
        repo.git(&["rev-parse", "t-inside"]),
        was_tag,
        "and a tag is not a ref `--update-refs` moves"
    );
}

/// The refs are put back with everything else when the replay is called
/// off part-way, so a plan abandoned over a conflict leaves nothing
/// half-moved.
#[tokio::test]
async fn an_aborted_replay_puts_the_refs_it_would_have_moved_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "c1");
    repo.commit_file("f.txt", "two\n", "c2");
    repo.commit_file("f.txt", "three\n", "c3");
    repo.git(&["branch", "inside", "HEAD~1"]);
    let was_inside = repo.git(&["rev-parse", "inside"]);
    let was_head = repo.git(&["rev-parse", "HEAD"]);

    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let mut steps = sequencer::plan_for(&exec, &repo.path, "HEAD~2", &cancel)
        .await
        .expect("plan");
    // Edit the first of them: the replay stops there with the refs still
    // only promised.
    steps[0].action = TodoAction::Edit;
    let options = RebaseOptions {
        update_refs: true,
        ..RebaseOptions::default()
    };
    let outcome = sequencer::rebase_interactive(
        &exec,
        &repo_info,
        "HEAD~2",
        &steps,
        &options,
        &helper(),
        &cancel,
    )
    .await
    .expect("the replay runs");
    assert!(
        matches!(outcome, integrate::RebaseOutcome::Stopped),
        "{outcome:?}"
    );
    assert_eq!(current_op(&repo).await, Some(InProgress::Rebase));

    integrate::resolve(
        &exec,
        &repo.path,
        InProgress::Rebase,
        Continuation::Abort,
        &cancel,
    )
    .await
    .expect("abort");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), was_head);
    assert_eq!(
        repo.git(&["rev-parse", "inside"]),
        was_inside,
        "the branch is where it was, not where the replay was taking it"
    );
}
