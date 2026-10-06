//! Taking another worktree away, against real git: what goes, what
//! git keeps, and the report each refusal is.
//!
//! The half-done removal (a file held open part-way) needs a second
//! process holding the folder; its answer is pinned from a real run in
//! `report`'s unit tests.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::report::ReportKind;
use platitude_core::worktrees;

/// A repository with one commit and a linked worktree of it beside it, on a
/// branch of its own.
fn with_worktree(name: &str) -> (TestRepo, PathBuf) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    let worktree = repo.path.with_file_name(name);
    repo.git(&["worktree", "add", "-b", name, &worktree.to_string_lossy()]);
    (repo, worktree)
}

fn listed(repo: &mut TestRepo, worktree: &std::path::Path) -> bool {
    let leaf = worktree.file_name().unwrap().to_string_lossy().into_owned();
    repo.git(&["worktree", "list", "--porcelain"])
        .lines()
        .any(|l| l.starts_with("worktree ") && l.ends_with(&leaf))
}

/// The folder and its entry go — ignored files with them — and the
/// branch stays.
#[tokio::test]
async fn a_clean_worktree_goes_with_its_ignored_files_and_leaves_its_branch() {
    let (mut repo, worktree) = with_worktree("clean");
    std::fs::write(worktree.join(".gitignore"), "*.log\n").unwrap();
    repo.git_in(&worktree, &["add", ".gitignore"]);
    repo.git_in(&worktree, &["commit", "-m", "ignore logs"]);
    std::fs::write(worktree.join("build.log"), "noise\n").unwrap();
    let (exec, cancel) = env();

    worktrees::remove(
        &exec,
        &repo.path,
        &worktree.to_string_lossy(),
        "clean",
        &cancel,
    )
    .await
    .expect("git removes a worktree with nothing uncommitted");

    assert!(
        !worktree.exists(),
        "the folder is gone, ignored file and all"
    );
    assert!(
        !listed(&mut repo, &worktree),
        "and so is git's record of it"
    );
    assert!(
        repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/clean"]),
        "the branch it had out stays"
    );
}

/// A worktree whose folder is already gone: git drops the record.
#[tokio::test]
async fn a_worktree_whose_folder_is_gone_loses_its_record() {
    let (mut repo, worktree) = with_worktree("gone");
    std::fs::remove_dir_all(&worktree).unwrap();
    let (exec, cancel) = env();

    worktrees::remove(
        &exec,
        &repo.path,
        &worktree.to_string_lossy(),
        "gone",
        &cancel,
    )
    .await
    .expect("a missing folder is no refusal");

    assert!(!listed(&mut repo, &worktree));
}

/// Uncommitted work keeps the worktree, and the screen writes the reason:
/// git's own ends in advice to force it, which the screen does not offer.
#[tokio::test]
async fn uncommitted_work_keeps_the_worktree_and_the_reason_is_the_screens() {
    let (mut repo, worktree) = with_worktree("dirty");
    std::fs::write(worktree.join("draft.txt"), "untracked\n").unwrap();
    let (exec, cancel) = env();

    let err = worktrees::remove(
        &exec,
        &repo.path,
        &worktree.to_string_lossy(),
        "dirty",
        &cancel,
    )
    .await
    .expect_err("git keeps a worktree holding work");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeKept);
    assert_eq!(
        report.name, "dirty",
        "the heading names the worktree as the screen does"
    );
    assert!(
        report.reason.is_empty(),
        "the screen writes the reason: {}",
        report.reason
    );
    assert!(
        err.to_string().contains("modified or untracked"),
        "the log keeps git's words: {err}"
    );
    assert!(worktree.join("draft.txt").exists(), "nothing was deleted");
    assert!(listed(&mut repo, &worktree));
}

/// A lock the menu did not see (taken after it opened) keeps the worktree,
/// in git's words.
#[tokio::test]
async fn a_lock_keeps_the_worktree_in_gits_words() {
    let (mut repo, worktree) = with_worktree("locked");
    repo.git(&[
        "worktree",
        "lock",
        "--reason",
        "held",
        &worktree.to_string_lossy(),
    ]);
    let (exec, cancel) = env();

    let err = worktrees::remove(
        &exec,
        &repo.path,
        &worktree.to_string_lossy(),
        "locked",
        &cancel,
    )
    .await
    .expect_err("git keeps a locked worktree");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeKept);
    assert!(
        report.reason.contains("locked"),
        "git's own sentence: {}",
        report.reason
    );
    assert!(worktree.exists());
}

/// The repository's own worktree is never git's to remove.
#[tokio::test]
async fn the_main_worktree_is_refused() {
    let (repo, _worktree) = with_worktree("side");
    let (exec, cancel) = env();

    let err = worktrees::remove(
        &exec,
        &repo.path,
        &repo.path.to_string_lossy(),
        "main",
        &cancel,
    )
    .await
    .expect_err("git refuses the main worktree");

    assert_eq!(
        err.report().expect("a report").kind,
        ReportKind::WorktreeKept
    );
    assert!(repo.path.join("a.txt").exists());
}
