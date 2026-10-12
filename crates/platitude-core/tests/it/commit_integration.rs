//! Commit and amend on real repositories. What an amend does to HEAD is
//! git's own, and is in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::exec::env;
use crate::support::{TestRepo, info};
use platitude_core::GitError;
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

    let before = repo.git(&["rev-parse", "HEAD"]);
    let message = "subject line\n\nbody with \"quotes\" and 日本語\n#not-a-comment\n";
    commit::commit(
        &exec,
        &repo_info,
        message,
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");

    assert_ne!(repo.git(&["rev-parse", "HEAD"]), before, "HEAD moved");
    let stored = repo.git(&["log", "-1", "--format=%B"]);
    assert_eq!(stored, message.trim_end());
    assert_eq!(
        repo.git(&["show", "--name-only", "--format=", "HEAD"]),
        "a.txt"
    );
}

/// Reading HEAD back would put a second command behind the reply that can
/// fail on its own, and the write queue would report a commit that landed
/// as one that did not.
#[tokio::test]
async fn a_commit_asks_git_once_and_reads_nothing_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    let (exec, log, cancel) = crate::support::exec::logged();
    let repo_info = info(&repo).await;

    commit::commit(
        &exec,
        &repo_info,
        "one command",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");

    let ran: Vec<String> = log
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|(display, _)| display.clone())
        .collect();
    assert_eq!(ran.len(), 1, "one command and no read back: {ran:?}");
    assert!(ran[0].contains("commit"), "{ran:?}");
}

/// A commit that is empty on purpose (a release marker) takes a new
/// message like any other: git would refuse it as an amend that leaves the
/// commit empty, so the amend passes `--allow-empty` for one.
#[tokio::test]
async fn an_empty_commits_message_is_amended_on_its_own() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["commit", "--allow-empty", "-m", "chore: mark the release"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    commit::commit(
        &exec,
        &repo_info,
        "chore: mark the 1.0 release",
        CommitOptions {
            amend: true,
            ..Default::default()
        },
        &cancel,
    )
    .await
    .expect("the message changes and nothing else");

    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "chore: mark the 1.0 release"
    );
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(
        repo.git(&["rev-parse", "HEAD^{tree}"]),
        repo.git(&["rev-parse", "HEAD~1^{tree}"]),
        "still a marker"
    );
}

/// What the commit button asks before an amend is pressed: whether what is
/// staged takes back all HEAD changed. Only then, of all the shapes an
/// amend can be pressed over.
#[tokio::test]
async fn an_amend_empties_only_a_commit_whose_changes_are_staged_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();
    let empties = async |repo: &TestRepo| {
        commit::amend_would_empty(&exec, &repo.path, &cancel)
            .await
            .expect("a read")
    };
    assert!(!empties(&repo).await, "a root commit");

    repo.commit_file("a.txt", "two\n", "change a");
    assert!(!empties(&repo).await, "nothing staged");
    repo.write_file("b.txt", "new\n");
    repo.git(&["add", "--", "b.txt"]);
    assert!(!empties(&repo).await, "something staged beside the change");
    repo.git(&["rm", "--cached", "--quiet", "--", "b.txt"]);
    repo.write_file("a.txt", "one\n");
    repo.git(&["add", "--", "a.txt"]);
    assert!(empties(&repo).await, "the change staged back");

    repo.git(&["reset", "--quiet", "--hard"]);
    repo.git(&["commit", "--allow-empty", "-m", "marker"]);
    assert!(
        !empties(&repo).await,
        "an empty HEAD: a message-only amend, let through"
    );
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
    // Author and message come out of one command, NUL-separated: nothing
    // printable delimits a multi-line message.
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

/// A reader's `i18n.logOutputEncoding` — set for a terminal that is not
/// UTF-8 — would have git re-encode every message `log` prints; the
/// executor pins it to UTF-8, which is what every reader here parses.
#[tokio::test]
async fn a_readers_log_output_encoding_does_not_reach_what_is_read() {
    let mut repo = TestRepo::init();
    repo.git(&["config", "i18n.logOutputEncoding", "shift_jis"]);
    repo.commit_file("a.txt", "one\n", "日本語の件名");
    let (exec, cancel) = env();

    let head = commit::head_commit(&exec, &repo.path, &cancel)
        .await
        .expect("head commit")
        .expect("HEAD exists");
    assert_eq!(head.message, "日本語の件名");
}

/// A reader's `i18n.commitEncoding` would stamp the commits made here
/// with an `encoding` header the bytes do not match: every message this
/// application writes is UTF-8, and the executor says so.
#[tokio::test]
async fn a_commit_made_here_is_utf_8_whatever_commit_encoding_says() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.git(&["config", "i18n.commitEncoding", "shift_jis"]);
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    commit::commit(
        &exec,
        &repo_info,
        "日本語の件名",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");

    let object = repo.git_raw(&["cat-file", "commit", "HEAD"]);
    let text = String::from_utf8_lossy(&object);
    assert!(
        !text.contains("\nencoding "),
        "a UTF-8 commit carries no encoding header: {text}"
    );
    assert!(
        text.contains("日本語の件名"),
        "the message is the UTF-8 bytes that were written: {text}"
    );
}

/// A failed read must stay distinguishable from the blank prefill, or a
/// failure would blank a real message.
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
    // `--author`: the harness pins the author through the environment,
    // which config cannot outrank.
    repo.git(&[
        "commit",
        "--author=Other Person <other@example.com>",
        "-m",
        "written by someone else",
    ]);
    let (exec, cancel) = env();
    let info = info(&repo).await;

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

/// A commit of `a.txt` the outside is set up to turn down, and what came
/// back. `set_up` runs on a repository with a root commit and `a.txt`
/// staged.
async fn refused_commit(set_up: impl FnOnce(&mut TestRepo)) -> (TestRepo, GitError) {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    set_up(&mut repo);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let err = commit::commit(
        &exec,
        &repo_info,
        "feat: something the outside will not have",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect_err("the commit is turned down");
    (repo, err)
}

/// A report: nothing was half written and `--no-verify` stays off, so
/// there is no next move (デザイン規約 §答えの要らない報せ). The screen says
/// who said no; the hook's words — both streams, as git sends a commit
/// hook's stdout to its stderr — are the log's.
#[tokio::test]
async fn a_commit_a_hook_declines_is_reported_as_the_hooks() {
    let (mut repo, err) = refused_commit(|repo| {
        repo.write_hook(
            "pre-commit",
            "echo \"a.txt:1 trailing whitespace\"\necho \"lint found 1 problem\" >&2\nexit 1\n",
        );
    })
    .await;

    let Some(report) = err.report() else {
        panic!("a commit a hook refused is a report, not a failure of ours: {err}");
    };
    assert_eq!(report.kind, ReportKind::CommitHook, "{err}");
    assert!(
        report.remote.is_empty() && report.name.is_empty() && report.fact.is_empty(),
        "nothing over a network, no ref, no words: {report:?}"
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "root",
        "the commit was not made"
    );
    let said = err.to_string();
    assert!(
        said.contains("git commit")
            && said.contains("lint found 1 problem")
            && said.contains("a.txt:1 trailing whitespace"),
        "the hook's words, both streams, are there for the log: {said}"
    );
}

/// A hook that says nothing, the `commit-msg` hook, and 2.55's warning and
/// hints for `core.commentChar=auto` ahead of the hook's words: each is the
/// hook's no all the same.
#[tokio::test]
async fn a_hook_is_told_apart_from_whatever_git_says_around_it() {
    let (_, silent) = refused_commit(|repo| repo.write_hook("pre-commit", "exit 1\n")).await;
    assert_eq!(
        silent.report().map(|report| report.kind),
        Some(ReportKind::CommitHook),
        "a silent hook: {silent}"
    );
    let (_, message) = refused_commit(|repo| {
        repo.write_hook("commit-msg", "echo \"subject too long\" >&2\nexit 1\n");
    })
    .await;
    assert_eq!(
        message.report().map(|report| report.kind),
        Some(ReportKind::CommitHook),
        "commit-msg: {message}"
    );
    let (_, auto) = refused_commit(|repo| {
        repo.git(&["config", "core.commentChar", "auto"]);
        repo.write_hook("pre-commit", "echo \"lint found 1 problem\" >&2\nexit 1\n");
    })
    .await;
    assert_eq!(
        auto.report().map(|report| report.kind),
        Some(ReportKind::CommitHook),
        "commentChar=auto: {auto}"
    );
}

/// Another git holding the index's lock.
#[tokio::test]
async fn a_commit_another_git_holds_the_index_for_is_reported_as_locked() {
    let (_, err) = refused_commit(|repo| {
        std::fs::write(repo.path.join(".git/index.lock"), "").expect("lock");
    })
    .await;
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::CommitLocked),
        "{err}"
    );
}

/// A key that is not there, and ssh signing with no key named: git cannot
/// sign, and nothing was made.
#[tokio::test]
async fn a_commit_git_could_not_sign_is_reported_as_unsigned() {
    let (mut repo, missing) = refused_commit(|repo| {
        repo.git(&["config", "commit.gpgsign", "true"]);
        repo.git(&["config", "gpg.format", "ssh"]);
        let key = repo.path.join("no-such-key.pub");
        repo.git(&["config", "user.signingkey", &key.to_string_lossy()]);
    })
    .await;
    assert_eq!(
        missing.report().map(|report| report.kind),
        Some(ReportKind::CommitUnsigned),
        "a key that is not there: {missing}"
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "root");

    let (_, unnamed) = refused_commit(|repo| {
        repo.git(&["config", "commit.gpgsign", "true"]);
        repo.git(&["config", "gpg.format", "ssh"]);
    })
    .await;
    assert_eq!(
        unnamed.report().map(|report| report.kind),
        Some(ReportKind::CommitUnsigned),
        "no key named: {unnamed}"
    );
}

/// A report: nothing moved, and the box the name was typed into stays open
/// for the answer (デザイン規約 §答えの要らない報せ). A name taken since the
/// press is the one the box could not have answered.
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
    assert_eq!(
        report.fact, "",
        "git's words are not the screen's: its line points to the log"
    );
    assert!(
        err.to_string().contains("git branch") && err.to_string().contains("already exists"),
        "git's whole message is still there for the log: {err}"
    );
    assert_eq!(
        repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "main",
        "nothing moved"
    );
}

/// git moves the ref before it carries the branch's config section over,
/// so a config another git holds stops it half way: the branch has its new
/// name and the old one is gone. That is not "not renamed", whatever lock
/// the message names.
#[tokio::test]
async fn a_rename_stopped_after_the_ref_moved_is_reported_unfinished() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    std::fs::write(repo.path.join(".git/config.lock"), "").expect("lock");
    let (exec, cancel) = env();

    let err = branch::rename(&exec, &repo.path, "main", "renamed", false, &cancel)
        .await
        .expect_err("git stops when it cannot write the config");
    std::fs::remove_file(repo.path.join(".git/config.lock")).expect("unlock");
    let Some(report) = err.report() else {
        panic!("a rename git stopped half way is a report: {err}");
    };
    assert_eq!(report.kind, ReportKind::RenameUnfinished, "{err}");
    assert_eq!(report.name, "renamed", "the row now carries the new name");
    assert_eq!(report.fact, "", "the screen says it in its own words");
    assert_eq!(
        repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "renamed",
        "the ref moved before git stopped"
    );
}

/// Another git holding the new name's lock — at that moment, or one that
/// crashed and left the file — is its own report, which says so.
#[tokio::test]
async fn a_rename_another_git_holds_the_lock_for_is_reported_as_locked() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    std::fs::write(repo.path.join(".git/refs/heads/held.lock"), "").expect("lock");
    let (exec, cancel) = env();

    let err = branch::rename(&exec, &repo.path, "main", "held", false, &cancel)
        .await
        .expect_err("git will not write a ref another process has locked");
    let Some(report) = err.report() else {
        panic!("a lock is a report, not a failure of ours: {err}");
    };
    assert_eq!(report.kind, ReportKind::RenameLocked, "{err}");
    assert_eq!(report.fact, "", "the screen says it in its own words");
    assert_eq!(
        repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "main",
        "nothing moved"
    );
}

/// What an amend does to HEAD is git's, behind the fixed `--amend` the
/// tests above already pass — so the full gate runs it
/// (`-- --ignored ::periodic::`), not every change.
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "what git writes behind a fixed --amend: not worth the pre-merge run"]
    async fn amend_replaces_the_head_commit() {
        let mut repo = TestRepo::init();
        let first = repo.commit_file_id("a.txt", "one\n", "original subject");
        repo.write_file("b.txt", "two\n");
        repo.git(&["add", "--", "b.txt"]);
        let (exec, cancel) = env();
        let repo_info = info(&repo).await;

        commit::commit(
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

        assert_ne!(
            repo.git(&["rev-parse", "HEAD"]),
            first,
            "amend rewrites the commit"
        );
        assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "reworded subject");
        let files = repo.git(&["show", "--name-only", "--format=", "HEAD"]);
        assert!(files.contains("a.txt") && files.contains("b.txt"));
        assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "1");
    }
}
