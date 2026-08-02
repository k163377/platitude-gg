//! Commit / amend and local branch operations on real repositories.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

use platitude_core::branch::{self, CheckoutTarget};
use platitude_core::commit::{self, CommitOptions};
use platitude_core::process::GitExecutor;
use platitude_core::repo::RepoInfo;
use support::TestRepo;
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

#[tokio::test]
async fn commits_staged_content_with_a_multiline_message() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let message = "subject line\n\nbody with \"quotes\" and 日本語\n#not-a-comment\n";
    let oid = commit::commit(
        &exec,
        &repo_info,
        message,
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");

    assert_eq!(repo.git(&["rev-parse", "HEAD"]), oid.to_hex());
    let stored = repo.git(&["log", "-1", "--format=%B"]);
    assert_eq!(stored, message.trim_end());
    assert_eq!(
        repo.git(&["show", "--name-only", "--format=", "HEAD"]),
        "a.txt"
    );
}

#[tokio::test]
async fn amend_replaces_the_head_commit() {
    let mut repo = TestRepo::init();
    let first = repo.commit_file("a.txt", "one\n", "original subject");
    repo.write_file("b.txt", "two\n");
    repo.git(&["add", "--", "b.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let amended = commit::commit(
        &exec,
        &repo_info,
        "reworded subject",
        CommitOptions {
            amend: true,
            ..Default::default()
        },
        &cancel,
    )
    .await
    .expect("amend");

    assert_ne!(amended.to_hex(), first, "amend rewrites the commit");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "reworded subject");
    let files = repo.git(&["show", "--name-only", "--format=", "HEAD"]);
    assert!(files.contains("a.txt") && files.contains("b.txt"));
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "1");
}

#[tokio::test]
async fn amend_without_a_message_keeps_the_old_one() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "keep me");
    repo.write_file("a.txt", "two\n");
    repo.git(&["add", "--", "a.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    commit::commit(
        &exec,
        &repo_info,
        "",
        CommitOptions {
            amend: true,
            ..Default::default()
        },
        &cancel,
    )
    .await
    .expect("amend --no-edit");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "keep me");
}

#[tokio::test]
async fn an_empty_message_is_refused_for_a_new_commit() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("b.txt", "two\n");
    repo.git(&["add", "--", "b.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let err = commit::commit(
        &exec,
        &repo_info,
        "   \n\n",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect_err("empty message rejected");
    assert!(err.to_string().contains("empty message"), "{err}");
}

#[tokio::test]
async fn committing_nothing_surfaces_gits_own_message() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let err = commit::commit(
        &exec,
        &repo_info,
        "nothing here",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect_err("nothing staged");
    assert!(
        err.to_string().contains("nothing to commit"),
        "git's wording is passed through: {err}"
    );
}

#[tokio::test]
async fn head_message_and_merge_detection() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "subject");
    repo.git(&["commit", "--amend", "-m", "subject\n\nbody line\n"]);
    let (exec, cancel) = env();

    assert_eq!(
        commit::head_message(&exec, &repo.path, &cancel)
            .await
            .expect("head message"),
        "subject\n\nbody line"
    );
    assert!(
        !commit::head_is_merge(&exec, &repo.path, &cancel)
            .await
            .expect("merge check")
    );

    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("b.txt", "two\n", "side");
    repo.git(&["checkout", "main"]);
    repo.commit_file("c.txt", "three\n", "main");
    repo.git(&["merge", "--no-ff", "-m", "merge side", "side"]);
    assert!(
        commit::head_is_merge(&exec, &repo.path, &cancel)
            .await
            .expect("merge check")
    );
}

#[tokio::test]
async fn branch_create_switch_rename_delete() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();

    branch::create(&exec, &repo.path, "feature", None, false, &cancel)
        .await
        .expect("create");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");

    branch::checkout(
        &exec,
        &repo.path,
        &CheckoutTarget::Branch {
            name: "feature".into(),
        },
        &cancel,
    )
    .await
    .expect("switch");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "feature");

    branch::rename(&exec, &repo.path, "feature", "renamed", false, &cancel)
        .await
        .expect("rename");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "renamed");

    branch::checkout(
        &exec,
        &repo.path,
        &CheckoutTarget::Branch {
            name: "main".into(),
        },
        &cancel,
    )
    .await
    .expect("switch back");
    branch::delete(&exec, &repo.path, "renamed", false, &cancel)
        .await
        .expect("delete merged branch");
    assert!(!repo.git(&["branch", "--list"]).contains("renamed"));
}

#[tokio::test]
async fn detached_checkout_is_explicit() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("a.txt", "two\n", "second");
    let (exec, cancel) = env();

    branch::checkout(
        &exec,
        &repo.path,
        &CheckoutTarget::Detach { rev: root.clone() },
        &cancel,
    )
    .await
    .expect("detach");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");
}

#[tokio::test]
async fn unmerged_branch_needs_the_forced_delete() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "wip"]);
    repo.commit_file("b.txt", "two\n", "unmerged work");
    repo.git(&["checkout", "main"]);
    let (exec, cancel) = env();

    assert!(
        !branch::is_merged_into(&exec, &repo.path, "wip", "HEAD", &cancel)
            .await
            .expect("merge check"),
        "wip is not reachable from main"
    );
    let err = branch::delete(&exec, &repo.path, "wip", false, &cancel)
        .await
        .expect_err("plain delete refused");
    assert!(err.to_string().contains("not fully merged"), "{err}");

    branch::delete(&exec, &repo.path, "wip", true, &cancel)
        .await
        .expect("forced delete");
    assert!(!repo.git(&["branch", "--list"]).contains("wip"));
}

/// A local branch created from a remote-tracking ref must track it, so the
/// sidebar badge and push defaults are right from the first checkout.
#[tokio::test]
async fn checkout_of_a_remote_branch_creates_a_tracking_branch() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["checkout", "-b", "published"]);
    origin.commit_file("b.txt", "two\n", "published work");
    origin.git(&["checkout", "main"]);

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    let (exec, cancel) = env();

    branch::checkout(
        &exec,
        &clone.path,
        &CheckoutTarget::Track {
            remote_ref: "origin/published".into(),
            local: "published".into(),
        },
        &cancel,
    )
    .await
    .expect("track");

    assert_eq!(
        clone.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "published"
    );
    assert_eq!(
        clone.git(&["rev-parse", "--abbrev-ref", "HEAD@{upstream}"]),
        "origin/published"
    );
}
