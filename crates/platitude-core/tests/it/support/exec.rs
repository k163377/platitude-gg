//! What a test runs git with: an executor, a token, and the observer that
//! watches what went out.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use platitude_core::process::{CommandEnd, CommandObserver, GitExecutor, Kept};
use tokio_util::sync::CancellationToken;

struct IsolatedGit {
    _dir: tempfile::TempDir,
    global_config: PathBuf,
    xdg_config: PathBuf,
}

static ISOLATED_GIT: OnceLock<IsolatedGit> = OnceLock::new();

fn isolated_git() -> &'static IsolatedGit {
    ISOLATED_GIT.get_or_init(|| {
        let dir = tempfile::tempdir().expect("create isolated git config dir");
        let global_config = dir.path().join("global-config");
        let xdg_config = dir.path().join("xdg-config");
        std::fs::write(
            &global_config,
            b"[core]\n\tfsync = none\n[gc]\n\tauto = 0\n",
        )
        .expect("write isolated global config");
        std::fs::create_dir(&xdg_config).expect("create isolated xdg config dir");
        IsolatedGit {
            _dir: dir,
            global_config,
            xdg_config,
        }
    })
}

fn isolated_env() -> Vec<(OsString, OsString)> {
    let config = isolated_git();
    vec![
        (
            OsString::from("GIT_CONFIG_GLOBAL"),
            config.global_config.clone().into_os_string(),
        ),
        (OsString::from("GIT_CONFIG_NOSYSTEM"), OsString::from("1")),
        (
            OsString::from("XDG_CONFIG_HOME"),
            config.xdg_config.clone().into_os_string(),
        ),
    ]
}

/// Git executor for integration tests. The raw `GitExecutor::new()` remains
/// available for tests that intentionally exercise the host configuration.
///
/// The stock wall-clock budget is raised to the suite's overall backstop,
/// not lifted: under a loaded suite one git round trip inflates by more
/// than an order of magnitude (`wait.rs`), so the everyday cap would
/// decide by load — but
/// a *wedged* git must still fail the awaiting test by name. Session
/// waits have `Patience` under this; a test that awaits the executor
/// directly has nothing else.
pub fn isolated() -> GitExecutor {
    GitExecutor::new()
        .with_stock_timeout(super::wait::OVERALL_BUDGET)
        .with_env(isolated_env())
}

/// An executor and a token nothing ever cancels — what a test that only
/// wants to run git needs, and the cancellation path has tests of its own.
pub fn env() -> (GitExecutor, CancellationToken) {
    (isolated(), CancellationToken::new())
}

/// The same pair, reporting every invocation to `observer`.
///
/// `user` marks the commands the user asked for, and it is the caller's to
/// say because the two sides of it are different questions: a test of the
/// command log wants the user's half, a test that a background read stays
/// out of the log wants the other. It also decides whether the observer is
/// asked at all — `records()` runs against this flag.
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
    fn started(&self, display: &str, _full: &str, _kept: Kept) -> u64 {
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

/// [`observed_env`] watched by a fresh [`Log`], on the user handle — what
/// a test of the command log wants.
pub fn logged() -> (GitExecutor, Arc<Log>, CancellationToken) {
    let log = Arc::new(Log::default());
    let (exec, cancel) = observed_env(log.clone(), Kept::Asked);
    (exec, log, cancel)
}

/// [`isolated`] with `--global` pointed at a file of the caller's own —
/// for the reads and writes that reach outside a repository
/// (`TestRepo::global_config` is one such file per repository). The
/// isolation [`isolated`] gives is a single file for the whole suite, and
/// the suite runs in parallel.
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
                isolated_git().xdg_config.clone().into_os_string(),
            ),
        ])
}

/// [`isolated_global`] watched by a fresh [`Log`], on the user handle.
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
    fn started(&self, _display: &str, _full: &str, _kept: Kept) -> u64 {
        0
    }
    fn finished(&self, _id: u64, end: CommandEnd, _elapsed_ms: u64, _message: &str) {
        self.0.lock().unwrap().push(end);
    }
}

/// The same, keeping what git *said* with each end.
///
/// The only way left to read the words of a stop that is an answer: it
/// comes back as a landing rather than an error, so nothing carries git's
/// message to the caller and the command log is where a person reads it
/// (デザイン規約 §git が言ったことを読む場所).
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
    fn started(&self, _display: &str, _full: &str, _kept: Kept) -> u64 {
        0
    }
    fn finished(&self, _id: u64, end: CommandEnd, _elapsed_ms: u64, message: &str) {
        self.0.lock().unwrap().push((end, message.to_string()));
    }
}
