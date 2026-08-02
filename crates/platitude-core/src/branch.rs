//! Checkout and local branch management.
//!
//! Uses `switch` rather than `checkout` throughout. `checkout` doubles as a
//! file-restoring command, so a branch whose name collides with a path is
//! ambiguous; `switch` only ever moves HEAD and says so in its errors. The
//! same split is why unstaging uses `restore` (see [`crate::stage`]).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// What a checkout should land on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutTarget {
    /// An existing local branch.
    Branch { name: String },
    /// Any commit-ish (commit, tag, remote-tracking ref): detaches HEAD.
    /// Detaching is always explicit — never a side effect of a plain name.
    Detach { rev: String },
    /// A remote-tracking branch: creates `local` tracking it and switches.
    Track { remote_ref: String, local: String },
}

/// Moves HEAD to `target`.
pub async fn checkout(
    executor: &GitExecutor,
    workdir: &Path,
    target: &CheckoutTarget,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir).arg("switch");
    let cmd = match target {
        CheckoutTarget::Branch { name } => cmd.args(["--", name.as_str()]),
        CheckoutTarget::Detach { rev } => cmd.args(["--detach", "--", rev.as_str()]),
        CheckoutTarget::Track { remote_ref, local } => {
            cmd.args(["--create", local, "--track", remote_ref])
        }
    };
    executor.run(cmd, cancel).await.map(drop)
}

/// Creates a branch at `start_point` (HEAD when `None`), optionally
/// switching to it.
pub async fn create(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    start_point: Option<&str>,
    switch_to: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir);
    let cmd = if switch_to {
        cmd.args(["switch", "--create", name])
    } else {
        cmd.args(["branch", "--", name])
    };
    let cmd = match start_point {
        Some(start) => cmd.arg(start),
        None => cmd,
    };
    executor.run(cmd, cancel).await.map(drop)
}

/// Deletes a local branch. `force` maps to `-D` (drops unmerged work);
/// without it git refuses to delete an unmerged branch itself.
pub async fn delete(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    force: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let flag = if force { "-D" } else { "-d" };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["branch", flag, "--", name]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Renames a local branch. `force` allows overwriting an existing name.
pub async fn rename(
    executor: &GitExecutor,
    workdir: &Path,
    from: &str,
    to: &str,
    force: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let flag = if force { "-M" } else { "-m" };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["branch", flag, "--", from, to]);
    executor.run(cmd, cancel).await.map(drop)
}

/// True when every commit of `rev` is already reachable from `into`.
///
/// This is what makes deleting a branch safe; the UI asks before offering
/// the forced delete.
pub async fn is_merged_into(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    into: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["merge-base", "--is-ancestor", rev, into]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: format!("git merge-base --is-ancestor {rev} {into}"),
            code,
            stderr: out.stderr_utf8().trim().to_string(),
        }),
    }
}
