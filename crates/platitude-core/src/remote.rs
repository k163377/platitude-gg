//! Remotes: listing, fetch and push.
//!
//! This is the only place the application causes network traffic, and it
//! causes it the same way a terminal would — by running git. Authentication
//! is git's (`GIT_TERMINAL_PROMPT=0` keeps a missing credential helper from
//! hanging the process instead of failing).
//!
//! Network commands take a timeout rather than running unbounded: a wedged
//! connection must not leave a subprocess running forever. Cancellation is
//! the normal way to stop one early; the timeout is the backstop for a
//! connection that neither finishes nor fails.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Default time budget for commands that talk to a remote.
///
/// Three minutes covers an ordinary fetch of a large repository over a slow
/// link without leaving a hung connection running for an hour. A user on a
/// genuinely slow line can raise it (the setting is persisted in Phase 4).
pub const DEFAULT_NETWORK_TIMEOUT: Duration = Duration::from_secs(180);

/// A configured remote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    /// `remote.<name>.pushurl` when set, otherwise the fetch URL.
    pub push_url: String,
}

/// Lists configured remotes.
///
/// Reads config keys rather than `git remote -v`, whose two-lines-per-remote
/// shape is meant for humans.
pub async fn list(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<Remote>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "config",
        "-z",
        "--get-regexp",
        r"^remote\..*\.(url|pushurl)$",
    ]);
    // Exit code 1 just means "no matching keys" (a repository with no
    // remotes), which is not a failure.
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 1 {
        return Ok(Vec::new());
    }
    if out.code != 0 {
        return Err(GitError::Failed {
            command: "git config --get-regexp remote".to_string(),
            code: out.code,
            stderr: out.failure_message(),
        });
    }
    Ok(parse_remote_config(&out.stdout))
}

/// Parses `git config -z --get-regexp`: `key\nvalue` records, NUL-terminated.
fn parse_remote_config(bytes: &[u8]) -> Vec<Remote> {
    let mut remotes: Vec<Remote> = Vec::new();
    for record in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        let Some((key, value)) = text.split_once('\n') else {
            continue;
        };
        // `remote.<name>.url` — the name itself may contain dots, so take
        // the first and last segments and treat the middle as the name.
        let Some(rest) = key.strip_prefix("remote.") else {
            continue;
        };
        let Some((name, field)) = rest.rsplit_once('.') else {
            continue;
        };
        let entry = match remotes.iter_mut().find(|r| r.name == name) {
            Some(existing) => existing,
            None => {
                remotes.push(Remote {
                    name: name.to_string(),
                    ..Default::default()
                });
                // The push above guarantees a last element.
                match remotes.last_mut() {
                    Some(e) => e,
                    None => continue,
                }
            }
        };
        match field {
            "url" => {
                entry.fetch_url = value.to_string();
                if entry.push_url.is_empty() {
                    entry.push_url = value.to_string();
                }
            }
            "pushurl" => entry.push_url = value.to_string(),
            _ => {}
        }
    }
    remotes.sort_by(|a, b| a.name.cmp(&b.name));
    remotes
}

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

/// How hard a push may overwrite the remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushForce {
    /// Fast-forward only; git refuses anything else.
    None,
    /// `--force-with-lease`. `expect` pins the remote commit the user
    /// actually saw. Without it the lease is checked against the local
    /// remote-tracking ref, which a background fetch can advance behind the
    /// user's back — turning the safety net into a plain force.
    WithLease { expect: Option<String> },
    /// `--force`: unconditional. The UI confirms before choosing this.
    Force,
}

/// What to push where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSpec {
    pub remote: String,
    /// Local ref to push (a branch name, or `HEAD`).
    pub local: String,
    /// Branch name on the remote side.
    pub remote_branch: String,
    /// Record the pushed branch as upstream (`--set-upstream`).
    pub set_upstream: bool,
    pub force: PushForce,
}

/// `git push` for one branch.
pub async fn push(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &PushSpec,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).arg("push").timeout(timeout);
    if spec.set_upstream {
        cmd = cmd.arg("--set-upstream");
    }
    match &spec.force {
        PushForce::None => {}
        PushForce::WithLease { expect: None } => cmd = cmd.arg("--force-with-lease"),
        PushForce::WithLease { expect: Some(oid) } => {
            cmd = cmd.arg(format!("--force-with-lease={}:{oid}", spec.remote_branch));
        }
        PushForce::Force => cmd = cmd.arg("--force"),
    }
    let refspec = format!("{}:refs/heads/{}", spec.local, spec.remote_branch);
    cmd = cmd.args(["--", &spec.remote, &refspec]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git push <remote> --delete <branch>`: removes a branch on the remote.
/// Destructive — the caller confirms first.
pub async fn delete_remote_branch(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    branch: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["push", "--delete", "--", remote, branch])
        .timeout(timeout);
    executor.run(cmd, cancel).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(records: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for r in records {
            v.extend_from_slice(r.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_urls_and_push_urls() {
        let bytes = z(&[
            "remote.origin.url\nhttps://example.invalid/a.git",
            "remote.origin.pushurl\nssh://example.invalid/a.git",
            "remote.upstream.url\nhttps://example.invalid/b.git",
        ]);
        let remotes = parse_remote_config(&bytes);
        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(remotes[0].fetch_url, "https://example.invalid/a.git");
        assert_eq!(remotes[0].push_url, "ssh://example.invalid/a.git");
        assert_eq!(remotes[1].name, "upstream");
        assert_eq!(
            remotes[1].push_url, remotes[1].fetch_url,
            "push falls back to the fetch URL"
        );
    }

    #[test]
    fn remote_names_may_contain_dots() {
        let bytes = z(&["remote.my.fork.url\nfile:///tmp/x"]);
        let remotes = parse_remote_config(&bytes);
        assert_eq!(remotes[0].name, "my.fork");
    }

    #[test]
    fn empty_config_yields_no_remotes() {
        assert!(parse_remote_config(b"").is_empty());
    }
}
