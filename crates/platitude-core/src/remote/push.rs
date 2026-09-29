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
use super::standing::PushTrack;

/// How hard a push may overwrite the remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushForce {
    /// Fast-forward only; git refuses anything else.
    None,
    /// `--force-with-lease`. `expect` pins the remote commit the user saw;
    /// without it the lease checks the remote-tracking ref, which a
    /// background fetch can advance, turning it into a plain force.
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
/// git's order (git-config(5)): the branch's `pushRemote`, then
/// `remote.pushDefault`, then what the branch tracks, and only then
/// `fallback_remote`. Reading only the last sends a fork workflow's pushes
/// to the repository it forked from.
///
/// The branch goes under its own name wherever it is not going to the
/// remote it tracks, as git's triangular-push refspec does. A detached
/// HEAD is an error.
///
/// The upstream is read from configuration: `origin/main` cannot be split
/// reliably (remote names and branch names may both hold slashes).
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

    // Read fresh: the marks can move in a terminal between two of this
    // application's reads. The same read as the status tick, so the two
    // cannot drift apart.
    let marks = push_marks(executor, workdir, &branch, cancel).await?;

    decide_push(&branch, tracks, merge, marks, fallback_remote, force)
}

/// What the four reads above come to: the destination, the name the
/// branch goes under there, and whether this push records an upstream.
///
/// Pure so this module's tests can walk every arrangement of the three
/// keys; through git each would cost a clone and two bare repositories.
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

    // Recorded only where the branch tracks nothing. A branch that already
    // tracks something keeps it: `--set-upstream` to another remote
    // rewrites `branch.<name>.remote`, where the branch fetches from.
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
/// pushes fast-forward only; a commit makes it a lease pinned to what was
/// on screen, so a remote that moved since is refused
/// (デザイン規約 §相手の履歴を置き換える).
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
/// [`crate::ReportKind::Outdated`].
pub async fn push(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &PushSpec,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // `--porcelain` puts a fixed per-ref result on stdout; the refusal's
    // kind is not parseable from stderr's prose.
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

/// How `branch` stands against the tracking ref of where a mark sends
/// its push ([`super::pushes_elsewhere`]): one `for-each-ref`, git naming
/// the ref through that remote's fetch refspecs (`%(push)`) and counting
/// against it (`%(push:track)`).
///
/// Asked as `push.default=current`, the branch under its own name — the
/// refspec [`plan_current_push`] sends there. The default `simple` names
/// no destination that is not the upstream (`@{push}`: `cannot resolve
/// 'simple' push to a single destination`).
pub async fn push_track(
    executor: &GitExecutor,
    workdir: &Path,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<PushTrack, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "-c",
            "push.default=current",
            "for-each-ref",
            "--format=%(push)%00%(push:remoteref)%00%(push:track)",
        ])
        .arg(format!("refs/heads/{branch}"));
    let out = executor.run(cmd, cancel).await?;
    Ok(parse_push_track(branch, &out.stdout))
}

/// One [`push_track`] line: `<ref>\0<remote ref>\0<track>`. Every way of
/// having nothing to count against answers the empty [`PushTrack`].
fn parse_push_track(branch: &str, bytes: &[u8]) -> PushTrack {
    let line = bytes.split(|b| *b == b'\n').next().unwrap_or_default();
    let mut fields = line.split(|b| *b == 0);
    let (Some(tracking), Some(remote_ref), Some(track)) =
        (fields.next(), fields.next(), fields.next())
    else {
        return PushTrack::default();
    };
    // Set only by a `remote.<name>.push` refspec, which moves `%(push)`
    // where the send's own refspec does not go.
    if !remote_ref.is_empty() && remote_ref != format!("refs/heads/{branch}").as_bytes() {
        return PushTrack::default();
    }
    // Only a remote branch is on screen for a lease to pin to; `[gone]`
    // names a ref nothing has fetched.
    let Some(tracking) = std::str::from_utf8(tracking)
        .ok()
        .and_then(|name| name.strip_prefix("refs/remotes/"))
    else {
        return PushTrack::default();
    };
    if track == b"[gone]" {
        return PushTrack::default();
    }
    let (ahead, behind) = crate::refs::parse_track(track);
    PushTrack {
        tracking: tracking.to_string(),
        ahead: i32::try_from(ahead).unwrap_or(i32::MAX),
        behind: i32::try_from(behind).unwrap_or(i32::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::marks::PushDefault;
    use crate::remote::standing::push_target;

    const REMOTES: [&str; 2] = ["fork", "origin"];

    /// Empty stands for a key that is not set.
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

    /// An error rather than a refspec against an empty name.
    #[test]
    fn a_repository_with_no_remote_has_nowhere_to_send() {
        let err = plan("main", "", "", "", "", "").expect_err("nowhere to push");
        assert!(err.to_string().contains("no remote to push to"), "{err}");
        assert!(
            plan("main", "", "", "", "", "origin").is_ok(),
            "a fallback is a destination"
        );
    }

    /// git 2.55's `push_track` lines, one per arrangement.
    #[test]
    fn a_push_track_counts_only_against_a_remote_branch_the_send_reaches() {
        let track = |line: &[u8]| parse_push_track("main", line);
        assert_eq!(
            track(b"refs/remotes/fork/main\0\0[ahead 1, behind 1]\n"),
            PushTrack {
                tracking: "fork/main".to_string(),
                ahead: 1,
                behind: 1,
            }
        );
        assert_eq!(
            track(b"refs/remotes/fork/main\0\0\n"),
            PushTrack {
                tracking: "fork/main".to_string(),
                ahead: 0,
                behind: 0,
            },
            "level with it is a count too"
        );
        assert_eq!(
            track(b"refs/remotes/fork/main\0refs/heads/main\0[ahead 1]\n").tracking,
            "fork/main",
            "a push refspec that sends the branch under its own name"
        );
        for (line, why) in [
            (&b"refs/remotes/fork/main\0\0[gone]\n"[..], "never fetched"),
            (b"\0\0\n", "no fetch refspec takes the branch in"),
            (
                b"refs/remotes/fork/wip\0refs/heads/wip\0\n",
                "a push refspec that renames the branch: the send goes elsewhere",
            ),
            (
                b"refs/mirror/fork/main\0\0[ahead 1]\n",
                "not a remote branch: nothing on screen to lease against",
            ),
            (b"", "no such branch"),
        ] {
            assert_eq!(track(line), PushTrack::default(), "{why}");
        }
    }

    /// The toolbar label ([`push_target`], from the snapshot) and the send
    /// ([`decide_push`], from git's keys) spell one order twice, so this
    /// walks every arrangement of the three keys against both.
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
