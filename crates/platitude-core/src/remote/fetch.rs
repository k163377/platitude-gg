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
        .timeout(timeout)
        .paced_elsewhere();
    let cmd = match remote {
        Some(name) => cmd.args(["--", name]),
        None => cmd.arg("--all"),
    };
    executor.run(cmd, cancel).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_fetch_prunes_and_names_one_remote_or_all() {
        let (exec, asked) = crate::refusing::git();
        let cancel = CancellationToken::new();
        let workdir = crate::refusing::nowhere();
        for remote in [Some("origin"), None] {
            fetch(&exec, &workdir, remote, Duration::from_secs(1), &cancel)
                .await
                .expect_err("there is no git here to fetch with");
        }
        assert_eq!(
            asked.displays(),
            ["git fetch --prune -- origin", "git fetch --prune --all"]
        );
    }
}
