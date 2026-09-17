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
use super::marks::{PushMarks, push_marks};
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
/// **Where a push goes is its own question.** git decides it in this
/// order (git-config(5)): the branch's own `pushRemote`, then the
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
/// A detached HEAD has no branch to push, and says so.
///
/// The upstream is read from configuration: a remote may be named
/// `my/fork`, and a branch name may contain slashes, so splitting
/// `origin/main` cannot be done reliably.
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

    // Read here: a push must go where git would send it now, and the
    // marks can be moved from a terminal between two of this
    // application's reads. One short local `git config` in front of a
    // command that reaches the network — the same read the status tick
    // makes, so the two cannot drift apart.
    let marks = push_marks(executor, workdir, &branch, cancel).await?;

    decide_push(&branch, tracks, merge, marks, fallback_remote, force)
}

/// What the four reads above come to, with nothing left to ask git: the
/// destination, the name the branch goes under there, and whether this
/// push is the one that records an upstream.
///
/// Pure, and asked of every arrangement of the three keys in this
/// module's own tests — the order is the half of [`plan_current_push`]
/// that has ever been wrong, and walking it through git costs a clone,
/// two bare repositories and a `config` write per arrangement.
fn decide_push(
    branch: &str,
    tracks: Option<String>,
    merge: Option<String>,
    marks: PushMarks,
    fallback_remote: &str,
    force: PushForce,
) -> Result<PushSpec, GitError> {
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
        _ => branch.to_string(),
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
        local: branch.to_string(),
        remote_branch,
        set_upstream,
        force,
    })
}

/// Where the branch that is checked out should go when the user has just
/// said so.
///
/// `expect` is the commit the question showed as being over there. Empty
/// sends the push fast-forward only; a commit turns it into the same
/// leased overwrite the toolbar offers a diverged branch — and the lease
/// is pinned to what was on screen, so a remote that moved since is
/// refused (§相手の履歴を置き換える).
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
/// [`ReportKind::Outdated`], so the caller can go and find out what the
/// remote actually holds.
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
        .timeout(timeout)
        .paced_elsewhere();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::marks::PushDefault;
    use crate::remote::standing::push_target;

    const REMOTES: [&str; 2] = ["fork", "origin"];

    /// The four reads, spelled the way git answers them: what the branch
    /// tracks, the ref it merges with, and the two marks. Empty stands
    /// for a key that is not set.
    fn decide(
        branch: &str,
        tracks: &str,
        merge: &str,
        push_remote: &str,
        push_default: &str,
    ) -> PushSpec {
        plan(branch, tracks, merge, push_remote, push_default, "origin").expect("a plan")
    }

    fn plan(
        branch: &str,
        tracks: &str,
        merge: &str,
        push_remote: &str,
        push_default: &str,
        fallback: &str,
    ) -> Result<PushSpec, GitError> {
        let some = |value: &str| (!value.is_empty()).then(|| value.to_string());
        decide_push(
            branch,
            some(tracks),
            some(merge),
            PushMarks {
                push_remote: some(push_remote),
                push_default: some(push_default).map(|remote| PushDefault {
                    remote,
                    local: true,
                }),
            },
            fallback,
            PushForce::None,
        )
    }

    /// git's own order, every step of it: the branch's own `pushRemote`
    /// beats the repository's `remote.pushDefault`, which beats whatever
    /// the branch tracks, and only with none of the three set does the
    /// caller's fallback come into it (git-config(5); measured 2.55).
    /// **Reading just the last of them** is how a fork workflow's pushes
    /// go to the repository it forked from.
    #[test]
    fn a_push_goes_where_git_would_send_it() {
        assert_eq!(
            decide("main", "origin", "refs/heads/main", "", "").remote,
            "origin"
        );
        assert_eq!(
            decide("main", "origin", "refs/heads/main", "", "fork").remote,
            "fork",
            "the repository's mark beats what the branch tracks"
        );
        assert_eq!(
            decide("main", "origin", "refs/heads/main", "fork", "").remote,
            "fork",
            "so does the branch's own"
        );
        assert_eq!(
            decide("main", "fork", "refs/heads/main", "origin", "fork").remote,
            "origin",
            "and the branch's own beats the repository's"
        );
        assert_eq!(
            decide("topic", "", "", "", "").remote,
            "origin",
            "nothing set anywhere falls back to the caller's remote"
        );
    }

    /// **The upstream names a branch on one remote and says nothing about
    /// any other.** Going to the remote it tracks keeps that name; going
    /// anywhere else it goes under the branch's own, which is the refspec
    /// git builds for a triangular push (measured).
    #[test]
    fn a_branch_keeps_its_upstreams_name_only_on_its_upstreams_remote() {
        let tracked = decide("topic", "origin", "refs/heads/elsewhere", "", "");
        assert_eq!(
            (tracked.remote.as_str(), tracked.remote_branch.as_str()),
            ("origin", "elsewhere")
        );

        let marked = decide("topic", "origin", "refs/heads/elsewhere", "", "fork");
        assert_eq!(
            (marked.remote.as_str(), marked.remote_branch.as_str()),
            ("fork", "topic"),
            "not the upstream's name"
        );
        // A merge ref that is not under refs/heads/ is taken as written.
        assert_eq!(
            decide("topic", "origin", "trunk", "", "").remote_branch,
            "trunk"
        );
    }

    /// Recorded only where the branch tracks nothing at all — a branch
    /// that already tracks something goes on tracking it, because
    /// `--set-upstream` to another remote rewrites where the branch
    /// *fetches* from (measured).
    #[test]
    fn only_a_branch_that_tracks_nothing_records_where_it_went() {
        assert!(decide("topic", "", "", "", "fork").set_upstream);
        assert!(
            decide("topic", "origin", "", "", "").set_upstream,
            "half a mark is no mark: the next push would have nothing to compare against"
        );
        assert!(!decide("main", "origin", "refs/heads/main", "", "").set_upstream);
        assert!(
            !decide("main", "origin", "refs/heads/main", "fork", "").set_upstream,
            "a branch that tracks origin must go on tracking it"
        );
    }

    /// A repository with no remote at all has nowhere to send anything,
    /// and says so rather than building a refspec against an empty name.
    #[test]
    fn a_repository_with_no_remote_has_nowhere_to_send() {
        let err = plan("main", "", "", "", "", "").expect_err("nowhere to push");
        assert!(err.to_string().contains("no remote to push to"), "{err}");
        assert!(
            plan("main", "", "", "", "", "origin").is_ok(),
            "a fallback is a destination"
        );
    }

    /// **The label and the send must name the same remote.** The toolbar
    /// says where a push is going from configuration the snapshot carries
    /// ([`push_target`]); the send works it out again from the keys git
    /// answers with. Two spellings of one order is how the first came to
    /// read `remote.pushDefault` while the second read the branch's own
    /// mark first — so this walks every arrangement of the three keys and
    /// holds the two answers against each other.
    #[test]
    fn the_label_names_the_remote_the_send_uses() {
        // (branch.main.pushRemote, remote.pushDefault, what the branch tracks)
        for (push_remote, push_default, tracks) in [
            ("", "", "origin"),
            ("", "fork", "origin"),
            ("fork", "", "origin"),
            ("fork", "origin", "origin"),
            ("origin", "fork", "origin"),
            ("origin", "", "origin"),
            ("", "", ""),
            ("", "fork", ""),
            ("fork", "", ""),
        ] {
            let merge = if tracks.is_empty() {
                ""
            } else {
                "refs/heads/main"
            };
            let spec = decide("main", tracks, merge, push_remote, push_default);
            let upstream = if tracks.is_empty() {
                String::new()
            } else {
                format!("{tracks}/main")
            };
            assert_eq!(
                push_target(
                    "main",
                    &upstream,
                    push_remote,
                    push_default,
                    "origin",
                    REMOTES
                ),
                format!("{}/{}", spec.remote, spec.remote_branch),
                "pushRemote={push_remote:?} pushDefault={push_default:?} tracks={tracks:?}: \
                 the label and the send disagree"
            );
        }
    }
}
