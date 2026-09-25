//! What a remote carries under `refs/tags/`, asked of the remote
//! itself because nothing local records it.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// What a remote carries under `refs/tags/`: each tag name and the commit
/// it designates over there.
///
/// Nothing local records this (a fetched tag lands in `refs/tags/` beside
/// the ones made here), so it runs on the fetch path, where the user has
/// already agreed to pay for the network.
///
/// Do not add `--refs`: it drops the peeled `^{}` line and keeps the tag
/// object, which compares equal to no local commit.
pub async fn list_tags(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Vec<RemoteTag>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-remote", "--tags", "--", remote])
        .timeout(timeout)
        .paced_elsewhere();
    let out = executor.run(cmd, cancel).await?;
    Ok(parse_ls_remote_tags(&out.stdout))
}

/// Sends one tag to one remote.
///
/// `expect` makes it a leased overwrite pinned to the commit the remote was
/// last seen holding this tag on; empty sends it plain, which git refuses
/// where the name already stands on something else over there.
///
/// No refusal here is read as outdated or queues anything behind it: unlike
/// a branch, a tag is not put right by fetching (`--prune` keeps the local
/// tag, `--prune-tags` exits 1). A far-side rule (`[remote rejected]`)
/// reads as a report, as for a branch (`super::refusal::refused`,
/// デザイン規約 §答えの要らない報せ) — `--porcelain` is what makes it readable.
pub async fn push_tag(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    tag: &str,
    expect: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let refspec = format!("refs/tags/{tag}:refs/tags/{tag}");
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["push", "--porcelain"])
        .timeout(timeout)
        .paced_elsewhere();
    if !expect.is_empty() {
        cmd = cmd.arg(format!("--force-with-lease=refs/tags/{tag}:{expect}"));
    }
    cmd = cmd.args(["--", remote, &refspec]);
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    Err(super::refusal::refused(command, &out, remote, tag, false))
}

/// Takes one tag off one remote. Nothing here is touched.
///
/// The name must stay qualified as `refs/tags/<name>`: a bare name that is
/// also a branch over there is refused and neither is deleted. The
/// qualified form exits 0 on a name the remote lacks, so whether there is
/// anything to delete is the caller's to know (`offers::TagSides`).
/// A far-side refusal is read as in [`push_tag`], hence `--porcelain`.
pub async fn delete_remote_tag(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    tag: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "push",
            "--porcelain",
            "--delete",
            "--",
            remote,
            &format!("refs/tags/{tag}"),
        ])
        .timeout(timeout)
        .paced_elsewhere();
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    Err(super::refusal::refused(command, &out, remote, tag, true))
}

/// Replaces a tag on a remote with one under a new name — no git command
/// does this; the tag's counterpart to [`super::replace_remote_branch`].
///
/// The new name goes up from the local copy (a tag has no tracking ref),
/// which the local rename left on the old object ([`crate::tag::rename`]).
/// Whether the remote's copy stands on that same object is the caller's to
/// know: a drifted one would change object as well as name
/// (`offers::TagSides`, デザイン規約 §手元の改名の後のリモート).
///
/// The push goes first so a refused name leaves the old one standing.
/// Whatever hung off the old name over there (a release) stays behind on
/// it; the UI warns before this runs.
pub async fn replace_remote_tag(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    from: &str,
    to: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    push_tag(executor, workdir, remote, to, "", timeout, cancel).await?;
    delete_remote_tag(executor, workdir, remote, from, timeout, cancel).await
}

/// One tag as a remote advertises it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTag {
    pub name: crate::Name,
    /// The commit it designates, annotated tags peeled — comparable with
    /// [`crate::refs::RefEntry::commit_oid`].
    pub commit: Oid,
    /// True when the remote advertised a tag object to peel.
    pub annotated: bool,
}

/// Parses `git ls-remote --tags` (`<oid>\t<refname>` lines), sorted by name.
///
/// Lines outside `refs/tags/` or with an unreadable oid are skipped, so one
/// odd advertisement does not cost every other tag its badge.
pub fn parse_ls_remote_tags(bytes: &[u8]) -> Vec<RemoteTag> {
    let mut out: Vec<RemoteTag> = Vec::new();
    // Pairing by scanning `out` would be quadratic over tens of thousands of
    // tags.
    let mut by_name: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for line in bytes.split(|b| *b == b'\n') {
        let Some((oid, refname)) = split_ls_remote_line(line) else {
            continue;
        };
        let Some(name) = refname.strip_prefix("refs/tags/") else {
            continue;
        };
        let (name, peeled) = match name.strip_suffix("^{}") {
            Some(base) => (base, true),
            None => (name, false),
        };
        match by_name.get(name) {
            // A second line for a name is an annotated tag's pair. Only the
            // peeled line carries the commit, so it wins whichever comes first.
            Some(&seen) if !out[seen].annotated => {
                out[seen].commit = oid;
                out[seen].annotated = peeled;
            }
            Some(_) => {}
            None => {
                by_name.insert(name, out.len());
                out.push(RemoteTag {
                    name: name.into(),
                    commit: oid,
                    annotated: peeled,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub(super) fn split_ls_remote_line(line: &[u8]) -> Option<(Oid, &str)> {
    let tab = line.iter().position(|b| *b == b'\t')?;
    let oid = Oid::from_hex(&line[..tab]).ok()?;
    let refname = std::str::from_utf8(&line[tab + 1..]).ok()?.trim_end();
    Some((oid, refname))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Real `ls-remote --tags` stdout (git 2.51): one annotated tag
    /// (`ann-1`, advertised twice) and three lightweight ones.
    const LS_REMOTE_TAGS: &[u8] = b"\
a56decf8286d58887d4e0bf7bda73adb7a7ec957\trefs/tags/ann-1\n\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/tags/ann-1^{}\n\
b132edc248d8082a5019061b53fc2e3de31cc085\trefs/tags/drift\n\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/tags/light-1\n\
b132edc248d8082a5019061b53fc2e3de31cc085\trefs/tags/only-remote\n";

    #[test]
    fn an_annotated_tag_is_read_at_the_commit_it_peels_to() {
        let tags = parse_ls_remote_tags(LS_REMOTE_TAGS);
        assert_eq!(
            tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["ann-1", "drift", "light-1", "only-remote"],
            "the ^{{}} line is the same tag, not another one"
        );
        assert_eq!(
            tags[0].commit.to_hex(),
            "4b85e9d7b2bc03f629f3331705f5049e4522ce31",
            "the peeled commit, not the tag object"
        );
        assert!(tags[0].annotated);
        assert!(!tags[1].annotated, "a lightweight tag has nothing to peel");
    }

    #[test]
    fn the_peeled_line_wins_whichever_side_it_arrives_on() {
        // Nothing in the protocol promises the tag object comes first.
        let reversed = b"\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/tags/ann-1^{}\n\
a56decf8286d58887d4e0bf7bda73adb7a7ec957\trefs/tags/ann-1\n";
        let tags = parse_ls_remote_tags(reversed);
        assert_eq!(tags.len(), 1);
        assert_eq!(
            tags[0].commit.to_hex(),
            "4b85e9d7b2bc03f629f3331705f5049e4522ce31"
        );
        assert!(tags[0].annotated);
    }

    #[test]
    fn a_remote_with_no_tags_lists_none() {
        // git prints nothing and exits 0.
        assert!(parse_ls_remote_tags(b"").is_empty());
    }

    #[test]
    fn lines_outside_the_tag_namespace_are_left_out() {
        let bytes = b"\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\tHEAD\n\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/heads/main\n\
b132edc248d8082a5019061b53fc2e3de31cc085\trefs/tags/v1\n\
garbage-without-a-tab\n\
nothex\trefs/tags/v2\n";
        let tags = parse_ls_remote_tags(bytes);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1");
    }
}
