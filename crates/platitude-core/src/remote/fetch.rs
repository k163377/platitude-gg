//! `git fetch --prune`.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// `git fetch --prune`. `remote` of `None` fetches every remote.
pub async fn fetch(
    executor: &GitExecutor,
    workdir: &Path,
    remote: Option<&str>,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["fetch", "--prune"])
        .timeout(timeout);
    let cmd = match remote {
        Some(name) => cmd.args(["--", name]),
        None => cmd.arg("--all"),
    };
    executor.run(cmd, cancel).await.map(|_| ())
}
