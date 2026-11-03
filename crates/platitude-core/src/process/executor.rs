//! Running what [`GitCommand`] describes: the executor, its fixed
//! arguments and environment, and the slot every spawn waits for (the
//! child supervision is [`super::child`], the slots [`super::slots`]).

use std::ffi::OsString;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use super::child::{ChildOutcome, Out, Stdout, run_child};
use super::command::{
    CommandEnd, CommandObserver, GitCommand, GitOutput, Kept, TimeBudget, shell_quote,
};
use super::family::Family;
use super::live::Live;
use super::slots::{GroupKey, Lead, Pace, Priority, Shared, Slots, WayIn};
use super::trace::Trace;
use super::trace_sink::TraceSink;
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

/// Configuration arguments prepended to every invocation.
///
/// Every pin is a form checked at the minimum git (`version.rs`); the
/// index of all pins is gitの扱い.md §発行前の釘付け.
const FIXED_ARGS: [&str; 13] = [
    // A user's colour and path quoting would put escape codes and
    // `"…"`-quoted paths into the records the parsers read.
    "-c",
    "color.ui=false",
    "-c",
    "core.quotepath=false",
    // log.showSignature=true would interleave gpg output with --format
    // records.
    "-c",
    "log.showSignature=false",
    // Every message read here is parsed as UTF-8, and every message
    // written here is UTF-8 bytes: a reader's terminal encoding (Shift_JIS,
    // say) would re-encode what `log` prints, and a reader's commit
    // encoding would stamp the commits made here with an `encoding` header
    // the bytes do not match (user decision — ask before changing).
    "-c",
    "i18n.logOutputEncoding=utf-8",
    "-c",
    "i18n.commitEncoding=utf-8",
    // A working-tree `git diff` refreshes the index under `.git/index.lock`
    // unless this says not to: neither `--no-optional-locks` nor
    // `GIT_OPTIONAL_LOCKS=0` reaches diff (measured on 2.55). A write dies
    // on that lock without waiting, so a poll's diff outside the queue kills
    // a concurrent commit (`fatal: Unable to create ... index.lock: File
    // exists`). No answer changes: a file whose stat alone moved reads as
    // unchanged to a patch, `--name-only` and `--quiet` alike. The price is
    // that nobody refreshes the index's stat cache, so a tree whose stats all
    // moved with the content the same (copied from outside git) is hashed
    // again on every read. A slow pre-commit hook holds no lock — git gives
    // it back before running the hook — so `--preset slowhook` covers only
    // "the write is in flight". Held by
    // `tests/it/exec.rs::a_working_tree_read_leaves_the_index_untouched`.
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
///
/// Under `GIT_OPTIONAL_LOCKS=0` the index is never written back, so
/// `core.fsmonitor` and `core.untrackedCache` never update their token or
/// cache and speed nothing up: not adopted for `status`
/// (ci/baseline/git-slots-windows-x64.md §`status` の高速化機構).
const FIXED_ENV: [(&str, &str); 4] = [
    ("LC_ALL", "C"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_OPTIONAL_LOCKS", "0"),
    ("GIT_EDITOR", "true"),
];

/// Environment taken off every invocation before anything is set, as
/// inherited from whatever launched this process — a terminal inside a
/// hook, a `git` alias, an IDE's run configuration. git's own
/// repository-locating variables (environment.c `local_repo_env`, the
/// minimum to 2.55: what git clears for a command it runs in another
/// repository) would point every command here at some other repository,
/// as would a ref namespace (`GIT_NAMESPACE`); a reflog action
/// (`GIT_REFLOG_ACTION`, which a rebase exports to what it runs) would
/// relabel the reflog lines `discards::read` reads; the rest reshape a
/// diff or a pathspec under the parsers. A command's own `env` is applied
/// after, so a scratch index (`GIT_INDEX_FILE`) still reaches its git.
const INHERITED_ENV_DROPPED: [&str; 23] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_PREFIX",
    "GIT_GRAFT_FILE",
    "GIT_SHALLOW_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_EXTERNAL_DIFF",
    "GIT_DIFF_OPTS",
    "GIT_LITERAL_PATHSPECS",
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
    "GIT_NAMESPACE",
    "GIT_REFLOG_ACTION",
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
    /// Where this handle's running processes are counted, shared by every
    /// clone ([`GitExecutor::counted`]); `None` counts nothing.
    live: Option<Arc<Live>>,
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
            live: None,
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
            live: self.live.clone(),
        }
    }

    /// Returns a handle that counts its running processes in `live` — install
    /// before the handle is cloned into anything, or a clone counts nowhere.
    #[must_use]
    pub fn counted(mut self, live: Arc<Live>) -> Self {
        self.live = Some(live);
        self
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
    /// on a background handle ([`GitExecutor::background`]) — unless it
    /// carries input, which the share's key does not hold.
    pub async fn run_unchecked(
        &self,
        cmd: GitCommand,
        cancel: &CancellationToken,
    ) -> Result<GitOutput, GitError> {
        if self.priority == Priority::Background && cmd.input.is_none() {
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
        let out = self
            .execute(&cmd, cancel, &mut Out::Streamed(on_stdout), None)
            .await?;
        if out.code != 0 {
            return Err(GitError::Failed {
                command: described,
                code: out.code,
                stderr: out.stderr_utf8().trim().to_string(),
            });
        }
        Ok(out)
    }

    /// [`Self::run_streaming`], each chunk sent to `to` and the read
    /// waiting while `to`'s receiver is behind: git's output stops in its
    /// pipe until the receiver catches up, so no more of it stands in
    /// memory than the channel holds. A receiver gone ends git, and the run
    /// fails. A run that sent anything is not asked once more
    /// (`config_unread`): its receiver would read the start twice.
    pub async fn run_piped(
        &self,
        cmd: GitCommand,
        cancel: &CancellationToken,
        to: &tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> Result<GitOutput, GitError> {
        let described = cmd.describe();
        let out = self
            .execute(&cmd, cancel, &mut Out::Piped(to), None)
            .await?;
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
            .execute(cmd, cancel, &mut Out::Kept(&mut stdout), lead)
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

    /// Environment layers: the inherited repository taken off, then fixed,
    /// executor, per-command — later layers win.
    fn assemble(&self, cmd: &GitCommand) -> Command {
        let mut command = Command::new(self.program.as_ref());
        command.args(FIXED_ARGS);
        command.args(&cmd.args);
        if let Some(dir) = &cmd.cwd {
            command.current_dir(dir);
        }
        for name in INHERITED_ENV_DROPPED {
            command.env_remove(name);
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
        let stdin = match cmd.input {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        };
        command
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command
    }

    /// One command, from the ask to the reap. The observer hears of it at
    /// the ask, not the spawn: a row is owed from the moment the reader
    /// pressed, queue or no queue. Keep the telling ahead of the slot's
    /// `acquire`; `CommandStarted` therefore says asked, not queued, and a
    /// test that needs the queue waits on `Slots::settled`.
    ///
    /// `lead` is told of the spawn as it happens — the instant an identical
    /// ask stops being able to share this run.
    ///
    /// A read git ran without a configuration file it could not open runs
    /// once more, in the same slot and under the same row
    /// ([`super::config_unread`]): the row says what the last run said.
    async fn execute(
        &self,
        cmd: &GitCommand,
        cancel: &CancellationToken,
        out: &mut Out<'_>,
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
        let budget = match cmd.timeout {
            TimeBudget::Stock => self.stock_timeout,
            TimeBudget::At(timeout) => Some(timeout),
            TimeBudget::Never => None,
        };
        let row = Row {
            described: &described,
            waited: asked.elapsed(),
            budget,
            report: &report,
        };
        // When the first spawn started: the row's time is the whole ask's.
        let mut first: Option<Instant> = None;
        // The spawn asked once more over, held until the next is counted.
        let mut prior: Option<Attempt> = None;
        loop {
            let attempt = self
                .attempt(cmd, &row, cancel, out, lead.take(), prior.take())
                .await?;
            let again = first.is_none()
                && matches!(&attempt.outcome, ChildOutcome::Finished { code, stderr, .. }
                if super::config_unread::again(
                    &cmd.args,
                    *code,
                    &String::from_utf8_lossy(stderr),
                    attempt.restartable,
                    out.is_kept(),
                ));
            let started = *first.get_or_insert(attempt.started);
            if again {
                tracing::info!(
                    command = %described,
                    "git could not open a configuration file; asking once more"
                );
                out.restart();
                prior = Some(attempt);
                continue;
            }
            let clocks = Clocks {
                waited: row.waited,
                started,
                spawned: attempt.spawned,
            };
            return Self::ended(
                cmd,
                described.clone(),
                attempt.outcome,
                attempt.left,
                budget,
                clocks,
                &report,
            );
        }
    }

    /// One spawn of `cmd` and its reap, the slot already held
    /// ([`Self::execute`]). Every way it ends before git does is reported
    /// here; git's own end is the caller's to report. `prior`, the spawn
    /// this one asks again over, is let go once this one is counted and any
    /// end it reports has been reported, so the count never reads none with
    /// the row's end still owed.
    async fn attempt(
        &self,
        cmd: &GitCommand,
        row: &Row<'_>,
        cancel: &CancellationToken,
        out: &mut Out<'_>,
        lead: Option<&mut Lead>,
        prior: Option<Attempt>,
    ) -> Result<Attempt, GitError> {
        // Counted before the spawn, the token read after: an owner that
        // cancels and then finds none running (`Live::idle`) cannot pass a
        // git that is still being spawned, and none is spawned after.
        let running = self.live.as_ref().map(Live::enter);
        if cancel.is_cancelled() {
            drop(running);
            tracing::debug!(command = %row.described, "git cancelled before its spawn");
            (row.report)(CommandEnd::Cancelled, row.waited, Duration::ZERO, "");
            return Err(GitError::Cancelled {
                command: row.described.to_string(),
            });
        }
        drop(prior);
        if let Some(lead) = lead {
            lead.spawning();
        }
        // The receiver closes once git has ended and its record is read.
        let (mut command, trace_sink) = self.assemble_traced(cmd);

        tracing::debug!(
            command = %row.described,
            waited_ms = row.waited.as_millis() as u64,
            "spawning git"
        );
        let started = Instant::now();
        // The count goes with the run: let go once it is gone (`family`). A
        // run paced from outside is stopped at git alone: what it starts can
        // be the person's own (a browser a credential helper opened).
        let whole = cmd.pace == Pace::Here;
        let (mut child, mut family) = match super::family::spawn(&mut command, whole, running) {
            Ok(spawned) => spawned,
            Err(source) => {
                (row.report)(
                    CommandEnd::Failed,
                    row.waited,
                    started.elapsed(),
                    &source.to_string(),
                );
                return Err(Self::unspawned(cmd, row.described.to_string(), source));
            }
        };
        // The spawn's own cost: on Windows most of a round trip
        // (ci/baseline/code-costs-windows-x64.md).
        let spawned = started.elapsed();

        let mut stdout = Stdout::new(out, cmd.logs_stdout);
        let outcome = run_child(
            &mut child,
            &mut family,
            cmd.input.as_ref(),
            row.budget,
            cancel,
            &mut stdout,
        )
        .await;
        let (printed, kept) = stdout.ended();
        // Reaped (or killed) whichever way it ended: the life is spent.
        super::meter::charge(started.elapsed());
        let outcome = outcome.map_err(|source| {
            (row.report)(
                CommandEnd::Failed,
                row.waited,
                started.elapsed(),
                &source.to_string(),
            );
            GitError::Io {
                command: row.described.to_string(),
                source,
            }
        })?;
        Ok(Attempt {
            outcome,
            left: Left {
                stdout: kept,
                trace: Self::record_of(trace_sink).await,
            },
            started,
            spawned,
            restartable: out.is_kept() || !printed,
            _family: family,
        })
    }

    /// What a traced command's gits recorded, once git is reaped
    /// ([`TraceSink::finish`]); nothing for one not traced.
    async fn record_of(sink: Option<TraceSink>) -> Trace {
        match sink {
            Some(sink) => Trace::parse(&sink.finish().await),
            None => Trace::default(),
        }
    }

    /// [`Self::assemble`], and for a traced command (`GitCommand::traced`)
    /// the receiver its gits write their record to ([`TraceSink`]).
    fn assemble_traced(&self, cmd: &GitCommand) -> (Command, Option<TraceSink>) {
        let mut command = self.assemble(cmd);
        let sink = match cmd.traced {
            true => TraceSink::open(),
            false => None,
        };
        if let Some(sink) = &sink {
            command.env("GIT_TRACE2_EVENT", sink.target());
            // A user's `trace2.eventBrief` leaves out the times that tell a
            // hook's own git from the command's (`super::trace::Trace`).
            command.env("GIT_TRACE2_EVENT_BRIEF", "false");
            // The command is the root of its own record: a parent's session
            // id — this application run under a traced git — would prefix
            // every id in it (trace2/tr2_sid.c).
            command.env_remove("GIT_TRACE2_PARENT_SID");
        }
        (command, sink)
    }

    /// A spawn that failed: git missing where the folder it was to run in
    /// is there, anything else as the spawn's own error.
    fn unspawned(cmd: &GitCommand, described: String, source: std::io::Error) -> GitError {
        if source.kind() == std::io::ErrorKind::NotFound
            && cmd.cwd.as_ref().is_none_or(|d| d.is_dir())
        {
            GitError::GitNotFound { source }
        } else {
            GitError::Spawn {
                command: described,
                source,
            }
        }
    }

    /// The child's end: the log line, the observer's report, and the
    /// answer or the error.
    fn ended(
        cmd: &GitCommand,
        described: String,
        outcome: ChildOutcome,
        left: Left,
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
            ChildOutcome::Finished {
                code,
                stderr,
                unfed,
            } => {
                // git closed its end with its input still being sent. A yes
                // then is no answer to what was asked; a no is git's answer,
                // and the pipe it broke on its way out says nothing more.
                if let Some(unfed) = unfed {
                    if code == 0 {
                        let source = std::io::Error::new(
                            unfed.kind(),
                            format!("git ended without taking all of its input: {unfed}"),
                        );
                        tracing::warn!(command = %described, error = %source, "git answered over input it did not take");
                        report(
                            CommandEnd::Failed,
                            waited,
                            started.elapsed(),
                            &source.to_string(),
                        );
                        return Err(GitError::Io {
                            command: described,
                            source,
                        });
                    }
                    tracing::debug!(command = %described, code, "git ended before taking all of its input");
                }
                tracing::debug!(
                    command = %described,
                    code,
                    waited_ms = waited.as_millis() as u64,
                    spawn_ms = spawned.as_millis() as u64,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "git finished"
                );
                let said = String::from_utf8_lossy(&stderr);
                let said = said.trim_end();
                let answered = match cmd.answer_code {
                    _ if code == 0 => cmd.answer_code.is_some(),
                    // Only the code this command named is an answer: a 128
                    // from one that named 1 is still a failure.
                    Some(answer) => {
                        code == answer
                            && !cmd.not_an_answer.is_some_and(|words| words.found_in(said))
                            && !cmd
                                .not_an_answer_when
                                .is_some_and(|refused| refused(said, &left.trace))
                    }
                    None => false,
                };
                let shown = String::from_utf8_lossy(&left.stdout);
                let message = match shown.trim_end() {
                    out if out.is_empty() || code == 0 || answered => said.to_string(),
                    out if said.is_empty() => out.to_string(),
                    out => format!("{out}\n{said}"),
                };
                report(
                    if answered {
                        CommandEnd::Answered(code)
                    } else {
                        CommandEnd::Exited(code)
                    },
                    waited,
                    started.elapsed(),
                    &message,
                );
                Ok(GitOutput {
                    code,
                    stdout: Vec::new(),
                    stderr,
                    trace: left.trace,
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

/// The log row one ask reports to, once, whichever of its spawns ends it —
/// and the slot wait and budget every spawn of it runs under.
struct Row<'r> {
    described: &'r str,
    waited: Duration,
    budget: Option<Duration>,
    report: &'r (dyn Fn(CommandEnd, Duration, Duration, &str) + Sync),
}

/// One spawn's end, to answer with or to ask once more over.
struct Attempt {
    outcome: ChildOutcome,
    left: Left,
    started: Instant,
    spawned: Duration,
    /// Nothing it printed has been handed on: a kept run, or a stream that
    /// ended before its first byte.
    restartable: bool,
    /// The run's count, let go only after its end is reported: an owner that
    /// finds none running (`Live::idle`) has heard every end.
    _family: Family,
}

/// What one command's end is reported against.
struct Clocks {
    waited: Duration,
    started: Instant,
    spawned: Duration,
}

/// What a run left besides its end.
struct Left {
    /// What the row may show of stdout: empty unless the command keeps it
    /// (`GitCommand::logs_stdout`).
    stdout: Vec<u8>,
    /// git's record of the programs it ran (`GitCommand::traced`).
    trace: Trace,
}
