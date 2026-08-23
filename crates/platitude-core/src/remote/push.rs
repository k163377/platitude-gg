//! Where a push goes, how hard it may overwrite, and what a refusal
//! means.
//!
//! The half of the same question that runs no git — what a push could do
//! before one is sent — is [`super::standing`].

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

use super::list::{config_value, current_branch};
use super::marks::{branch_push_remote, push_default};

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
/// to the fork (実測 2.55).
///
/// The branch is pushed under **its own name** wherever it is not going to
/// the remote it tracks: an upstream names a branch on one remote and says
/// nothing about any other (実測: the refspec git builds for a triangular
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
    // it now, and the mark can be moved from a terminal between two of this
    // application's reads. Two short local `git config` calls in front of a
    // command that reaches the network.
    let pushes_to = match branch_push_remote(executor, workdir, &branch, cancel).await? {
        Some(name) => Some(name),
        None => push_default(executor, workdir, cancel)
            .await?
            .map(|marked| marked.remote),
    };

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
    // from (実測 — it is the whole of what a mark on a second remote is
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
/// [`GitError::PushOutdated`] rather than a plain failure, so the caller can
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
    let stderr = out.failure_message();
    if is_outdated(&out.stdout_utf8()) {
        return Err(GitError::PushOutdated {
            command,
            code: out.code,
            stderr,
        });
    }
    Err(GitError::Failed {
        command,
        code: out.code,
        stderr,
    })
}

/// Whether a `--porcelain` push result refused a ref for knowing the remote
/// only as it used to be.
///
/// The lines are `<flag>\t<from>:<to>\t<summary>`, where `!` is a refusal.
/// Three summaries say the same thing: a plain push found commits it would
/// drop (`fetch first`, or `non-fast-forward` for a ref that is not the
/// current branch's upstream), or a lease was pinned to a commit the remote
/// has since left (`stale info`). All three are answered by fetching.
///
/// Anything else — a hook, a protected branch, an unreachable host — is a
/// refusal to pass on as it is. A host that could not be reached at all
/// prints no ref lines, so it cannot be mistaken for one of these.
fn is_outdated(porcelain: &str) -> bool {
    porcelain.lines().any(|line| {
        let mut fields = line.split('\t');
        fields.next() == Some("!")
            && fields.nth(1).is_some_and(|summary| {
                summary.contains("(fetch first)")
                    || summary.contains("(stale info)")
                    || summary.contains("(non-fast-forward)")
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded from git 2.51 pushing to a local bare repository (the
    /// refusals from a second clone pushing first, from a lease pinned to
    /// what it had left, and from a `pre-receive` hook exiting non-zero).
    /// Every line here is `--porcelain` output as git wrote it.
    const REFUSED_FETCH_FIRST: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[rejected] (fetch first)\nDone\n";
    const REFUSED_STALE_LEASE: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[rejected] (stale info)\nDone\n";
    const FORCED_UPDATE: &str = "To C:/tmp/remote.git\n\
         +\trefs/heads/main:refs/heads/main\t34158f1...6e03ce6 (forced update)\nDone\n";
    const UP_TO_DATE: &str = "To C:/tmp/remote.git\n\
         =\trefs/heads/main:refs/heads/main\t[up to date]\nDone\n";
    const NEW_BRANCH: &str = "To C:/tmp/remote.git\n\
         *\trefs/heads/side:refs/heads/side\t[new branch]\nDone\n";

    #[test]
    fn a_refusal_a_fetch_would_answer_is_recognised() {
        assert!(is_outdated(REFUSED_FETCH_FIRST));
        assert!(is_outdated(REFUSED_STALE_LEASE));
    }

    #[test]
    fn pushes_that_landed_are_not_refusals() {
        assert!(!is_outdated(FORCED_UPDATE));
        assert!(!is_outdated(UP_TO_DATE));
        assert!(!is_outdated(NEW_BRANCH));
    }

    /// A refusal the remote decided on its own terms. Fetching tells us
    /// nothing about it, so it must not be dressed up as something to
    /// retry — and neither must a host that never answered, which prints
    /// no ref lines at all.
    #[test]
    fn refusals_a_fetch_cannot_help_with_are_left_alone() {
        assert!(!is_outdated(
            "To C:/tmp/remote.git\n\
             !\trefs/heads/main:refs/heads/main\t[remote rejected] (pre-receive hook declined)\nDone\n"
        ));
        assert!(!is_outdated(""));
    }
}
