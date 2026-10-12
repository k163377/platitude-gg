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

/// A reader's `diff.context=0` would leave the diff's hunks without
/// context, and `apply` places no context-free hunk mid-file: the part could
/// never be taken. The read pins the context (`details::DIFF_SHAPE_ARGS`),
/// so the part applies as on any other machine.
#[tokio::test]
async fn a_readers_context_setting_does_not_keep_a_part_from_applying() {
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
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("the hunk carries its context whatever diff.context says");
    let (staged, unstaged, _) = buckets(&repo).await;
    assert_eq!(staged, vec!["a.txt".to_string()], "{staged:?}");
    assert!(
        unstaged.is_empty(),
        "the whole change went in: {unstaged:?}"
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

/// The fingerprint is checked before the intent-to-add mark, and the patch
/// is built from the diff read after it: a file that moves in between would
/// have the selection's lines counted in the new bytes, staging others than
/// the ones chosen. A `post-index-change` hook moves it right after the
/// mark, the once: the hunks read after it are not the ones checked, so the
/// stage is refused as stale and the mark taken back.
#[tokio::test]
async fn an_untracked_file_that_moves_under_the_mark_is_refused_as_stale() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("new.txt", "keep\ndrop\n");
    repo.write_hook(
        "post-index-change",
        "[ -e moved ] && exit 0\n: > moved\nprintf 'pushed in\\nkeep\\ndrop\\n' > new.txt\n",
    );
    repo.write_file(".git/info/exclude", "moved\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();
    let target = DiffTarget::Untracked {
        path: "new.txt".into(),
    };
    let seen = fp(&repo_info, &target).await;

    // Body: 0 "+keep", 1 "+drop" — keep only the first.
    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::lines(0, [0])],
        seen,
        &cancel,
    )
    .await
    .expect_err("the file moved under the mark");
    assert_eq!(
        err.report().map(|r| r.kind),
        Some(ReportKind::StaleStage),
        "{err}"
    );
    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty(),
        "no mark left, nothing staged: {staged:?} {unstaged:?}"
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

/// `a.txt` changed in the working tree, `s.txt` changed and staged.
fn staged_and_unstaged() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\ntwo\n", "root");
    repo.commit_file("s.txt", "s\n", "staged one");
    repo.write_file("s.txt", "s staged\n");
    repo.git(&["add", "s.txt"]);
    repo.write_file("a.txt", "one\ntwo changed\n");
    repo
}

fn report_kind(err: platitude_core::GitError) -> Option<ReportKind> {
    err.report().map(|r| r.kind)
}

/// Another git holding the index's lock turns every whole-file verb down,
/// and each is the lock's report for its verb, as a commit's is. Nothing
/// moves.
#[tokio::test]
async fn another_gits_index_lock_is_reported_for_each_whole_file_verb() {
    let repo = staged_and_unstaged();
    let (exec, cancel) = env();
    let before = buckets(&repo).await;
    let lock = repo.path.join(".git/index.lock");
    std::fs::write(&lock, "").expect("lock");
    let a = || vec!["a.txt".to_string()];
    let s = || vec!["s.txt".to_string()];
    let side = vec![("a.txt".to_string(), stage::DiscardSide::Unstaged)];

    let refused = [
        (
            report_kind(
                stage::stage_paths(&exec, &repo.path, &a(), &cancel)
                    .await
                    .unwrap_err(),
            ),
            ReportKind::StageLocked,
        ),
        (
            report_kind(
                stage::stage_all(&exec, &repo.path, &cancel)
                    .await
                    .unwrap_err(),
            ),
            ReportKind::StageLocked,
        ),
        (
            report_kind(
                stage::unstage_paths(&exec, &repo.path, &s(), &cancel)
                    .await
                    .unwrap_err(),
            ),
            ReportKind::UnstageLocked,
        ),
        (
            report_kind(
                stage::unstage_all(&exec, &repo.path, &cancel)
                    .await
                    .unwrap_err(),
            ),
            ReportKind::UnstageLocked,
        ),
        (
            report_kind(
                stage::discard_rows(&exec, &repo.path, &side, &cancel)
                    .await
                    .unwrap_err(),
            ),
            ReportKind::DiscardLocked,
        ),
    ];
    std::fs::remove_file(&lock).expect("unlock");

    for (got, owed) in refused {
        assert_eq!(got, Some(owed));
    }
    let after = buckets(&repo).await;
    assert_eq!(after.0, before.0, "nothing was staged or unstaged");
    assert_eq!(after.1, before.1, "nothing was thrown away");
}

/// "Nothing was discarded" holds only until a side has gone: a lock met by
/// the staged side, after the unstaged one went, is git's failure as git
/// said it. The lock is taken between the two by a `post-index-change`
/// hook, which runs once the first side's write has let the index go.
#[tokio::test]
async fn a_lock_met_after_a_side_was_discarded_is_gits_failure() {
    let repo = staged_and_unstaged();
    repo.write_hook("post-index-change", "touch .git/index.lock\n");
    let (exec, cancel) = env();
    let rows = vec![
        ("a.txt".to_string(), stage::DiscardSide::Unstaged),
        ("s.txt".to_string(), stage::DiscardSide::Staged),
    ];

    let err = stage::discard_rows(&exec, &repo.path, &rows, &cancel)
        .await
        .unwrap_err();
    let lock = repo.path.join(".git/index.lock");
    assert!(lock.exists(), "the hook took the lock between the sides");
    std::fs::remove_file(&lock).expect("unlock");
    match &err {
        platitude_core::GitError::Failed { stderr, .. } => {
            assert!(stderr.contains("index.lock': File exists"), "{stderr}");
        }
        other => panic!("past a side that went, git's own failure: {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
        "one\ntwo\n",
        "the unstaged side went"
    );
}

/// The same lock turns a part staged or unstaged down with the lock's
/// report for its verb. A part thrown away writes the working tree alone,
/// takes no lock, and goes.
#[tokio::test]
async fn another_gits_index_lock_turns_parts_down_but_not_a_part_thrown_away() {
    let repo = staged_and_unstaged();
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();
    let unstaged = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };
    let staged = DiffTarget::Staged {
        path: "s.txt".into(),
        orig_path: None,
    };
    let seen_unstaged = fp(&repo_info, &unstaged).await;
    let seen_staged = fp(&repo_info, &staged).await;
    let lock = repo.path.join(".git/index.lock");
    std::fs::write(&lock, "").expect("lock");

    let refused = [
        (
            report_kind(
                stage::apply_partial(
                    &exec,
                    &repo_info,
                    &unstaged,
                    &[HunkSelect::whole(0)],
                    seen_unstaged,
                    &cancel,
                )
                .await
                .unwrap_err(),
            ),
            ReportKind::StageLocked,
        ),
        (
            report_kind(
                stage::apply_partial(
                    &exec,
                    &repo_info,
                    &staged,
                    &[HunkSelect::whole(0)],
                    seen_staged,
                    &cancel,
                )
                .await
                .unwrap_err(),
            ),
            ReportKind::UnstageLocked,
        ),
    ];
    let before = buckets(&repo).await;
    stage::discard_partial(
        &exec,
        &repo_info,
        &unstaged,
        &[HunkSelect::whole(0)],
        seen_unstaged,
        &cancel,
    )
    .await
    .expect("a part thrown away writes no index");
    std::fs::remove_file(&lock).expect("unlock");

    for (got, owed) in refused {
        assert_eq!(got, Some(owed));
    }
    let (staged_now, unstaged_now, _) = buckets(&repo).await;
    assert_eq!(staged_now, before.0, "nothing was staged or unstaged");
    assert!(unstaged_now.is_empty(), "the part went: {unstaged_now:?}");
}
