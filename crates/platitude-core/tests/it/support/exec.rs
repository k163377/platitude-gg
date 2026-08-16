//! What a test runs git with: an executor, a token, and the observer that
//! watches what went out.

use std::sync::{Arc, Mutex};

use platitude_core::process::{CommandEnd, CommandObserver, GitExecutor};
use tokio_util::sync::CancellationToken;

/// An executor and a token nothing ever cancels — what a test that only
/// wants to run git needs, and the cancellation path has tests of its own.
pub fn env() -> (GitExecutor, CancellationToken) {
    (GitExecutor::new(), CancellationToken::new())
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
    user: bool,
) -> (GitExecutor, CancellationToken) {
    (
        GitExecutor::new().observed(observer, user),
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

/// [`observed_env`] watched by a fresh [`Log`], on the user handle — what
/// a test of the command log wants.
pub fn logged() -> (GitExecutor, Arc<Log>, CancellationToken) {
    let log = Arc::new(Log::default());
    let (exec, cancel) = observed_env(log.clone(), true);
    (exec, log, cancel)
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
    fn records(&self, _user: bool) -> bool {
        true
    }
    fn started(&self, _display: &str, _full: &str, _user: bool) -> u64 {
        0
    }
    fn finished(&self, _id: u64, end: CommandEnd, _elapsed_ms: u64, _message: &str) {
        self.0.lock().unwrap().push(end);
    }
}
