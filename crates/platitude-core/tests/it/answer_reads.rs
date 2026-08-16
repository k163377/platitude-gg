//! Reads outside the configuration family that answer with their exit
//! code, pinned the way config_reads.rs pins the config ones: unmarked,
//! each of these logged a failed row over an ordinary answer — the HEAD
//! reads on every refresh and both graph passes of a detached or unborn
//! repository (規約 core.md §終了コードで答える問い合わせはコマンドログの
//! 失敗にしない).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::{assert_answered, logged};
use platitude_core::details::{self, DiffTarget};
use platitude_core::process::CommandEnd;
use platitude_core::{refs, stash};

/// `symbolic-ref -q` exits 1 to say "detached", on every read of HEAD.
#[tokio::test]
async fn reading_a_detached_head_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["switch", "--detach", "HEAD"]);
    let (exec, log, cancel) = logged();

    let head = refs::head_state(&exec, &repo.path, &cancel)
        .await
        .expect("head state");

    assert!(head.branch.is_none(), "detached HEAD names no branch");
    assert!(head.oid.is_some(), "the commit is still there");
    assert_eq!(
        log.ends_of(&["symbolic-ref"]),
        vec![CommandEnd::Answered(1)]
    );
    assert_eq!(
        log.ends_of(&["rev-parse", "--verify"]),
        vec![CommandEnd::Answered(0)]
    );
}

/// `rev-parse --verify -q HEAD` exits 1 to say "unborn" — the state every
/// freshly initialised repository opens in.
#[tokio::test]
async fn reading_an_unborn_head_answers_by_code() {
    let repo = TestRepo::init();
    let (exec, log, cancel) = logged();

    let head = refs::head_state(&exec, &repo.path, &cancel)
        .await
        .expect("head state");

    assert!(head.oid.is_none(), "an unborn branch resolves to no commit");
    assert_eq!(
        log.ends_of(&["rev-parse", "--verify"]),
        vec![CommandEnd::Answered(1)]
    );
}

/// The diff of a file nothing tracks yet: `diff --no-index` exits 1
/// whenever the sides differ, which against `/dev/null` is every time.
#[tokio::test]
async fn diffing_an_untracked_file_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("fresh.txt", "new\n");
    let (exec, log, cancel) = logged();

    let raw = details::file_diff_raw(
        &exec,
        &repo.path,
        &DiffTarget::Untracked {
            path: "fresh.txt".to_string(),
        },
        &cancel,
    )
    .await
    .expect("untracked diff");

    assert!(!raw.is_empty(), "the whole file is the diff");
    assert_eq!(log.ends_of(&["--no-index"]), vec![CommandEnd::Answered(1)]);
}

/// Renaming a stash probes the shifted entry before dropping the old one;
/// 0 is "where the store pushed it" and 1 is the refusing arm — either way
/// an answer, not a failure.
#[tokio::test]
async fn renaming_a_stash_probes_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "two\n");
    repo.git(&["stash", "push", "-m", "before"]);
    let (exec, log, cancel) = logged();

    stash::rename(&exec, &repo.path, "stash@{0}", "after", &cancel)
        .await
        .expect("rename");

    assert_answered(
        &log.ends_of(&["rev-parse", "--verify"]),
        "the shifted-entry probe",
    );
}
