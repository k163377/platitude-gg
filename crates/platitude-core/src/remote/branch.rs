//! Branches on a remote: what one holds now, and removing or
//! replacing one.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

use super::push::{PushForce, PushSpec, porcelain_push, push};
use super::tags::split_ls_remote_line;

/// A branch on a remote: the remote as git is handed it, and the branch's
/// name over there — not the branch here it goes from or is measured for,
/// nor the remote-tracking ref that mirrors it. The remote may be `.`,
/// git's name for this repository: an upstream that is a branch here
/// (`branch.<name>.remote = .`).
///
/// Built from the two names as they were given apart, never cut out of
/// `origin/feature/x`: both may hold a slash. Nothing here checks them;
/// git judges the names, and the callers refuse what they refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBranch {
    pub remote: String,
    pub branch: String,
}

impl RemoteBranch {
    /// The branch's full name over there, `refs/heads/<branch>` — beside a
    /// tag of the same name the bare one is ambiguous.
    pub fn head_ref(&self) -> String {
        format!("refs/heads/{}", self.branch)
    }

    /// The remote-tracking ref here, `refs/remotes/<remote>/<branch>`: the
    /// name the default fetch refspec gives it, not one resolved through
    /// the remote's own `fetch` refspecs. `.` has none — no ref goes by
    /// `refs/remotes/./<branch>`.
    pub fn tracking_ref(&self) -> String {
        format!("refs/remotes/{self}")
    }
}

/// `<remote>/<branch>`, the short name its remote-tracking branch is
/// listed under (`origin/feature/x`) — written from the pair, never cut
/// back into one.
impl fmt::Display for RemoteBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.remote, self.branch)
    }
}

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
    target: &RemoteBranch,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    let refname = target.head_ref();
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "ls-remote",
            "--heads",
            "--end-of-options",
            &target.remote,
            &refname,
        ])
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
    /// Asking the remote failed — no answer (bad URL, no network) or an
    /// answer that refused the ask (no credentials); git's words say which.
    /// A failed ask is not "nothing is there".
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
/// `expect` is the commit the screen showed the branch on, and the delete
/// is leased to it: a remote that moved since is refused
/// (`[rejected] (stale info)`, read as outdated) rather than losing
/// commits this repository never had. The oid travels from the press —
/// the remote-tracking ref is no pin, a background fetch moves it
/// ([`super::PushForce`]).
///
/// The name goes qualified as `refs/heads/<branch>`, as a tag's does
/// ([`super::delete_remote_tag`]): a bare name is matched against every
/// ref over there, so one the remote already dropped fails before the
/// lease is weighed (`remote ref does not exist`) and one a tag shares
/// fails as ambiguous. Qualified, a dropped branch is the lease's refusal
/// like any other move.
///
/// Read like an ordinary push (`--porcelain`, then
/// [`super::refusal::refusal`]): a far-side refusal (protected branch,
/// hook) is a report to pass on (デザイン規約 §リモートブランチを消す).
///
/// What tracked it keeps its upstream here; [`untrack_deleted`] is the
/// step after.
pub async fn delete_remote_branch(
    executor: &GitExecutor,
    workdir: &Path,
    target: &RemoteBranch,
    expect: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let sent = super::refusal::Sent::branch(target, true);
    let lease = format!("--force-with-lease={}:{expect}", sent.refname);
    let cmd = porcelain_push(workdir, timeout).arg(lease).args([
        "--delete",
        "--",
        &target.remote,
        &sent.refname,
    ]);
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    Err(super::refusal::refusal(command, &out, &sent))
}

/// After [`delete_remote_branch`] went: takes the upstream off the local
/// branches that tracked `target` ([`crate::branch::unset_upstream`]), and
/// returns them.
///
/// An upstream left naming a deleted branch is what the `[gone]` warning
/// reads, and that warning is for a branch the far side dropped on its own
/// (merged and deleted there), so the now useless branch here is not
/// forgotten. A delete asked for here is no such case: the warning would
/// only nag about the reader's own act.
///
/// Judged by the push alone: the delete went where `target.remote` pushes,
/// and so does every branch tracking `target`. A `pushurl` naming
/// another repository than the fetch URL changes nothing — the reader
/// deleted the branch their pushes go to, and the next fetch reading the
/// name back from the other side is that configuration's own doing.
/// Another remote name writing to the same repository is not followed: its
/// branches keep the upstream (P3-確認事項 §リモートの別名).
///
/// Only settings in this repository's own config file, and only where
/// they are the upstream git goes by: one inherited from an included or
/// the global file is configuration kept elsewhere on purpose, left as it
/// is.
pub async fn untrack_deleted(
    executor: &GitExecutor,
    workdir: &Path,
    target: &RemoteBranch,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let out = config::get_regexp_at(
        executor,
        workdir,
        config::ConfigScope::Local,
        r"^branch\.",
        "git config --local --get-regexp branch",
        cancel,
    )
    .await?;
    let written_here = parse_tracking(&out, target);
    if written_here.is_empty() {
        return Ok(Vec::new());
    }
    // The file's own say is not the branch's: an include read after it, a
    // worktree's config, or a value the global file adds can make the
    // upstream another one, which stays.
    let in_effect = tracking_branches(executor, workdir, target, cancel).await?;
    let tracking: Vec<String> = written_here
        .into_iter()
        .filter(|name| in_effect.contains(name))
        .collect();
    for name in &tracking {
        crate::branch::unset_upstream(executor, workdir, name, cancel).await?;
    }
    Ok(tracking)
}

/// Replaces `old` with a branch named `to` on the same remote — not a
/// rename: the far side sees a create and a delete, and whatever hung off
/// the old name (an open PR, a protection rule) stays behind.
///
/// The new name is pushed from the remote-tracking ref, since a moved-on
/// local branch of the same name would publish its commits too. The old
/// name's delete is leased to `expect`, as in [`delete_remote_branch`]; a
/// refused lease leaves both names over there and the tracking alone.
/// Local branches tracking the old name are then repointed: a stale
/// upstream would send the next push back to the deleted name.
pub async fn replace_remote_branch(
    executor: &GitExecutor,
    workdir: &Path,
    old: &RemoteBranch,
    to: &str,
    expect: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let spec = PushSpec {
        target: RemoteBranch {
            remote: old.remote.clone(),
            branch: to.to_string(),
        },
        local: old.tracking_ref(),
        set_upstream: false,
        force: PushForce::None,
    };
    push(executor, workdir, &spec, timeout, cancel).await?;
    delete_remote_branch(executor, workdir, old, expect, timeout, cancel).await?;
    for branch in tracking_branches(executor, workdir, old, cancel).await? {
        crate::branch::set_upstream(executor, workdir, &branch, &spec.target, cancel).await?;
    }
    Ok(())
}

/// Local branches configured to track `target`.
///
/// One read of the whole `branch.` section: git has no direct query for
/// it, and the section is small.
async fn tracking_branches(
    executor: &GitExecutor,
    workdir: &Path,
    target: &RemoteBranch,
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
    Ok(parse_tracking(&out, target))
}

/// Picks the branches whose `remote` and `merge` both name `target` out of
/// `git config -z --get-regexp ^branch\.` output, each once, in name order.
///
/// Read as git reads the setting: the last `remote` wins, and `merge` is
/// multi-valued — a branch merging more than this one name (an octopus
/// `pull`) does not track it alone, and is not picked.
fn parse_tracking(bytes: &[u8], target: &RemoteBranch) -> Vec<String> {
    let merge_ref = target.head_ref();
    // Per branch: the last `remote` read, and every `merge`.
    let mut settings: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
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
        let (remotes, merges) = match field {
            "remote" | "merge" => settings.entry(name.to_string()).or_default(),
            _ => continue,
        };
        if field == "remote" {
            *remotes = value.to_string();
        } else {
            merges.push(value.to_string());
        }
    }
    settings
        .into_iter()
        .filter(|(_, (last, merges))| *last == target.remote && *merges == [merge_ref.as_str()])
        .map(|(name, _)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::z;

    fn far(remote: &str, branch: &str) -> RemoteBranch {
        RemoteBranch {
            remote: remote.to_string(),
            branch: branch.to_string(),
        }
    }

    /// Each name is spelled from the two halves as given, slashes and all.
    #[test]
    fn the_pair_spells_its_names_from_both_halves() {
        let pair = far("team/fork", "feature/x");
        assert_eq!(pair.head_ref(), "refs/heads/feature/x");
        assert_eq!(pair.tracking_ref(), "refs/remotes/team/fork/feature/x");
        assert_eq!(pair.to_string(), "team/fork/feature/x");
    }

    /// `team/fork` + `feature/x` and `team` + `fork/feature/x` both read
    /// `team/fork/feature/x` joined; only the pair tells them apart.
    #[test]
    fn tracking_reads_the_two_names_apart_where_both_hold_a_slash() {
        let bytes = z(&[
            "branch.mine.remote\nteam/fork",
            "branch.mine.merge\nrefs/heads/feature/x",
            "branch.lookalike.remote\nteam",
            "branch.lookalike.merge\nrefs/heads/fork/feature/x",
        ]);
        assert_eq!(
            parse_tracking(&bytes, &far("team/fork", "feature/x")),
            ["mine"]
        );
        assert_eq!(
            parse_tracking(&bytes, &far("team", "fork/feature/x")),
            ["lookalike"]
        );
    }

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
        assert_eq!(
            parse_tracking(&bytes, &far("origin", "billing")),
            ["billing"]
        );
    }

    #[test]
    fn a_branch_name_with_dots_keeps_its_name() {
        let bytes = z(&[
            "branch.release.8.4.remote\norigin",
            "branch.release.8.4.merge\nrefs/heads/release.8.4",
        ]);
        assert_eq!(
            parse_tracking(&bytes, &far("origin", "release.8.4")),
            ["release.8.4"]
        );
    }

    /// The setting as git goes by it: the last `remote` read, and a
    /// `merge` naming this branch alone. A name written twice is one
    /// branch.
    #[test]
    fn tracking_is_read_as_git_resolves_it() {
        let bytes = z(&[
            // A later file moved it to another remote.
            "branch.moved.remote\norigin",
            "branch.moved.merge\nrefs/heads/topic",
            "branch.moved.remote\nupstream",
            // An octopus pull merges topic and main.
            "branch.octopus.remote\norigin",
            "branch.octopus.merge\nrefs/heads/main",
            "branch.octopus.merge\nrefs/heads/topic",
            // Written twice, the same both times.
            "branch.twice.remote\norigin",
            "branch.twice.merge\nrefs/heads/topic",
            "branch.twice.remote\norigin",
        ]);
        assert_eq!(parse_tracking(&bytes, &far("origin", "topic")), ["twice"]);
    }
}
