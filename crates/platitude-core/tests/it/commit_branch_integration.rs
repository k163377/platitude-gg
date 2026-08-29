//! Commit / amend and local branch operations on real repositories.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::exec::env;
use crate::support::{TestRepo, info};
use platitude_core::branch::{self, CheckoutOutcome, CheckoutTarget, ResetMode};
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

    let head = commit::head_commit(&exec, &repo.path, &cancel)
        .await
        .expect("head commit");
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
        .expect("head commit");
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
        .expect("head commit");
    assert_eq!(head.author_name, "Test User");
    assert_eq!(head.author_email, "test@example.com");
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

/// Nothing offered here detaches HEAD, but git and the command line still
/// leave it detached — after a bisect, a `checkout <tag>`, an interrupted
/// rebase. That is a state to be worked from and left, and leaving it is
/// an ordinary switch.
#[tokio::test]
async fn a_detached_head_switches_back_onto_a_branch() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("a.txt", "two\n", "second");
    let (exec, cancel) = env();
    repo.git(&["checkout", "--detach", &root]);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");

    branch::checkout(
        &exec,
        &repo.path,
        &CheckoutTarget::Branch {
            name: "main".into(),
        },
        &cancel,
    )
    .await
    .expect("switch off the detached HEAD");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
}

/// Two commits on `main`; the returned id is the first of them, which is
/// where each reset takes the branch back to.
fn one_commit_back() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("a.txt", "two\n", "second");
    (repo, root)
}

/// The keeping-it-staged move: the branch goes back, the files do not,
/// and what the dropped commit wrote is ready to be committed again.
#[tokio::test]
async fn a_soft_reset_moves_the_branch_and_leaves_the_work_staged() {
    let (mut repo, root) = one_commit_back();
    let (exec, cancel) = env();

    branch::reset(&exec, &repo.path, &root, ResetMode::Soft, &cancel)
        .await
        .expect("soft reset");

    assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "two\n",
        "the working tree kept what the commit left behind wrote"
    );
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]),
        "a.txt",
        "and the difference is staged"
    );
}

/// The same move with the index cleared: the content is still on disk,
/// but nothing of it is staged.
#[tokio::test]
async fn a_mixed_reset_moves_the_branch_and_unstages_the_work() {
    let (mut repo, root) = one_commit_back();
    let (exec, cancel) = env();

    branch::reset(&exec, &repo.path, &root, ResetMode::Mixed, &cancel)
        .await
        .expect("mixed reset");

    assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "two\n"
    );
    assert_eq!(repo.git(&["diff", "--cached", "--name-only"]), "");
    assert_eq!(
        repo.git(&["diff", "--name-only"]),
        "a.txt",
        "the difference is there to stage again"
    );
}

/// The discarding move — the one the UI asks about. Tracked work goes,
/// staged or not; untracked files are none of a reset's business.
#[tokio::test]
async fn a_hard_reset_throws_tracked_work_away_and_leaves_untracked_files() {
    let (mut repo, root) = one_commit_back();
    repo.write_file("a.txt", "uncommitted\n");
    repo.write_file("staged.txt", "also mine\n");
    repo.git(&["add", "--", "staged.txt"]);
    repo.write_file("untracked.txt", "never recorded\n");
    let (exec, cancel) = env();

    branch::reset(&exec, &repo.path, &root, ResetMode::Hard, &cancel)
        .await
        .expect("hard reset");

    assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "one\n",
        "back to the content of the commit it landed on"
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "?? untracked.txt");
    assert!(
        !repo.path.join("staged.txt").exists(),
        "a staged new file is tracked work: it goes with the rest"
    );
    assert!(repo.path.join("untracked.txt").exists());
}

/// `main` and `other` differ in `both.txt` and agree on `same.txt`;
/// `theirs.txt` exists only on `other`. HEAD is left on `main`.
fn two_branches() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.write_file("both.txt", "base\n");
    repo.write_file("same.txt", "shared\n");
    repo.git(&["add", "--all"]);
    repo.git(&["commit", "-m", "root"]);
    repo.git(&["switch", "-c", "other"]);
    repo.write_file("both.txt", "theirs\n");
    repo.write_file("theirs.txt", "only over there\n");
    repo.git(&["add", "--all"]);
    repo.git(&["commit", "-m", "other"]);
    repo.git(&["switch", "main"]);
    repo
}

async fn move_to_other(repo: &TestRepo) -> CheckoutOutcome {
    let (exec, cancel) = env();
    branch::checkout(
        &exec,
        &repo.path,
        &CheckoutTarget::Branch {
            name: "other".into(),
        },
        &cancel,
    )
    .await
    .expect("switch")
}

/// The everyday case: work that is not in the way travels with the move,
/// and nothing has to be asked (デザイン規約 §未コミット変更がある状態での移動).
#[tokio::test]
async fn a_move_carries_uncommitted_work_along() {
    let mut repo = two_branches();
    repo.write_file("same.txt", "mine\n");

    assert!(matches!(move_to_other(&repo).await, CheckoutOutcome::Moved));
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("same.txt")).unwrap(),
        "mine\n",
        "the change came along"
    );
}

/// Work that *is* in the way stops the move dead — git changes nothing,
/// which is what makes it safe to go round the long way afterwards.
#[tokio::test]
async fn a_move_is_refused_when_the_changes_are_in_the_way() {
    let mut repo = two_branches();
    repo.write_file("both.txt", "mine\n");

    let outcome = move_to_other(&repo).await;
    let CheckoutOutcome::Blocked(refusal) = outcome else {
        panic!("expected a refusal, got {outcome:?}");
    };
    assert!(
        refusal.to_string().contains("would be overwritten"),
        "git's own words came back: {refusal}"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("both.txt")).unwrap(),
        "mine\n",
        "the refusal left the working tree alone"
    );
}

/// Untracked files are refused in words of their own, and the session has
/// to recognise those too: they are the case a stash gets *most* of the
/// way past, carrying every tracked change while the untracked file stays
/// behind in the entry.
#[tokio::test]
async fn untracked_files_in_the_way_are_a_refusal_too() {
    let mut repo = two_branches();
    repo.write_file("theirs.txt", "mine, uncommitted\n");

    let outcome = move_to_other(&repo).await;
    let CheckoutOutcome::Blocked(refusal) = outcome else {
        panic!("expected a refusal, got {outcome:?}");
    };
    assert!(
        refusal.to_string().contains("untracked working tree file"),
        "{refusal}"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
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

/// Landing on a remote branch whose local counterpart already exists.
///
/// `Track` cannot do it — `--create` refuses a name that is taken — which
/// is the whole reason `ForceCreate` exists: it moves the local branch to
/// the remote's commit and lands there in one command.
#[tokio::test]
async fn a_diverged_local_branch_is_moved_onto_the_remote_one() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.commit_file("b.txt", "two\n", "what the remote has");

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["checkout", "-b", "main", "origin/main~1"]);
    clone.commit_file("c.txt", "local\n", "only mine");
    let only_mine = clone.git(&["rev-parse", "HEAD"]);
    let (exec, cancel) = env();

    let err = branch::checkout(
        &exec,
        &clone.path,
        &CheckoutTarget::Track {
            remote_ref: "origin/main".into(),
            local: "main".into(),
        },
        &cancel,
    )
    .await
    .expect_err("--create cannot take a name that exists");
    assert!(err.to_string().contains("already exists"), "{err}");

    let outcome = branch::checkout(
        &exec,
        &clone.path,
        &CheckoutTarget::ForceCreate {
            local: "main".into(),
            start: "origin/main".into(),
        },
        &cancel,
    )
    .await
    .expect("force-create");

    assert!(matches!(outcome, CheckoutOutcome::Moved));
    assert_eq!(clone.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(
        clone.git(&["rev-parse", "HEAD"]),
        clone.git(&["rev-parse", "origin/main"]),
        "the branch now stands where the remote one does"
    );
    assert!(
        !branch::is_merged_into(&exec, &clone.path, &only_mine, "main", &cancel)
            .await
            .expect("merge check"),
        "the commit only the local branch had is no longer on it"
    );
}

/// The reference point `branch --delete` measures "merged" against: the
/// configured upstream where there is one, HEAD otherwise. `upstream_of`
/// resolves the first half; the pairing pinned here is the side git's
/// own refusal takes — a branch merged into HEAD but ahead of its
/// upstream still reads as unmerged (measured; the delete row's early
/// `-D` rides on this composition).
#[tokio::test]
async fn the_upstream_is_the_delete_reference_point() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    // Starting from the remote ref configures its upstream; starting the
    // second branch from a local commit leaves none.
    clone.git(&["checkout", "-b", "topic", "origin/main"]);
    clone.commit_file("b.txt", "two\n", "ahead of the upstream");
    clone.git(&["checkout", "-b", "keeper"]);
    let (exec, cancel) = env();

    assert_eq!(
        branch::upstream_of(&exec, &clone.path, "topic", &cancel)
            .await
            .expect("upstream read")
            .as_deref(),
        Some("refs/remotes/origin/main"),
        "the configured upstream comes back as the full refname"
    );
    assert_eq!(
        branch::upstream_of(&exec, &clone.path, "keeper", &cancel)
            .await
            .expect("upstream read"),
        None,
        "a branch started from a local commit has none"
    );

    assert!(
        branch::is_merged_into(&exec, &clone.path, "refs/heads/topic", "HEAD", &cancel)
            .await
            .expect("merge check"),
        "HEAD stands on the same commit"
    );
    assert!(
        !branch::is_merged_into(
            &exec,
            &clone.path,
            "refs/heads/topic",
            "refs/remotes/origin/main",
            &cancel
        )
        .await
        .expect("merge check"),
        "the upstream does not reach the new commit, which is the measure git refuses over"
    );
}

/// What `--set-upstream-to` is given has to be the full remote-tracking
/// refname. The shorthand git prints is a rev-parse spelling, and a local
/// branch of that exact name makes it **ambiguous** — git refuses the
/// whole command rather than choosing (実測), which would leave the
/// question answered on screen and nothing written. Pinned with the
/// collision in place, since that is the only shape the two spellings
/// disagree on.
#[tokio::test]
async fn the_upstream_is_named_by_the_one_spelling_that_reads_one_way() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["branch", "feature/x"]);

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["checkout", "-b", "topic", "origin/main"]);
    // The collision: a local branch called exactly what the shorthand for
    // the remote one is.
    clone.git(&["branch", "origin/feature/x", "origin/main"]);
    let (exec, cancel) = env();

    branch::set_upstream(
        &exec,
        &clone.path,
        "topic",
        "refs/remotes/origin/feature/x",
        &cancel,
    )
    .await
    .expect("set upstream");
    assert_eq!(
        branch::upstream_of(&exec, &clone.path, "topic", &cancel)
            .await
            .expect("upstream read")
            .as_deref(),
        Some("refs/remotes/origin/feature/x"),
        "the remote branch, not the local one wearing its name"
    );
    clone.git_expect_failure(&["branch", "--set-upstream-to=origin/feature/x", "topic"]);
}

/// Configuration about a branch rather than a move onto one: **the branch
/// another working copy has checked out takes it** (実測), where the same
/// row's delete is refused outright. Which rows that leaves is
/// `offers::ref_menu`'s answer; this is the half of it git owns.
#[tokio::test]
async fn a_branch_another_working_copy_holds_still_takes_an_upstream() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["branch", "topic", "origin/main"]);
    let held = clone.path.join("held");
    let held_arg = held.to_string_lossy().to_string();
    clone.git(&["worktree", "add", &held_arg, "topic"]);
    let (exec, cancel) = env();

    branch::set_upstream(
        &exec,
        &clone.path,
        "topic",
        "refs/remotes/origin/main",
        &cancel,
    )
    .await
    .expect("the other working copy is no refusal here");
    assert_eq!(
        branch::upstream_of(&exec, &clone.path, "topic", &cancel)
            .await
            .expect("upstream read")
            .as_deref(),
        Some("refs/remotes/origin/main")
    );
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
    write_hook(
        &repo,
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

/// Writes one of the repository's own hooks and makes it runnable.
fn write_hook(repo: &TestRepo, name: &str, body: &str) {
    let hooks = repo.path.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).expect("create hooks dir");
    let hook = hooks.join(name);
    std::fs::write(&hook, format!("#!/bin/sh\n{body}")).expect("write the hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
            .expect("make the hook executable");
    }
}
