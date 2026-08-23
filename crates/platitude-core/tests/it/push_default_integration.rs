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
use crate::support::exec::{env, logged_global};
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

/// **The label and the send must name the same remote.** The toolbar
/// says where a push is going from configuration the snapshot carries
/// (`remote::push_target`); the send works it out again from git
/// (`remote::plan_current_push`). Two spellings of one order is how the
/// first came to read `remote.pushDefault` while the second read the
/// branch's own mark first — so this walks every arrangement of the
/// three keys and holds the two answers against each other.
#[tokio::test]
async fn the_label_names_the_remote_the_send_uses() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();
    let remotes = ["fork", "origin"];

    // (branch.main.pushRemote, remote.pushDefault) — unset written as "".
    for (push_remote, push_default) in [
        ("", ""),
        ("", "fork"),
        ("fork", ""),
        ("fork", "origin"),
        ("origin", "fork"),
        ("origin", ""),
    ] {
        // Exit 5 where the key was never there, which is the state asked
        // for rather than a failure (実測).
        repo.git_ok(&["config", "--unset", "branch.main.pushRemote"]);
        remote::clear_push_default(&exec, &repo.path, &cancel)
            .await
            .expect("clear the mark");
        if !push_remote.is_empty() {
            repo.git(&["config", "branch.main.pushRemote", push_remote]);
        }
        if !push_default.is_empty() {
            remote::set_push_default(&exec, &repo.path, push_default, &cancel)
                .await
                .expect("mark");
        }

        let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
            .await
            .expect("a plan");
        let label = remote::push_target(
            "main",
            "origin/main",
            push_remote,
            push_default,
            "origin",
            remotes,
        );
        assert_eq!(
            label,
            format!("{}/{}", plan.remote, plan.remote_branch),
            "pushRemote={push_remote:?} pushDefault={push_default:?}: \
             the label and the send disagree"
        );
    }
}

/// The fork arrangement the label used to get wrong: the branch marks its
/// own destination, the repository marks none, and the counts on screen
/// are about the remote the branch tracks rather than the one it is going
/// to.
#[tokio::test]
async fn a_branch_marked_at_a_fork_says_so() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();
    let remotes = ["fork", "origin"];

    repo.git(&["config", "branch.main.pushRemote", "fork"]);

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "fork");

    assert_eq!(
        remote::push_target("main", "origin/main", "fork", "", "origin", remotes),
        "fork/main"
    );
    assert_eq!(
        remote::push_standing(
            false,
            "main",
            "origin/main",
            true,
            2,
            0,
            "fork",
            "",
            remotes
        ),
        remote::PushStanding::Elsewhere,
        "the counts are about origin, and the push is going to the fork"
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

/// The keys the snapshot reads, through the one reader both halves share
/// — and one process for the pair.
#[tokio::test]
async fn the_marks_read_back_and_unset_is_an_answer() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    assert!(
        marks.push_remote.is_none() && marks.push_default.is_none(),
        "unset is an answer, not a failure"
    );

    repo.git(&["config", "branch.main.pushRemote", "fork"]);
    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    assert_eq!(marks.push_remote.as_deref(), Some("fork"));

    assert!(
        remote::push_marks(&exec, &repo.path, "topic", &cancel)
            .await
            .expect("an answer")
            .push_remote
            .is_none(),
        "another branch's mark is not this one's"
    );
}

/// A branch named with regex metacharacters reads its own mark and nobody
/// else's: the name goes into the `--get-regexp` pattern escaped (実測
/// 2.55: unescaped, `wip.v2+x` answers with `wipAv22x`'s mark), and the
/// send resolves the same way.
#[tokio::test]
async fn a_metacharacter_branch_reads_its_own_mark() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    repo.git(&["switch", "--create", "wip.v2+x"]);
    repo.git(&["config", "branch.wip.v2+x.pushRemote", "fork"]);
    repo.git(&["config", "branch.wipAv22x.pushRemote", "origin"]);

    let marks = remote::push_marks(&exec, &repo.path, "wip.v2+x", &cancel)
        .await
        .expect("an answer");
    assert_eq!(marks.push_remote.as_deref(), Some("fork"));
    let decoy = remote::push_marks(&exec, &repo.path, "wipAv22x", &cancel)
        .await
        .expect("an answer");
    assert_eq!(
        decoy.push_remote.as_deref(),
        Some("origin"),
        "the decoy keeps its own mark"
    );

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "fork");
    assert_eq!(plan.remote_branch, "wip.v2+x", "the branch's own name");
}

/// The scope the label used to go stale in: a mark written into the
/// user's global configuration. The read sees it with its level, the
/// label spells the same destination the send resolves, and the marks
/// git weighs above it still win.
#[tokio::test]
async fn a_global_mark_is_read_and_sent_alike() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, _log, cancel) = logged_global(repo.global_config());
    let remotes = ["fork", "origin"];

    repo.git(&["config", "--global", "remote.pushDefault", "fork"]);

    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    let marked = marks.push_default.clone().expect("the global mark is read");
    assert_eq!(marked.remote, "fork");
    assert!(
        !marked.local,
        "another level set it; it cannot be unset here"
    );

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    let label = remote::push_target(
        "main",
        "origin/main",
        marks.push_remote.as_deref().unwrap_or(""),
        &marked.remote,
        "origin",
        remotes,
    );
    assert_eq!(
        label,
        format!("{}/{}", plan.remote, plan.remote_branch),
        "the label and the send disagree at global scope"
    );

    // This repository's own mark beats the global one…
    remote::set_push_default(&exec, &repo.path, "home", &cancel)
        .await
        .expect("mark home locally");
    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    let marked = marks.push_default.expect("a mark");
    assert_eq!(marked.remote, "home");
    assert!(marked.local, "the repository's own config decided");

    // …and the branch's own mark beats them both, for the read and the
    // send alike.
    repo.git(&["config", "branch.main.pushRemote", "origin"]);
    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    assert_eq!(marks.push_remote.as_deref(), Some("origin"));
    let plan = remote::plan_current_push(&exec, &repo.path, "fork", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "origin");
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
