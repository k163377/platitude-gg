//! Stash operations and remote traffic, exercised entirely offline: the
//! remote is a `file://` URL of a second local repository (実装計画 §11.3).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

use platitude_core::GitError;
use platitude_core::process::GitExecutor;
use platitude_core::remote::{self, PushForce, PushSpec};
use platitude_core::stash::{self, PushOptions};
use platitude_core::status;
use support::TestRepo;
use tokio_util::sync::CancellationToken;

fn env() -> (GitExecutor, CancellationToken) {
    (GitExecutor::new(), CancellationToken::new())
}

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// A bare repository serving as `origin`, plus a working clone of it.
fn origin_and_clone() -> (TestRepo, TestRepo) {
    let mut seed = TestRepo::init();
    seed.commit_file("a.txt", "one\n", "root");

    let mut bare = TestRepo::init();
    // Reuse the temp dir machinery, then turn the repo into a bare mirror.
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["config", "core.bare", "true"]);
    seed.git(&["remote", "add", "origin", &bare.file_url()]);
    seed.git(&["push", "origin", "main"]);
    (bare, seed)
}

#[tokio::test]
async fn stash_push_drop_and_pop() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "edited\n");
    let (exec, cancel) = env();

    stash::push(
        &exec,
        &repo.path,
        "my work in progress",
        PushOptions::default(),
        &[],
        &cancel,
    )
    .await
    .expect("stash push");

    let entries = stash::load(&exec, &repo.path, &cancel)
        .await
        .expect("stash list");
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0].message.contains("my work in progress"),
        "custom message kept: {}",
        entries[0].message
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "one\n",
        "working tree restored"
    );

    stash::pop(&exec, &repo.path, &entries[0].name, &cancel)
        .await
        .expect("stash pop");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "edited\n"
    );
    assert!(
        stash::load(&exec, &repo.path, &cancel)
            .await
            .expect("stash list")
            .is_empty(),
        "pop removes the entry"
    );
}

#[tokio::test]
async fn stash_includes_untracked_when_asked() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("new.txt", "fresh\n");
    let (exec, cancel) = env();

    stash::push(
        &exec,
        &repo.path,
        "with untracked",
        PushOptions {
            include_untracked: true,
            ..Default::default()
        },
        &[],
        &cancel,
    )
    .await
    .expect("stash push -u");
    assert!(
        !repo.path.join("new.txt").exists(),
        "untracked file stashed"
    );

    let entries = stash::load(&exec, &repo.path, &cancel).await.expect("list");
    stash::apply(&exec, &repo.path, &entries[0].name, &cancel)
        .await
        .expect("apply");
    assert!(repo.path.join("new.txt").exists());
    assert_eq!(
        stash::load(&exec, &repo.path, &cancel)
            .await
            .expect("list")
            .len(),
        1,
        "apply keeps the entry"
    );

    stash::drop(&exec, &repo.path, &entries[0].name, &cancel)
        .await
        .expect("drop");
    assert!(
        stash::load(&exec, &repo.path, &cancel)
            .await
            .expect("list")
            .is_empty()
    );
}

#[tokio::test]
async fn stash_keep_index_leaves_the_staged_part_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "staged\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("b.txt", "unstaged\n");
    repo.git(&["add", "--", "b.txt"]);
    repo.write_file("b.txt", "unstaged edited\n");
    let (exec, cancel) = env();

    stash::push(
        &exec,
        &repo.path,
        "keep index",
        PushOptions {
            keep_index: true,
            ..Default::default()
        },
        &[],
        &cancel,
    )
    .await
    .expect("stash --keep-index");

    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(s.staged().count(), 2, "index survives");
    assert_eq!(s.unstaged().count(), 0, "worktree matches the index");
}

#[tokio::test]
async fn stash_limited_to_one_path_leaves_the_rest_behind() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "one\n", "second");
    // The file that goes is changed on both sides at once, which the
    // path form handles: unlike `--staged`, it takes the whole path.
    repo.write_file("a.txt", "staged\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("a.txt", "staged then edited\n");
    repo.write_file("b.txt", "stays\n");
    repo.write_file("new.txt", "untracked, stays\n");
    let (exec, cancel) = env();

    stash::push(
        &exec,
        &repo.path,
        "just a.txt",
        PushOptions {
            include_untracked: true,
            ..Default::default()
        },
        &["a.txt".to_string()],
        &cancel,
    )
    .await
    .expect("stash push -- a.txt");

    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "one\n",
        "the named path went"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("b.txt")).unwrap(),
        "stays\n",
        "everything else stayed"
    );
    assert!(
        repo.path.join("new.txt").exists(),
        "an untracked file outside the path stays"
    );

    stash::pop_with_index(&exec, &repo.path, "stash@{0}", &cancel)
        .await
        .expect("pop --index");
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(
        s.partially_staged().count(),
        1,
        "the staged/unstaged split of the stashed path came back"
    );
}

#[tokio::test]
async fn staged_only_stash_needs_a_tree_split_git_can_separate() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "one\n", "second");
    repo.write_file("a.txt", "staged\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("b.txt", "unstaged\n");
    let (exec, cancel) = env();

    let staged_only = PushOptions {
        staged_only: true,
        ..Default::default()
    };
    stash::push(&exec, &repo.path, "index only", staged_only, &[], &cancel)
        .await
        .expect("stash --staged");
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(s.staged().count(), 0, "the index went");
    assert_eq!(s.unstaged().count(), 1, "the rest of the tree stayed");

    // Changed on both sides: git writes the entry and then fails to take
    // the staged half out of the tree, leaving the entry behind with
    // nothing else done. The UI refuses before reaching this (measured).
    repo.git(&["add", "--", "b.txt"]);
    repo.write_file("b.txt", "unstaged again\n");
    let before = stash::load(&exec, &repo.path, &cancel).await.expect("list");
    let err = stash::push(&exec, &repo.path, "doomed", staged_only, &[], &cancel)
        .await
        .expect_err("git cannot separate a file changed on both sides");
    assert!(
        err.to_string().contains("Cannot remove worktree changes"),
        "git's own wording: {err}"
    );
    let after = stash::load(&exec, &repo.path, &cancel).await.expect("list");
    assert_eq!(after.len(), before.len() + 1, "the entry is written anyway");
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(s.partially_staged().count(), 1, "and the tree is untouched");
}

#[tokio::test]
async fn lists_remotes_from_config() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();

    assert!(
        remote::list(&exec, &repo.path, &cancel)
            .await
            .expect("list")
            .is_empty(),
        "a fresh repository has no remotes"
    );

    repo.git(&["remote", "add", "origin", "file:///tmp/a"]);
    repo.git(&["remote", "add", "upstream", "file:///tmp/b"]);
    repo.git(&["remote", "set-url", "--push", "origin", "file:///tmp/push"]);

    let remotes = remote::list(&exec, &repo.path, &cancel)
        .await
        .expect("list");
    assert_eq!(remotes.len(), 2);
    assert_eq!(remotes[0].name, "origin");
    assert_eq!(remotes[0].fetch_url, "file:///tmp/a");
    assert_eq!(remotes[0].push_url, "file:///tmp/push");
    assert_eq!(remotes[1].name, "upstream");
}

#[tokio::test]
async fn push_then_fetch_moves_commits_between_repositories() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.commit_file("b.txt", "two\n", "second");
    remote::push(
        &exec,
        &work.path,
        &PushSpec {
            remote: "origin".into(),
            local: "main".into(),
            remote_branch: "main".into(),
            set_upstream: true,
            force: PushForce::None,
        },
        NET,
        &cancel,
    )
    .await
    .expect("push");

    assert_eq!(
        bare.git(&["log", "-1", "--format=%s", "main"]),
        "second",
        "the remote advanced"
    );
    assert_eq!(
        work.git(&["rev-parse", "--abbrev-ref", "main@{upstream}"]),
        "origin/main",
        "--set-upstream recorded the tracking branch"
    );

    // A third repository fetches what was pushed.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    remote::fetch(&exec, &other.path, Some("origin"), NET, &cancel)
        .await
        .expect("fetch");
    assert_eq!(
        other.git(&["log", "-1", "--format=%s", "origin/main"]),
        "second"
    );
}

#[tokio::test]
async fn a_non_fast_forward_push_is_refused_until_forced() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    // Someone else publishes first.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-b", "main", "origin/main"]);
    other.commit_file("theirs.txt", "theirs\n", "their work");
    other.git(&["push", "origin", "main"]);

    // Our own divergent history.
    work.git(&["commit", "--amend", "-m", "rewritten root"]);
    let spec = PushSpec {
        remote: "origin".into(),
        local: "main".into(),
        remote_branch: "main".into(),
        set_upstream: false,
        force: PushForce::None,
    };
    let err = remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect_err("non-fast-forward");
    assert!(
        err.to_string().contains("rejected") || err.to_string().contains("non-fast-forward"),
        "git's own rejection is passed through: {err}"
    );
    assert!(
        matches!(err, GitError::PushOutdated { .. }),
        "a refusal a fetch would answer is told apart from one it would not: {err}"
    );

    // A lease pinned to a commit the remote has moved past also fails, and
    // for the same reason: this window is looking at an older remote.
    let stale = work.git(&["rev-parse", "origin/main"]);
    let leased = PushSpec {
        force: PushForce::WithLease {
            expect: Some(stale),
        },
        ..spec.clone()
    };
    let err = remote::push(&exec, &work.path, &leased, NET, &cancel)
        .await
        .expect_err("stale lease");
    assert!(matches!(err, GitError::PushOutdated { .. }), "{err}");

    // Plain force wins.
    let forced = PushSpec {
        force: PushForce::Force,
        ..spec
    };
    remote::push(&exec, &work.path, &forced, NET, &cancel)
        .await
        .expect("forced push");
    assert_eq!(
        bare.git(&["log", "-1", "--format=%s", "main"]),
        "rewritten root"
    );
}

#[tokio::test]
async fn deleting_a_remote_branch_prunes_on_the_next_fetch() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["checkout", "-b", "temp"]);
    work.commit_file("t.txt", "t\n", "temp work");
    remote::push(
        &exec,
        &work.path,
        &PushSpec {
            remote: "origin".into(),
            local: "temp".into(),
            remote_branch: "temp".into(),
            set_upstream: false,
            force: PushForce::None,
        },
        NET,
        &cancel,
    )
    .await
    .expect("push temp");
    assert!(bare.git(&["branch", "--list"]).contains("temp"));

    remote::delete_remote_branch(&exec, &work.path, "origin", "temp", NET, &cancel)
        .await
        .expect("delete remote branch");
    assert!(!bare.git(&["branch", "--list"]).contains("temp"));

    remote::fetch(&exec, &work.path, Some("origin"), NET, &cancel)
        .await
        .expect("fetch --prune");
    assert!(
        !work
            .git(&["branch", "-r", "--list"])
            .contains("origin/temp"),
        "--prune dropped the stale remote-tracking ref"
    );
}

/// The rename git has no command for: the name moves, what the branch
/// pointed at does not, and the local branch that tracked it comes along.
#[tokio::test]
async fn renaming_a_remote_branch_moves_the_name_and_the_tracking() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);
    let tip = work.git(&["rev-parse", "billing"]);
    // Renaming publishes nothing: this commit is only here.
    work.commit_file("b.txt", "b2\n", "not published");

    remote::rename_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        NET,
        &cancel,
    )
    .await
    .expect("rename remote branch");

    let listed = bare.git(&["branch", "--list"]);
    assert!(listed.contains("billing-v2"), "{listed}");
    assert!(
        !listed.contains("  billing\n"),
        "the old name is gone: {listed}"
    );
    assert_eq!(
        bare.git(&["rev-parse", "billing-v2"]),
        tip,
        "the new name points where the old one did, not at what is only here"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing-v2",
        "the branch that tracked it follows the name"
    );
    assert!(
        !work
            .git(&["branch", "-r", "--list"])
            .contains("origin/billing\n"),
        "the tracking ref for the old name went with the delete"
    );
}

/// A push git turns down leaves the old name where it was: nothing is
/// deleted on the strength of a half-finished rename.
#[tokio::test]
async fn a_rename_whose_push_fails_deletes_nothing() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);
    // Somebody else's work already stands under the new name, on a line of
    // its own, so the push cannot be a fast-forward.
    work.git(&["switch", "-c", "someone-else", "main"]);
    work.commit_file("c.txt", "c\n", "not ours");
    work.git(&["push", "origin", "someone-else:billing-v2"]);
    work.git(&["switch", "billing"]);

    let error = remote::rename_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        NET,
        &cancel,
    )
    .await
    .expect_err("the push is refused");
    assert!(
        matches!(error, GitError::PushOutdated { .. }) || matches!(error, GitError::Failed { .. }),
        "{error:?}"
    );
    assert!(
        bare.git(&["branch", "--list"]).contains("billing"),
        "the old name is still there"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing",
        "and nothing was re-pointed"
    );
}
