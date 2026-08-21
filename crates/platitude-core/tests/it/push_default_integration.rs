//! Where a push goes when the repository, rather than the branch, decides
//! it (`remote.pushDefault`) — against real git, with `file://` remotes so
//! nothing leaves the machine (実装計画 §11.3).
//!
//! Every expectation here was recorded from git itself before it was
//! written down: the order it resolves the three keys in, the name it
//! pushes a branch under once the destination is not the one it tracks,
//! and the exit codes its reads answer with.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::remote::{self, PushForce};

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// Two bare repositories and a clone that has both as remotes: `origin`,
/// which `main` tracks, and `fork`, which nothing points at yet.
fn origin_fork_and_clone() -> (TestRepo, TestRepo, TestRepo) {
    let mut origin = TestRepo::init();
    let origin_path = origin.path.clone();
    origin.git_in(&origin_path, &["config", "core.bare", "true"]);

    let mut fork = TestRepo::init();
    let fork_path = fork.path.clone();
    fork.git_in(&fork_path, &["config", "core.bare", "true"]);

    let mut work = TestRepo::init();
    work.commit_file("a.txt", "one\n", "root");
    work.git(&["remote", "add", "origin", &origin.file_url()]);
    work.git(&["remote", "add", "fork", &fork.file_url()]);
    work.git(&["push", "--set-upstream", "origin", "main"]);
    (origin, fork, work)
}

#[tokio::test]
async fn nothing_marked_sends_a_branch_where_it_tracks() {
    let (_origin, _fork, repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "origin");
    assert_eq!(plan.remote_branch, "main");
    assert!(!plan.set_upstream, "the branch already tracks something");
}

/// The whole point of the mark: a branch that tracks `origin/main` pushes
/// somewhere else, under its own name, and keeps tracking origin.
#[tokio::test]
async fn a_marked_remote_takes_the_push_from_the_upstream() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    remote::set_push_default(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "fork");
    assert_eq!(plan.remote_branch, "main", "the branch's own name");
    assert!(
        !plan.set_upstream,
        "a branch that tracks origin must go on tracking it"
    );

    remote::push(&exec, &repo.path, &plan, NET, &cancel)
        .await
        .expect("the push lands");
    assert_eq!(
        repo.git(&["config", "--get", "branch.main.remote"]).trim(),
        "origin",
        "where the branch fetches from is untouched"
    );
}

/// git's own order: the branch's own push remote wins over the
/// repository's mark, which wins over what the branch tracks.
#[tokio::test]
async fn a_branchs_own_push_remote_beats_the_mark() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    remote::set_push_default(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");
    repo.git(&["config", "branch.main.pushRemote", "origin"]);

    let plan = remote::plan_current_push(&exec, &repo.path, "fork", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "origin");
}

/// A branch nobody has pushed yet goes to the mark rather than to the
/// fallback the caller offered, and records the upstream as a first push
/// always has.
#[tokio::test]
async fn a_branch_with_no_upstream_goes_to_the_mark() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    remote::set_push_default(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");
    repo.git(&["switch", "--create", "topic"]);

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "fork");
    assert_eq!(plan.remote_branch, "topic");
    assert!(plan.set_upstream, "the first push records where it went");
}

/// The upstream names a branch on one remote and says nothing about any
/// other: going to the remote it tracks keeps that name, going anywhere
/// else uses the branch's own.
#[tokio::test]
async fn an_upstream_under_another_name_is_kept_only_on_its_own_remote() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    repo.git(&["switch", "--create", "topic"]);
    repo.git(&["push", "origin", "topic:elsewhere"]);
    repo.git(&["config", "branch.topic.remote", "origin"]);
    repo.git(&["config", "branch.topic.merge", "refs/heads/elsewhere"]);

    let tracked = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(tracked.remote, "origin");
    assert_eq!(tracked.remote_branch, "elsewhere");

    remote::set_push_default(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");
    let marked = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(marked.remote, "fork");
    assert_eq!(marked.remote_branch, "topic", "not the upstream's name");
}

#[tokio::test]
async fn the_mark_reads_back_with_the_level_that_set_it() {
    let (_origin, _fork, repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    assert!(
        remote::push_default(&exec, &repo.path, &cancel)
            .await
            .expect("an answer")
            .is_none(),
        "unset is an answer, not a failure"
    );

    remote::set_push_default(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");
    let marked = remote::push_default(&exec, &repo.path, &cancel)
        .await
        .expect("an answer")
        .expect("a mark");
    assert_eq!(marked.remote, "fork");
    assert!(marked.local, "this repository's own config set it");

    remote::clear_push_default(&exec, &repo.path, &cancel)
        .await
        .expect("clear");
    assert!(
        remote::push_default(&exec, &repo.path, &cancel)
            .await
            .expect("an answer")
            .is_none()
    );
    remote::clear_push_default(&exec, &repo.path, &cancel)
        .await
        .expect("clearing a mark that is already gone is the state asked for");
}

/// The list and the mark come back from one read, which is what the
/// session keeps.
#[tokio::test]
async fn one_read_answers_for_both() {
    let (_origin, _fork, repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    remote::set_push_default(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");

    let read = remote::read(&exec, &repo.path, &cancel)
        .await
        .expect("read");
    let names: Vec<&str> = read.list.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["fork", "origin"]);
    assert_eq!(read.push_default.expect("a mark").remote, "fork");
}
