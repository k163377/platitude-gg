//! Rebase: each way the replay stops, and the ways out of it. What git
//! itself does with a plain rebase — the replay, its clean-tree refusal,
//! the commits it drops or a `--skip` loses — is recorded in [`periodic`].

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::{current_op, helper};
use platitude_core::conflict::{self, Side};
use platitude_core::integrate::{self, Continuation, InProgress, RebaseOptions};
use platitude_core::process::Kept;
use platitude_core::sequencer::{self, RebaseStep, TodoAction};
use platitude_core::{opstate, status};

/// A rebase that stopped part-way is a landing of its own — git left
/// the replay standing, and the badge, the exit card and the
/// conflicted rows are the whole of what happened
/// (by design. デザイン規約 §進行中の操作から出る).
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
    let (progress, _) = integrate::rebase_standing(&exec, &repo.path, &cancel)
        .await
        .expect("progress");
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
        integrate::rebase_standing(&exec, &repo.path, &cancel)
            .await
            .expect("progress")
            .0
            .is_none()
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "topic two");
}

/// The one failure that leaves `rebase-merge` standing: a rebase asked
/// for while one is already in progress. git spends 128 on it (measured
/// 2.55) — every other failure leaves nothing behind — so reading the
/// marker without the code would call it a stop, and git's sentence
/// about what is really there would never reach the screen.
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
    // The stop is an answer, so nothing carries git's words back to the
    // caller: the command log is where they are read, and this is the
    // observer that stands in for it (デザイン規約 §git が言ったことを読む場所).
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
    // Two flags are set at once here — the stopped pick leaves
    // CHERRY_PICK_HEAD behind — and only one of them is the operation.
    // Anything naming what is in progress has to ask `from_state`: a
    // badge that lists the flags reads
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

/// **What the pre-merge run leaves out**: what git does with a plain rebase
/// once it is asked — the replay itself, the wording of its clean-tree
/// check, the verbatim-upstream commit it drops unasked, and the work a
/// `--skip` takes with it. The reading of those answers is held before every merge:
/// the refusal's classifier by `integrate::rebase`'s unit tests, the
/// `Blocked` and `Done` landings by `session_integration::carry_rewrite`,
/// and the skip road by the tests above. Run by the full gate
/// (`-- --ignored ::periodic::`) rather than by every change.
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

    /// A branch holding a commit of its own while `main` has moved on — the
    /// shape someone asks a `rebase <current> onto it` for.
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

    /// git's clean-tree refusal for a *plain* rebase, in git's own words.
    ///
    /// A refusal is an answer: it arrives as `Blocked`, and the caller goes
    /// round through a stash. The interactive path words the same two
    /// refusals identically — that is what lets one carry serve both
    /// (規約 §未コミット変更がある状態で履歴を書き換える) — and both halves
    /// are exercised here because git words the staged one differently from
    /// the unstaged one.
    ///
    /// The third case decides whether a stash is taken at all: untracked
    /// files are in nobody's way, and a rebase over a tree holding only those
    /// goes straight through.
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

    /// What reaches a person as "skip or not?" is always the hard one: a
    /// commit whose change is already upstream *to the letter* is dropped by
    /// git without stopping, so it never gets as far as the UI.
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

    /// And skipping has a price: the commit left out takes its own work with
    /// it, wherever else that work does or does not exist. Only the reflog
    /// holds it afterwards — the same standing a hard reset leaves behind,
    /// which is the one this app already asks to be held for.
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
}
