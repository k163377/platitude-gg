//! What staging refuses once git has been asked: vanished selections,
//! stale fingerprints, a file that moves before git's apply, a conflicted
//! file's combined diff.
//!
//! Targets refused off the target alone (a diff of history, a side with
//! no working tree) are pinned where that is decided (`stage::partial`).

use std::sync::atomic::{AtomicBool, Ordering};

use crate::support::exec::env;
use crate::support::stage::{buckets, fp};
use crate::support::{TestRepo, info};
use platitude_core::details::{self, DiffTarget};
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
        err.report().is_some_and(|r| r.fact.is_empty()),
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

/// The diff `target` gives as the file stands now.
///
/// Read on a thread of its own: under the lock this thread has no real
/// `file_diff_raw` until a rule says so, and a rule that calls the real one
/// would answer every call after it (mry keeps the mock and the lock per
/// thread, and the first rule that matches wins).
async fn read_now(repo: &TestRepo, target: &DiffTarget) -> Vec<u8> {
    let (path, target) = (repo.path.clone(), target.clone());
    tokio::task::spawn_blocking(move || {
        let (exec, cancel) = env();
        tokio::runtime::Handle::current()
            .block_on(details::file_diff_raw(&exec, &path, &target, &cancel))
    })
    .await
    .expect("the read's thread")
    .expect("diff")
}

/// Holds the first read of a diff on this thread at what `target` gives
/// now, `moves` the file, and answers every later read as it stands after;
/// returns the fingerprint of the first. That stands for another git (or a
/// formatter) moving the file between the fingerprint check and git's
/// apply: the check passes, git finds a patch that does not fit, and a read
/// after that shows the move.
async fn hold_the_read(
    repo: &mut TestRepo,
    target: &DiffTarget,
    moves: impl FnOnce(&mut TestRepo),
) -> u64 {
    let before = read_now(repo, target).await;
    moves(repo);
    let after = read_now(repo, target).await;
    assert_ne!(before, after, "the move shows in the diff");
    let seen = details::fingerprint(&before);
    let first = AtomicBool::new(true);
    details::mock_file_diff_raw().returns_with(move || {
        Ok(if first.swap(false, Ordering::SeqCst) {
            before.clone()
        } else {
            after.clone()
        })
    });
    seen
}

/// What git says when the patch no longer fits (`patch does not apply` and
/// its kin, exit 1) on a file that moved since it was read is the same
/// refusal as a fingerprint that moved, with git's words kept for the log.
fn assert_moved_under_the_apply(err: &platitude_core::error::GitError, kind: ReportKind) {
    assert_eq!(err.report().map(|r| r.kind), Some(kind), "{err}");
    assert!(
        err.report().is_some_and(|r| r.fact.is_empty()),
        "the words are the screen's own: {err}"
    );
    assert!(
        err.to_string().contains("patch does not apply"),
        "the log keeps git's answer: {err}"
    );
}

#[tokio::test]
#[mry::lock(details::file_diff_raw)]
async fn an_index_that_moves_under_the_stage_is_refused_as_stale() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\ntwo\nthree\n", "root");
    repo.write_file("a.txt", "one\ntwo changed\nthree\n");
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };
    let seen = hold_the_read(&mut repo, &target, |repo| {
        repo.git(&["add", "--", "a.txt"]);
    })
    .await;

    let (exec, cancel) = env();
    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("the index moved under the patch");
    assert_moved_under_the_apply(&err, ReportKind::StaleStage);
}

#[tokio::test]
#[mry::lock(details::file_diff_raw)]
async fn an_index_that_moves_under_the_unstage_is_refused_as_stale() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\ntwo\nthree\n", "root");
    repo.write_file("a.txt", "one\ntwo changed\nthree\n");
    repo.git(&["add", "--", "a.txt"]);
    let repo_info = info(&repo).await;
    let target = DiffTarget::Staged {
        path: "a.txt".into(),
        orig_path: None,
    };
    let seen = hold_the_read(&mut repo, &target, |repo| {
        repo.write_file("a.txt", "one\ntwo changed again\nthree\n");
        repo.git(&["add", "--", "a.txt"]);
    })
    .await;

    let (exec, cancel) = env();
    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("the index moved under the patch");
    assert_moved_under_the_apply(&err, ReportKind::StaleUnstage);
    assert_eq!(
        repo.git(&["show", ":a.txt"]),
        "one\ntwo changed again\nthree",
        "nothing was unstaged"
    );
}

#[tokio::test]
#[mry::lock(details::file_diff_raw)]
async fn a_file_that_moves_under_the_discard_is_refused_as_stale() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\ntwo\nthree\n", "root");
    repo.write_file("a.txt", "one\ntwo changed\nthree\n");
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };
    let seen = hold_the_read(&mut repo, &target, |repo| {
        repo.write_file("a.txt", "one\ntwo changed again\nthree\n");
    })
    .await;

    let (exec, cancel) = env();
    let err = stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("the file moved under the patch");
    assert_moved_under_the_apply(&err, ReportKind::StaleDiscard);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "one\ntwo changed again\nthree\n",
        "nothing was thrown away"
    );
}

/// git can turn a patch down that nothing moved under: a diff read with
/// `diff.context=0` has hunks without context, and `apply` will not place
/// one that does not end the file. The file reads the same afterwards, so
/// this is not the stale refusal — it is git's to say, and the log has it.
#[tokio::test]
async fn a_patch_git_refuses_on_an_unmoved_file_is_not_called_stale() {
    let mut repo = TestRepo::init();
    let base: String = (1..=9).map(|n| format!("line {n}\n")).collect();
    repo.commit_file("a.txt", &base, "root");
    repo.write_file("a.txt", &base.replace("line 5\n", "line 5 changed\n"));
    repo.git(&["config", "diff.context", "0"]);
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };

    let (exec, cancel) = env();
    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect_err("git places no context-free hunk mid-file");
    assert!(err.report().is_none(), "not a refusal of this end's: {err}");
    assert!(err.to_string().contains("patch does not apply"), "{err}");
    let (staged, _, _) = buckets(&repo).await;
    assert!(staged.is_empty(), "{staged:?}");
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
