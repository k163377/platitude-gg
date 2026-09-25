//! Branches on a remote: what one holds now, and removing or
//! replacing one.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

use super::push::{PushForce, PushSpec, push};
use super::tags::split_ls_remote_line;

/// What a remote carries under this exact branch name, if anything.
///
/// Asked before a first push: git silently fast-forwards a same-named
/// branch our history contains and refuses one it does not, and the
/// returned commit lets the caller tell the two apart before sending.
///
/// The pattern must be the full `refs/heads/<name>`: `ls-remote` matches a
/// bare name against a ref's tail, so `topic` would hit `feature/topic`.
pub async fn branch_tip(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    branch: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    let refname = format!("refs/heads/{branch}");
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-remote", "--heads", "--end-of-options", remote, &refname])
        .timeout(timeout)
        .paced_elsewhere();
    let out = executor.run(cmd, cancel).await?;
    // Absence is an empty stdout with exit 0.
    Ok(out
        .stdout
        .split(|b| *b == b'\n')
        .filter_map(split_ls_remote_line)
        .find(|(_, name)| *name == refname)
        .map(|(oid, _)| oid))
}

/// What a first push under a given name would meet on the far side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteBranchState {
    /// Nothing is there under that name: the push makes the branch.
    Free,
    /// It is there and this history contains it: the push moves it on.
    FastForward,
    /// It is there with commits this history lacks: git refuses the push.
    Refused,
    /// It is there, but its commit is not in this repository, so the
    /// histories cannot be compared without a fetch.
    Unknown,
    /// The remote never answered (bad URL, no credentials, no network).
    /// Silence is not "nothing is there".
    Unreachable,
}

impl RemoteBranchState {
    /// The wire name the UI reads; spelled out so a variant rename cannot
    /// change it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::FastForward => "fast-forward",
            Self::Refused => "refused",
            Self::Unknown => "unknown",
            Self::Unreachable => "unreachable",
        }
    }
}

/// `git push <remote> --delete <branch>`: removes a branch on the remote.
/// Destructive — the caller confirms first.
///
/// Read like an ordinary push (`--porcelain`, then
/// [`super::refusal::refusal`]): a far-side refusal (protected branch,
/// hook) is a report to pass on (デザイン規約 §リモートブランチを消す).
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
        .args(["push", "--porcelain", "--delete", "--", remote, branch])
        .timeout(timeout)
        .paced_elsewhere();
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    Err(super::refusal::refusal(command, &out, remote, branch, true))
}

/// Replaces a branch on a remote with one under a new name — not a
/// rename: the far side sees a create and a delete, and whatever hung off
/// the old name (an open PR, a protection rule) stays behind.
///
/// The new name is pushed from the remote-tracking ref, since a moved-on
/// local branch of the same name would publish its commits too. Local
/// branches tracking the old name are then repointed: a stale upstream
/// would send the next push back to the deleted name.
pub async fn replace_remote_branch(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    from: &str,
    to: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let spec = PushSpec {
        remote: remote.to_string(),
        local: format!("refs/remotes/{remote}/{from}"),
        remote_branch: to.to_string(),
        set_upstream: false,
        force: PushForce::None,
    };
    push(executor, workdir, &spec, timeout, cancel).await?;
    delete_remote_branch(executor, workdir, remote, from, timeout, cancel).await?;
    for branch in tracking_branches(executor, workdir, remote, from, cancel).await? {
        crate::branch::set_upstream(executor, workdir, &branch, remote, to, cancel).await?;
    }
    Ok(())
}

/// Local branches configured to track `<remote>/<branch>`.
///
/// One read of the whole `branch.` section: git has no direct query for
/// it, and the section is small.
async fn tracking_branches(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let out = config::get_regexp(
        executor,
        workdir,
        r"^branch\.",
        "git config --get-regexp branch",
        cancel,
    )
    .await?;
    Ok(parse_tracking(&out, remote, branch))
}

/// Picks the branches whose `remote` and `merge` both name the same remote
/// branch out of `git config -z --get-regexp ^branch\.` output.
fn parse_tracking(bytes: &[u8], remote: &str, branch: &str) -> Vec<String> {
    let merge_ref = format!("refs/heads/{branch}");
    let mut remotes: Vec<(String, String)> = Vec::new();
    let mut merges: Vec<(String, String)> = Vec::new();
    for record in config::parse_z_records(bytes) {
        let Some(value) = record.value() else {
            continue;
        };
        // `branch.<name>.<field>` — a branch name may contain dots, so the
        // field is the last segment and everything between is the name.
        let Some(rest) = record.key().strip_prefix("branch.") else {
            continue;
        };
        let Some((name, field)) = rest.rsplit_once('.') else {
            continue;
        };
        match field {
            "remote" => remotes.push((name.to_string(), value.to_string())),
            "merge" => merges.push((name.to_string(), value.to_string())),
            _ => {}
        }
    }
    remotes
        .into_iter()
        .filter(|(_, value)| value == remote)
        .map(|(name, _)| name)
        .filter(|name| {
            merges
                .iter()
                .any(|(other, value)| other == name && *value == merge_ref)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::z;

    #[test]
    fn tracking_needs_both_halves_to_agree() {
        let bytes = z(&[
            "branch.billing.remote\norigin",
            "branch.billing.merge\nrefs/heads/billing",
            // Same name over there, but on another remote.
            "branch.mirror.remote\nupstream",
            "branch.mirror.merge\nrefs/heads/billing",
            // Same remote, another branch.
            "branch.main.remote\norigin",
            "branch.main.merge\nrefs/heads/main",
            // Half a setting is no setting.
            "branch.orphan.remote\norigin",
        ]);
        assert_eq!(parse_tracking(&bytes, "origin", "billing"), ["billing"]);
    }

    #[test]
    fn a_branch_name_with_dots_keeps_its_name() {
        let bytes = z(&[
            "branch.release.8.4.remote\norigin",
            "branch.release.8.4.merge\nrefs/heads/release.8.4",
        ]);
        assert_eq!(
            parse_tracking(&bytes, "origin", "release.8.4"),
            ["release.8.4"]
        );
    }
}
