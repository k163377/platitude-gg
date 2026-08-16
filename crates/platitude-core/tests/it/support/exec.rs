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
