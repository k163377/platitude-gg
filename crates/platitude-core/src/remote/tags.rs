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
/// Nothing local records this. A branch on a remote leaves `refs/remotes/`
/// behind, so its badge survives going offline; a fetched tag lands in
/// `refs/tags/` next to the ones made here and the two become
/// indistinguishable. Asking the remote is the only way to tell them apart,
/// which is why this is on the fetch path rather than the poll: it is the
/// one place the user has already agreed to pay for the network.
///
/// An annotated tag is advertised twice — the tag object, then the commit
/// it peels to under a `^{}` suffix — and it is the commit that has to line
/// up with what the local listing peels to, so the peeled line wins wherever
/// both appear. `--refs` cannot do that filtering: it drops the peeled line
/// and keeps the tag object, which compares equal to nothing (measured).
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
        .timeout(timeout);
    let out = executor.run(cmd, cancel).await?;
    Ok(parse_ls_remote_tags(&out.stdout))
}

/// Sends one tag to one remote.
///
/// `expect` turns the push into a leased overwrite pinned to the commit
/// the remote was last seen holding this tag on — empty sends it plain,
/// which git refuses outright where the name is already over there on
/// something else.
///
/// **A refusal here is not the one a fetch answers.** A branch that is
/// turned down for being behind is put right by fetching; a tag is not —
/// `--prune` leaves the local tag where it is and `--prune-tags` exits 1
/// (実測) — so this returns [`GitError::Failed`] whatever the refusal was
/// and nothing is queued behind it. `git push` for branches is
/// [`super::push::push`]; the two share no refspec, since a tag's is
/// `refs/tags/` on both sides.
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
        .timeout(timeout);
    if !expect.is_empty() {
        cmd = cmd.arg(format!("--force-with-lease=refs/tags/{tag}:{expect}"));
    }
    cmd = cmd.args(["--", remote, &refspec]);
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    Err(GitError::Failed {
        command,
        code: out.code,
        stderr: out.failure_message(),
    })
}

/// Takes one tag off one remote. Nothing here is touched.
///
/// **The name is fully qualified, and it has to be.** A bare `--delete
/// <name>` is resolved against everything the remote carries, so a name
/// that is a branch over there as well is refused outright — `error: dst
/// refspec dup matches more than one`, with neither of the two deleted
/// (実測 git 2.55). `refs/tags/<name>` names the one ref meant, and the
/// branch beside it is left alone.
///
/// **A name the remote has not got is not an error in this spelling.**
/// The qualified form needs no resolution over there, so git answers
/// `warning: deleting a non-existent ref` and exits 0 (実測) — where the
/// bare form would have failed. Whether there is anything to delete is
/// therefore the caller's to know before it asks (`offers::TagSides`);
/// git will not be the one to say.
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
            "--delete",
            "--",
            remote,
            &format!("refs/tags/{tag}"),
        ])
        .timeout(timeout);
    executor.run(cmd, cancel).await.map(drop)
}

/// One tag as a remote advertises it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTag {
    pub name: crate::Name,
    /// The commit it designates, annotated tags peeled — the same thing
    /// [`crate::refs::RefEntry::commit_oid`] answers for a local one, so
    /// the two can be compared directly.
    pub commit: Oid,
    /// True when the remote advertised a tag object to peel.
    pub annotated: bool,
}

/// Parses `git ls-remote --tags`: `<oid>\t<refname>` lines, sorted by name.
///
/// The `From <url>` banner goes to stderr, so stdout is only ref lines.
/// Anything outside `refs/tags/` and any unreadable oid is skipped rather
/// than failing the listing — one odd advertisement must not cost every
/// other tag its badge.
pub fn parse_ls_remote_tags(bytes: &[u8]) -> Vec<RemoteTag> {
    let mut out: Vec<RemoteTag> = Vec::new();
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
        match out.iter_mut().find(|t| t.name == name) {
            // Two lines for one name means an annotated tag: the pair is
            // the tag object and the commit under it. Only the peeled line
            // carries the commit, so it wins whichever side it arrives on.
            Some(seen) if !seen.annotated => {
                seen.commit = oid;
                seen.annotated = peeled;
            }
            Some(_) => {}
            None => out.push(RemoteTag {
                name: name.into(),
                commit: oid,
                annotated: peeled,
            }),
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
    /// Recorded from git 2.51 running `ls-remote --tags` against a local
    /// repository holding one annotated tag (`ann-1`, advertised twice) and
    /// three lightweight ones. Byte for byte as git wrote it to stdout.
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
        // git advertises the tag object first, but nothing in the protocol
        // promises the order, and a reversed pair must read the same.
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
        // Measured: git prints nothing at all and exits 0.
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
