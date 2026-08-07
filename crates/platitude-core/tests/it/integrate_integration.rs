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

/// What reaches a person as "skip or not?" is never the easy case: a
/// commit whose change is already upstream *to the letter* is dropped by
/// git without stopping, so it never gets as far as the UI.
#[tokio::test]
async fn a_commit_already_upstream_verbatim_never_stops_the_rebase() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("f.txt", "same\n", "the very same change");
    repo.commit_file("g.txt", "extra\n", "keeper");
    repo.git(&["checkout", "main"]);
    // Byte-for-byte what topic did, landed upstream by another route.
    repo.commit_file("f.txt", "same\n", "someone else got there first");
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
    .expect("an identical change replays without stopping");

    assert_eq!(current_op(&repo).await, None, "nothing left to continue");
    let subjects = repo.git(&["log", "--format=%s"]);
    assert!(
        !subjects.contains("the very same change"),
        "git drops the emptied commit itself: {subjects}"
    );
    assert!(subjects.contains("keeper"));
}

/// And skipping is not free: the commit left out takes its own work with
/// it, wherever else that work does or does not exist. Only the reflog
/// holds it afterwards — the same standing a hard reset leaves behind,
/// which is the one this app already asks to be held for.
#[tokio::test]
async fn skipping_drops_work_that_is_nowhere_else() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.write_file("f.txt", "topic\n");
    repo.write_file("only-here.txt", "nowhere else\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-m", "conflicts, and carries its own file"]);
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo.git(&["checkout", "topic"]);
    let before = repo.git(&["rev-parse", "topic"]);
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
        .expect("skip");

    assert!(
        !repo.path.join("only-here.txt").exists(),
        "the skipped commit's own file goes with it"
    );
    // Reachable only by hash: no branch, no tag, nothing in the UI points
    // at it any more.
    let orphan = repo.git(&["log", "--format=%s", "-1", before.trim()]);
    assert_eq!(orphan, "conflicts, and carries its own file");
    let described = repo.git(&["log", "--format=%s", "--all"]);
    assert!(
        !described.contains("conflicts, and carries its own file"),
        "no ref reaches it: {described}"
    );
}

/// The free skip, and whether it reaches a person after all: resolving a
/// conflict by taking the upstream side wholesale leaves the commit with
/// nothing to say, and `--continue` has to decide what that means.
#[tokio::test]
async fn resolving_a_conflict_to_match_upstream_then_continuing() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("f.txt", "topic\n", "same idea, other words");
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
    // Taking upstream's side outright, which is what `Take theirs`-style
    // resolution does — and which leaves this commit contributing nothing.
    conflict::take_side(&exec, &repo.path, &["f.txt".into()], Side::Ours, &cancel)
        .await
        .expect("take the upstream side");

    // A plain rebase drops the emptied commit itself: `--empty=drop` is
    // the merge backend's default, so nobody is asked anything.
    integrate::resolve_current(&exec, &repo.path, Continuation::Continue, &cancel)
        .await
        .expect("continue carries on past the emptied commit");

    assert_eq!(current_op(&repo).await, None);
    let subjects = repo.git(&["log", "--format=%s"]);
    assert!(
        !subjects.contains("same idea, other words"),
        "the emptied commit went quietly: {subjects}"
    );
}

/// The other emptied-commit path, and the one that does reach a person:
/// interactive rebase — what this app drives for squash / reword / drop —
/// stops on a commit that came out empty and asks for `--skip` by name.
/// So there *is* a state where skipping costs nothing, and the gesture on
/// that row cannot be chosen from the plain rebase's behaviour alone.
#[tokio::test]
async fn an_interactive_rebase_stops_on_an_emptied_commit_and_names_skip() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    let doomed = repo.commit_file("f.txt", "a\nX\n", "adds X");
    repo.commit_file("h.txt", "keep\n", "keeper");
    repo.git(&["checkout", "main"]);
    // Same net line, different patch: not a clean cherry-pick of `doomed`,
    // so the cherry-pick filter cannot be what drops it.
    repo.write_file("f.txt", "a\nX\n");
    repo.write_file("g.txt", "unrelated\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-m", "X arrives with company"]);
    repo.git(&["checkout", "topic"]);
    let (exec, cancel) = env();

    let steps = vec![
        RebaseStep::pick(doomed.clone(), "adds X"),
        RebaseStep {
            action: TodoAction::Pick,
            oid: repo.git(&["rev-parse", "topic"]),
            subject: "keeper".into(),
            message: None,
        },
    ];
    let outcome = sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        "main",
        &steps,
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await;

    let message = match outcome {
        Ok(_) => "REBASE FINISHED".to_string(),
        Err(e) => e.to_string(),
    };
    assert_eq!(current_op(&repo).await, Some(InProgress::Rebase));
    assert!(
        message.contains("The previous cherry-pick is now empty"),
        "git's reason: {message}"
    );
    assert!(
        message.contains("git rebase --skip"),
        "git names the way out: {message}"
    );
    // Two flags are set at once here — the stopped pick leaves
    // CHERRY_PICK_HEAD behind — and only one of them is the operation.
    // Anything naming what is in progress has to ask `from_state`, not
    // list the flags: the badge did the latter and said
    // `REBASING · CHERRY-PICKING` for one rebase.
    let (exec2, cancel2) = env();
    let state = opstate::detect(&exec2, &repo.path, &cancel2)
        .await
        .expect("op state");
    assert!(state.rebasing && state.cherry_picking, "got: {state:?}");
    assert_eq!(InProgress::from_state(&state), Some(InProgress::Rebase));
    // And nothing is conflicted while it stands there, so a UI cannot
    // tell this stop from an `edit` stop by the file list alone.
    let state = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(state.conflicted().count(), 0);
}

/// What each stopped operation leaves behind to name its two sides by.
/// The UI calls them by branch name, and this is what there is to build
/// those names out of — measured, because the answer differs per
/// operation and per rebase backend.
#[tokio::test]
async fn what_a_stopped_operation_says_about_its_two_sides() {
    // ---- merge: HEAD is ours, MERGE_HEAD is theirs ----
    let mut repo = conflicting_branches();
    repo.git_expect_failure(&["merge", "side"]);
    let merge_head = repo.git(&["rev-parse", "MERGE_HEAD"]);
    assert!(!merge_head.is_empty());
    assert_eq!(
        repo.git(&[
            "name-rev",
            "--name-only",
            "--refs=refs/heads/*",
            &merge_head
        ]),
        "side",
        "the branch merged in is named by its tip"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");

    // ---- rebase, merge backend: head-name is theirs, onto is ours ----
    let mut repo = conflicting_branches();
    repo.git(&["checkout", "side"]);
    repo.git_expect_failure(&["rebase", "main"]);
    // The branch being replayed, as a full ref — the one side that comes
    // back already named.
    assert_eq!(
        read_git_file(&mut repo, "rebase-merge/head-name"),
        "refs/heads/side"
    );
    // The side being landed on is a bare object name, so it has to be
    // asked for by name separately.
    let onto = read_git_file(&mut repo, "rebase-merge/onto");
    assert_eq!(onto.len(), 40, "a raw sha, not a ref: {onto}");
    assert_eq!(
        repo.git(&["name-rev", "--name-only", "--refs=refs/heads/*", &onto]),
        "main"
    );

    // ---- rebase, apply backend: neither file is there ----
    let mut repo = conflicting_branches();
    repo.git(&["checkout", "side"]);
    repo.git_expect_failure(&["rebase", "--apply", "main"]);
    assert_eq!(
        read_git_file(&mut repo, "rebase-apply/head-name"),
        "refs/heads/side",
        "the apply backend keeps the same name under its own directory"
    );

    // ---- cherry-pick: the side coming in is a commit, not a branch ----
    let mut repo = conflicting_branches();
    repo.git_expect_failure(&["cherry-pick", "side"]);
    let picked = repo.git(&["rev-parse", "CHERRY_PICK_HEAD"]);
    assert!(!picked.is_empty());
    assert_eq!(
        repo.git(&["name-rev", "--name-only", "--refs=refs/heads/*", &picked]),
        "side",
        "a tip picks up its branch's name; an older commit would be side~N"
    );
}

/// And what `sides` makes of all that — including the reversal, which is
/// the whole reason the names are read rather than assumed.
#[tokio::test]
async fn sides_names_each_side_by_what_it_actually_is() {
    let (exec, cancel) = env();

    // A merge lands the other branch on this one.
    let repo = conflicting_branches();
    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect_err("conflict");
    let s = conflict::sides(&exec, &repo.path, InProgress::Merge, &cancel)
        .await
        .expect("sides");
    assert_eq!(s.ours, "main");
    assert_eq!(s.theirs, "side");

    // A rebase swaps them: `side` is the one being replayed, so it is
    // `theirs`, and `main` is what it is landing on.
    let mut repo = conflicting_branches();
    repo.git(&["checkout", "side"]);
    integrate::rebase(
        &exec,
        &repo.path,
        "main",
        &RebaseOptions::default(),
        &cancel,
    )
    .await
    .expect_err("conflict");
    let s = conflict::sides(&exec, &repo.path, InProgress::Rebase, &cancel)
        .await
        .expect("sides");
    assert_eq!(s.ours, "main", "the upstream being landed on");
    assert_eq!(s.theirs, "side", "the branch being replayed");

    // A cherry-pick brings one commit onto the current branch.
    let repo = conflicting_branches();
    integrate::cherry_pick(&exec, &repo.path, &["side".into()], &cancel)
        .await
        .expect_err("conflict");
    let s = conflict::sides(&exec, &repo.path, InProgress::CherryPick, &cancel)
        .await
        .expect("sides");
    assert_eq!(s.ours, "main");
    assert_eq!(s.theirs, "side");
}

/// Nothing to name a side by is answered with nothing, not with a guess
/// or an error: a commit no branch reaches comes back empty and the UI
/// falls back to its own wording.
#[tokio::test]
async fn a_side_no_branch_reaches_is_left_unnamed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "gone"]);
    let orphan = repo.commit_file("f.txt", "orphan\n", "off on its own");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    // The branch goes; the commit stays reachable only by its hash.
    repo.git(&["branch", "-D", "gone"]);
    let (exec, cancel) = env();

    integrate::cherry_pick(&exec, &repo.path, &[orphan], &cancel)
        .await
        .expect_err("conflict");
    let s = conflict::sides(&exec, &repo.path, InProgress::CherryPick, &cancel)
        .await
        .expect("sides");
    assert_eq!(s.ours, "main");
    assert_eq!(s.theirs, "", "git says `undefined`, which is not a name");
}

/// Reads a file in the git directory by the name git knows it by.
fn read_git_file(repo: &mut TestRepo, rel: &str) -> String {
    let path = repo.git(&["rev-parse", "--git-path", rel]);
    std::fs::read_to_string(repo.path.join(path.trim()))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

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
    // The plan reaches back to the parent so that dropping the newest
    // commit is not an all-drop plan — but that parent rides along as an
    // ordinary pick and comes out with the object name it went in with.
    // Nothing before the dropped commit is rewritten.
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
    use platitude_core::process::{CommandEnd, CommandObserver};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Ends(Mutex<Vec<CommandEnd>>);
    impl CommandObserver for Ends {
        fn records(&self, _user: bool) -> bool {
            true
        }
        fn started(&self, _display: &str, _full: &str, _user: bool) -> u64 {
            0
        }
        fn finished(&self, _id: u64, end: CommandEnd, _elapsed_ms: u64, _message: &str) {
            self.0.lock().unwrap().push(end);
        }
    }

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

/// A range holding a merge is refused for a drop the same way it is for
/// a squash: a plain interactive rebase would flatten the history rather
/// than leave one commit out of it.
#[tokio::test]
async fn dropping_across_a_merge_is_refused() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let doomed = repo.commit_file("b.txt", "two\n", "the one to go");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("c.txt", "three\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("d.txt", "four\n", "main work");
    repo.git(&["merge", "--no-ff", "-m", "bring side in", "side"]);
    let (exec, cancel) = env();

    let err = sequencer::plan_edit(&exec, &repo.path, &doomed, sequencer::Edit::Drop, &cancel)
        .await
        .expect_err("a merge is in the way");
    assert!(err.to_string().contains("merge commit"), "{err}");
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
