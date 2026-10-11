//! What a git invocation is, before it runs: the command builder, its
//! time budget, what it returned, and the observer that watches it.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use super::slots::Pace;
use super::trace::Trace;
use crate::operation::OperationId;

/// Wraps a path so git reads it back as that exact path (a file really
/// named `:(glob)x` has to round-trip). Only for arguments git parses as
/// pathspecs: `git diff --no-index` takes filenames, which go in bare.
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
    /// The code is the answer the command named (`merge-base
    /// --is-ancestor` says "no" with 1), or such a command's 0 — so the log
    /// can tell an answer-shaped read from a plain success.
    Answered(i32),
    TimedOut,
    Cancelled,
    /// git never started, or the pipes died under it; the message says why.
    Failed,
}

/// What the command log keeps of one handle's invocations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Kept {
    /// Every one of them, as a row: the commands the reader asked for.
    Asked,
    /// The reads a session makes on its own, kept only while background
    /// reads are switched on: a poll tick's commands would bury what
    /// somebody did.
    #[default]
    Unasked,
    /// The same, except that a git which said no leaves its row anyway:
    /// the fetches nobody asked for (the panel a failure raises must hold
    /// the command that raised it).
    UnaskedUnlessItFails,
}

/// Watches every git invocation, for the command log.
///
/// Called from tokio worker threads, on every command's hot path:
/// implementations return at once.
pub trait CommandObserver: Send + Sync + 'static {
    /// Whether this handle's invocations are kept at all. Asked before the
    /// strings are built, so unrecorded timer reads cost nothing.
    fn records(&self, kept: Kept) -> bool;

    /// A command was asked for; its end is reported under the returned id.
    /// `display` is the log line, `full` the same with the fixed
    /// configuration and environment spelled out, to paste into a terminal.
    /// `operation` is the write it runs under
    /// ([`super::GitExecutor::under`]), `None` for a read.
    fn started(&self, display: &str, full: &str, kept: Kept, operation: Option<OperationId>)
    -> u64;

    /// `waited_ms` is the queue wait for a slot (`super::slots`),
    /// `elapsed_ms` spawn to reap — apart, so a slow answer reads as a
    /// slow git or a busy application. A command cancelled while waiting
    /// ends `Cancelled` with nothing run.
    fn finished(&self, id: u64, end: CommandEnd, waited_ms: u64, elapsed_ms: u64, message: &str);
}

/// A command's time budget. `Stock` is resolved by the executor, so a test
/// harness can lift it in one place. Not comparable on purpose:
/// `At(DEFAULT_TIMEOUT)` and `Stock` resolve alike, so `==` would answer
/// the wrong question.
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
    /// Applied after the fixed environment (`FIXED_ENV`), so it overrides.
    pub(super) env: Vec<(OsString, OsString)>,
    /// The non-zero exit code this command answers by, where it has one.
    pub(super) answer_code: Option<i32>,
    /// What git says when that code is no answer after all
    /// ([`Self::except_when_it_says`]).
    pub(super) not_an_answer: &'static [&'static str],
    /// The same as a reading of what git said and recorded, where no one
    /// phrase tells it ([`Self::except_when`]).
    pub(super) not_an_answer_when: Option<fn(&str, &Trace) -> bool>,
    /// Whether a failed row in the log carries stdout too ([`Self::logs_stdout`]).
    pub(super) logs_stdout: bool,
    /// Whether git's trace of the programs it ran is kept ([`Self::traced`]).
    pub(super) traced: bool,
    /// Which share of the slots it waits for ([`super::slots`]).
    pub(super) pace: Pace,
}

impl GitCommand {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            cwd: None,
            timeout: TimeBudget::Stock,
            env: Vec::new(),
            answer_code: None,
            not_an_answer: &[],
            not_an_answer_when: None,
            logs_stdout: false,
            traced: false,
            pace: Pace::Here,
        }
    }

    /// Marks a command paced by something other than this machine (a
    /// network peer, a credential prompt, a person in another program), so
    /// its slot is outside the click's reserve ([`super::slots::Pace`]).
    pub fn paced_elsewhere(mut self) -> Self {
        self.pace = Pace::Elsewhere;
        self
    }

    /// Marks the exit code this command answers with, so the command log
    /// keeps the row without raising itself (デザイン規約 §git が言ったことを読む場所).
    /// Named per command: a code means what its command says it means.
    pub fn answers_by_code(mut self, code: i32) -> Self {
        self.answer_code = Some(code);
        self
    }

    /// Marks the words with which the [`Self::answers_by_code`] code is a
    /// refusal after all, on stderr — where a command answers and refuses
    /// with one code (a rebase stops with 1, and git 2.55 refuses its
    /// `pre-rebase` hook's no with 1 too). The row is then a failure with
    /// git's words under it, which a report sends the reader to.
    pub fn except_when_it_says(mut self, words: &'static [&'static str]) -> Self {
        self.not_an_answer = words;
        self
    }

    /// [`Self::except_when_it_says`] for a refusal no single phrase names —
    /// `said` reads stderr whole and, for a [`Self::traced`] command, what
    /// git recorded (a rebase stopped by a commit a hook turned down says
    /// only what it says after any stop; its trace has the hook's end).
    pub fn except_when(mut self, said: fn(&str, &Trace) -> bool) -> Self {
        self.not_an_answer_when = Some(said);
        self
    }

    /// Puts stdout under a failed row in the log too, ahead of stderr —
    /// for a command whose answer is half on stdout (a push's ref lines,
    /// its `pre-push` hook's words, which come before git's own `failed to
    /// push` on stderr). The rest keep stdout out: a read's is the data, and
    /// kept it would hold the whole of it.
    pub fn logs_stdout(mut self) -> Self {
        self.logs_stdout = true;
        self
    }

    /// Keeps git's trace of the programs it ran — every hook with how it
    /// ended, every other program with its arguments ([`GitOutput::trace`],
    /// [`Trace`]) — for a write whose refusal git's words do not name.
    pub fn traced(mut self) -> Self {
        self.traced = true;
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

/// `Clone` only for identical background reads sharing one run
/// (`super::slots`); nothing else copies output.
#[derive(Debug, Default, Clone)]
pub struct GitOutput {
    /// Exit code; `-1` when the process was terminated by a signal.
    pub code: i32,
    /// Raw stdout. Empty for streaming runs (bytes went to the callback).
    pub stdout: Vec<u8>,
    /// Raw stderr, capped at an internal limit.
    pub stderr: Vec<u8>,
    /// What git recorded of the programs it ran, for a
    /// [`GitCommand::traced`] command; empty for the rest.
    pub trace: Trace,
}

impl GitOutput {
    pub fn stdout_utf8(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    pub fn stderr_utf8(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stderr)
    }

    /// git's own explanation of a failure, unedited: stderr, else stdout
    /// (rules/core.md §git サブプロセス規約).
    pub fn failure_message(&self) -> String {
        let stderr = self.stderr_utf8().trim().to_string();
        if stderr.is_empty() {
            self.stdout_utf8().trim().to_string()
        } else {
            stderr
        }
    }
}
