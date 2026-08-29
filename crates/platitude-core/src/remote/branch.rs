//! Branches on a remote: what one holds now, and removing or
//! renaming one.

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
/// Asked before a first push, because git answers two different ways to a
/// name that is already over there: it fast-forwards one that our history
/// contains — silently advancing somebody else's branch — and refuses one
/// it does not (実測, both). The commit is returned rather than a yes, so
/// the caller can tell those two apart before anything is sent.
///
/// **The pattern has to be the full `refs/heads/<name>`.** `ls-remote`
/// matches a bare name against the *tail* of a ref, so asking for `topic`
/// answers yes when the remote only has `feature/topic` (実測).
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
        .timeout(timeout);
    let out = executor.run(cmd, cancel).await?;
    // A remote that has nothing to say answers with an empty stdout and
    // exit 0, so the absence is in the output rather than in the code.
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
    /// It is there and this history contains it, so the push lands and
    /// moves it on.
    FastForward,
    /// It is there with commits this history does not have. **git refuses
    /// this push** (実測), so nothing can happen by pressing.
    Refused,
    /// It is there, and what it holds cannot be read from here — the
    /// commit it names is not in this repository, so the two histories
    /// cannot be compared without fetching it first.
    Unknown,
    /// The remote never answered: a URL typed wrong, credentials that are
    /// not there, no network. Silence is not "nothing is there".
    Unreachable,
}

impl RemoteBranchState {
    /// The wire name the UI reads. Spelled out rather than derived so the
    /// two ends cannot drift apart on a rename.
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
/// Read the same way an ordinary push is (`--porcelain`, then
/// [`super::refusal::refusal`]): the far side is the only thing standing between a
/// branch and its deletion, and a name it keeps for a rule of its own —
/// a protected branch, a hook — is a report to pass on rather than a
/// failure of ours (デザイン規約 §リモートブランチを消す).
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
        .timeout(timeout);
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    Err(super::refusal::refusal(command, &out, remote, branch, true))
}

/// Renames a branch on a remote: the composition git has no command for.
///
/// The new name is pushed **from the remote-tracking ref**, not from a
/// local branch of the same name: the question asked was about a name, and
/// a local branch that has moved on since would publish its commits as
/// well. Then the old name goes, and any local branch that tracked it is
/// pointed at the new one — `push --delete` prunes the tracking ref an
/// upstream setting names, and a stale one sends the next push straight
/// back to the name just deleted.
///
/// This is not the rename a forge offers: the far side sees a branch
/// created and a branch deleted, so whatever hung off the old name — an
/// open pull request, a protected-branch rule — does not follow it. The UI
/// warns about this before it runs.
pub async fn rename_remote_branch(
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
    let moved = format!("refs/remotes/{remote}/{to}");
    for branch in tracking_branches(executor, workdir, remote, from, cancel).await? {
        crate::branch::set_upstream(executor, workdir, &branch, &moved, cancel).await?;
    }
    Ok(())
}

/// Local branches configured to track `<remote>/<branch>`.
///
/// One read of the whole `branch.` section rather than a lookup per branch:
/// which local branches point at a remote one is not something git answers
/// directly, and the section is small.
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
        // What a branch tracks is the value; a key written without one
        // names nothing to compare against.
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
