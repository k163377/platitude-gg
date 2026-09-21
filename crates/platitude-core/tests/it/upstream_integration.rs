//! What a branch follows: the tracking branch a checkout of a remote ref
//! creates, the upstream a diverged branch is moved onto, and the two
//! things the upstream is the reference point for.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::branch::{self, CheckoutOutcome, CheckoutTarget};

/// What `%(upstream)` prints for a local branch — the spelling the refs
/// listing carries (`RefEntry::upstream`) and joins the delete's
/// reference point on; `None` where nothing is configured.
fn upstream_of(repo: &mut TestRepo, branch: &str) -> Option<String> {
    let printed = repo.git(&[
        "for-each-ref",
        "--format=%(upstream)",
        &format!("refs/heads/{branch}"),
    ]);
    (!printed.is_empty()).then_some(printed)
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

/// Moving a branch says nothing about what it reads, so the move must not
/// re-point it.
///
/// **`switch --force-create` applies `branch.autoSetupMerge` to a
/// remote-tracking start point, existing branch or not** (measured 2.55:
/// `origin/topic` became `up/2.21`, and `checkout -B` does the same; plain
/// `reset --hard` and `branch -f` leave it alone). The reflog says only
/// `reset: moving to <start>`, so nothing on screen or in the log names
/// the upstream that was thrown away — which is why `--no-track` is on the
/// command and not left to the default.
#[tokio::test]
async fn moving_a_branch_onto_a_remote_ref_leaves_its_upstream_alone() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["checkout", "-b", "topic"]);
    origin.commit_file("b.txt", "two\n", "what the remote has");
    origin.git(&["checkout", "main"]);
    origin.commit_file("c.txt", "three\n", "where the branch is moved to");

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["checkout", "-b", "topic", "origin/topic"]);
    let (exec, cancel) = env();

    assert_eq!(
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/topic"),
        "the branch reads its own remote before the move"
    );

    branch::checkout(
        &exec,
        &clone.path,
        &CheckoutTarget::ForceCreate {
            local: "topic".into(),
            start: "origin/main".into(),
        },
        &cancel,
    )
    .await
    .expect("force-create");

    assert_eq!(
        clone.git(&["rev-parse", "HEAD"]),
        clone.git(&["rev-parse", "origin/main"]),
        "the branch stands where it was moved"
    );
    assert_eq!(
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/topic"),
        "and still reads what it read before"
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
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/main"),
        "the configured upstream comes back as the full refname"
    );
    assert_eq!(
        upstream_of(&mut clone, "keeper"),
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
    let err = branch::delete(&exec, &clone.path, "topic", false, &cancel)
        .await
        .expect_err("refused over the upstream while HEAD stands on the tip");
    assert!(err.to_string().contains("not fully merged"), "{err}");
}

/// The other half of the reference point: an upstream that is a local
/// branch (`remote = .`). `%(upstream)` spells it as a `refs/heads/`
/// name, which is the key the refs listing joins it on
/// (`BranchItem::upstream_oid`), and git measures the delete against it
/// just the same — a tip HEAD does not reach goes once that branch does
/// (measured; what git prints about HEAD is a warning).
#[tokio::test]
async fn a_local_upstream_is_the_reference_point_too() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("b.txt", "two\n", "topic work");
    repo.git(&["checkout", "main"]);
    repo.git(&["merge", "--ff-only", "topic"]);
    repo.git(&["branch", "--set-upstream-to=main", "topic"]);
    repo.git(&["checkout", "-b", "elsewhere", &root]);
    let (exec, cancel) = env();

    assert_eq!(
        upstream_of(&mut repo, "topic").as_deref(),
        Some("refs/heads/main"),
        "a local upstream comes back under refs/heads/, the key the listing holds it by"
    );
    assert!(
        !branch::is_merged_into(&exec, &repo.path, "refs/heads/topic", "HEAD", &cancel)
            .await
            .expect("merge check"),
        "HEAD stands on the root and does not reach the tip"
    );
    branch::delete(&exec, &repo.path, "topic", false, &cancel)
        .await
        .expect("the tip is on its upstream, which is the measure");
    assert!(!repo.git(&["branch", "--list", "topic"]).contains("topic"));
}

/// An upstream that names nothing — here one whose branch was deleted
/// after being set — is still configured (`%(upstream)` prints the
/// name), but it is not the measure: git falls back to HEAD (measured).
/// The listing join answers the same way by looking the name up, so
/// the delete row reads HEAD there too.
#[tokio::test]
async fn an_upstream_that_names_nothing_leaves_head_as_the_reference_point() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "later work");
    repo.git(&["branch", "topic", "main"]);
    repo.git(&["branch", "helper", "main"]);
    repo.git(&["branch", "--set-upstream-to=helper", "topic"]);
    repo.git(&["branch", "-D", "helper"]);
    repo.git(&["checkout", "-b", "elsewhere", &root]);
    let (exec, cancel) = env();

    assert_eq!(
        upstream_of(&mut repo, "topic").as_deref(),
        Some("refs/heads/helper"),
        "the configuration outlives the branch it named"
    );
    let err = branch::delete(&exec, &repo.path, "topic", false, &cancel)
        .await
        .expect_err("HEAD is the measure once the upstream resolves to nothing");
    assert!(err.to_string().contains("not fully merged"), "{err}");

    repo.git(&["checkout", "main"]);
    branch::delete(&exec, &repo.path, "topic", false, &cancel)
        .await
        .expect("HEAD reaches the tip now");
    assert!(!repo.git(&["branch", "--list", "topic"]).contains("topic"));
}

/// What `--set-upstream-to` is given has to be the full remote-tracking
/// refname. The shorthand git prints is a rev-parse spelling, and a
/// local branch of that exact name makes it **ambiguous** — git refuses
/// the whole command (measured), which would leave the question
/// answered on screen and nothing written. Pinned with the collision
/// in place, since that is the only shape the two spellings disagree
/// on.
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
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/feature/x"),
        "the remote branch, not the local one wearing its name"
    );
    clone.git_expect_failure(&["branch", "--set-upstream-to=origin/feature/x", "topic"]);
}

/// Configuration about a branch: **the branch another working copy has
/// checked out takes it** (measured), where the same row's delete is
/// refused outright. Which rows that leaves is `offers::ref_menu`'s
/// answer; this is the half of it git owns.
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
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/main")
    );
}
