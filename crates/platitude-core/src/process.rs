//! git subprocess execution — the single place that spawns `git`.
//!
//! Policy (.claude/rules/core.md / 実装計画 §2):
//! - argument vectors only, never shell strings
//! - fixed config arguments and environment keep output machine-readable,
//!   prompt-free and lock-friendly
//! - every run is cancellable and (optionally) time-limited; the process is
//!   killed when either fires
//! - on Windows no console window is shown

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;

use crate::error::GitError;

#[cfg(test)]
mod tests;

/// Default time budget for short-lived commands. The streaming log walks
/// and `mergetool` (open-ended, user-paced) opt out via
/// [`GitCommand::no_timeout`]; network commands set their own, longer
/// budget instead (`remote`).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Keep at most this much stderr; git error messages are short, and a
/// runaway process must not grow memory unboundedly.
const STDERR_CAP: usize = 256 * 1024;

const STDOUT_CHUNK: usize = 64 * 1024;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Configuration arguments prepended to every invocation.
const FIXED_ARGS: [&str; 7] = [
    "-c",
    "color.ui=false",
    "-c",
    "core.quotepath=false",
    // A repo with log.showSignature=true would interleave gpg output with
    // machine-readable --format records; force it off.
    "-c",
    "log.showSignature=false",
    "--no-optional-locks",
];

/// Environment applied to every invocation.
///
/// - `LC_ALL=C`: stable, locale-independent messages and sorting
/// - `GIT_TERMINAL_PROMPT=0`: never hang on a credential prompt (auth is
///   delegated to credential helpers)
/// - `GIT_OPTIONAL_LOCKS=0`: belt-and-suspenders with `--no-optional-locks`
/// - `GIT_EDITOR=true`: an accidentally editor-spawning command exits
///   immediately instead of hanging. Interactive rebase leaves it in
///   place and adds `GIT_SEQUENCE_EDITOR` on top — rewords rely on the
///   `true` (sequencer.rs)
///
/// Deliberately absent: `GIT_LITERAL_PATHSPECS`. It disarms pathspec magic
/// for git's *internal* use too — with it set, `git stash push -u` reports
/// success and silently leaves untracked files in the working tree. Paths
/// are quoted individually with [`literal_pathspec`] instead.
const FIXED_ENV: [(&str, &str); 4] = [
    ("LC_ALL", "C"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_OPTIONAL_LOCKS", "0"),
    ("GIT_EDITOR", "true"),
];

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
fn shell_quote(arg: &str) -> std::borrow::Cow<'_, str> {
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
    /// The process ran and its non-zero code is the answer that was asked
    /// for, not a failure (`merge-base --is-ancestor` says "no" with 1).
    /// The log keeps the row and its code; it just does not raise itself.
    Answered(i32),
    TimedOut,
    Cancelled,
    /// git never started, or the pipes died under it. The reported message
    /// is the reason rather than git's own output.
    Failed,
}

/// Watches every git invocation, for the command log.
///
/// Called from tokio worker threads, on the hot path of every command:
/// implementations must not block.
pub trait CommandObserver: Send + Sync + 'static {
    /// Whether this handle's invocations are being kept at all. Asked
    /// before the strings are built, so the reads a repository page makes
    /// on a timer cost nothing while nobody is recording them.
    fn records(&self, user: bool) -> bool;

    /// A command is about to be spawned; the returned id is what its end
    /// is reported under. `display` is the log line, `full` the same
    /// command with the fixed configuration and environment spelled out,
    /// so it can be pasted into a terminal and do the same thing.
    fn started(&self, display: &str, full: &str, user: bool) -> u64;

    fn finished(&self, id: u64, end: CommandEnd, elapsed_ms: u64, message: &str);
}

#[derive(Debug, Clone)]
pub struct GitCommand {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    timeout: Option<Duration>,
    /// Applied after [`FIXED_ENV`], so a command can override a default
    /// or add its own (interactive rebase adds `GIT_SEQUENCE_EDITOR`).
    env: Vec<(OsString, OsString)>,
    /// The non-zero exit code this command answers by, where it has one.
    answer_code: Option<i32>,
}

impl GitCommand {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            cwd: None,
            timeout: Some(DEFAULT_TIMEOUT),
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
        self.timeout = Some(timeout);
        self
    }

    /// Removes the time budget; the command is bounded only by cancellation.
    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Human-readable form for logs and error messages.
    pub(crate) fn describe(&self) -> String {
        let mut s = String::from("git");
        self.append_args(&mut s);
        s
    }

    fn append_args(&self, out: &mut String) {
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

/// Spawns git subprocesses. Cheap to clone; shared across sessions.
#[derive(Clone)]
pub struct GitExecutor {
    program: Arc<OsString>,
    /// Environment applied to every invocation from this executor. Kept on
    /// the executor so a test harness can isolate Git without mutating the
    /// process-global environment; command-level values override these.
    env: Arc<Vec<(OsString, OsString)>>,
    observer: Option<Arc<dyn CommandObserver>>,
    /// Whether invocations made through this handle are ones the user
    /// asked for, as opposed to background reads. Carried here rather
    /// than on the command so the callers stay unaware of it: the
    /// session hands out a different handle for each.
    user: bool,
}

impl std::fmt::Debug for GitExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitExecutor")
            .field("program", &self.program)
            .field("env_overrides", &self.env.len())
            .field("observed", &self.observer.is_some())
            .field("user", &self.user)
            .finish()
    }
}

impl Default for GitExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl GitExecutor {
    /// Uses `git` resolved from `PATH`.
    pub fn new() -> Self {
        Self::of(OsString::from("git"))
    }

    /// Uses an explicit git binary (tests, portable installs).
    pub fn with_program(program: impl Into<OsString>) -> Self {
        Self::of(program.into())
    }

    fn of(program: OsString) -> Self {
        Self {
            program: Arc::new(program),
            env: Arc::new(Vec::new()),
            observer: None,
            user: false,
        }
    }

    /// Returns an executor with environment defaults applied to every Git
    /// subprocess. Per-command [`GitCommand::env`] values take precedence.
    pub fn with_env<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.env = Arc::new(
            vars.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        );
        self
    }

    /// Returns a handle that reports its invocations to `observer`.
    /// `user` marks the commands the user asked for.
    pub fn observed(&self, observer: Arc<dyn CommandObserver>, user: bool) -> Self {
        Self {
            program: Arc::clone(&self.program),
            env: Arc::clone(&self.env),
            observer: Some(observer),
            user,
        }
    }

    /// The command as it would have to be typed to do the same thing:
    /// the fixed environment and configuration are part of what ran.
    fn describe_full(&self, cmd: &GitCommand) -> String {
        let mut s = String::new();
        for (k, v) in FIXED_ENV {
            s.push_str(k);
            s.push('=');
            s.push_str(&shell_quote(v));
            s.push(' ');
        }
        for (k, v) in self.env.iter() {
            s.push_str(&k.to_string_lossy());
            s.push('=');
            s.push_str(&shell_quote(&v.to_string_lossy()));
            s.push(' ');
        }
        for (k, v) in &cmd.env {
            s.push_str(&k.to_string_lossy());
            s.push('=');
            s.push_str(&shell_quote(&v.to_string_lossy()));
            s.push(' ');
        }
        s.push_str(&shell_quote(&self.program.to_string_lossy()));
        for a in FIXED_ARGS {
            s.push(' ');
            s.push_str(&shell_quote(a));
        }
        cmd.append_args(&mut s);
        s
    }

    /// Runs to completion and fails on non-zero exit.
    pub async fn run(
        &self,
        cmd: GitCommand,
        cancel: &CancellationToken,
    ) -> Result<GitOutput, GitError> {
        let described = cmd.describe();
        let out = self.run_unchecked(cmd, cancel).await?;
        if out.code != 0 {
            return Err(GitError::Failed {
                command: described,
                code: out.code,
                stderr: out.failure_message(),
            });
        }
        Ok(out)
    }

    /// Runs to completion; the caller inspects the exit code itself.
    pub async fn run_unchecked(
        &self,
        cmd: GitCommand,
        cancel: &CancellationToken,
    ) -> Result<GitOutput, GitError> {
        let mut stdout = Vec::new();
        let mut out = self
            .execute(&cmd, cancel, &mut |chunk| stdout.extend_from_slice(chunk))
            .await?;
        out.stdout = stdout;
        Ok(out)
    }

    /// Streams stdout to `on_stdout` chunk by chunk (arbitrary boundaries).
    /// Fails on non-zero exit. The returned output has an empty `stdout`.
    pub async fn run_streaming(
        &self,
        cmd: GitCommand,
        cancel: &CancellationToken,
        on_stdout: &mut (dyn FnMut(&[u8]) + Send),
    ) -> Result<GitOutput, GitError> {
        let described = cmd.describe();
        let out = self.execute(&cmd, cancel, on_stdout).await?;
        if out.code != 0 {
            return Err(GitError::Failed {
                command: described,
                code: out.code,
                stderr: out.stderr_utf8().trim().to_string(),
            });
        }
        Ok(out)
    }

    async fn execute(
        &self,
        cmd: &GitCommand,
        cancel: &CancellationToken,
        on_stdout: &mut (dyn FnMut(&[u8]) + Send),
    ) -> Result<GitOutput, GitError> {
        let described = cmd.describe();

        let mut command = Command::new(self.program.as_ref());
        command.args(FIXED_ARGS);
        command.args(&cmd.args);
        if let Some(dir) = &cmd.cwd {
            command.current_dir(dir);
        }
        for (k, v) in FIXED_ENV {
            command.env(k, v);
        }
        for (k, v) in self.env.iter() {
            command.env(k, v);
        }
        for (k, v) in &cmd.env {
            command.env(k, v);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);

        tracing::debug!(command = %described, "spawning git");
        let started = Instant::now();
        let watch = match self.observer.as_ref() {
            Some(o) if o.records(self.user) => Some((
                o,
                o.started(&described, &self.describe_full(cmd), self.user),
            )),
            _ => None,
        };
        let report = |end: CommandEnd, message: &str| {
            if let Some((observer, id)) = &watch {
                observer.finished(*id, end, started.elapsed().as_millis() as u64, message);
            }
        };

        let mut child = command.spawn().map_err(|source| {
            report(CommandEnd::Failed, &source.to_string());
            if source.kind() == std::io::ErrorKind::NotFound
                && cmd.cwd.as_ref().is_none_or(|d| d.is_dir())
            {
                GitError::GitNotFound { source }
            } else {
                GitError::Spawn {
                    command: described.clone(),
                    source,
                }
            }
        })?;

        let outcome = run_child(&mut child, cmd.timeout, cancel, on_stdout)
            .await
            .map_err(|source| {
                report(CommandEnd::Failed, &source.to_string());
                GitError::Io {
                    command: described.clone(),
                    source,
                }
            })?;

        match outcome {
            ChildOutcome::Finished { code, stderr } => {
                tracing::debug!(
                    command = %described,
                    code,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "git finished"
                );
                report(
                    // Only the code this command named is an answer: a 128
                    // from one that named 1 is still a failure.
                    if cmd.answer_code.is_some_and(|a| code == 0 || code == a) {
                        CommandEnd::Answered(code)
                    } else {
                        CommandEnd::Exited(code)
                    },
                    String::from_utf8_lossy(&stderr).trim_end(),
                );
                Ok(GitOutput {
                    code,
                    stdout: Vec::new(),
                    stderr,
                })
            }
            ChildOutcome::TimedOut => {
                tracing::warn!(command = %described, "git timed out; killed");
                report(CommandEnd::TimedOut, "");
                Err(GitError::TimedOut {
                    command: described,
                    // `unwrap_or` only for the error message: TimedOut cannot
                    // happen without a configured timeout.
                    timeout: cmd.timeout.unwrap_or(Duration::ZERO),
                })
            }
            ChildOutcome::Cancelled => {
                tracing::debug!(command = %described, "git cancelled; killed");
                report(CommandEnd::Cancelled, "");
                Err(GitError::Cancelled { command: described })
            }
        }
    }
}

enum ChildOutcome {
    Finished { code: i32, stderr: Vec<u8> },
    TimedOut,
    Cancelled,
}

/// Drives a spawned child: pumps stdout into `on_stdout`, accumulates capped
/// stderr, and races completion against the timeout and the cancel token.
/// The child is killed and reaped when either fires.
async fn run_child(
    child: &mut Child,
    timeout: Option<Duration>,
    cancel: &CancellationToken,
    on_stdout: &mut (dyn FnMut(&[u8]) + Send),
) -> std::io::Result<ChildOutcome> {
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let deadline = async {
        match timeout {
            Some(d) => tokio::time::sleep(d).await,
            None => std::future::pending().await,
        }
    };

    let work = async {
        let stdout_fut = async {
            if let Some(r) = stdout_pipe.as_mut() {
                let mut buf = vec![0u8; STDOUT_CHUNK];
                loop {
                    let n = r.read(&mut buf).await?;
                    if n == 0 {
                        break;
                    }
                    on_stdout(&buf[..n]);
                }
            }
            Ok::<(), std::io::Error>(())
        };
        let stderr_fut = async {
            let mut acc = Vec::new();
            if let Some(r) = stderr_pipe.as_mut() {
                let mut buf = vec![0u8; 8 * 1024];
                loop {
                    let n = r.read(&mut buf).await?;
                    if n == 0 {
                        break;
                    }
                    let room = STDERR_CAP.saturating_sub(acc.len());
                    acc.extend_from_slice(&buf[..n.min(room)]);
                }
            }
            Ok::<Vec<u8>, std::io::Error>(acc)
        };
        let (out_res, err_res) = tokio::join!(stdout_fut, stderr_fut);
        out_res?;
        let stderr = err_res?;
        let status = child.wait().await?;
        Ok::<ChildOutcome, std::io::Error>(ChildOutcome::Finished {
            code: status.code().unwrap_or(-1),
            stderr,
        })
    };

    // `biased`: prefer cancellation over a simultaneously-completed process.
    // When one branch wins, the losing futures are dropped before the arm
    // body runs, releasing their borrow of `child` so it can be killed.
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            kill_and_reap(child).await;
            Ok(ChildOutcome::Cancelled)
        }
        _ = deadline => {
            kill_and_reap(child).await;
            Ok(ChildOutcome::TimedOut)
        }
        res = work => res,
    }
}

async fn kill_and_reap(child: &mut Child) {
    if let Err(e) = child.start_kill() {
        tracing::debug!(error = %e, "kill failed (process already exited?)");
    }
    if let Err(e) = child.wait().await {
        tracing::debug!(error = %e, "failed to reap killed process");
    }
}
