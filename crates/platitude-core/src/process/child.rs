//! Driving a spawned git child: the pump, the caps, and the race
//! against the timeout and the cancel token.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

use super::executor::{STDERR_CAP, STDOUT_CHUNK};

pub(super) enum ChildOutcome {
    Finished { code: i32, stderr: Vec<u8> },
    TimedOut,
    Cancelled,
}

/// Drives a spawned child: pumps stdout into `on_stdout`, accumulates capped
/// stderr, and races completion against the timeout and the cancel token.
/// The child is killed and reaped when either fires.
pub(super) async fn run_child(
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
