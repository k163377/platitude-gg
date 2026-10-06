//! Running what [`GitCommand`] describes: the executor, its fixed
//! arguments and environment, and the slot every spawn waits for (the
//! child supervision is [`super::child`], the slots [`super::slots`]).

use std::ffi::OsString;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use super::child::{ChildOutcome, run_child};
use super::command::{
    CommandEnd, CommandObserver, GitCommand, GitOutput, Kept, TimeBudget, shell_quote,
};
use super::slots::{GroupKey, Lead, Priority, Shared, Slots, WayIn};
use crate::error::GitError;
use crate::operation::OperationId;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Default time budget, which is to say for reads: streaming log walks and
/// `mergetool` opt out ([`GitCommand::no_timeout`]), network commands set
/// their own, and the write queue's local lane lifts it
/// (`operation::Lane::Local`) — killing git mid-write loses what it was
/// writing.
///
/// Counted from the spawn: a budget that counted the slot wait would kill
/// a healthy git for the queue in front of it.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Configuration arguments prepended to every invocation.
const FIXED_ARGS: [&str; 9] = [
    "-c",
    "color.ui=false",
    "-c",
    "core.quotepath=false",
    // log.showSignature=true would interleave gpg output with --format
    // records.
    "-c",
    "log.showSignature=false",
    // A working-tree `git diff` refreshes the index under `.git/index.lock`
    // unless this says not to (`--no-optional-locks` does not reach
    // diff), and a poll's diff on that lock kills a concurrent commit.
    // No answer changes (rules-refs/core.md の `diff.autoRefreshIndex` の行).
    "-c",
    "diff.autoRefreshIndex=false",
    "--no-optional-locks",
];

/// Environment applied to every invocation.
///
/// - `LC_ALL=C`: stable, locale-independent messages and sorting
/// - `GIT_TERMINAL_PROMPT=0`: a credential prompt fails at once
/// - `GIT_OPTIONAL_LOCKS=0`: belt-and-suspenders with `--no-optional-locks`
///   (neither reaches `diff`; see `diff.autoRefreshIndex`)
/// - `GIT_EDITOR=true`: an editor-spawning command exits at once
///
/// Deliberately absent: `GIT_LITERAL_PATHSPECS` — it reaches git's
/// internal pathspecs too, and `git stash push -u` then reports success
/// while leaving untracked files behind. Paths are wrapped with
/// [`super::literal_pathspec`] instead.
const FIXED_ENV: [(&str, &str); 4] = [
    ("LC_ALL", "C"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_OPTIONAL_LOCKS", "0"),
    ("GIT_EDITOR", "true"),
];

/// Spawns git subprocesses. Cheap to clone; every clone waits in the same
/// [`Slots`] ([`GitExecutor::scheduled`]). A bare executor caps nothing,
/// which is what a test wants of one.
#[derive(Clone)]
pub struct GitExecutor {
    program: Arc<OsString>,
    /// Applied to every invocation, under the command's own: a test harness
    /// isolates git here without touching the process environment.
    env: Arc<Vec<(OsString, OsString)>>,
    observer: Option<Arc<dyn CommandObserver>>,
    /// What the command log makes of this handle's invocations. Carried on
    /// the handle so callers stay unaware: the session hands out one per
    /// value.
    kept: Kept,
    /// The write this handle's commands belong to, for the log
    /// ([`GitExecutor::under`]); `None` on every read.
    operation: Option<OperationId>,
    /// What [`TimeBudget::Stock`] resolves to; `None` lifts it
    /// ([`GitExecutor::without_stock_timeouts`]). Commands that named their
    /// own budget keep it either way.
    stock_timeout: Option<Duration>,
    /// Where every spawn waits its turn ([`super::slots`]).
    slots: Arc<Slots>,
    /// Who is waiting on this handle's commands ([`Priority`]), carried
    /// like `kept` ([`GitExecutor::background`]).
    priority: Priority,
}

impl std::fmt::Debug for GitExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitExecutor")
            .field("program", &self.program)
            .field("env_overrides", &self.env.len())
            .field("observed", &self.observer.is_some())
            .field("kept", &self.kept)
            .field("operation", &self.operation)
            .field("priority", &self.priority)
            .finish()
    }
}

impl Default for GitExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl GitExecutor {
    /// Uses `git` resolved from `PATH` — or, where PATH names the launcher
    /// Git for Windows installs there, the git behind it
    /// ([`super::program`]).
    pub fn new() -> Self {
        Self::of(super::program::default_program())
    }

    /// Uses an explicit git binary. As for PATH, the Git for Windows
    /// launcher is swapped for the git behind it
    /// ([`super::program::spawnable`]); every other path is taken as given.
    pub fn with_program(program: impl Into<OsString>) -> Self {
        let named = program.into();
        Self::of(super::program::spawnable(std::path::Path::new(&named)).into_os_string())
    }

    fn of(program: OsString) -> Self {
        Self {
            program: Arc::new(program),
            env: Arc::new(Vec::new()),
            observer: None,
            kept: Kept::Unasked,
            operation: None,
            stock_timeout: Some(DEFAULT_TIMEOUT),
            slots: Arc::new(Slots::unbounded()),
            priority: Priority::Interactive,
        }
    }

    /// Lifts the stock time budget from every command that did not set one
    /// of its own; they are then bounded by cancellation alone. For the
    /// write queue's local lane (`operation::Lane::Local`) and for test
    /// harnesses, where a wall-clock cap decides by load
    /// (ci/baseline/code-costs-windows-x64.md §テストとハーネス). The
    /// application's reads keep the stock budget.
    pub fn without_stock_timeouts(mut self) -> Self {
        self.stock_timeout = None;
        self
    }

    /// Replaces the stock time budget for every command that did not set
    /// one of its own. The test harness raises it to its overall failure
    /// backstop: a wedged git then fails the awaiting test by name, ahead
    /// of the CI kill.
    pub fn with_stock_timeout(mut self, budget: Duration) -> Self {
        self.stock_timeout = Some(budget);
        self
    }

    /// The budget a command that named none runs on — `None` where the
    /// stock budget was lifted ([`GitExecutor::without_stock_timeouts`]).
    #[must_use]
    pub fn stock_timeout(&self) -> Option<Duration> {
        self.stock_timeout
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

    /// Runs everything through `slots` — install before the handle is
    /// cloned into anything, or a clone waits elsewhere.
    #[must_use]
    pub fn scheduled(mut self, slots: Arc<Slots>) -> Self {
        self.slots = slots;
        self
    }

    /// A handle for the reads nobody is waiting on (a session's timer):
    /// served after interactive ones, outside the click's reserve
    /// ([`Priority::Background`]). Identical buffered runs through it share
    /// one process while the first is still queued ([`super::slots`]).
    #[must_use]
    pub fn background(mut self) -> Self {
        self.priority = Priority::Background;
        self
    }

    #[must_use]
    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// For the application to move the limits and read the report.
    #[must_use]
    pub fn slots(&self) -> &Arc<Slots> {
        &self.slots
    }

    /// Returns a handle that reports its invocations to `observer`.
    /// `kept` is what the log makes of them.
    pub fn observed(&self, observer: Arc<dyn CommandObserver>, kept: Kept) -> Self {
        Self {
            program: Arc::clone(&self.program),
            env: Arc::clone(&self.env),
            observer: Some(observer),
            kept,
            operation: self.operation,
            stock_timeout: self.stock_timeout,
            slots: Arc::clone(&self.slots),
            priority: self.priority,
        }
    }

    /// Returns a handle whose every invocation is reported as part of
    /// `operation`: the write queue hands it to the task, so a compound
    /// write's commands stand in the log under one id.
    #[must_use]
    pub fn under(mut self, operation: OperationId) -> Self {
        self.operation = Some(operation);
        self
    }

    /// The command as it would have to be typed to do the same thing,
    /// fixed environment and configuration included.
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

    /// Runs to completion; the caller inspects the exit code itself. Shared
    /// on a background handle ([`GitExecutor::background`]).
    pub async fn run_unchecked(
        &self,
        cmd: GitCommand,
        cancel: &CancellationToken,
    ) -> Result<GitOutput, GitError> {
        if self.priority == Priority::Background {
            return self.run_shared(&cmd, cancel).await;
        }
        self.run_buffered(&cmd, cancel, None).await
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
        let out = self.execute(&cmd, cancel, on_stdout, None).await?;
        if out.code != 0 {
            return Err(GitError::Failed {
                command: described,
                code: out.code,
                stderr: out.stderr_utf8().trim().to_string(),
            });
        }
        Ok(out)
    }

    async fn run_buffered(
        &self,
        cmd: &GitCommand,
        cancel: &CancellationToken,
        lead: Option<&mut Lead>,
    ) -> Result<GitOutput, GitError> {
        let mut stdout = Vec::new();
        let mut out = self
            .execute(
                cmd,
                cancel,
                &mut |chunk| stdout.extend_from_slice(chunk),
                lead,
            )
            .await?;
        out.stdout = stdout;
        Ok(out)
    }

    /// Lead, or follow a leader still queued with the same command. A
    /// follower whose leader left without an answer (cancelled, unwound)
    /// asks again, and leads if nobody else is queued by then.
    async fn run_shared(
        &self,
        cmd: &GitCommand,
        cancel: &CancellationToken,
    ) -> Result<GitOutput, GitError> {
        loop {
            match self.slots.lead_or_follow(self.group_key(cmd)) {
                WayIn::Lead(mut lead) => {
                    let outcome = self.run_buffered(cmd, cancel, Some(&mut lead)).await;
                    match Shared::of(&outcome) {
                        Some(shared) => lead.answer(shared),
                        // Cancelled: the followers were not, and are told
                        // nothing — dropping the lead is what tells them.
                        None => drop(lead),
                    }
                    return outcome;
                }
                WayIn::Follow(told) => {
                    tokio::select! {
                        biased;
                        () = cancel.cancelled() => {
                            return Err(GitError::Cancelled {
                                command: cmd.describe(),
                            });
                        }
                        answer = told => match answer {
                            Ok(shared) => {
                                tracing::debug!(command = %cmd.describe(), "git answered by a shared run");
                                return shared.outcome();
                            }
                            Err(_) => {
                                tracing::debug!(command = %cmd.describe(), "the shared run left without an answer; asking again");
                            }
                        },
                    }
                }
            }
        }
    }

    fn group_key(&self, cmd: &GitCommand) -> GroupKey {
        GroupKey {
            program: (*self.program).clone(),
            cwd: cmd.cwd.clone(),
            args: cmd.args.clone(),
            env: self
                .env
                .iter()
                .cloned()
                .chain(cmd.env.iter().cloned())
                .collect(),
        }
    }

    /// Environment layers: fixed, executor, per-command — later layers win.
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

    /// One command, from the ask to the reap. The observer hears of it at
    /// the ask, not the spawn: a row is owed from the moment the reader
    /// pressed, queue or no queue.
    ///
    /// `lead` is told of the spawn as it happens — the instant an identical
    /// ask stops being able to share this run.
    async fn execute(
        &self,
        cmd: &GitCommand,
        cancel: &CancellationToken,
        on_stdout: &mut (dyn FnMut(&[u8]) + Send),
        mut lead: Option<&mut Lead>,
    ) -> Result<GitOutput, GitError> {
        let described = cmd.describe();
        let asked = Instant::now();
        let watch = match self.observer.as_ref() {
            Some(o) if o.records(self.kept) => Some((
                o,
                o.started(
                    &described,
                    &self.describe_full(cmd),
                    self.kept,
                    self.operation,
                ),
            )),
            _ => None,
        };
        let report = |end: CommandEnd, waited: Duration, ran: Duration, message: &str| {
            if let Some((observer, id)) = &watch {
                observer.finished(
                    *id,
                    end,
                    waited.as_millis() as u64,
                    ran.as_millis() as u64,
                    message,
                );
            }
        };

        // The token ends the wait as it ends the run: nothing is spawned
        // for a selection that moved on.
        let slot = tokio::select! {
            biased;
            () = cancel.cancelled() => None,
            slot = self.slots.acquire(self.priority, cmd.pace) => slot,
        };
        let Some(_slot) = slot else {
            tracing::debug!(command = %described, "git cancelled while waiting for a slot");
            report(CommandEnd::Cancelled, asked.elapsed(), Duration::ZERO, "");
            return Err(GitError::Cancelled { command: described });
        };
        let waited = asked.elapsed();
        if let Some(lead) = &mut lead {
            lead.spawning();
        }
        let mut command = self.assemble(cmd);

        tracing::debug!(
            command = %described,
            waited_ms = waited.as_millis() as u64,
            "spawning git"
        );
        let started = Instant::now();
        let mut child = command.spawn().map_err(|source| {
            report(
                CommandEnd::Failed,
                waited,
                started.elapsed(),
                &source.to_string(),
            );
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
        // The spawn's own cost: on Windows most of a round trip
        // (ci/baseline/code-costs-windows-x64.md).
        let spawned = started.elapsed();

        let budget = match cmd.timeout {
            TimeBudget::Stock => self.stock_timeout,
            TimeBudget::At(timeout) => Some(timeout),
            TimeBudget::Never => None,
        };
        let outcome = run_child(&mut child, budget, cancel, on_stdout).await;
        // Reaped (or killed) whichever way it ended: the life is spent.
        super::meter::charge(started.elapsed());
        let outcome = outcome.map_err(|source| {
            report(
                CommandEnd::Failed,
                waited,
                started.elapsed(),
                &source.to_string(),
            );
            GitError::Io {
                command: described.clone(),
                source,
            }
        })?;

        let clocks = Clocks {
            waited,
            started,
            spawned,
        };
        Self::ended(cmd, described, outcome, budget, clocks, &report)
    }

    /// The child's end: the log line, the observer's report, and the
    /// answer or the error.
    fn ended(
        cmd: &GitCommand,
        described: String,
        outcome: ChildOutcome,
        budget: Option<Duration>,
        clocks: Clocks,
        report: &dyn Fn(CommandEnd, Duration, Duration, &str),
    ) -> Result<GitOutput, GitError> {
        let Clocks {
            waited,
            started,
            spawned,
        } = clocks;
        match outcome {
            ChildOutcome::Finished { code, stderr } => {
                tracing::debug!(
                    command = %described,
                    code,
                    waited_ms = waited.as_millis() as u64,
                    spawn_ms = spawned.as_millis() as u64,
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
                    waited,
                    started.elapsed(),
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
                report(CommandEnd::TimedOut, waited, started.elapsed(), "");
                Err(GitError::TimedOut {
                    command: described,
                    // `unwrap_or` only for the error message: TimedOut cannot
                    // happen without a configured timeout.
                    timeout: budget.unwrap_or(Duration::ZERO),
                })
            }
            ChildOutcome::Cancelled => {
                tracing::debug!(command = %described, "git cancelled; killed");
                report(CommandEnd::Cancelled, waited, started.elapsed(), "");
                Err(GitError::Cancelled { command: described })
            }
        }
    }
}

/// What one command's end is reported against.
struct Clocks {
    waited: Duration,
    started: Instant,
    spawned: Duration,
}
