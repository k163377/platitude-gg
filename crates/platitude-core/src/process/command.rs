//! What a git invocation is, before it runs: the command builder, its
//! time budget, what it returned, and the observer that watches it.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use crate::operation::OperationId;

/// Wraps a path from git's own output so git reads it back as that exact
/// path, never as pathspec magic (a file really named `:(glob)x` has to
/// round-trip).
///
/// Only for arguments git parses as pathspecs. `git diff --no-index` takes
/// filenames, not pathspecs, and must not be given a prefix.
pub fn literal_pathspec(path: &str) -> String {
    format!(":(literal){path}")
}

/// Quotes an argument the way a POSIX shell would need it, so a command
/// read out of a log can be pasted back unchanged.
pub(super) fn shell_quote(arg: &str) -> std::borrow::Cow<'_, str> {
    let plain = !arg.is_empty()
        && arg.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'/' | b'.' | b'=' | b':' | b'+')
        });
    if plain {
        std::borrow::Cow::Borrowed(arg)
    } else {
        std::borrow::Cow::Owned(format!("'{}'", arg.replace('\'', r"'\''")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEnd {
    /// The process ran and returned this code (`-1` = killed by a signal).
    Exited(i32),
    /// The process ran and its code is one of the answers the command
    /// named, not a failure (`merge-base --is-ancestor` says "no" with 1;
    /// a marked command's 0 lands here too, so the log can tell an
    /// answer-shaped read from a plain success). The log keeps the row
    /// and its code; it just does not raise itself.
    Answered(i32),
    TimedOut,
    Cancelled,
    /// git never started, or the pipes died under it. The reported message
    /// is the reason rather than git's own output.
    Failed,
}

/// What the command log keeps of one handle's invocations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Kept {
    /// Every one of them, as a row, from the spawn: the commands the
    /// reader asked for.
    Asked,
    /// The reads a session makes on its own — kept only while background
    /// reads are switched on, since a poll tick runs five commands and
    /// would bury what somebody actually did.
    #[default]
    Unasked,
    /// The same, except that a git which said no leaves its row anyway:
    /// the fetches nobody asked for. The panel the first of them raises
    /// has to hold the command that raised it, and one that lands every
    /// interval would bury the log all the same.
    UnaskedUnlessItFails,
}

/// Watches every git invocation, for the command log.
///
/// Called from tokio worker threads, on the hot path of every command:
/// implementations must not block.
pub trait CommandObserver: Send + Sync + 'static {
    /// Whether this handle's invocations are being kept at all. Asked
    /// before the strings are built, so the reads a repository page makes
    /// on a timer cost nothing while nobody is recording them.
    fn records(&self, kept: Kept) -> bool;

    /// A command is about to be spawned; the returned id is what its end
    /// is reported under. `display` is the log line, `full` the same
    /// command with the fixed configuration and environment spelled out,
    /// so it can be pasted into a terminal and do the same thing.
    /// `operation` is the write the command runs under, where the handle
    /// was given one ([`super::GitExecutor::under`]) — a compound write
    /// is several commands under one id — and `None` for a read.
    fn started(&self, display: &str, full: &str, kept: Kept, operation: Option<OperationId>)
    -> u64;

    fn finished(&self, id: u64, end: CommandEnd, elapsed_ms: u64, message: &str);
}

/// A command's time budget: the stock default — resolved by the executor,
/// so a test harness can lift it in one place — an explicit bound, or
/// none at all. Deliberately not comparable: `At(DEFAULT_TIMEOUT)` and
/// `Stock` resolve to the same bound, so an `==` would answer the wrong
/// question.
#[derive(Debug, Clone, Copy)]
pub(super) enum TimeBudget {
    Stock,
    At(Duration),
    Never,
}

#[derive(Debug, Clone)]
pub struct GitCommand {
    pub(super) args: Vec<OsString>,
    pub(super) cwd: Option<PathBuf>,
    pub(super) timeout: TimeBudget,
    /// Applied after the fixed environment (`FIXED_ENV`), so a command can
    /// override a default or add its own (interactive rebase adds
    /// `GIT_SEQUENCE_EDITOR`).
    pub(super) env: Vec<(OsString, OsString)>,
    /// The non-zero exit code this command answers by, where it has one.
    pub(super) answer_code: Option<i32>,
}

impl GitCommand {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            cwd: None,
            timeout: TimeBudget::Stock,
            env: Vec::new(),
            answer_code: None,
        }
    }

    /// Marks the exit code this command answers with rather than fails on,
    /// so the command log keeps the row without raising itself over it
    /// (デザイン規約 §git が言ったことを読む場所). Named per command: a code
    /// means what the command that returned it says it means.
    pub fn answers_by_code(mut self, code: i32) -> Self {
        self.answer_code = Some(code);
        self
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, A>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = TimeBudget::At(timeout);
        self
    }

    /// Removes the time budget; the command is bounded only by cancellation.
    pub fn no_timeout(mut self) -> Self {
        self.timeout = TimeBudget::Never;
        self
    }

    /// Human-readable form for logs and error messages.
    pub(crate) fn describe(&self) -> String {
        let mut s = String::from("git");
        self.append_args(&mut s);
        s
    }

    pub(super) fn append_args(&self, out: &mut String) {
        for a in &self.args {
            out.push(' ');
            out.push_str(&shell_quote(&a.to_string_lossy()));
        }
    }
}

impl Default for GitCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct GitOutput {
    /// Exit code; `-1` when the process was terminated by a signal.
    pub code: i32,
    /// Raw stdout. Empty for streaming runs (bytes went to the callback).
    pub stdout: Vec<u8>,
    /// Raw stderr, capped at an internal limit.
    pub stderr: Vec<u8>,
}

impl GitOutput {
    pub fn stdout_utf8(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    pub fn stderr_utf8(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stderr)
    }

    /// git's own explanation of a failure, passed through unedited.
    ///
    /// Usually stderr, but some porcelain reports on stdout and exits
    /// non-zero with stderr empty (`git commit` printing "nothing to
    /// commit" is the one users hit daily), so stdout is the fallback.
    pub fn failure_message(&self) -> String {
        let stderr = self.stderr_utf8().trim().to_string();
        if stderr.is_empty() {
            self.stdout_utf8().trim().to_string()
        } else {
            stderr
        }
    }
}
