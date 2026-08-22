//! What staging refuses: commit diffs, vanished selections, stale
//! fingerprints.

use crate::support::exec::env;
use crate::support::stage::{buckets, fp};
use crate::support::{TestRepo, info};
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::{stage, status};

#[tokio::test]
async fn staging_a_commit_diff_is_rejected() {
    let mut repo = TestRepo::init();
    let oid = repo.commit_file("f.txt", "x\n", "root");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Commit {
            oid: platitude_core::Oid::from_hex_str(&oid).unwrap(),
            parent: None,
            path: "f.txt".into(),
            orig_path: None,
        },
        &[HunkSelect::whole(0)],
        0,
        &cancel,
    )
    .await
    .expect_err("committed diffs are not stageable");
    assert!(err.to_string().contains("cannot be staged"));
}

/// A fresh `git init` has no HEAD and an empty index; emptying that index
/// is a no-op, not a fatal (実測 2026-08-07: without --ignore-unmatch,
/// `git rm --cached -r -- .` exits 128 on "did not match any files").
#[tokio::test]
async fn unstage_all_on_an_unborn_empty_index_succeeds() {
    let repo = TestRepo::init();
    let (exec, cancel) = env();
    stage::unstage_all(&exec, &repo.path, &cancel)
        .await
        .expect("unstaging nothing succeeds at its job");
}

/// A selection that indexes a diff the file no longer produces is a
/// refusal, not a write that quietly did nothing: the graph refresh
/// after a "successful" no-op would show the user nothing happened,
/// with no words saying why.
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
    assert!(format!("{err}").contains("no longer"), "{err}");

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
    assert!(format!("{err}").contains("no longer"), "{err}");
}

/// A failed partial stage of an untracked file must not leave the
/// intent-to-add mark behind — the file would silently change buckets
/// (and with it, which discard the row offers).
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
/// changes: the fingerprint the UI saw no longer matches the re-run
/// bytes — on both the staging and the discarding side. This is the
/// formatter-on-save case: the file moves on after the diff was read
/// but before the queued write runs, and positional indices would land
/// on the wrong hunk.
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
    assert!(format!("{err}").contains("changed since"), "{err}");

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
    assert!(format!("{err}").contains("changed since"), "{err}");

    let (staged, unstaged, _) = buckets(&repo).await;
    assert!(staged.is_empty(), "nothing was staged: {staged:?}");
    assert_eq!(unstaged, vec!["a.txt"], "nothing was discarded");
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
    assert!(format!("{err}").contains("changed since"), "{err}");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty(),
        "no intent-to-add mark left: {staged:?} {unstaged:?}"
    );
    assert_eq!(untracked, vec!["new.txt"]);
}

/// A conflicted file's diff is the combined form, which has no single old
/// side for a rebuilt patch to sit on — `git apply` refuses the shape
/// outright. The pane withholds the pieces there, so nothing should ask;
/// this is the floor under that, and it must say so in this app's words
/// rather than let git complain about a fragment nobody wrote.
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
    assert!(format!("{err}").contains("still conflicted"), "{err}");

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
    assert!(format!("{err}").contains("still conflicted"), "{err}");

    // Refused, not half-done: the path is still exactly as git left it.
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
