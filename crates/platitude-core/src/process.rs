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

/// How a git invocation ended.
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

/// One git invocation: arguments, working directory and time budget.
#[derive(Debug, Clone)]
pub struct GitCommand {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    timeout: Option<Duration>,
    /// Applied after [`FIXED_ENV`], so a command can override a default
    /// or add its own (interactive rebase adds `GIT_SEQUENCE_EDITOR`).
    env: Vec<(OsString, OsString)>,
    /// This command answers by exit code, so a non-zero one is data.
    answers_by_code: bool,
}

impl GitCommand {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            cwd: None,
            timeout: Some(DEFAULT_TIMEOUT),
            env: Vec::new(),
            answers_by_code: false,
        }
    }

    /// Marks a command whose non-zero exit is the answer rather than a
    /// failure, so the command log keeps the row without raising itself
    /// over it (デザイン規約 §git が言ったことを読む場所).
    pub fn answers_by_code(mut self) -> Self {
        self.answers_by_code = true;
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

    /// Directory the command runs in (normally the repository work tree).
    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// Sets one environment variable for this invocation only, overriding
    /// the fixed defaults.
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

/// Captured result of a finished invocation.
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
            observer: None,
            user: false,
        }
    }

    /// Returns a handle that reports its invocations to `observer`.
    /// `user` marks the commands the user asked for.
    pub fn observed(&self, observer: Arc<dyn CommandObserver>, user: bool) -> Self {
        Self {
            program: Arc::clone(&self.program),
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
                    // Only 0 and 1 are answers ("yes" / "no"); a 128 from
                    // a command marked answers_by_code is still a failure
                    // and must look like one in the log.
                    if cmd.answers_by_code && (code == 0 || code == 1) {
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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// A cross-platform command that sleeps for ~30s, used to exercise the
    /// timeout / cancellation paths without depending on git behavior.
    fn sleeper() -> Command {
        #[cfg(windows)]
        {
            let mut c = Command::new("powershell");
            c.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ]);
            c
        }
        #[cfg(not(windows))]
        {
            let mut c = Command::new("sh");
            c.args(["-c", "sleep 30"]);
            c
        }
    }

    fn prepare(mut c: Command) -> Command {
        c.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        c
    }

    #[tokio::test]
    async fn timeout_kills_the_child() {
        let started = Instant::now();
        let mut child = prepare(sleeper()).spawn().unwrap();
        let cancel = CancellationToken::new();
        let outcome = run_child(
            &mut child,
            Some(Duration::from_millis(300)),
            &cancel,
            &mut |_| {},
        )
        .await
        .unwrap();
        assert!(matches!(outcome, ChildOutcome::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(15));
    }

    #[tokio::test]
    async fn cancellation_kills_the_child() {
        let started = Instant::now();
        let mut child = prepare(sleeper()).spawn().unwrap();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });
        let outcome = run_child(&mut child, None, &cancel, &mut |_| {})
            .await
            .unwrap();
        assert!(matches!(outcome, ChildOutcome::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(15));
    }

    #[test]
    fn describe_joins_arguments() {
        let cmd = GitCommand::new().args(["log", "--topo-order"]);
        assert_eq!(cmd.describe(), "git log --topo-order");
    }

    #[test]
    fn describe_quotes_what_a_shell_would_split_or_read() {
        let cmd = GitCommand::new().args(["stash", "push", "-m", "work in progress"]);
        assert_eq!(cmd.describe(), "git stash push -m 'work in progress'");
        let cmd = GitCommand::new().args(["add", "--", &literal_pathspec("a b.txt")]);
        assert_eq!(cmd.describe(), "git add -- ':(literal)a b.txt'");
    }

    #[test]
    fn the_full_form_spells_out_what_is_always_applied() {
        let exec = GitExecutor::new();
        let full = exec.describe_full(&GitCommand::new().args(["status", "--porcelain=v2"]));
        assert!(full.starts_with("LC_ALL=C "), "{full}");
        assert!(full.contains("GIT_TERMINAL_PROMPT=0"), "{full}");
        assert!(
            full.contains("git -c color.ui=false"),
            "the fixed configuration is part of what ran: {full}"
        );
        assert!(full.ends_with(" status --porcelain=v2"), "{full}");
    }

    #[test]
    fn a_per_command_environment_override_shows_up_in_the_full_form() {
        let exec = GitExecutor::new();
        let full = exec.describe_full(
            &GitCommand::new()
                .env("GIT_EDITOR", "pg-todo-editor")
                .arg("rebase"),
        );
        assert!(full.contains("GIT_EDITOR=pg-todo-editor"), "{full}");
    }

    #[derive(Default)]
    struct Recorder {
        seen: Mutex<Vec<(u64, String, CommandEnd, String)>>,
    }

    impl CommandObserver for Recorder {
        fn records(&self, _user: bool) -> bool {
            true
        }

        fn started(&self, display: &str, _full: &str, _user: bool) -> u64 {
            let mut seen = self.seen.lock().unwrap();
            let id = seen.len() as u64;
            seen.push((id, display.to_string(), CommandEnd::Failed, String::new()));
            id
        }

        fn finished(&self, id: u64, end: CommandEnd, _elapsed_ms: u64, message: &str) {
            let mut seen = self.seen.lock().unwrap();
            if let Some(entry) = seen.get_mut(id as usize) {
                entry.2 = end;
                entry.3 = message.to_string();
            }
        }
    }

    #[tokio::test]
    async fn the_observer_hears_about_a_command_that_never_started() {
        let recorder = Arc::new(Recorder::default());
        let exec = GitExecutor::with_program("pg-no-such-program")
            .observed(Arc::clone(&recorder) as Arc<dyn CommandObserver>, true);
        let out = exec
            .run_unchecked(GitCommand::new().arg("status"), &CancellationToken::new())
            .await;
        assert!(out.is_err());
        let seen = recorder.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].1, "git status");
        assert_eq!(seen[0].2, CommandEnd::Failed);
        assert!(!seen[0].3.is_empty(), "the reason is reported");
    }
}
