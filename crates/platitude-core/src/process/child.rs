//! Driving a spawned git child: the pump, the caps, and the race
//! against the timeout and the cancel token.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio_util::sync::CancellationToken;

use super::command::Input;
use super::family::Family;

/// Stderr kept at most: git's messages are short, and a runaway process
/// is capped here.
const STDERR_CAP: usize = 256 * 1024;

const STDOUT_CHUNK: usize = 64 * 1024;

/// Where a run's stdout goes.
pub(super) enum Out<'a> {
    /// Kept whole for the caller — and so dropped again for a run asked
    /// once more ([`super::config_unread`]).
    Kept(&'a mut Vec<u8>),
    /// Handed on as it comes.
    Streamed(&'a mut (dyn FnMut(&[u8]) + Send)),
    /// Sent on as it comes, the read waiting while the receiver is behind:
    /// git's output stops in its pipe until the receiver catches up. A
    /// receiver gone breaks the pipe.
    Piped(&'a tokio::sync::mpsc::Sender<Vec<u8>>),
}

impl Out<'_> {
    /// Hands `chunk` on; `false` where its receiver is gone.
    async fn take(&mut self, chunk: &[u8]) -> bool {
        match self {
            Out::Kept(kept) => {
                kept.extend_from_slice(chunk);
                true
            }
            Out::Streamed(on_stdout) => {
                on_stdout(chunk);
                true
            }
            Out::Piped(to) => to.send(chunk.to_vec()).await.is_ok(),
        }
    }

    pub(super) fn is_kept(&self) -> bool {
        matches!(self, Out::Kept(_))
    }

    /// Drops what the run kept, for the run asked once more.
    pub(super) fn restart(&mut self) {
        if let Out::Kept(kept) = self {
            kept.clear();
        }
    }
}

/// One run's stdout: where it goes, the row's copy of it where the command
/// keeps one (`GitCommand::logs_stdout`), and whether any of it went.
pub(super) struct Stdout<'s, 'a> {
    out: &'s mut Out<'a>,
    log: Option<Vec<u8>>,
    printed: bool,
}

impl<'s, 'a> Stdout<'s, 'a> {
    pub(super) fn new(out: &'s mut Out<'a>, logs: bool) -> Self {
        Self {
            out,
            log: logs.then(Vec::new),
            printed: false,
        }
    }

    /// Whether any of it went, and the row's copy — empty where the
    /// command keeps none.
    pub(super) fn ended(self) -> (bool, Vec<u8>) {
        (self.printed, self.log.unwrap_or_default())
    }

    /// Hands on all of `pipe`, chunk by chunk, to its end; a broken pipe
    /// where the receiver is gone.
    async fn pump(&mut self, pipe: &mut ChildStdout) -> std::io::Result<()> {
        let mut buf = vec![0u8; STDOUT_CHUNK];
        loop {
            let n = pipe.read(&mut buf).await?;
            if n == 0 {
                return Ok(());
            }
            let chunk = &buf[..n];
            if let Some(log) = self.log.as_mut() {
                log.extend_from_slice(chunk);
            }
            self.printed = true;
            if !self.out.take(chunk).await {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "the reader of git's output is gone",
                ));
            }
        }
    }
}

pub(super) enum ChildOutcome {
    Finished {
        code: i32,
        stderr: Vec<u8>,
        /// The broken pipe the send met where git ended before it had read
        /// all of its input. The executor decides what that makes of the run
        /// ([`super::GitExecutor`]'s `ended`).
        unfed: Option<std::io::Error>,
    },
    TimedOut,
    Cancelled,
}

/// The run is ended ([`Family::end`]) when the timeout or the token fires,
/// a pipe git prints to breaks (its stdout's receiver gone among them), or
/// its input cannot be sent, and git is reaped on every road out of here. A
/// whole run's stop whose end cannot be told is an error, not a cancel.
///
/// `input` is sent while stdout and stderr are read — one future each,
/// polled together, so a git that answers as it reads cannot stall with
/// its output full and the send waiting on it — and stdin is closed once
/// it has all gone, so git reads its end. A failed send, the timeout and
/// the token leave stdin open until git is ended (a local the send only
/// borrows, closed by [`stop`] after the end): an end git read early would
/// hand it what came before as the whole of its input. A git that closed
/// its end first (a broken pipe) ends as it will. A run dropped whole drops
/// stdin ahead of `Family`'s end; the commands with input are writes, whose
/// lane is never dropped (it waits even on close — `operation::Lane::Local`).
pub(super) async fn run_child(
    child: &mut Child,
    family: &mut Family,
    input: Option<&Input>,
    timeout: Option<Duration>,
    cancel: &CancellationToken,
    stdout: &mut Stdout<'_, '_>,
) -> std::io::Result<ChildOutcome> {
    let mut stdin_pipe = child.stdin.take();
    if let (Some(pipe), Some(input)) = (&stdin_pipe, input) {
        widen(pipe, input.bytes().len());
    }
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let deadline = async {
        match timeout {
            Some(d) => tokio::time::sleep(d).await,
            None => std::future::pending().await,
        }
    };

    let work = async {
        let feed = async {
            let (Some(pipe), Some(input)) = (stdin_pipe.as_mut(), input) else {
                return Ok(None);
            };
            match send(pipe, input.bytes()).await {
                Ok(()) => {
                    // Closed: git reads the end of its input.
                    stdin_pipe = None;
                    Ok(None)
                }
                Err(broken) if broken.kind() == std::io::ErrorKind::BrokenPipe => {
                    stdin_pipe = None;
                    Ok(Some(broken))
                }
                Err(failed) => Err(failed),
            }
        };
        let stdout_fut = async {
            match stdout_pipe.as_mut() {
                Some(pipe) => stdout.pump(pipe).await,
                None => Ok(()),
            }
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
        // Either pipe's error ends the reading at once: a receiver gone
        // leaves git alive, blocked on its stdout, and its stderr open.
        let readers = async {
            let ((), stderr) = tokio::try_join!(stdout_fut, stderr_fut)?;
            Ok::<Vec<u8>, std::io::Error>(stderr)
        };
        // A send that fails stops the wait at once: git, its stdin still
        // open, would read on without end.
        let (unfed, stderr) = tokio::try_join!(feed, readers)?;
        let status = child.wait().await?;
        Ok::<ChildOutcome, std::io::Error>(ChildOutcome::Finished {
            code: status.code().unwrap_or(-1),
            stderr,
            unfed,
        })
    };

    // `biased`: cancellation wins over a process completing at the same
    // time. The losing futures are dropped before the arm body runs, which
    // releases their borrow of `child` so it can be killed.
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            stop(child, family, stdin_pipe.take()).await?;
            Ok(ChildOutcome::Cancelled)
        }
        _ = deadline => {
            stop(child, family, stdin_pipe.take()).await?;
            Ok(ChildOutcome::TimedOut)
        }
        res = work => {
            match res {
                Ok(outcome) => {
                    family.settle();
                    Ok(outcome)
                }
                Err(e) => {
                    // The pipe's error is the one told.
                    if let Err(end) = stop(child, family, stdin_pipe.take()).await {
                        tracing::debug!(error = %end, "a run broken off could not be told gone");
                    }
                    Err(e)
                }
            }
        }
    }
}

/// Ends a stopped run — the token, the deadline, a broken pipe git prints
/// to, a failed send — and only then closes git's stdin, open still where
/// the road left it open ([`ending`]).
async fn stop(
    child: &mut Child,
    family: &mut Family,
    stdin: Option<ChildStdin>,
) -> std::io::Result<()> {
    let ended = family.end(child).await;
    ending(stdin.is_some());
    drop(stdin);
    ended
}

/// Where a stopped run has ended git, told whether git's stdin is still
/// open — which, for a run stopped mid-send, it must be: open once git is
/// gone, it was open while git could read. A test watches the order here
/// (mry; the mock exists in debug builds only, on the thread that set it).
/// It does nothing else: nothing outside the process layer can see the
/// order (git, ended, reads no more), so this seam is kept for a test
/// alone — the exception `.claude/rules-refs/core.md` names in its stdin
/// entry.
#[mry::mry]
pub(super) fn ending(_stdin_open: bool) {}

/// How wide git's stdin is made on Linux for an input past it, where a
/// pipe starts at 64 KiB. Measured with a patch of tens of megabytes:
/// through 64 KiB it crossed into `cat` alone several times slower than
/// through 1 MiB, and git read it slower than by path by about as much,
/// most of that gone through 1 MiB (ci/baseline/input-costs-linux-x64.md
/// has the numbers, and what they do not separate). An input this size or
/// less crosses the pipe as made in at most sixteen turns, and one past
/// 64 KiB stays one git can be seen to leave unread (`super::GitExecutor`'s
/// `ended`). 1 MiB is the kernel's default ceiling for an unprivileged
/// process (`fs.pipe-max-size`). Windows' pipe measured within the noise of
/// the file, and macOS has no way to widen one.
#[cfg(target_os = "linux")]
pub(super) const PIPE_WIDTH: usize = 1 << 20;

/// Widens git's stdin to [`PIPE_WIDTH`] for an input past it; a pipe that
/// will not widen (a lower ceiling, the user's pipe pages spent) is sent
/// through as it is.
#[cfg(target_os = "linux")]
fn widen(pipe: &ChildStdin, input: usize) {
    use std::os::fd::AsFd;
    if input <= PIPE_WIDTH {
        return;
    }
    if let Err(error) = linux::set_pipe_size(pipe.as_fd(), PIPE_WIDTH) {
        tracing::debug!(%error, "git's stdin stays as narrow as it was made");
    }
}

#[cfg(not(target_os = "linux"))]
fn widen(_: &ChildStdin, _: usize) {}

#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "a pipe's width has no binding in std, and one call does not earn a dependency"
)]
pub(super) mod linux {
    use std::os::fd::{AsRawFd, BorrowedFd};

    const F_SETPIPE_SZ: i32 = 1031;

    // SAFETY: transcribed from glibc's fcntl.h (`fcntl` is variadic); an
    // fd and integers in, an integer out.
    unsafe extern "C" {
        fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    }

    /// Asks for `width` and answers the width the kernel gave. A test counts
    /// the asks over the real call (mry, as [`super::ending`]).
    #[mry::mry(skip_args(BorrowedFd))]
    pub(in crate::process) fn set_pipe_size(
        pipe: BorrowedFd<'_>,
        width: usize,
    ) -> std::io::Result<usize> {
        let asked = i32::try_from(width).map_err(std::io::Error::other)?;
        // SAFETY: an fd borrowed open for the call, and an integer.
        match unsafe { fcntl(pipe.as_raw_fd(), F_SETPIPE_SZ, asked) } {
            -1 => Err(std::io::Error::last_os_error()),
            given => Ok(given as usize),
        }
    }
}

/// All of `bytes` into git's stdin.
async fn send(pipe: &mut ChildStdin, bytes: &[u8]) -> std::io::Result<()> {
    pipe.write_all(bytes).await?;
    finish(pipe).await
}

/// The send's last step. Where the pipe hands writes to a thread
/// (Windows), the last one's end is only known here. A test makes it fail
/// once the rest is through (mry, as [`ending`]).
#[mry::mry(skip_args(ChildStdin))]
pub(super) async fn finish(pipe: &mut ChildStdin) -> std::io::Result<()> {
    pipe.flush().await
}
