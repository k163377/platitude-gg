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
use super::marks::push_marks;

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
    Err(refusal(
        command,
        &out,
        &spec.remote,
        &spec.remote_branch,
        false,
    ))
}

/// A refusal the far side made on its own terms, and what it said for
/// itself.
///
/// Nothing here can be put right from this end: the branch is protected,
/// a rule stands over it, a `pre-receive` hook turned the push away. The
/// two ends of the same push — `fetch first` and this — are told apart
/// because only one of them has a next move
/// ([`GitError::PushOutdated`]), and this one is a report rather than a
/// failure of the application's (デザイン規約 §可否・警告の出し場所).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRefusal {
    /// The remote as the user named it (`origin`).
    pub remote: String,
    /// The branch on the far side the refusal is about.
    pub branch: String,
    /// Whether what was asked for was that branch's removal.
    pub deleting: bool,
    /// **The far side's own words**, with git's `remote:` framing taken
    /// off. Written by whoever runs the server, so it is carried across
    /// rather than interpreted: the screen quotes it under a sentence of
    /// its own (`Words.remoteRefused`).
    pub reason: String,
}

/// Which of the three a non-zero push is: outdated, turned down over
/// there, or a plain failure.
pub(super) fn refusal(
    command: String,
    out: &crate::process::GitOutput,
    remote: &str,
    branch: &str,
    deleting: bool,
) -> GitError {
    let stderr = out.failure_message();
    let porcelain = out.stdout_utf8();
    if is_outdated(&porcelain) {
        return GitError::PushOutdated {
            command,
            code: out.code,
            stderr,
        };
    }
    let Some(reason) = refused_reason(&porcelain, &out.stderr_utf8()) else {
        return GitError::Failed {
            command,
            code: out.code,
            stderr,
        };
    };
    GitError::RemoteRefused {
        command,
        code: out.code,
        stderr,
        refusal: Box::new(RemoteRefusal {
            remote: remote.to_string(),
            branch: branch.to_string(),
            deleting,
            reason,
        }),
    }
}

/// Whether the far side is the one that said no, and how it explained
/// itself.
///
/// **`[remote rejected]` is git's own separation** and the whole of what
/// this reads: `[rejected]` is a refusal this end worked out from what it
/// holds, and every one of those has a next move here. A host that could
/// not be reached prints no ref lines at all, so silence answers `None`.
///
/// The explanation is the far side's, never ours: its `remote:` lines
/// where it wrote any, and git's own parenthetical where it wrote none
/// (a bare `receive-pack` refusing a deletion says nothing else).
fn refused_reason(porcelain: &str, stderr: &str) -> Option<String> {
    let mut summary = None;
    for line in porcelain.lines() {
        let mut fields = line.split('\t');
        if fields.next() != Some("!") {
            continue;
        }
        summary = fields.nth(1);
        if summary.is_some_and(|said| said.contains("[remote rejected]")) {
            break;
        }
        summary = None;
    }
    let summary = summary?;
    let words = remote_words(stderr);
    Some(if words.is_empty() {
        bracket_reason(summary)
    } else {
        words
    })
}

/// What the far side said for itself, out of the lines git copies to
/// stderr under `remote:`.
///
/// The `error:` some servers put in front of every line goes with the
/// framing: what is left is read under a sentence that has already said
/// what did not happen, and a second word for "this went wrong" there
/// only makes a report look like a fault of the application's.
///
/// **The lines are joined into one.** What reads them is a report with a
/// heading of its own, and a report is a heading and one line under it
/// (デザイン規約 §長さ) — so they run on as the sentences they are, and
/// what does not fit is read in the log with the command it came from.
/// Which of them carries the rule is not something this end can know: a
/// forge writes the summary first and the rule it broke after it, a hook
/// writes whatever its author wrote.
fn remote_words(stderr: &str) -> String {
    let mut said: Vec<&str> = Vec::new();
    for line in stderr.lines() {
        let Some(rest) = line.trim_end().strip_prefix("remote:") else {
            continue;
        };
        let rest = rest.trim();
        let rest = rest
            .strip_prefix("error:")
            .or_else(|| rest.strip_prefix("ERROR:"))
            .unwrap_or(rest)
            .trim();
        if !rest.is_empty() {
            said.push(rest);
        }
    }
    said.join(" ")
}

/// git's own reason out of `[remote rejected] (deletion prohibited)` —
/// what is inside the brackets, or the whole summary where there are
/// none to read.
fn bracket_reason(summary: &str) -> String {
    let inner = summary
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')'))
        .map(|(inner, _)| inner.trim());
    match inner {
        Some(inner) if !inner.is_empty() => inner.to_string(),
        _ => summary.trim().to_string(),
    }
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
/// Anything else — a hook, a protected branch, an unreachable host — is
/// not something a fetch helps with; whether the far side decided it is
/// [`refused_reason`]'s question. A host that could not be reached at all
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
        assert!(!is_outdated(REFUSED_BY_THE_FAR_SIDE));
        assert!(!is_outdated(""));
    }

    /// The same run of git 2.51 against a bare repository whose
    /// `pre-receive` hook exits non-zero, and the words GitHub writes
    /// through it (実測 — the two `remote: error:` lines are exactly what
    /// a protected branch answers a deletion with).
    const REFUSED_BY_THE_FAR_SIDE: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[remote rejected] (pre-receive hook declined)\nDone\n";
    const FAR_SIDE_WORDS: &str = "remote: error: GH006: Protected branch update failed for \
         refs/heads/main.        \nremote: error: Cannot delete a protected branch        \n\
         remote: \nTo https://github.com/owner/repo.git\n \
         ! [remote rejected] main (protected branch hook declined)\n\
         error: failed to push some refs to 'https://github.com/owner/repo.git'\n";

    #[test]
    fn the_far_sides_own_words_are_what_a_refusal_carries() {
        assert_eq!(
            refused_reason(REFUSED_BY_THE_FAR_SIDE, FAR_SIDE_WORDS).as_deref(),
            Some(
                "GH006: Protected branch update failed for refs/heads/main. \
                 Cannot delete a protected branch"
            )
        );
    }

    /// A server that says nothing for itself still has to leave the
    /// screen something to read, and git's own parenthetical is it.
    #[test]
    fn a_silent_far_side_leaves_gits_parenthetical() {
        assert_eq!(
            refused_reason(REFUSED_BY_THE_FAR_SIDE, "To C:/tmp/remote.git\n").as_deref(),
            Some("pre-receive hook declined")
        );
    }

    /// The refusals this end worked out for itself are not the far side
    /// speaking, whatever else is on stderr — nor is a host that never
    /// answered at all.
    #[test]
    fn a_refusal_from_this_end_is_not_the_far_side_speaking() {
        assert_eq!(refused_reason(REFUSED_FETCH_FIRST, FAR_SIDE_WORDS), None);
        assert_eq!(refused_reason(REFUSED_STALE_LEASE, ""), None);
        assert_eq!(
            refused_reason("", "fatal: could not read from remote"),
            None
        );
        assert_eq!(refused_reason(NEW_BRANCH, ""), None);
    }
}
