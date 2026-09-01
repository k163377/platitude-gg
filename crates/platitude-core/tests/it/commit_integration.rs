//! Commit and amend on real repositories: what git records, what it
//! refuses, and what a hook that says no comes back as.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::exec::env;
use crate::support::{TestRepo, info};
use platitude_core::branch;
use platitude_core::commit::{self, CommitOptions};
use platitude_core::report::ReportKind;

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
    let first = repo.commit_file_id("a.txt", "one\n", "original subject");
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

    let head = commit::head_commit(&exec, &repo.path, &cancel)
        .await
        .expect("head commit")
        .expect("HEAD exists");
    assert_eq!(head.message, "subject\n\nbody line");
    // Author and message come out of one command, NUL-separated: a
    // multi-line message may not be told apart by anything printable.
    assert_eq!(head.author_name, "Test User");
    assert_eq!(head.author_email, "test@example.com");
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

/// An unborn branch answers `None`: there is nothing to amend, and the
/// blank prefill must be distinguishable from a read that failed (a
/// failure would blank a real message).
#[tokio::test]
async fn head_commit_on_an_unborn_branch_is_no_commit_not_an_error() {
    let repo = TestRepo::init();
    let (exec, cancel) = env();
    let head = commit::head_commit(&exec, &repo.path, &cancel)
        .await
        .expect("asking is not an error");
    assert_eq!(head, None);
}

#[tokio::test]
async fn amending_keeps_the_author_until_reset_author_is_asked_for() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "theirs\n");
    repo.git(&["add", "--", "a.txt"]);
    // `--author` rather than `-c user.name`: the harness pins the author
    // through the environment, which config cannot outrank.
    repo.git(&[
        "commit",
        "--author=Other Person <other@example.com>",
        "-m",
        "written by someone else",
    ]);
    let (exec, cancel) = env();
    let info = info(&repo).await;

    // A plain amend records this machine as the committer and leaves the
    // author where it was — which is why taking over has to be asked for.
    commit::commit(
        &exec,
        &info,
        "reworded",
        CommitOptions {
            amend: true,
            ..Default::default()
        },
        &cancel,
    )
    .await
    .expect("amend");
    let head = commit::head_commit(&exec, &repo.path, &cancel)
        .await
        .expect("head commit")
        .expect("HEAD exists");
    assert_eq!(head.author_name, "Other Person");
    assert_eq!(head.author_email, "other@example.com");

    commit::commit(
        &exec,
        &info,
        "mine now",
        CommitOptions {
            amend: true,
            reset_author: true,
            ..Default::default()
        },
        &cancel,
    )
    .await
    .expect("amend --reset-author");
    let head = commit::head_commit(&exec, &repo.path, &cancel)
        .await
        .expect("head commit")
        .expect("HEAD exists");
    assert_eq!(head.author_name, "Test User");
    assert_eq!(head.author_email, "test@example.com");
}

/// A hook that says no is not a failure of this application's: nothing was
/// half written, and `--no-verify` is never passed, so there is no next
/// move here either (デザイン規約 §答えの要らない報せ).
///
/// **The hook's own words are what goes under the heading, from both
/// streams.** A linter wrapped in a hook writes its complaint to stdout
/// and its own noise to stderr, so reading stderr alone quotes the wrapper
/// and drops the complaint.
#[tokio::test]
async fn a_commit_a_hook_declines_comes_back_as_a_report_in_the_hooks_words() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_hook(
        "pre-commit",
        "echo \"a.txt:1 trailing whitespace\"\necho \"lint found 1 problem\" >&2\nexit 1\n",
    );
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let err = commit::commit(
        &exec,
        &repo_info,
        "feat: something the hook will not have",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect_err("the hook declines");

    let Some(report) = err.report() else {
        panic!("a commit a hook refused is a report, not a failure of ours: {err}");
    };
    assert_eq!(report.kind, ReportKind::Commit);
    assert!(
        report.remote.is_empty() && report.name.is_empty(),
        "nothing over a network and no ref: {report:?}"
    );
    assert!(
        report.reason.contains("lint found 1 problem")
            && report.reason.contains("a.txt:1 trailing whitespace"),
        "both streams come across: {}",
        report.reason
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "root",
        "the commit was not made"
    );
    assert!(
        err.to_string().contains("git commit"),
        "git's whole message is still there for the log: {err}"
    );
}

/// The same door for a commit git itself refuses: nothing here can answer
/// a signing key that will not sign, so it reads the same way a hook does.
#[tokio::test]
async fn a_commit_git_itself_refuses_reads_the_same_way() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    // A signing program that is not there: git's own refusal, on stderr,
    // with no hook in sight.
    repo.git(&["config", "commit.gpgsign", "true"]);
    repo.git(&["config", "gpg.program", "no-such-signer-here"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let err = commit::commit(
        &exec,
        &repo_info,
        "feat: something that cannot be signed",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect_err("nothing can sign this");

    let Some(report) = err.report() else {
        panic!("a commit git would not make is a report as well: {err}");
    };
    assert_eq!(report.kind, ReportKind::Commit);
    assert!(
        !report.reason.is_empty(),
        "git said why, and that is what goes under the heading"
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "root");
}

/// A name git will not take is a report, not an error: nothing moved, and
/// the box the name was typed into is still open to take the answer
/// (デザイン規約 §答えの要らない報せ). The name it carries is the one the row still
/// has, since the rename is exactly what did not happen.
#[tokio::test]
async fn a_rename_to_a_name_that_is_taken_is_reported_under_the_old_name() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["branch", "taken"]);
    let (exec, cancel) = env();

    let err = branch::rename(&exec, &repo.path, "main", "taken", false, &cancel)
        .await
        .expect_err("git will not take a name that is already there");
    let Some(report) = err.report() else {
        panic!("a name git would not take is a report, not a failure of ours: {err}");
    };
    assert_eq!(report.kind, ReportKind::RenameRefused);
    assert_eq!(report.name, "main", "the row still carries the old name");
    assert!(
        report.reason.contains("already exists"),
        "git said why, and that is what goes under the box: {}",
        report.reason
    );
    assert!(
        err.to_string().contains("git branch"),
        "git's whole message is still there for the log: {err}"
    );
    assert_eq!(
        repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "main",
        "nothing moved"
    );
}
