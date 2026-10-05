//! Rebase: each way the replay stops, and the ways out of it.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::{current_op, helper};
use platitude_core::conflict::{self, Side};
use platitude_core::integrate::{self, Continuation, InProgress, RebaseOptions};
use platitude_core::process::Kept;
use platitude_core::sequencer::{self, RebaseStep, TodoAction};
use platitude_core::{opstate, status};

/// A stop is a landing, not a failure (デザイン規約 §進行中の操作から出る).
fn stopped(outcome: Result<integrate::RebaseOutcome, platitude_core::error::GitError>) {
    match outcome.expect("a stop is an answer, not a failure") {
        integrate::RebaseOutcome::Stopped => {}
        other => panic!("the replay was expected to stop: {other:?}"),
    }
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

    stopped(
        integrate::rebase(
            &exec,
            &repo.path,
            "main",
            &RebaseOptions::default(),
            &cancel,
        )
        .await,
    );

    assert_eq!(current_op(&repo).await, Some(InProgress::Rebase));
    let (progress, _) = integrate::rebase_standing(&repo.path.join(".git"));
    let progress = progress.expect("a rebase is running");
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
        integrate::rebase_standing(&repo.path.join(".git"))
            .0
            .is_none()
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "topic two");
}

/// git exits 128 here with `rebase-merge` still standing, so reading the
/// marker without the code would call it a stop and hide git's sentence.
#[tokio::test]
async fn a_second_rebase_over_one_already_standing_is_still_a_failure() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("f.txt", "topic\n", "topic change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo.git(&["checkout", "topic"]);
    let (exec, cancel) = env();
    let opts = RebaseOptions::default();

    stopped(integrate::rebase(&exec, &repo.path, "main", &opts, &cancel).await);
    integrate::rebase(&exec, &repo.path, "main", &opts, &cancel)
        .await
        .expect_err("a rebase over one already standing is a failure");
    assert_eq!(
        current_op(&repo).await,
        Some(InProgress::Rebase),
        "and the one that was standing is still standing"
    );
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

    stopped(
        integrate::rebase(
            &exec,
            &repo.path,
            "main",
            &RebaseOptions::default(),
            &cancel,
        )
        .await,
    );
    integrate::resolve_current(&exec, &repo.path, Continuation::Skip, &cancel)
        .await
        .expect("skip the conflicting commit");

    assert_eq!(current_op(&repo).await, None);
    let subjects = repo.git(&["log", "--format=%s"]);
    assert!(!subjects.contains("doomed commit"), "got: {subjects}");
    assert!(subjects.contains("keeper"));
}

/// Taking the upstream side wholesale leaves the commit empty.
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

    stopped(
        integrate::rebase(
            &exec,
            &repo.path,
            "main",
            &RebaseOptions::default(),
            &cancel,
        )
        .await,
    );
    // In a rebase `Ours` is the upstream side.
    conflict::take_side(&exec, &repo.path, &["f.txt".into()], Side::Ours, &cancel)
        .await
        .expect("take the upstream side");

    // `--empty=drop` is the merge backend's default: nobody is asked.
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

/// Unlike the plain rebase, the interactive one (squash / reword / drop)
/// stops on a commit that came out empty and names `--skip`: a state where
/// skipping costs nothing, so the skip gesture cannot be chosen from the
/// plain rebase alone.
#[tokio::test]
async fn an_interactive_rebase_stops_on_an_emptied_commit_and_names_skip() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    let doomed = repo.commit_file_id("f.txt", "a\nX\n", "adds X");
    repo.commit_file("h.txt", "keep\n", "keeper");
    repo.git(&["checkout", "main"]);
    // Same net line, different patch: not a clean cherry-pick of `doomed`,
    // so the cherry-pick filter cannot be what drops it.
    repo.write_file("f.txt", "a\nX\n");
    repo.write_file("g.txt", "unrelated\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-m", "X arrives with company"]);
    repo.git(&["checkout", "topic"]);
    // The stop is an answer, so git's words reach only the command log;
    // this observer stands in for it (デザイン規約 §git が言ったことを読む場所).
    let said = std::sync::Arc::new(crate::support::Said::default());
    let (exec, cancel) = crate::support::exec::observed_env(said.clone(), Kept::Asked);

    let steps = vec![
        RebaseStep::pick(doomed.clone(), "adds X"),
        RebaseStep {
            action: TodoAction::Pick,
            oid: repo.git(&["rev-parse", "topic"]),
            subject: "keeper".into(),
            message: None,
        },
    ];
    stopped(
        sequencer::rebase_interactive(
            &exec,
            &info(&repo).await,
            "main",
            &steps,
            &RebaseOptions::default(),
            &helper(),
            &cancel,
        )
        .await,
    );

    assert_eq!(current_op(&repo).await, Some(InProgress::Rebase));
    let message = said
        .message_of(platitude_core::process::CommandEnd::Answered(1))
        .expect("the stop was recorded as an answer");
    assert!(
        message.contains("The previous cherry-pick is now empty"),
        "git's reason: {message}"
    );
    assert!(
        message.contains("git rebase --skip"),
        "git names the way out: {message}"
    );
    // The stopped pick leaves CHERRY_PICK_HEAD too: name the operation
    // through `from_state`, or a badge reads `REBASING · CHERRY-PICKING`.
    let (exec2, cancel2) = env();
    let state = opstate::detect(&exec2, &repo.path, &cancel2)
        .await
        .expect("op state");
    assert!(state.rebasing && state.cherry_picking, "got: {state:?}");
    assert_eq!(InProgress::from_state(&state), Some(InProgress::Rebase));
    // Nothing is conflicted, so the file list cannot tell this from an
    // `edit` stop.
    let state = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(state.conflicted().count(), 0);
}

/// What git does with a plain rebase. Reading its answers is held pre-merge:
/// the refusal's classifier by `integrate::rebase`'s unit tests, `Blocked` /
/// `Done` by `session_integration::carry_rewrite`, the skip by the tests
/// above (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "git's own replay behind a fixed command line: not worth the pre-merge run"]
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

    fn behind_main() -> TestRepo {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "one\n", "root");
        repo.git(&["checkout", "-b", "topic"]);
        repo.commit_file("b.txt", "two\n", "topic one");
        repo.git(&["checkout", "main"]);
        repo.commit_file("d.txt", "four\n", "main moved");
        repo.git(&["checkout", "topic"]);
        repo
    }

    /// A refusal arrives as `Blocked` and the caller carries the work through
    /// a stash. The interactive path words both refusals the same, which lets
    /// one carry serve both (デザイン規約 §未コミット変更がある状態で履歴を書き換える);
    /// git words staged and unstaged differently, so both are here. Untracked
    /// files do not stop a rebase, so they take no stash.
    #[tokio::test]
    #[ignore = "git's own clean-tree wording: not worth the pre-merge run"]
    async fn a_dirty_tree_stops_a_plain_rebase_before_it_touches_anything() {
        let (exec, cancel) = env();
        let opts = RebaseOptions::default();
        let refusal = |outcome| match outcome {
            integrate::RebaseOutcome::Blocked(error) => error.to_string(),
            integrate::RebaseOutcome::Done => panic!("git replayed over work it would lose"),
            integrate::RebaseOutcome::Stopped => {
                panic!("git began a rebase it should have refused")
            }
        };

        let mut unstaged = behind_main();
        unstaged.write_file("a.txt", "changed, never staged\n");
        let before = unstaged.git(&["rev-parse", "topic"]);
        let said = refusal(
            integrate::rebase(&exec, &unstaged.path, "main", &opts, &cancel)
                .await
                .expect("a refusal is an answer"),
        );
        assert!(
            said.contains("cannot rebase:") && said.contains("unstaged changes"),
            "the unstaged half of git's check: {said}"
        );
        assert_eq!(
            unstaged.git(&["rev-parse", "topic"]),
            before,
            "refused before touching anything"
        );

        let mut staged = behind_main();
        staged.write_file("a.txt", "changed and staged\n");
        staged.git(&["add", "--", "a.txt"]);
        let said = refusal(
            integrate::rebase(&exec, &staged.path, "main", &opts, &cancel)
                .await
                .expect("a refusal is an answer"),
        );
        assert!(
            said.contains("cannot rebase:") && said.contains("uncommitted changes"),
            "the staged half of git's check: {said}"
        );

        let mut untracked = behind_main();
        untracked.write_file("brand-new.txt", "in nobody's way\n");
        integrate::rebase(&exec, &untracked.path, "main", &opts, &cancel)
            .await
            .expect("untracked files do not stop a rebase");
        assert_eq!(
            untracked.git(&["status", "--porcelain"]),
            "?? brand-new.txt",
            "and they are still sitting there afterwards"
        );
    }

    /// A commit already upstream to the letter is dropped without stopping,
    /// so "skip or not?" never reaches the UI for it.
    #[tokio::test]
    #[ignore = "git dropping a verbatim-upstream commit itself: not worth the pre-merge run"]
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

    /// A skipped commit takes its own work with it; only the reflog holds it
    /// afterwards, as after a hard reset, which this app asks to be held for.
    #[tokio::test]
    #[ignore = "what git's --skip loses: not worth the pre-merge run"]
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

        stopped(
            integrate::rebase(
                &exec,
                &repo.path,
                "main",
                &RebaseOptions::default(),
                &cancel,
            )
            .await,
        );
        integrate::resolve_current(&exec, &repo.path, Continuation::Skip, &cancel)
            .await
            .expect("skip");

        assert!(
            !repo.path.join("only-here.txt").exists(),
            "the skipped commit's own file goes with it"
        );
        let orphan = repo.git(&["log", "--format=%s", "-1", before.trim()]);
        assert_eq!(orphan, "conflicts, and carries its own file");
        let described = repo.git(&["log", "--format=%s", "--all"]);
        assert!(
            !described.contains("conflicts, and carries its own file"),
            "no ref reaches it: {described}"
        );
    }
}
