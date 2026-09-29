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
/// A plain push onto a name the remote holds on **the same commit** is
/// answered as sent: git refuses it (`already exists`) wherever the objects
/// differ — a lightweight tag over there for an annotated one here, or
/// another annotation — but the screen shows one commit on both sides, so
/// it is no collision to report. The remote's object is left as it is.
///
/// A lease the remote has left, and a name it holds on another commit,
/// read as outdated; what puts them right is reading the remote's tags
/// again, not a fetch, which the session does (`session::push_tag`). A
/// far-side rule (`[remote rejected]`) reads as a report, as for a branch
/// (`super::refusal::tag_refusal`, デザイン規約 §答えの要らない報せ) —
/// `--porcelain` is what makes either readable.
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
    if super::refusal::is_taken(&out.stdout_utf8())
        && holds_same_commit(executor, workdir, remote, tag, timeout, cancel).await?
    {
        return Ok(());
    }
    Err(super::refusal::tag_refusal(
        command, &out, remote, tag, false,
    ))
}

/// Whether `remote` holds `tag` on the commit the tag here peels to.
async fn holds_same_commit(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    tag: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let Some(there) = holding(executor, workdir, remote, tag, timeout, cancel).await? else {
        return Ok(false);
    };
    let cmd = GitCommand::new().cwd(workdir).args([
        "rev-parse",
        "--verify",
        "--end-of-options",
        &format!("refs/tags/{tag}^{{commit}}"),
    ]);
    let here = executor.run(cmd, cancel).await?;
    Ok(here.stdout_utf8().trim() == there.commit.to_hex())
}

/// Takes one tag off one remote. Nothing here is touched.
///
/// `expect` is the commit the screen showed the tag on, and the delete is
/// leased to it: a name the remote has moved or dropped since is refused
/// (`[rejected] (stale info)`) and read as outdated, as in [`push_tag`].
/// See [`lease_on`] for why the lease is not `expect` itself.
///
/// The name must stay qualified as `refs/tags/<name>`: a bare name that is
/// also a branch over there is refused and neither is deleted.
/// A far-side refusal is read as in [`push_tag`], hence `--porcelain`.
pub async fn delete_remote_tag(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    tag: &str,
    expect: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let pinned = lease_on(executor, workdir, remote, tag, expect, timeout, cancel).await?;
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "push",
            "--porcelain",
            &format!("--force-with-lease=refs/tags/{tag}:{pinned}"),
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
    Err(super::refusal::tag_refusal(
        command, &out, remote, tag, true,
    ))
}

/// What a delete of this tag is leased to: the object the remote holds it
/// on, where that peels to `expect`, and `expect` otherwise.
///
/// A lease compares the ref's own value, which for an annotated tag is the
/// tag object — leased to the commit it peels to, the delete is refused as
/// stale, and git takes no `^{}` there. The screen holds only the commit
/// ([`RemoteTag::commit`]), so the object is asked for here; a remote that
/// has left `expect` gets `expect` itself, which git then refuses.
async fn lease_on(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    tag: &str,
    expect: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<String, GitError> {
    let there = holding(executor, workdir, remote, tag, timeout, cancel).await?;
    Ok(pin_for(there.as_ref(), expect))
}

/// One tag as one remote holds it now: the ref's own value, and the commit
/// it peels to (the same oid for a lightweight tag).
struct Holding {
    object: Oid,
    commit: Oid,
}

/// What `remote` holds `tag` on where the push goes; `None` where it lacks
/// the name.
///
/// Asked of the push URL (`remote get-url --push`, the fetch URL where none
/// is set): `ls-remote <name>` reads the fetch URL, and a `pushurl` naming
/// another repository would weigh what the send never reaches.
async fn holding(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    tag: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Option<Holding>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["remote", "get-url", "--push", "--", remote]);
    let url = executor
        .run(cmd, cancel)
        .await?
        .stdout_utf8()
        .trim()
        .to_string();
    let refname = format!("refs/tags/{tag}");
    let peeled = format!("{refname}^{{}}");
    // Both names: a bare pattern does not match the `^{}` line.
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-remote", "--tags", "--", &url, &refname, &peeled])
        .timeout(timeout)
        .paced_elsewhere();
    let out = executor.run(cmd, cancel).await?;
    Ok(parse_holding(&out.stdout, &refname))
}

/// [`holding`] off the `ls-remote` lines for one tag.
fn parse_holding(bytes: &[u8], refname: &str) -> Option<Holding> {
    let mut object = None;
    let mut commit = None;
    for (oid, name) in bytes
        .split(|b| *b == b'\n')
        .filter_map(split_ls_remote_line)
    {
        if name == refname {
            object = Some(oid);
        } else if name.strip_suffix("^{}") == Some(refname) {
            commit = Some(oid);
        }
    }
    let object = object?;
    Some(Holding {
        object,
        commit: commit.unwrap_or(object),
    })
}

/// [`lease_on`]'s choice: the remote's object where it peels to `expect`.
fn pin_for(there: Option<&Holding>, expect: &str) -> String {
    match there {
        Some(there) if there.commit.to_hex() == expect => there.object.to_hex(),
        _ => expect.to_string(),
    }
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
/// The old name's delete is leased to `expect`, as in
/// [`delete_remote_tag`]; refused, both names stay over there.
/// Whatever hung off the old name over there (a release) stays behind on
/// it; the UI warns before this runs.
#[expect(clippy::too_many_arguments)]
pub async fn replace_remote_tag(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    from: &str,
    to: &str,
    expect: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    push_tag(executor, workdir, remote, to, "", timeout, cancel).await?;
    delete_remote_tag(executor, workdir, remote, from, expect, timeout, cancel).await
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

    /// Real `ls-remote --tags -- origin refs/tags/<t> refs/tags/<t>^{}`
    /// stdout (git 2.55 and 2.43 alike).
    const ANNOTATED: &[u8] = b"\
2ed5d8dca01fe0551588949ea26e6dc65e69cae8\trefs/tags/ann\n\
a9771664a8219f845abfd8ce6f5d7af521247539\trefs/tags/ann^{}\n";
    const LIGHTWEIGHT: &[u8] = b"\
a9771664a8219f845abfd8ce6f5d7af521247539\trefs/tags/light\n";
    const SHOWN: &str = "a9771664a8219f845abfd8ce6f5d7af521247539";
    const ELSEWHERE: &str = "8e54e2935b5e9b6cf32833403904fd3f68da81f2";

    fn pin(bytes: &[u8], refname: &str, expect: &str) -> String {
        pin_for(parse_holding(bytes, refname).as_ref(), expect)
    }

    #[test]
    fn an_annotated_tag_is_leased_to_its_object_where_it_peels_to_the_commit_shown() {
        assert_eq!(
            pin(ANNOTATED, "refs/tags/ann", SHOWN),
            "2ed5d8dca01fe0551588949ea26e6dc65e69cae8"
        );
        assert_eq!(pin(LIGHTWEIGHT, "refs/tags/light", SHOWN), SHOWN);
    }

    /// Leased to the commit shown, a remote that has moved or dropped the
    /// name refuses the delete itself.
    #[test]
    fn a_tag_the_remote_has_left_is_leased_to_the_commit_shown() {
        assert_eq!(pin(ANNOTATED, "refs/tags/ann", ELSEWHERE), ELSEWHERE);
        assert_eq!(pin(LIGHTWEIGHT, "refs/tags/light", ELSEWHERE), ELSEWHERE);
        assert_eq!(pin(b"", "refs/tags/ann", SHOWN), SHOWN);
    }
}
