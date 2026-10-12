//! Driving a spawned git child: the pump, the caps, and the race
//! against the timeout and the cancel token.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

use super::family::Family;

/// Stderr kept at most: git's messages are short, and a runaway process
/// is capped here.
const STDERR_CAP: usize = 256 * 1024;

const STDOUT_CHUNK: usize = 64 * 1024;

pub(super) enum ChildOutcome {
    Finished { code: i32, stderr: Vec<u8> },
    TimedOut,
    Cancelled,
}

/// The run is ended ([`Family::end`]) when the timeout or the token fires,
/// or a pipe breaks, and git is reaped on every road out of here. A whole
/// run's stop whose end cannot be told is an error, not a cancel.
pub(super) async fn run_child(
    child: &mut Child,
    family: &mut Family,
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

    // `biased`: cancellation wins over a process completing at the same
    // time. The losing futures are dropped before the arm body runs, which
    // releases their borrow of `child` so it can be killed.
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            family.end(child).await?;
            Ok(ChildOutcome::Cancelled)
        }
        _ = deadline => {
            family.end(child).await?;
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
                    if let Err(end) = family.end(child).await {
                        tracing::debug!(error = %end, "a run broken off could not be told gone");
                    }
                    Err(e)
                }
            }
        }
    }
}
