//! Where a push goes and how hard it may overwrite.
//!
//! The half of the same question that runs no git — what a push could do
//! before one is sent — is [`super::standing`]; what a non-zero one *was*
//! is [`super::refusal`].

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

use super::list::{config_value, current_branch};
use super::marks::push_marks;
use super::refusal::refusal;

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

/// Works out where the branch that is checked out should be pushed.
///
/// **Where a push goes is not where a branch fetches from.** git decides it
/// in this order (git-config(5)): the branch's own `pushRemote`, then the
/// repository's `remote.pushDefault`, then whatever the branch tracks —
/// and only if none of those is set does `fallback_remote` come into it.
/// Reading just the last of the three sends a fork workflow's pushes to the
/// repository it forked from, while the same `git push` in a terminal goes
/// to the fork (measured, 2.55).
///
/// The branch is pushed under **its own name** wherever it is not going to
/// the remote it tracks: an upstream names a branch on one remote and says
/// nothing about any other (measured: the refspec git builds for a triangular
/// push is `<branch>:<branch>`).
///
/// A detached HEAD has no branch to push, and says so rather than guessing.
///
/// The upstream is read from configuration rather than parsed out of
/// `origin/main`: a remote may be named `my/fork`, and a branch name may
/// contain slashes, so splitting that string cannot be done reliably.
pub async fn plan_current_push(
    executor: &GitExecutor,
    workdir: &Path,
    fallback_remote: &str,
    force: PushForce,
    cancel: &CancellationToken,
) -> Result<PushSpec, GitError> {
    let branch = current_branch(executor, workdir, cancel).await?;
    let tracks = config_value(
        executor,
        workdir,
        &format!("branch.{branch}.remote"),
        cancel,
    )
    .await?;
    let merge = config_value(executor, workdir, &format!("branch.{branch}.merge"), cancel).await?;

    // Read here rather than handed in: a push must go where git would send
    // it now, and the marks can be moved from a terminal between two of
    // this application's reads. One short local `git config` in front of a
    // command that reaches the network — the same read the status tick
    // makes, so the two cannot drift apart.
    let marks = push_marks(executor, workdir, &branch, cancel).await?;
    let pushes_to = marks
        .push_remote
        .or_else(|| marks.push_default.map(|marked| marked.remote));

    let remote = match pushes_to.or_else(|| tracks.clone()) {
        Some(remote) => remote,
        None if fallback_remote.is_empty() => {
            return Err(GitError::UnexpectedOutput {
                command: "git push".to_string(),
                message: "this repository has no remote to push to".to_string(),
            });
        }
        None => fallback_remote.to_string(),
    };

    let remote_branch = match (&tracks, &merge) {
        (Some(tracks), Some(merge)) if *tracks == remote => merge
            .strip_prefix("refs/heads/")
            .unwrap_or(merge)
            .to_string(),
        _ => branch.clone(),
    };

    // Recorded only where the branch tracks nothing at all, which is what
    // makes the *next* push need no decision. A branch that already tracks
    // something keeps tracking it: `--set-upstream` to another remote
    // rewrites `branch.<name>.remote`, and that is where the branch fetches
    // from (measured — it is the whole of what a mark on a second remote is
    // for).
    let set_upstream = tracks.is_none() || merge.is_none();

    Ok(PushSpec {
        remote,
        local: branch,
        remote_branch,
        set_upstream,
        force,
    })
}

/// Where the branch that is checked out should go when the user has just
/// said so, rather than when configuration already knows.
///
/// `expect` is the commit the question showed as being over there. Empty
/// sends the push fast-forward only; a commit turns it into the same
/// leased overwrite the toolbar offers a diverged branch — and the lease
/// is pinned to what was on screen, so a remote that moved since is
/// refused rather than flattened (§相手の履歴を置き換える).
pub async fn plan_publish(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    remote_branch: &str,
    expect: &str,
    cancel: &CancellationToken,
) -> Result<PushSpec, GitError> {
    let branch = current_branch(executor, workdir, cancel).await?;
    Ok(PushSpec {
        remote: remote.to_string(),
        remote_branch: remote_branch.to_string(),
        local: branch,
        set_upstream: true,
        force: if expect.is_empty() {
            PushForce::None
        } else {
            PushForce::WithLease {
                expect: Some(expect.to_string()),
            }
        },
    })
}

/// `git push` for one branch.
///
/// A refusal that a fetch would answer comes back as
/// [`ReportKind::Outdated`] rather than a plain failure, so the caller can
/// go and find out what the remote actually holds.
pub async fn push(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &PushSpec,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // `--porcelain` puts a fixed per-ref result on stdout. Which kind of
    // refusal this was has to be read from somewhere, and the sentence that
    // says so on stderr is prose written for a terminal.
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["push", "--porcelain"])
        .timeout(timeout);
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

    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    Err(refusal(
        command,
        &out,
        &spec.remote,
        &spec.remote_branch,
        false,
    ))
}
