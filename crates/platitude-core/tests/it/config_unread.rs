//! A configuration file git cannot open: the read asked once more, and the
//! opening that names it (`process::config_unread`, internal-docs/gitの扱い.md
//! §設定の書き換えと重なった読み).
//!
//! The file here stays unreadable — a folder in its place on Windows, a
//! link to itself elsewhere (the container runs as root, which reads past
//! any permission) — so every run meets it, and the number of runs is the
//! witness. The moment another git replaces a file, which the second run
//! gets past, is Windows' alone and has no station to stop it at.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::support::TestRepo;
use crate::support::exec::{env, observed_env};
use crate::support::wait::bounded;
use platitude_core::OperationId;
use platitude_core::process::{CommandEnd, CommandObserver, Kept, Meter};
use platitude_core::{GitCommand, GitError, GitExecutor, GitOutput, repo};
use tokio_util::sync::CancellationToken;

/// A repository whose own configuration git cannot open.
#[expect(clippy::expect_used)]
fn locked() -> TestRepo {
    let repo = TestRepo::init();
    let config = repo.path.join(".git").join("config");
    std::fs::remove_file(&config).expect("the configuration");
    #[cfg(windows)]
    std::fs::create_dir(&config).expect("a folder in its place");
    #[cfg(unix)]
    std::os::unix::fs::symlink("config", &config).expect("a link to itself");
    repo
}

/// `cmd` run to its end, and how many gits it took.
#[expect(clippy::expect_used)]
async fn counted(
    executor: &GitExecutor,
    cmd: GitCommand,
    cancel: &CancellationToken,
) -> (GitOutput, u32) {
    let meter = Arc::new(Meter::default());
    let out = bounded("the read", meter.over(executor.run_unchecked(cmd, cancel)))
        .await
        .expect("git ran");
    (out, meter.children())
}

/// A folder whose configuration stays unreadable is its own kind, neither
/// "not a repository" nor a plain failure: git's words, which name the
/// file, go under a heading of the screen's own.
#[tokio::test]
async fn an_opening_over_a_configuration_git_cannot_open_names_it() {
    let repo = locked();
    let (executor, cancel) = env();
    let meter = Arc::new(Meter::default());
    let err = bounded(
        "repo open",
        meter.over(repo::open(&executor, &repo.path, &cancel)),
    )
    .await
    .unwrap_err();
    match err {
        GitError::ConfigUnreadable { path, stderr } => {
            assert_eq!(path, repo.path);
            assert!(
                stderr.contains("unable to access '.git/config'"),
                "git names the file: {stderr}"
            );
        }
        other => panic!("an unreadable configuration is its own kind: {other:?}"),
    }
    assert_eq!(meter.children(), 2, "the opening was asked once more first");
}

/// A read runs once more and answers once; a write runs once, since it may
/// have written; a read with nothing in its way runs once.
#[tokio::test]
async fn a_read_runs_once_more_and_a_write_does_not() {
    let repo = locked();
    let (executor, cancel) = env();

    // The command line's own setting is always there to read: where git goes
    // on without the file (Windows), the answer is that one value, once.
    let read =
        GitCommand::new()
            .cwd(&repo.path)
            .args(["config", "-z", "--get-regexp", "^color\\.ui$"]);
    let (out, runs) = counted(&executor, read, &cancel).await;
    assert_eq!(runs, 2, "{}", out.stderr_utf8());
    if out.code == 0 {
        assert_eq!(out.stdout, b"color.ui\nfalse\0", "kept once, not twice");
    }

    let write = GitCommand::new()
        .cwd(&repo.path)
        .args(["config", "--local", "core.pggprobe", "1"]);
    let (out, runs) = counted(&executor, write, &cancel).await;
    assert_ne!(out.code, 0, "{}", out.stdout_utf8());
    assert_eq!(runs, 1, "a write is never asked twice");

    let healthy = TestRepo::init();
    let fine = GitCommand::new()
        .cwd(&healthy.path)
        .args(["rev-parse", "--absolute-git-dir"]);
    let (out, runs) = counted(&executor, fine, &cancel).await;
    assert_eq!(out.code, 0, "{}", out.stderr_utf8());
    assert_eq!(runs, 1);
}

/// Every row asked for and every end told, as they came — an end told twice
/// for one row is two entries here.
#[derive(Default)]
struct Told {
    rows: AtomicU64,
    ends: Mutex<Vec<(u64, CommandEnd, String)>>,
}

impl CommandObserver for Told {
    fn records(&self, _kept: Kept) -> bool {
        true
    }
    fn started(&self, _: &str, _: &str, _: Kept, _: Option<OperationId>) -> u64 {
        self.rows.fetch_add(1, Ordering::SeqCst)
    }
    fn finished(&self, id: u64, end: CommandEnd, _: u64, _: u64, message: &str) {
        self.ends
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((id, end, message.to_string()));
    }
}

/// The run asked once more is not a row of its own: the log holds one row
/// for the read, ended once, in the last run's words.
#[tokio::test]
async fn a_read_asked_once_more_is_one_row_ended_once() {
    let repo = locked();
    let told = Arc::new(Told::default());
    let (executor, cancel) = observed_env(Arc::clone(&told) as _, Kept::Asked);
    let read = GitCommand::new()
        .cwd(&repo.path)
        .args(["rev-parse", "--absolute-git-dir"]);
    let (out, runs) = counted(&executor, read, &cancel).await;
    assert_eq!(out.code, 128, "{}", out.stderr_utf8());
    assert_eq!(runs, 2);
    assert_eq!(told.rows.load(Ordering::SeqCst), 1);
    let ends = told.ends.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(ends.len(), 1, "{ends:?}");
    let (id, end, message) = &ends[0];
    assert_eq!((*id, *end), (0, CommandEnd::Exited(128)));
    assert!(
        message.contains("unable to access '.git/config'"),
        "{message}"
    );
}

/// A stream that died before its first byte has handed nothing on, so it
/// starts over as a kept read does.
#[tokio::test]
async fn a_stream_that_died_before_printing_runs_once_more() {
    let repo = locked();
    let (executor, cancel) = env();
    let meter = Arc::new(Meter::default());
    let mut printed = Vec::new();
    let walk = GitCommand::new()
        .cwd(&repo.path)
        .args(["log", "-z", "--format=%H"]);
    let result = bounded(
        "the walk",
        meter.over(executor.run_streaming(walk, &cancel, &mut |chunk: &[u8]| {
            printed.extend_from_slice(chunk)
        })),
    )
    .await;
    assert!(result.is_err(), "the file is still unreadable: {result:?}");
    assert!(printed.is_empty());
    assert_eq!(meter.children(), 2);
}
