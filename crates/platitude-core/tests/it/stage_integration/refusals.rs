//! What staging refuses once git has been asked: vanished selections,
//! stale fingerprints, a conflicted file's combined diff.
//!
//! Targets refused off the target alone (a diff of history, a side with
//! no working tree) are pinned where that is decided (`stage::partial`).

use crate::support::exec::env;
use crate::support::stage::{buckets, fp};
use crate::support::{TestRepo, info};
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::report::ReportKind;
use platitude_core::{stage, status};

/// A fresh `git init` has no HEAD and an empty index; emptying that index
/// is a no-op (without `--ignore-unmatch`, `git rm --cached -r -- .` exits 128).
#[tokio::test]
async fn unstage_all_on_an_unborn_empty_index_succeeds() {
    let repo = TestRepo::init();
    let (exec, cancel) = env();
    stage::unstage_all(&exec, &repo.path, &cancel)
        .await
        .expect("unstaging nothing succeeds at its job");
}

/// A selection that indexes a diff the file no longer produces is a
/// refusal: a "successful" no-op would show nothing happened, with no
/// words saying why.
#[tokio::test]
async fn a_vanished_selection_is_an_error_not_a_silent_success() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "one changed\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();

    let target = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };
    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(99)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect_err("hunk 99 is not in the diff");
    assert!(err.report().is_some(), "{err}");

    let err = stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(99)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect_err("same refusal on the discarding side");
    assert!(err.report().is_some(), "{err}");
}

/// A failed partial stage of an untracked file takes the intent-to-add
/// mark back off — one left behind changes the file's bucket (and
/// with it, which discard the row offers).
#[tokio::test]
async fn a_failed_untracked_partial_stage_leaves_the_file_untracked() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("new.txt", "fresh\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();

    let target = DiffTarget::Untracked {
        path: "new.txt".into(),
    };
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(99)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect_err("hunk 99 is not in the diff");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(staged.is_empty(), "no half-staged leftovers: {staged:?}");
    assert!(unstaged.is_empty(), "{unstaged:?}");
    assert_eq!(untracked, vec!["new.txt"], "back in the untracked bucket");
}
/// A selection carried from an older diff is refused once the file
/// changes, on both the staging and the discarding side: a formatter on
/// save moves the file before the queued write runs, and positional
/// indices would land on the wrong hunk.
#[tokio::test]
async fn a_selection_from_a_stale_diff_is_refused() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\ntwo\nthree\n", "root");
    repo.write_file("a.txt", "one\ntwo changed\nthree\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();
    let target = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };
    let seen = fp(&repo_info, &target).await;

    repo.write_file("a.txt", "prelude\none\ntwo changed\nthree\n");

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("stale fingerprint is refused");
    // The heading turns on the direction, so the refusal carries it
    // (デザイン規約 §答えの要らない報せ).
    assert_eq!(
        err.report().map(|r| r.kind),
        Some(ReportKind::StaleStage),
        "{err}"
    );
    assert!(
        err.report().is_some_and(|r| r.reason.is_empty()),
        "nobody outside said anything — the words are the screen's own"
    );

    let err = stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("stale fingerprint refuses the discard too");
    assert_eq!(
        err.report().map(|r| r.kind),
        Some(ReportKind::StaleDiscard),
        "{err}"
    );

    let (staged, unstaged, _) = buckets(&repo).await;
    assert!(staged.is_empty(), "nothing was staged: {staged:?}");
    assert_eq!(unstaged, vec!["a.txt"], "nothing was discarded");

    // Off the staged side, the refusal is an unstage.
    repo.git(&["add", "--", "a.txt"]);
    let staged_side = DiffTarget::Staged {
        path: "a.txt".into(),
        orig_path: None,
    };
    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &staged_side,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("the fingerprint is a different file's either way");
    assert_eq!(
        err.report().map(|r| r.kind),
        Some(ReportKind::StaleUnstage),
        "{err}"
    );
}

/// An untracked partial stage checks the fingerprint against the same
/// `--no-index` bytes the UI read — before the intent-to-add mark, which
/// changes what the diff command even is. A stale one leaves the file
/// fully untracked, mark and all.
#[tokio::test]
async fn a_stale_untracked_selection_is_refused_before_the_mark() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("new.txt", "fresh\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();
    let target = DiffTarget::Untracked {
        path: "new.txt".into(),
    };
    let seen = fp(&repo_info, &target).await;

    repo.write_file("new.txt", "fresh\nand more\n");

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("stale fingerprint is refused");
    assert_eq!(
        err.report().map(|r| r.kind),
        Some(ReportKind::StaleStage),
        "{err}"
    );

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty(),
        "no intent-to-add mark left: {staged:?} {unstaged:?}"
    );
    assert_eq!(untracked, vec!["new.txt"]);
}

/// A conflicted file's diff is the combined form, which has no single old
/// side for a rebuilt patch (`git apply` refuses it). The pane withholds
/// the pieces there; this is the floor under that, in this app's words.
#[tokio::test]
async fn no_part_of_a_conflicted_file_can_be_taken_or_thrown_away() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\nthree\n", "base");
    repo.git(&["checkout", "-b", "side"]);
    repo.write_file("f.txt", "one\nTHEIRS\nthree\n");
    repo.git(&["commit", "-am", "their side"]);
    repo.git(&["checkout", "main"]);
    repo.write_file("f.txt", "one\nOURS\nthree\n");
    repo.git(&["commit", "-am", "our side"]);
    repo.git_expect_failure(&["merge", "--no-edit", "side"]);

    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    let seen = fp(&repo_info, &target).await;

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("a combined diff cannot be staged in pieces");
    assert_eq!(
        err.report().map(|r| r.kind),
        Some(ReportKind::ConflictedPart),
        "{err}"
    );

    let err = stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("nor thrown away in pieces");
    assert_eq!(
        err.report().map(|r| r.kind),
        Some(ReportKind::ConflictedPart),
        "{err}"
    );

    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(s.conflicted().count(), 1, "still one unmerged path");
    assert!(
        std::fs::read_to_string(repo.path.join("f.txt"))
            .unwrap()
            .contains("<<<<<<<"),
        "the markers are untouched"
    );
}
