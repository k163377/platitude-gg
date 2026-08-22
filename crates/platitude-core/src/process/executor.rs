//! Running what [`GitCommand`] describes: the executor, its fixed
//! arguments and environment, and the child supervision under it.

use std::ffi::OsString;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use super::child::{ChildOutcome, run_child};
use super::command::{CommandEnd, CommandObserver, GitCommand, GitOutput, TimeBudget, shell_quote};
use crate::error::GitError;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Default time budget for short-lived commands. The streaming log walks
/// and `mergetool` (open-ended, user-paced) opt out via
/// [`GitCommand::no_timeout`]; network commands set their own, longer
/// budget instead (`remote`).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Keep at most this much stderr; git error messages are short, and a
/// runaway process must not grow memory unboundedly.
pub(super) const STDERR_CAP: usize = 256 * 1024;

pub(super) const STDOUT_CHUNK: usize = 64 * 1024;

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
/// are quoted individually with [`super::literal_pathspec`] instead.
const FIXED_ENV: [(&str, &str); 4] = [
    ("LC_ALL", "C"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_OPTIONAL_LOCKS", "0"),
    ("GIT_EDITOR", "true"),
];

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
    /// What [`TimeBudget::Stock`] resolves to. `None` lifts the stock
    /// budget entirely — the test harness's setting, where wall time is
    /// load-dependent and must not decide correctness
    /// ([`GitExecutor::without_stock_timeouts`]); commands that named
    /// their own budget keep it either way.
    stock_timeout: Option<Duration>,
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
            stock_timeout: Some(DEFAULT_TIMEOUT),
        }
    }

    /// Lifts the stock time budget from every command that did not set
    /// one of its own: those commands are then bounded by cancellation
    /// alone. For test harnesses — under a loaded suite a git round trip
    /// inflates ~25×, and a wall-clock cap that generous decides by load,
    /// not correctness; the harness arms its own failure-detection
    /// backstops instead (.claude/rules/core.md). The shipped application
    /// keeps the stock budget.
    pub fn without_stock_timeouts(mut self) -> Self {
        self.stock_timeout = None;
        self
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
            stock_timeout: self.stock_timeout,
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

    /// The process command for `cmd`: program, arguments, the layered
    /// environment (fixed, executor, per-command — later layers win), and
    /// the platform wiring.
    fn assemble(&self, cmd: &GitCommand) -> Command {
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
        command
    }

    async fn execute(
        &self,
        cmd: &GitCommand,
        cancel: &CancellationToken,
        on_stdout: &mut (dyn FnMut(&[u8]) + Send),
    ) -> Result<GitOutput, GitError> {
        let described = cmd.describe();
        let mut command = self.assemble(cmd);

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

        let budget = match cmd.timeout {
            TimeBudget::Stock => self.stock_timeout,
            TimeBudget::At(timeout) => Some(timeout),
            TimeBudget::Never => None,
        };
        let outcome = run_child(&mut child, budget, cancel, on_stdout)
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
                    timeout: budget.unwrap_or(Duration::ZERO),
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
