//! What a test runs git with: an executor, a token, and the observer that
//! watches what went out.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use platitude_core::OperationId;
use platitude_core::process::{CommandEnd, CommandObserver, GitExecutor, Kept};
use tokio_util::sync::CancellationToken;

/// No fsync (the repositories are throwaway), no auto-gc mid-test.
const KNOBS: [(&str, &str); 2] = [("core.fsync", "none"), ("gc.auto", "0")];

/// Where the shared executor's `--global` file and XDG folder are: a folder
/// nobody makes, so git reads nothing there and a `--global` write through
/// it fails (a test that writes one has its own — [`isolated_global`]).
fn nowhere() -> PathBuf {
    super::temp::root().join(".no-config")
}

/// Configuration from no file outside the repository: [`KNOBS`] travel in
/// the environment (`GIT_CONFIG_COUNT`, git 2.31), since a file needs a
/// folder, and one shared by the suite would be a static never dropped —
/// left under `target/tmp` by every run. Also for a git a test runs by
/// hand, outside any executor.
pub fn isolated_env() -> Vec<(OsString, OsString)> {
    let mut env = vec![
        (
            OsString::from("GIT_CONFIG_GLOBAL"),
            nowhere().join("global-config").into_os_string(),
        ),
        (OsString::from("GIT_CONFIG_NOSYSTEM"), OsString::from("1")),
        (
            OsString::from("XDG_CONFIG_HOME"),
            nowhere().into_os_string(),
        ),
        super::temp::ceiling(),
        (
            OsString::from("GIT_CONFIG_COUNT"),
            OsString::from(KNOBS.len().to_string()),
        ),
    ];
    for (i, (key, value)) in KNOBS.iter().enumerate() {
        env.push((
            OsString::from(format!("GIT_CONFIG_KEY_{i}")),
            OsString::from(key),
        ));
        env.push((
            OsString::from(format!("GIT_CONFIG_VALUE_{i}")),
            OsString::from(value),
        ));
    }
    env
}

/// Git executor for integration tests. The raw `GitExecutor::new()` remains
/// available for tests that intentionally exercise the host configuration.
///
/// The stock budget is raised to the suite's overall backstop: the
/// everyday cap would decide by load (`wait.rs`), and a wedged git must
/// still fail a test that awaits the executor directly, with no `Patience`.
pub fn isolated() -> GitExecutor {
    GitExecutor::new()
        .with_stock_timeout(super::wait::OVERALL_BUDGET)
        .with_env(isolated_env())
}

/// An executor and a token nothing ever cancels.
pub fn env() -> (GitExecutor, CancellationToken) {
    (isolated(), CancellationToken::new())
}

/// [`isolated`] with `extra` set besides — `GitExecutor::with_env` replaces
/// the whole set, so an executor that adds one goes through here.
pub fn isolated_with(extra: &[(&str, &str)]) -> GitExecutor {
    let mut env = isolated_env();
    env.extend(
        extra
            .iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value))),
    );
    GitExecutor::new()
        .with_stock_timeout(super::wait::OVERALL_BUDGET)
        .with_env(env)
}

/// [`isolated`] with every commit and reflog line it writes dated `stamp`
/// (`GIT_AUTHOR_DATE` / `GIT_COMMITTER_DATE`): writes made in one second on
/// purpose, where a second boundary would decide the answer.
pub fn dated(stamp: &str) -> GitExecutor {
    isolated_with(&[("GIT_AUTHOR_DATE", stamp), ("GIT_COMMITTER_DATE", stamp)])
}

/// [`isolated`] with git's progress shown at once (`GIT_PROGRESS_DELAY=0`)
/// rather than after two seconds: a slow command's progress, on any
/// machine.
pub fn progress_at_once() -> GitExecutor {
    isolated_with(&[("GIT_PROGRESS_DELAY", "0")])
}

/// The same pair, reporting every invocation to `observer`.
///
/// `kept` is the caller's to say: a test of the command log wants
/// `Kept::Asked`, a test that a background read stays out of the log wants
/// another. `records()` is asked against it.
pub fn observed_env(
    observer: Arc<dyn CommandObserver>,
    kept: Kept,
) -> (GitExecutor, CancellationToken) {
    (
        isolated().observed(observer, kept),
        CancellationToken::new(),
    )
}

/// Every command the executor reported: the display line and how it
/// ended, in the order the commands ran. For tests that pin how specific
/// reads land in the command log.
#[derive(Default)]
pub struct Log(pub Mutex<Vec<(String, Option<CommandEnd>)>>);

impl Log {
    /// Every row's display, in the order they ran.
    pub fn displays(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .map(|(display, _)| display.clone())
            .collect()
    }

    /// The ends of the rows whose display carries every one of `needles`.
    pub fn ends_of(&self, needles: &[&str]) -> Vec<CommandEnd> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(display, _)| needles.iter().all(|n| display.contains(n)))
            .filter_map(|(_, end)| *end)
            .collect()
    }
}

impl CommandObserver for Log {
    fn records(&self, _kept: Kept) -> bool {
        true
    }
    fn started(
        &self,
        display: &str,
        _full: &str,
        _kept: Kept,
        _operation: Option<OperationId>,
    ) -> u64 {
        let mut rows = self.0.lock().unwrap();
        rows.push((display.to_string(), None));
        rows.len() as u64 - 1
    }
    fn finished(
        &self,
        id: u64,
        end: CommandEnd,
        _waited_ms: u64,
        _elapsed_ms: u64,
        _message: &str,
    ) {
        let mut rows = self.0.lock().unwrap();
        if let Some(row) = rows.get_mut(id as usize) {
            row.1 = Some(end);
        }
    }
}

/// [`observed_env`] watched by a fresh [`Log`], with `Kept::Asked`.
pub fn logged() -> (GitExecutor, Arc<Log>, CancellationToken) {
    let log = Arc::new(Log::default());
    let (exec, cancel) = observed_env(log.clone(), Kept::Asked);
    (exec, log, cancel)
}

/// [`isolated`] with `--global` pointed at a file of the caller's own
/// (`TestRepo::global_config`), for reads and writes that reach outside a
/// repository: [`isolated`] names no file there.
pub fn isolated_global(global_config: &Path) -> GitExecutor {
    GitExecutor::new()
        .with_stock_timeout(super::wait::OVERALL_BUDGET)
        .with_env(vec![
            (
                OsString::from("GIT_CONFIG_GLOBAL"),
                global_config.to_path_buf().into_os_string(),
            ),
            (OsString::from("GIT_CONFIG_NOSYSTEM"), OsString::from("1")),
            (
                OsString::from("XDG_CONFIG_HOME"),
                nowhere().into_os_string(),
            ),
            super::temp::ceiling(),
        ])
}

/// [`isolated_global`] watched by a fresh [`Log`], with `Kept::Asked`.
pub fn logged_global(global_config: &Path) -> (GitExecutor, Arc<Log>, CancellationToken) {
    let log = Arc::new(Log::default());
    let exec = isolated_global(global_config).observed(log.clone(), Kept::Asked);
    (exec, log, CancellationToken::new())
}

/// For the reads whose exit code depends on what the machine happens to
/// have configured: whichever way they answered, they answered.
#[track_caller]
pub fn assert_answered(reads: &[CommandEnd], what: &str) {
    assert!(!reads.is_empty(), "{what}: nothing was read at all");
    assert!(
        reads.iter().all(|e| matches!(e, CommandEnd::Answered(_))),
        "{what}: the log raises itself over an answer: {reads:?}"
    );
}

/// Collects every [`CommandEnd`] the executor reports, for tests that read
/// exit codes off the command-log path (`GitExecutor::observed`).
#[derive(Default)]
pub struct Ends(pub Mutex<Vec<CommandEnd>>);

impl CommandObserver for Ends {
    fn records(&self, _kept: Kept) -> bool {
        true
    }
    fn started(
        &self,
        _display: &str,
        _full: &str,
        _kept: Kept,
        _operation: Option<OperationId>,
    ) -> u64 {
        0
    }
    fn finished(
        &self,
        _id: u64,
        end: CommandEnd,
        _waited_ms: u64,
        _elapsed_ms: u64,
        _message: &str,
    ) {
        self.0.lock().unwrap().push(end);
    }
}

/// The same, keeping what git *said* with each end.
///
/// A stop that is an answer comes back as a landing, so the command log is
/// the only place its words reach (デザイン規約 §git が言ったことを読む場所).
#[derive(Default)]
pub struct Said(pub Mutex<Vec<(CommandEnd, String)>>);

impl Said {
    /// The message of the first row that ended this way, if any.
    pub fn message_of(&self, end: CommandEnd) -> Option<String> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .find(|(seen, _)| *seen == end)
            .map(|(_, said)| said.clone())
    }
}

impl CommandObserver for Said {
    fn records(&self, _kept: Kept) -> bool {
        true
    }
    fn started(
        &self,
        _display: &str,
        _full: &str,
        _kept: Kept,
        _operation: Option<OperationId>,
    ) -> u64 {
        0
    }
    fn finished(
        &self,
        _id: u64,
        end: CommandEnd,
        _waited_ms: u64,
        _elapsed_ms: u64,
        message: &str,
    ) {
        self.0.lock().unwrap().push((end, message.to_string()));
    }
}
