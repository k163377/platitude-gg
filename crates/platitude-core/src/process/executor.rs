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

/// Default time budget for short-lived commands — which is to say, for
/// reads: the streaming log walks and `mergetool` (open-ended,
/// user-paced) opt out via [`GitCommand::no_timeout`], network commands
/// set their own, longer budget instead (`remote`), and the write
/// queue's local lane lifts the stock budget wholesale
/// (`operation::Lane::Local`) — a local write is waited out to
/// completion, because killing git mid-write loses what it was writing
/// and a local git is only ever slow in proportion to the work.
///
/// **Counted from the spawn, not from the ask**: the time a command
/// spends waiting for a slot is the application's, and a budget that
/// counted it would kill a healthy git for the queue in front of it.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Configuration arguments prepended to every invocation.
const FIXED_ARGS: [&str; 9] = [
    "-c",
    "color.ui=false",
    "-c",
    "core.quotepath=false",
    // A repo with log.showSignature=true would interleave gpg output with
    // machine-readable --format records; force it off.
    "-c",
    "log.showSignature=false",
    // **A `git diff` against the work tree writes the index unless this
    // says not to**, and `--no-optional-locks` below does not stop it:
    // `git status` asks that flag before it locks, `git diff` never asks
    // (measured — a stat-dirty index is rewritten by a diff carrying the
    // flag, and left alone by one carrying this). What it takes to do so
    // is `.git/index.lock`, the same lock a write dies on rather than
    // waits for, so a poll's diff landing on a commit's own lock kills
    // the commit (`fatal: Unable to create ... index.lock: File exists`).
    // No answer changes: the refresh is git's own cache of "this stat
    // matched", and a file whose stat alone moved is compared by content
    // either way — the patch, `--name-only` and `--quiet` all say the
    // same with it off (measured). What it costs is that cache going
    // unmaintained, since nothing this end refreshes the index any more:
    // a work tree whose stats all moved without its contents changing is
    // re-hashed by every read rather than by the one after the first —
    // which on such a tree is most of what a `status` costs
    // (ci/baseline/code-costs-windows-x64.md). Ordinary editing leaves a
    // handful of such entries; a tree copied in from outside git leaves
    // all of them.
    "-c",
    "diff.autoRefreshIndex=false",
    "--no-optional-locks",
];

/// Environment applied to every invocation.
///
/// - `LC_ALL=C`: stable, locale-independent messages and sorting
/// - `GIT_TERMINAL_PROMPT=0`: never hang on a credential prompt (auth is
///   delegated to credential helpers)
/// - `GIT_OPTIONAL_LOCKS=0`: belt-and-suspenders with `--no-optional-locks`
///   — and, like it, only over the commands that ask (`diff` does not, so
///   the index lock it would take is turned off by the argument above)
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
///
/// **Every clone waits in the same slots** ([`Slots`], shared by
/// `Arc`): the application makes one set and hands it to the handle it
/// spawns everything through ([`GitExecutor::scheduled`]), and every
/// session, screen and dialog clones that handle. A bare executor caps
/// nothing, which is what a test wants of one.
#[derive(Clone)]
pub struct GitExecutor {
    program: Arc<OsString>,
    /// Environment applied to every invocation from this executor. Kept on
    /// the executor so a test harness can isolate Git without mutating the
    /// process-global environment; command-level values override these.
    env: Arc<Vec<(OsString, OsString)>>,
    observer: Option<Arc<dyn CommandObserver>>,
    /// What the command log makes of the invocations run through this
    /// handle. Carried here rather than on the command so the callers
    /// stay unaware of it: the session hands out a different handle for
    /// each answer.
    kept: Kept,
    /// The write the commands run through this handle belong to, for the
    /// log to say so ([`CommandObserver::started`]). Set by the write
    /// queue on the handle it gives a task ([`GitExecutor::under`]);
    /// `None` on every read.
    operation: Option<OperationId>,
    /// What [`TimeBudget::Stock`] resolves to. `None` lifts the stock
    /// budget entirely — the test harness's setting, where wall time is
    /// load-dependent and must not decide correctness
    /// ([`GitExecutor::without_stock_timeouts`]); commands that named
    /// their own budget keep it either way.
    stock_timeout: Option<Duration>,
    /// Where every spawn waits its turn ([`super::slots`]).
    slots: Arc<Slots>,
    /// Who is waiting on the commands run through this handle — what
    /// the slots serve first, and what they cap ([`Priority`]). Carried
    /// on the handle for the reason `kept` is: the session hands out a
    /// handle for the reads nobody is waiting on
    /// ([`GitExecutor::background`]), and the callers stay unaware.
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

    /// Uses an explicit git binary (a settings path, a portable install,
    /// the tests' stand-ins). One thing is looked behind, as for PATH: the
    /// launcher Git for Windows installs is spawned as the git behind it
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

    /// Lifts the stock time budget from every command that did not set
    /// one of its own: those commands are then bounded by cancellation
    /// alone. Two callers. The session's write queue puts its local lane
    /// on this — a local write is waited out, never killed
    /// (`operation::Lane::Local`). And test harnesses lift the
    /// budget from their whole executor — under a loaded suite a git
    /// round trip inflates by more than an order of magnitude
    /// (ci/baseline/code-costs-windows-x64.md §テストとハーネス), and a
    /// wall-clock cap that generous decides by load, not correctness; the
    /// harness arms its own failure-detection backstops instead
    /// (.claude/rules/core.md). The application's reads keep the stock
    /// budget.
    pub fn without_stock_timeouts(mut self) -> Self {
        self.stock_timeout = None;
        self
    }

    /// Replaces the stock time budget for every command that did not set
    /// one of its own. The test harness raises it to its overall failure
    /// backstop rather than lifting it: a wedged git then fails the
    /// awaiting test by name instead of hanging the binary to the CI
    /// kill.
    pub fn with_stock_timeout(mut self, budget: Duration) -> Self {
        self.stock_timeout = Some(budget);
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

    /// Runs everything through `slots` — the application's one set,
    /// installed on the handle before it is cloned into anything, so
    /// there is no clone that waits elsewhere. A handle that was never
    /// given one caps nothing.
    #[must_use]
    pub fn scheduled(mut self, slots: Arc<Slots>) -> Self {
        self.slots = slots;
        self
    }

    /// A handle whose commands nobody is waiting on: the reads a session
    /// makes on a timer of its own, served after the interactive ones
    /// and kept out of the click's reserve ([`Priority::Background`]).
    /// Buffered
    /// runs through it that ask the same question over the same tree
    /// share one process while the first is still queued
    /// ([`super::slots`]).
    #[must_use]
    pub fn background(mut self) -> Self {
        self.priority = Priority::Background;
        self
    }

    #[must_use]
    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// The slots this handle waits in: for the application to move the
    /// limits and read the report.
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
    /// `operation` — what the write queue hands the task it runs, so a
    /// compound write's commands stand in the log under the one id its
    /// acceptance returned.
    #[must_use]
    pub fn under(mut self, operation: OperationId) -> Self {
        self.operation = Some(operation);
        self
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
    ///
    /// Through a background handle, an identical command still queued
    /// for another caller answers this one too, and no second process
    /// is spawned ([`GitExecutor::background`]).
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

    /// The buffered run of a background handle: lead, or follow a
    /// leader still queued with the same command. A follower whose
    /// leader left without an answer — cancelled, or unwound — asks
    /// again, and leads if nobody else is queued by then; its own token
    /// ends the wait the way it ends any other.
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

    /// One command, from the ask to the reap: the wait for a slot, the
    /// spawn, and the child's run. The observer hears of it at the ask —
    /// a row is owed from the moment the reader pressed, whether or not
    /// the queue in front of it is empty — and at the end, with the wait
    /// and the run told apart.
    ///
    /// `lead` is the group a background run answers for: told of the
    /// spawn as it happens, which is the instant an identical ask stops
    /// being able to share this run.
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

        // The wait is the token's to end, the same as the run: a
        // selection that moved on or a screen that closed takes its
        // queued command with it, and nothing is spawned for it.
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
        // What starting the process cost on its own — the part of a
        // round trip that is the machine's and not git's, and on Windows
        // the larger part (ci/baseline/code-costs-windows-x64.md).
        let spawned = started.elapsed();

        let budget = match cmd.timeout {
            TimeBudget::Stock => self.stock_timeout,
            TimeBudget::At(timeout) => Some(timeout),
            TimeBudget::Never => None,
        };
        let outcome = run_child(&mut child, budget, cancel, on_stdout)
            .await
            .map_err(|source| {
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

    /// What the child's end comes to: the log line, the observer's
    /// report with the wait and the run told apart, and the answer or
    /// the error.
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

/// The three moments of one command the end is told against: what it
/// waited for a slot, when it was spawned, and what the spawn itself
/// took.
struct Clocks {
    waited: Duration,
    started: Instant,
    spawned: Duration,
}
