//! Writing Platitude GG's own record (破棄記録仕様.md §2), read back by
//! `records`: one line on [`RECORD_REF`]'s reflog per operation, under the
//! identity `Platitude GG` and never the user's (§2).
//!
//! - **A copy** of uncommitted work (§2.1): written by `copy`, with the
//!   commits, the line and the temporary indexes from here — the user's
//!   hooks, fsmonitor, split index and `safecrlf` kept out of them.
//! - **A note** of what any other operation took, made after it took it: a
//!   commit on what was taken (its first parent, the same tree) whose
//!   trailers say what to put back. A note `restored` says a part of a
//!   record was brought back.
//!
//! A ref written to the value it already holds writes no reflog line (§5),
//! so every line's value is a new commit — one of its own, by its
//! `Nonce:` ([`nonce`]).

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio_util::sync::CancellationToken;

use super::RECORD_REF;
use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};
use crate::scratch::ScratchFile;

/// Who writes the record (§2): the name on every commit and line.
const RECORDER: &str = "Platitude GG";

/// How many paths one `ls-files` / `update-index` / `add` takes on its
/// command line.
pub(super) const PATHS_PER_CALL: usize = 200;

/// A remote's branch as it stood before it went or was pushed over: where,
/// and its tip — the lease's commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTip {
    pub remote: String,
    pub branch: String,
    pub tip: Oid,
}

/// A branch as it stood before it went (§2): its tip, and what it was
/// measured against (`branch.<name>.remote` / `.merge`), which the delete
/// takes with the branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchBefore {
    pub name: String,
    pub tip: Oid,
    pub upstream: Option<(String, String)>,
}

/// A tag as it stood: the object it names (an annotated tag's own), and
/// the commit that peels to where this repository has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagBefore {
    pub name: String,
    pub object: Oid,
    pub commit: Option<Oid>,
}

/// A stash entry as it stood: its commit, its message, and the untracked
/// files' commit where it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashBefore {
    pub commit: Oid,
    pub message: String,
    pub untracked: Option<Oid>,
}

/// Reads a branch as it stands, before a delete takes it with its
/// upstream; `None` for no such branch.
pub async fn branch_before(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    cancel: &CancellationToken,
) -> Result<Option<BranchBefore>, GitError> {
    let Some(tip) = commit_of(executor, workdir, &format!("refs/heads/{name}"), cancel).await?
    else {
        return Ok(None);
    };
    let literal = crate::config::regexp_literal(name);
    let found = crate::config::get_regexp(
        executor,
        workdir,
        &format!("^branch\\.{literal}\\.(remote|merge)$"),
        "the branch's upstream",
        cancel,
    )
    .await?;
    let (mut remote, mut merge) = (None, None);
    for record in crate::config::parse_z_records(&found) {
        let value = record.value().unwrap_or_default().to_string();
        if record.key().ends_with(".remote") {
            remote = Some(value);
        } else {
            merge = Some(value);
        }
    }
    let upstream = remote.zip(merge).map(|(remote, merge)| {
        let branch = merge
            .strip_prefix("refs/heads/")
            .unwrap_or(&merge)
            .to_string();
        (remote, branch)
    });
    Ok(Some(BranchBefore {
        name: name.to_string(),
        tip,
        upstream,
    }))
}

/// Reads the tag `name` as `rev` (`refs/tags/<name>`) names it here, before
/// a delete or an overwrite; `None` where `rev` names nothing.
pub async fn tag_before(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Option<TagBefore>, GitError> {
    let Some(object) = object_of(executor, workdir, rev, cancel).await? else {
        return Ok(None);
    };
    let commit = commit_of(executor, workdir, &object.to_hex(), cancel).await?;
    Ok(Some(TagBefore {
        name: name.to_string(),
        object,
        commit,
    }))
}

/// Reads a stash entry as it stands, before a drop or pop; `None` for no
/// such entry.
pub async fn stash_before(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<Option<StashBefore>, GitError> {
    let entries = crate::stash::load(executor, workdir, cancel).await?;
    let Some(entry) = entries.into_iter().find(|entry| entry.name == selector) else {
        return Ok(None);
    };
    let untracked = commit_of(
        executor,
        workdir,
        &format!("{}^3", entry.oid.to_hex()),
        cancel,
    )
    .await?;
    Ok(Some(StashBefore {
        commit: entry.oid,
        message: entry.message,
        untracked,
    }))
}

/// The commit `rev` names, `None` where it names none or another kind of
/// object.
pub async fn commit_of(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    object_of(executor, workdir, &format!("{rev}^{{commit}}"), cancel).await
}

/// Notes a branch deleted here — and on a remote too, where both went: the
/// remote's tip is a part of its own only where it is not the branch's.
pub async fn record_branch_delete(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    before: &BranchBefore,
    remote: Option<&RemoteTip>,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut trailers = vec![
        ("Operation", "delete branch".to_string()),
        ("Branch", before.name.clone()),
    ];
    if let Some((remote, branch)) = &before.upstream {
        trailers.push(("Upstream", format!("{remote} {branch}")));
    }
    if let Some(remote) = remote {
        trailers.push(("Remote", remote.remote.clone()));
        trailers.push(("Remote-branch", remote.branch.clone()));
        if remote.tip != before.tip {
            trailers.push(("Remote-tip", remote.tip.to_hex()));
        }
    }
    let summary = format!("delete branch {}", before.name);
    note(
        executor,
        workdir,
        git_dir,
        Some(&before.tip),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes a branch deleted on a remote alone.
pub async fn record_remote_delete(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    remote: &RemoteTip,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let trailers = [
        ("Operation", "delete remote branch".to_string()),
        ("Remote", remote.remote.clone()),
        ("Branch", remote.branch.clone()),
    ];
    let summary = format!("delete {}/{}", remote.remote, remote.branch);
    note(
        executor,
        workdir,
        git_dir,
        Some(&remote.tip),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes a remote branch's tip pushed over: the tip it had.
pub async fn record_force_push(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    remote: &RemoteTip,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let trailers = [
        ("Operation", "force push".to_string()),
        ("Remote", remote.remote.clone()),
        ("Branch", remote.branch.clone()),
    ];
    let summary = format!("force-push {}/{}", remote.remote, remote.branch);
    note(
        executor,
        workdir,
        git_dir,
        Some(&remote.tip),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes a tag deleted here (`here`), on `remote` (its object there), or
/// both: the remote's object is a part of its own only where it is not the
/// one here, standing on its own commit (`Remote-commit:`) where this
/// repository has that.
pub async fn record_tag_delete(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    here: Option<&TagBefore>,
    remote: Option<(&str, &TagBefore)>,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let Some(first) = here.or(remote.map(|(_, tag)| tag)) else {
        return Ok(());
    };
    let operation = if here.is_some() {
        "delete tag"
    } else {
        "delete remote tag"
    };
    let mut trailers = vec![
        ("Operation", operation.to_string()),
        ("Tag", first.name.clone()),
        ("Object", first.object.to_hex()),
    ];
    if let Some((name, tag)) = remote {
        trailers.push(("Remote", name.to_string()));
        if tag.object != first.object {
            trailers.push(("Remote-object", tag.object.to_hex()));
            if let Some(commit) = &tag.commit {
                trailers.push(("Remote-commit", commit.to_hex()));
            }
        }
    }
    let summary = match (here, remote) {
        (None, Some((remote, tag))) => format!("delete tag {} on {remote}", tag.name),
        _ => format!("delete tag {}", first.name),
    };
    note(
        executor,
        workdir,
        git_dir,
        first.commit.as_ref(),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes a remote's tag pushed over: the object it named.
pub async fn record_tag_force_push(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    remote: &str,
    before: &TagBefore,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let trailers = [
        ("Operation", "force push tag".to_string()),
        ("Tag", before.name.clone()),
        ("Object", before.object.to_hex()),
        ("Remote", remote.to_string()),
    ];
    let summary = format!("force-push tag {} to {remote}", before.name);
    note(
        executor,
        workdir,
        git_dir,
        before.commit.as_ref(),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes a worktree removed: where it was, and what it had out.
pub async fn record_worktree_remove(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    path: &str,
    branch: Option<&str>,
    head: &Oid,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut trailers = vec![
        ("Operation", "remove worktree".to_string()),
        ("Worktree-path", path.to_string()),
    ];
    if let Some(branch) = branch {
        trailers.push(("Branch", branch.to_string()));
    }
    let summary = format!("remove worktree {path}");
    note(
        executor,
        workdir,
        git_dir,
        Some(head),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes a stash entry dropped, or popped (`popped`): the entry itself is
/// the note's first parent.
pub async fn record_stash(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    before: &StashBefore,
    popped: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let operation = if popped { "stash pop" } else { "stash drop" };
    let mut trailers = vec![
        ("Operation", operation.to_string()),
        ("Message", before.message.clone()),
    ];
    if let Some(untracked) = &before.untracked {
        trailers.push(("Untracked", untracked.to_hex()));
    }
    let summary = format!("{operation}: {}", before.message);
    note(
        executor,
        workdir,
        git_dir,
        Some(&before.commit),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes the branches one rebase moved together (`--update-refs`), each
/// with its old and new tip, so the list tells them as one (§2).
pub async fn record_moves(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    moved: &[(String, Oid, Oid)],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let Some((_, first_old, _)) = moved.first() else {
        return Ok(());
    };
    let mut trailers = vec![("Operation", "rebase".to_string())];
    trailers.extend(
        moved
            .iter()
            .map(|(reference, old, new)| ("Moved", moved_value(reference, old, new))),
    );
    let summary = format!("rebase {} branches", moved.len());
    note(
        executor,
        workdir,
        git_dir,
        Some(first_old),
        &summary,
        &trailers,
        cancel,
    )
    .await
}

/// Notes that part `part` of the record line `record` was brought back,
/// so the list reads it no more.
pub async fn record_restored(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    record: &Oid,
    part: usize,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let trailers = [
        ("Operation", "restored".to_string()),
        ("Record", record.to_hex()),
        ("Part", part.to_string()),
    ];
    note(
        executor,
        workdir,
        git_dir,
        Some(record),
        "restored",
        &trailers,
        cancel,
    )
    .await
}

/// Which worktree a git directory is (§2.1 `Worktree:`): the main one
/// ([`super::MAIN_WORKTREE`]), or a linked worktree's name under
/// `$GIT_DIR/worktrees/` — what still names it after its folder moved
/// (gitrepository-layout(5)), where `Worktree-path:` says where it was.
pub(super) fn worktree_id(git_dir: &Path) -> String {
    let linked = git_dir
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|parent| parent == "worktrees");
    match git_dir.file_name() {
        Some(id) if linked => id.to_string_lossy().into_owned(),
        _ => super::MAIN_WORKTREE.to_string(),
    }
}

/// `Moved:`'s value: the ref, its old tip, its new.
pub(super) fn moved_value(reference: &str, old: &Oid, new: &Oid) -> String {
    format!("{reference} {} {}", old.to_hex(), new.to_hex())
}

/// A note: a commit on `parent` with the parent's own tree (the empty one
/// with no parent), carrying `summary` and `trailers`, appended to the
/// record.
async fn note(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    parent: Option<&Oid>,
    summary: &str,
    trailers: &[(&str, String)],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let tree = match parent {
        Some(parent) => tree_of(executor, workdir, parent, cancel).await?,
        None => empty_tree(executor, workdir, git_dir, cancel).await?,
    };
    let parents: Vec<Oid> = parent.copied().into_iter().collect();
    let mut trailers = trailers.to_vec();
    trailers.push(("Nonce", nonce()));
    let value = commit(
        executor,
        workdir,
        &tree,
        &parents,
        &message(summary, &trailers),
        cancel,
    )
    .await?;
    append(executor, workdir, git_dir, &value, summary, cancel).await
}

/// A line's own value for `Nonce:`, so no two lines are one commit. A
/// commit is its tree, parents, message, identity and date to the second:
/// the same discard made again in the second it was restored would be the
/// line the restore's note names, and the list would take it for restored
/// too (§3).
pub(super) fn nonce() -> String {
    static MADE: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let count = MADE.fetch_add(1, Ordering::Relaxed);
    format!("{now:x}-{:x}-{count:x}", std::process::id())
}

/// The record's next line: `value`, said as `summary` — out of reach of
/// the user's `reference-transaction` hook, which may refuse any ref's
/// update (§2.1).
pub(super) async fn append(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    value: &Oid,
    summary: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = as_recorder(unhooked(workdir, git_dir).args([
        "update-ref",
        "--create-reflog",
        "-m",
        summary,
        RECORD_REF,
    ]))
    .arg(value.to_hex());
    executor.run(cmd, cancel).await.map(drop)
}

/// A commit of `tree` on `parents`, by the recorder. `commit-tree` signs
/// only when told to (§2.1).
pub(super) async fn commit(
    executor: &GitExecutor,
    workdir: &Path,
    tree: &Oid,
    parents: &[Oid],
    message: &str,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["commit-tree", &tree.to_hex()]);
    for parent in parents {
        cmd = cmd.args(["-p", &parent.to_hex()]);
    }
    let out = executor
        .run(as_recorder(cmd.args(["-m", message])), cancel)
        .await?;
    oid_out(&out.stdout_utf8(), "git commit-tree")
}

pub(super) async fn write_tree(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    index: &Path,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let cmd = isolated(workdir, git_dir, index).arg("write-tree");
    let out = executor.run(cmd, cancel).await?;
    oid_out(&out.stdout_utf8(), "git write-tree")
}

/// The empty tree, written from an index with nothing in it — the object
/// format's own id, whichever it is.
pub(super) async fn empty_tree(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let index = ScratchFile::fresh(git_dir, "empty-index").map_err(io_error)?;
    write_tree(executor, workdir, git_dir, index.path(), cancel).await
}

/// A command that runs none of the user's hooks: they are looked for in a
/// folder that is never there (§2.1).
fn unhooked(workdir: &Path, git_dir: &Path) -> GitCommand {
    let no_hooks = git_dir.join("platitude").join("no-hooks");
    GitCommand::new()
        .cwd(workdir)
        .arg("-c")
        .arg(format!("core.hooksPath={}", no_hooks.to_string_lossy()))
}

/// A command for a temporary index, with the user's settings that would
/// act on it kept out (§2.1): no hooks, no fsmonitor, no split index, and
/// line endings taken as they are. No checksum is written over it either:
/// hashing the whole of it is a good part of each write, and the file is
/// read again only by the commands made here.
pub(super) fn isolated(workdir: &Path, git_dir: &Path, index: &Path) -> GitCommand {
    unhooked(workdir, git_dir)
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.splitIndex=false",
            "-c",
            "core.safecrlf=false",
            "-c",
            "index.skipHash=true",
        ])
        .env("GIT_INDEX_FILE", index)
}

fn as_recorder(cmd: GitCommand) -> GitCommand {
    cmd.env("GIT_AUTHOR_NAME", RECORDER)
        .env("GIT_AUTHOR_EMAIL", "")
        .env("GIT_COMMITTER_NAME", RECORDER)
        .env("GIT_COMMITTER_EMAIL", "")
}

/// The object `rev` names, `None` where it names none (exit 1).
async fn object_of(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "--quiet", "--end-of-options", rev])
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Ok(None);
    }
    Ok(Oid::from_hex_str(out.stdout_utf8().trim()).ok())
}

pub(super) async fn tree_of(
    executor: &GitExecutor,
    workdir: &Path,
    commit: &Oid,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "--end-of-options"])
        .arg(format!("{}^{{tree}}", commit.to_hex()));
    let out = executor.run(cmd, cancel).await?;
    oid_out(&out.stdout_utf8(), "git rev-parse")
}

/// A commit message: the summary, a blank line, the trailers one a line.
pub(super) fn message(summary: &str, trailers: &[(&str, String)]) -> String {
    let lines: Vec<String> = trailers
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect();
    format!("{summary}\n\n{}", lines.join("\n"))
}

pub(super) fn oid_out(text: &str, command: &str) -> Result<Oid, GitError> {
    Oid::from_hex_str(text.trim()).map_err(|_| GitError::UnexpectedOutput {
        command: command.to_string(),
        message: "no object id came back".to_string(),
    })
}

pub(super) fn io_error(error: std::io::Error) -> GitError {
    GitError::Rejected {
        message: format!("could not make room for a temporary index: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_the_summary_then_one_trailer_a_line() {
        let text = message(
            "delete branch spike",
            &[
                ("Operation", "delete branch".into()),
                ("Branch", "spike".into()),
            ],
        );
        assert_eq!(
            text,
            "delete branch spike\n\nOperation: delete branch\nBranch: spike"
        );
    }
}
