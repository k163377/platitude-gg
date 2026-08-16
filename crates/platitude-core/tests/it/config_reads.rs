//! What the command log sees when this app reads git configuration.
//!
//! Every read here asks about keys that are usually not set, and git says
//! so by exiting 1. That exit is the answer, so the command has to be
//! marked as answering by code (`GitCommand::answers_by_code`): left
//! unmarked it is logged as a failure and the command panel opens itself
//! over it — for the remotes read, on every repository that has no remote,
//! every time the sidebar refreshes (規約 core.md §終了コードで答える問い
//! 合わせはコマンドログの失敗にしない).
//!
//! The flag shows in how the log ends the row — `Answered` for a marked
//! command, `Exited` for an unmarked one, whatever code it exited with —
//! so these tests read the ends rather than the answers. The reads that
//! cannot be made to exit 1 from a test repository (every one of them has
//! an identity, and `core.autocrlf` is set) still pin the flag that way.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use crate::support::TestRepo;
use crate::support::exec::observed_env;
use platitude_core::process::{CommandEnd, CommandObserver, GitExecutor};
use platitude_core::remote::{self, PushForce};
use platitude_core::{conflict, eol, identity};
use tokio_util::sync::CancellationToken;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// The command log's rows: what was spawned, and how it ended.
#[derive(Default)]
struct Log(Mutex<Vec<(String, Option<CommandEnd>)>>);

impl Log {
    /// How every configuration *read* of this run ended, in order. Both
    /// spellings (`--get`, `--get-regexp`) and no writes.
    fn config_reads(&self) -> Vec<CommandEnd> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(display, _)| display.contains(" config ") && display.contains("--get"))
            .filter_map(|(_, end)| *end)
            .collect()
    }
}

impl CommandObserver for Log {
    fn records(&self, _user: bool) -> bool {
        true
    }

    fn started(&self, display: &str, _full: &str, _user: bool) -> u64 {
        let mut rows = self.0.lock().unwrap();
        rows.push((display.to_string(), None));
        rows.len() as u64 - 1
    }

    fn finished(&self, id: u64, end: CommandEnd, _elapsed_ms: u64, _message: &str) {
        let mut rows = self.0.lock().unwrap();
        if let Some(row) = rows.get_mut(id as usize) {
            row.1 = Some(end);
        }
    }
}

fn logged() -> (GitExecutor, Arc<Log>, CancellationToken) {
    let log = Arc::new(Log::default());
    let (exec, cancel) = observed_env(log.clone(), true);
    (exec, log, cancel)
}

/// For the reads whose exit code depends on what the machine happens to
/// have configured: whichever way they answered, they answered.
#[track_caller]
fn assert_answered(reads: &[CommandEnd], what: &str) {
    assert!(!reads.is_empty(), "{what}: nothing was read at all");
    assert!(
        reads.iter().all(|e| matches!(e, CommandEnd::Answered(_))),
        "{what}: the log raises itself over an answer: {reads:?}"
    );
}

/// A bare repository serving as `origin`, plus a working clone of it.
fn origin_and_clone() -> (TestRepo, TestRepo) {
    let mut seed = TestRepo::init();
    seed.commit_file("a.txt", "one\n", "root");

    let mut bare = TestRepo::init();
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["config", "core.bare", "true"]);
    seed.git(&["remote", "add", "origin", &bare.file_url()]);
    seed.git(&["push", "origin", "main"]);
    (bare, seed)
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

    assert_eq!(log.config_reads(), vec![CommandEnd::Answered(0)]);
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
    assert_eq!(log.config_reads(), vec![CommandEnd::Answered(1)]);
}

/// Renaming a branch on a remote ends by re-pointing whatever tracked it,
/// which reads the whole `branch.` section.
#[tokio::test]
async fn reading_the_tracking_branches_answers_by_code() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, log, cancel) = logged();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);

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

    assert!(bare.git(&["branch", "--list"]).contains("billing-v2"));
    assert_answered(&log.config_reads(), "the tracking-branch read");
}

/// Where a push would go, for a branch that tracks nothing: two keys asked
/// for, neither of them there.
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
        log.config_reads(),
        vec![CommandEnd::Answered(1), CommandEnd::Answered(1)]
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

    assert_answered(&log.config_reads(), "the merge-tool reads");
}

/// Whether git normalises line endings on the way into the index.
#[tokio::test]
async fn reading_core_autocrlf_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, log, cancel) = logged();

    let normalises = eol::normalises(&exec, &repo.path, &cancel)
        .await
        .expect("normalises");

    assert!(!normalises, "the test repository sets core.autocrlf=false");
    assert_eq!(log.config_reads(), vec![CommandEnd::Answered(0)]);
}
