//! What the command log sees when this app reads git configuration.
//!
//! These keys are usually unset and git answers that with exit 1; unmarked
//! (`GitCommand::answers_by_code`), the panel opens over it — for remotes,
//! on every refresh of a repository with none
//! (core.md「終了コードで答える問い合わせは」). The tests read how the log
//! ends the row (`Answered` marked, `Exited` unmarked, whatever the code),
//! so reads a test repository cannot make exit 1 still pin the flag.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::{Log, assert_answered, logged};
use crate::support::remote::origin_and_clone;
use platitude_core::process::CommandEnd;
use platitude_core::remote::{self, PushForce};
use platitude_core::{conflict, eol, identity};

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// Every config read, in order: `--get` and `--get-regexp`, no writes.
fn config_reads(log: &Log) -> Vec<CommandEnd> {
    log.ends_of(&[" config ", "--get"])
}

/// Read on every refresh; a machine with no `user.name` exits 1 here.
#[tokio::test]
async fn reading_the_identity_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, log, cancel) = logged();

    identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load");

    assert_eq!(config_reads(&log), vec![CommandEnd::Answered(0)]);
}

/// Narrowed to one repository's own file, where the empty answer is the
/// usual one: settings asks it of every repository picked from the strip.
#[tokio::test]
async fn reading_one_repositorys_own_identity_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--local", "--unset", "user.name"]);
    repo.git(&["config", "--local", "--unset", "user.email"]);
    let (exec, log, cancel) = logged();

    let held = identity::load_local(&exec, &repo.path, &cancel)
        .await
        .expect("load_local");

    assert_eq!(held, identity::Identity::default());
    assert_eq!(config_reads(&log), vec![CommandEnd::Answered(1)]);
}

#[tokio::test]
async fn listing_remotes_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, log, cancel) = logged();

    let remotes = remote::list(&exec, &repo.path, &cancel)
        .await
        .expect("list");

    assert!(remotes.is_empty(), "a fresh repository has no remotes");
    assert_eq!(config_reads(&log), vec![CommandEnd::Answered(1)]);
}

/// Replacing a branch on a remote ends by re-pointing whatever tracked it,
/// which reads the whole `branch.` section.
#[tokio::test]
async fn reading_the_tracking_branches_answers_by_code() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, log, cancel) = logged();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);
    let tip = work.git(&["rev-parse", "origin/billing"]);

    remote::replace_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        &tip,
        NET,
        &cancel,
    )
    .await
    .expect("replace remote branch");

    assert!(bare.git(&["branch", "--list"]).contains("billing-v2"));
    assert_answered(&config_reads(&log), "the tracking-branch read");
}

/// A branch that tracks nothing: none of the four keys git resolves a
/// destination from is set.
#[tokio::test]
async fn planning_a_push_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, log, cancel) = logged();

    let spec = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("plan");

    assert!(
        spec.set_upstream,
        "nothing was tracked, so the push records it"
    );
    assert_eq!(
        config_reads(&log),
        // `branch.<name>.remote`, `branch.<name>.merge`, then
        // `branch.<name>.pushRemote` and `remote.pushDefault` in one process.
        vec![
            CommandEnd::Answered(1),
            CommandEnd::Answered(1),
            CommandEnd::Answered(1)
        ]
    );
}

#[tokio::test]
async fn reading_the_merge_tools_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, log, cancel) = logged();

    conflict::configured_tool(&exec, &repo.path, &cancel)
        .await
        .expect("configured tool");
    conflict::user_defined_tools(&exec, &repo.path, &cancel)
        .await
        .expect("user defined tools");

    assert_answered(&config_reads(&log), "the merge-tool reads");
}

/// A hand-written valueless key is git's boolean true and comes back as a
/// `-z` record with no newline; the CLI cannot write that key, so only a
/// real read pins the shape.
#[tokio::test]
async fn reading_core_autocrlf_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, log, cancel) = logged();

    let normalises = eol::normalises(&exec, &repo.path, &cancel)
        .await
        .expect("normalises");

    assert!(!normalises, "the test repository sets core.autocrlf=false");
    assert_eq!(config_reads(&log), vec![CommandEnd::Answered(0)]);

    // Appended, so the last record also has to win over the `false` the
    // fixture sets.
    let config = repo.path.join(".git").join("config");
    let mut text = std::fs::read_to_string(&config).expect("read config");
    text.push_str("[core]\n\tautocrlf\n");
    std::fs::write(&config, text).expect("write config");

    let normalises = eol::normalises(&exec, &repo.path, &cancel)
        .await
        .expect("normalises after the hand edit");
    assert!(normalises, "a valueless boolean key is git's true");
    assert_eq!(
        config_reads(&log),
        vec![CommandEnd::Answered(0), CommandEnd::Answered(0)]
    );
}
