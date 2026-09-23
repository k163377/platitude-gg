//! What the command log sees when this app reads git configuration.
//!
//! Every read here asks about keys that are usually not set, and git says
//! so by exiting 1. That exit is the answer, so the command has to be
//! marked as answering by code (`GitCommand::answers_by_code`): left
//! unmarked it is logged as a failure and the command panel opens itself
//! over it — for the remotes read, on every repository that has no remote,
//! every time the sidebar refreshes (規約 core.md §終了コードで答える問い
//! 合わせはコマンドログでも答え).
//!
//! The flag shows in how the log ends the row — `Answered` for a marked
//! command, `Exited` for an unmarked one, whatever code it exited with —
//! so these tests read the ends. The reads that cannot be made to exit 1
//! from a test repository (every one of them has an identity, and
//! `core.autocrlf` is set) still pin the flag that way.

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

/// How every configuration *read* of this run ended, in order. Both
/// spellings (`--get`, `--get-regexp`) and no writes.
fn config_reads(log: &Log) -> Vec<CommandEnd> {
    log.ends_of(&[" config ", "--get"])
}

/// The identity and signing keys, read on every refresh. A machine with no
/// `user.name` anywhere exits 1 here, and one signing key short of the
/// pattern is enough for git to answer 0 — the flag decides the same thing
/// either way.
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

/// The same read narrowed to one repository's own file, which is where a
/// repository that overrides nothing gives the empty answer: the settings
/// screen asks this of every repository the reader picks from the strip,
/// and most of them will never have set a thing.
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

/// The remotes, read the same way — and here the empty answer is the one a
/// test repository actually gives.
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

    remote::replace_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        NET,
        &cancel,
    )
    .await
    .expect("replace remote branch");

    assert!(bare.git(&["branch", "--list"]).contains("billing-v2"));
    assert_answered(&config_reads(&log), "the tracking-branch read");
}

/// Where a push would go, for a branch that tracks nothing: the four keys
/// git resolves a destination from, none of them there.
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
        // `branch.<name>.remote`, `branch.<name>.merge`, and the marks
        // read (`branch.<name>.pushRemote` and `remote.pushDefault` in
        // one process) — every key unset, and every read an answer the
        // command log lets through.
        vec![
            CommandEnd::Answered(1),
            CommandEnd::Answered(1),
            CommandEnd::Answered(1)
        ]
    );
}

/// The merge tool the conflict pane offers: which one git would launch,
/// and which ones the user wrote a command for.
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

/// Whether git normalises line endings on the way into the index — read
/// off the fixture's `false`, then off a hand-written valueless key,
/// which is git's boolean true and comes back as a `-z` record with no
/// newline in it. That shape is git's to emit, so only a real read pins
/// it; the CLI cannot even write the key that produces it.
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
