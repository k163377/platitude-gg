//! Bringing back one part of what an entry took away (破棄記録仕様.md §4).
//! Each is git's own write for the thing: a branch, a tag, a working copy,
//! a stash entry — and the copy of thrown-away work, put back the first way
//! that takes it whole.
//!
//! What is written is the operated working copy (where the work was thrown
//! away), the shared refs and a new working copy; never another copy's
//! files (§4).

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use super::Restore;
use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};
use crate::scratch::ScratchFile;

/// How a part came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restored {
    /// As the thing it was: the branch, tag, working copy or stash entry,
    /// or the work applied as it had been staged.
    Whole,
    /// The work applied, all of it unstaged: its staged half did not go
    /// back as it was, and the working tree's side held all it did.
    Unstaged,
    /// The work could not go into the working copy whole without a
    /// conflict, so it waits as a stash entry instead (§4: on the commit it
    /// stood on, dated now).
    AsStash,
}

/// Brings back `part`. A refusal is git's, as the write's error (§4: a
/// branch another copy has out, a commit gc took …).
pub async fn restore(
    executor: &GitExecutor,
    workdir: &Path,
    part: &Restore,
    cancel: &CancellationToken,
) -> Result<Restored, GitError> {
    match part {
        Restore::Branch {
            name,
            tip,
            upstream,
        } => {
            crate::branch::create(executor, workdir, name, Some(&tip.to_hex()), false, cancel)
                .await?;
            if let Some((remote, branch)) = upstream {
                crate::branch::set_upstream(executor, workdir, name, remote, branch, cancel)
                    .await?;
            }
            Ok(Restored::Whole)
        }
        Restore::Tag { name, object } => {
            // The name back on the very object — an annotated tag's own, not a
            // new tag made over it — refused where the name stands (the empty
            // old value). `git tag` would sign and ask for a message under
            // `tag.gpgSign`.
            let cmd = GitCommand::new()
                .cwd(workdir)
                .args(["update-ref", "--end-of-options"])
                .arg(format!("refs/tags/{name}"))
                .arg(object.to_hex())
                .arg("");
            executor.run(cmd, cancel).await?;
            Ok(Restored::Whole)
        }
        Restore::Worktree { path, branch, head } => {
            let cmd = GitCommand::new().cwd(workdir).args(["worktree", "add"]);
            let cmd = match branch {
                Some(branch) => cmd.args(["--", path, branch]),
                None => cmd.args(["--detach", "--", path]).arg(head.to_hex()),
            };
            executor.run(cmd, cancel).await?;
            Ok(Restored::Whole)
        }
        Restore::Stash { commit, message } => {
            store(executor, workdir, commit, message, cancel).await?;
            Ok(Restored::Whole)
        }
        Restore::Changes { copy, path, .. } => {
            // Into the copy the work was thrown out of, and no other (§4);
            // gone, it has nowhere to go.
            if copy_at(executor, path, cancel).await?.is_none() {
                return Err(GitError::Rejected {
                    message: format!("The worktree the changes came from is gone: {path}"),
                });
            }
            changes(executor, Path::new(path), copy, cancel).await
        }
    }
}

/// The git directory of the working copy whose top folder is `path`, as a
/// session opened there names it (`rev-parse --absolute-git-dir`, as
/// `repo::open` asks) — none where no working copy stands there: the folder
/// is gone, or it is a folder left where one was removed, which git takes
/// for a folder of whatever repository holds it (and `git apply` there
/// passes over every path outside it, answering 0).
pub(super) async fn copy_at(
    executor: &GitExecutor,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    let at = Path::new(path);
    if !at.is_dir() {
        return Ok(None);
    }
    let cmd = GitCommand::new()
        .cwd(at)
        .args(["rev-parse", "--show-toplevel", "--absolute-git-dir"])
        .answers_by_code(128);
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Ok(None);
    }
    let text = out.stdout_utf8();
    let mut lines = text.lines();
    let (Some(top), Some(git_dir)) = (lines.next(), lines.next()) else {
        return Ok(None);
    };
    let here = std::fs::canonicalize(at).ok();
    let topped = here.is_some() && std::fs::canonicalize(top).ok() == here;
    Ok(topped.then(|| git_dir.trim_end().to_string()))
}

/// Puts the copy back into the working copy at `copy_path` (§4): its
/// differences from what the discard left — the index's, the working
/// tree's and the untracked files — applied where they go on over what is
/// there now, the staged half as it was staged; the working tree's alone
/// where the staged half no longer goes on but the working tree's side
/// holds all it did ([`work_holds_the_staged`]); else a stash entry holding
/// the same differences. Nothing goes in over an operation standing in the
/// copy: its staged half would go into what that commits.
///
/// `git apply` writes all of a patch or none of it, so a refusal leaves the
/// index and the working tree as they were.
async fn changes(
    executor: &GitExecutor,
    copy_path: &Path,
    copy: &Oid,
    cancel: &CancellationToken,
) -> Result<Restored, GitError> {
    let standing = crate::opstate::detect(executor, copy_path, cancel).await?;
    let quiet =
        !(standing.rebasing || standing.merging || standing.cherry_picking || standing.reverting);
    if quiet && let Some(restored) = put_back(executor, copy_path, copy, cancel).await? {
        return Ok(restored);
    }
    let message = subject_of(executor, copy_path, copy, cancel).await?;
    let entry = redated(executor, copy_path, copy, cancel).await?;
    store(executor, copy_path, &entry, &message, cancel).await?;
    Ok(Restored::AsStash)
}

/// The copy's differences applied where they go on (`changes`); `None`
/// where the working tree's do not, or the index's do not and the working
/// tree's alone would lose some of them — nothing written. The index's are those
/// of its second parent from that parent's own (what the discard left in
/// the index), the working tree's those of the copy from its first parent
/// (what it left there), and the untracked files its third parent whole.
async fn put_back(
    executor: &GitExecutor,
    cwd: &Path,
    copy: &Oid,
    cancel: &CancellationToken,
) -> Result<Option<Restored>, GitError> {
    let hex = copy.to_hex();
    let index = patch_of(
        executor,
        cwd,
        &[],
        &[&format!("{hex}^2^"), &format!("{hex}^2")],
        cancel,
    )
    .await?;
    let mut work = patch_of(executor, cwd, &[], &[&format!("{hex}^1"), &hex], cancel).await?;
    let untracked = format!("{hex}^3");
    if super::record::commit_of(executor, cwd, &untracked, cancel)
        .await?
        .is_some()
    {
        work.extend(patch_of(executor, cwd, &["--root"], &[&untracked], cancel).await?);
    }
    let git_dir = absolute_git_dir(executor, cwd, cancel).await?;
    let held = |bytes: &[u8]| -> Result<Option<ScratchFile>, GitError> {
        if bytes.is_empty() {
            return Ok(None);
        }
        ScratchFile::create(&git_dir, "restore-patch", bytes)
            .map(Some)
            .map_err(|error| GitError::Rejected {
                message: format!("could not hold the changes to put back: {error}"),
            })
    };
    let (index, work) = (held(&index)?, held(&work)?);
    // `git apply` checks a file it creates against what stands at its own
    // path alone; where a file stands in place of one of its folders, Git
    // for Windows answers 0 having written nothing there.
    if !created_lie_free(executor, cwd, &hex, &untracked, cancel).await? {
        return Ok(None);
    }
    if let Some(work) = &work
        && !applied(executor, cwd, work.path(), &["--check"], cancel).await?
    {
        return Ok(None);
    }
    let staged = match &index {
        Some(index) => {
            applied(
                executor,
                cwd,
                index.path(),
                &["--check", "--cached"],
                cancel,
            )
            .await?
        }
        None => true,
    };
    if !staged && !work_holds_the_staged(executor, cwd, &hex, cancel).await? {
        return Ok(None);
    }
    if staged
        && let Some(index) = &index
        && !applied(executor, cwd, index.path(), &["--cached"], cancel).await?
    {
        return Ok(None);
    }
    if let Some(work) = &work
        && !applied(executor, cwd, work.path(), &[], cancel).await?
    {
        // What changed under the check: the staged half goes back out.
        if staged && let Some(index) = &index {
            applied(
                executor,
                cwd,
                index.path(),
                &["--cached", "--reverse"],
                cancel,
            )
            .await?;
        }
        return Ok(None);
    }
    Ok(Some(if staged {
        Restored::Whole
    } else {
        Restored::Unstaged
    }))
}

/// Whether the working tree's differences alone keep all the staged half
/// held: at every path the index's differences touch, the working tree's
/// side holds the same. Then putting the working tree's back unstaged loses
/// only that it was staged; a working tree that had undone some of what was
/// staged would lose that, and one that held none of it would put back
/// nothing.
async fn work_holds_the_staged(
    executor: &GitExecutor,
    cwd: &Path,
    copy: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let index = format!("{copy}^2");
    let staged = changed_paths(executor, cwd, &format!("{index}^"), &index, cancel).await?;
    let apart = changed_paths(executor, cwd, &index, copy, cancel).await?;
    Ok(staged.iter().all(|path| !apart.contains(path)))
}

/// The paths `diff-tree` finds changed between two commits' trees.
async fn changed_paths(
    executor: &GitExecutor,
    cwd: &Path,
    from: &str,
    to: &str,
    cancel: &CancellationToken,
) -> Result<std::collections::HashSet<Vec<u8>>, GitError> {
    let cmd = GitCommand::new().cwd(cwd).args([
        "diff-tree",
        "-r",
        "-z",
        "--name-only",
        "--no-renames",
        "--end-of-options",
        from,
        to,
        "--",
    ]);
    Ok(executor
        .run(cmd, cancel)
        .await?
        .stdout
        .split(|b| *b == 0)
        .filter(|path| !path.is_empty())
        .map(<[u8]>::to_vec)
        .collect())
}

/// Whether every file the copy puts back anew finds its place free: the
/// working tree's added files (over the copy's base) and the untracked
/// ones, nothing at the path itself and no file where one of its folders
/// goes.
async fn created_lie_free(
    executor: &GitExecutor,
    cwd: &Path,
    copy: &str,
    untracked: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let added = GitCommand::new().cwd(cwd).args([
        "diff-tree",
        "-r",
        "-z",
        "--name-only",
        "--no-renames",
        "--diff-filter=A",
        "--end-of-options",
        &format!("{copy}^1"),
        copy,
        "--",
    ]);
    let mut paths = executor.run(added, cancel).await?.stdout;
    let listed = GitCommand::new()
        .cwd(cwd)
        .args([
            "ls-tree",
            "-r",
            "-z",
            "--name-only",
            "--end-of-options",
            untracked,
        ])
        .answers_by_code(128);
    let listed = executor.run_unchecked(listed, cancel).await?;
    if listed.code == 0 {
        paths.extend(listed.stdout);
    }
    Ok(paths
        .split(|b| *b == 0)
        .filter(|path| !path.is_empty())
        .all(|path| lies_free(cwd, &String::from_utf8_lossy(path))))
}

/// Whether nothing stands where `path` (git's, `/`-separated) would be
/// written under `root`: neither the path itself nor a file where one of
/// its folders would be.
fn lies_free(root: &Path, path: &str) -> bool {
    let mut at = root.to_path_buf();
    let mut parts = path.split('/').peekable();
    while let Some(part) = parts.next() {
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Err(error) => return error.kind() == std::io::ErrorKind::NotFound,
            Ok(meta) if parts.peek().is_none() || !meta.is_dir() => return false,
            Ok(_) => {}
        }
    }
    false
}

/// A patch of what `diff-tree` says between `revs` (or a root commit's tree
/// whole, `--root` among `options`), binary files included, applying as
/// `git apply` takes it; empty where nothing differs.
async fn patch_of(
    executor: &GitExecutor,
    cwd: &Path,
    options: &[&str],
    revs: &[&str],
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
    let cmd = GitCommand::new()
        .cwd(cwd)
        .args([
            "diff-tree",
            "-p",
            "--binary",
            "--full-index",
            "--no-renames",
            "--no-color",
        ])
        .args(options)
        .arg("--end-of-options")
        .args(revs)
        .arg("--");
    Ok(executor.run(cmd, cancel).await?.stdout)
}

/// `git apply <options> <patch>` in the working copy: whether it went on.
/// Line endings are taken as they come, whatever `core.safecrlf` says, and
/// whitespace as it is, whatever `apply.whitespace` says — a restore puts
/// back what was there.
async fn applied(
    executor: &GitExecutor,
    cwd: &Path,
    patch: &Path,
    options: &[&str],
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(cwd)
        .args(["-c", "core.safecrlf=false", "apply", "--whitespace=nowarn"])
        .args(options)
        .arg(patch)
        .answers_by_code(1);
    Ok(executor.run_unchecked(cmd, cancel).await?.code == 0)
}

/// The working copy's own git directory, in full: where the patches wait.
async fn absolute_git_dir(
    executor: &GitExecutor,
    cwd: &Path,
    cancel: &CancellationToken,
) -> Result<PathBuf, GitError> {
    let cmd = GitCommand::new()
        .cwd(cwd)
        .args(["rev-parse", "--absolute-git-dir"]);
    let out = executor.run(cmd, cancel).await?;
    Ok(PathBuf::from(out.stdout_utf8().trim()))
}

async fn subject_of(
    executor: &GitExecutor,
    cwd: &Path,
    commit: &Oid,
    cancel: &CancellationToken,
) -> Result<String, GitError> {
    let cmd = GitCommand::new()
        .cwd(cwd)
        .args(["log", "-1", "--format=%s"])
        .arg(commit.to_hex())
        .arg("--");
    Ok(executor
        .run(cmd, cancel)
        .await?
        .stdout_utf8()
        .trim()
        .to_string())
}

/// The copy again — the same tree, parents and message, so the same
/// differences over what the discard left — dated now, so the stash list
/// puts it where it was made. Made by Platitude GG as the copy was, so no
/// identity of the user's is asked for (破棄記録仕様.md §2).
async fn redated(
    executor: &GitExecutor,
    cwd: &Path,
    copy: &Oid,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let read = GitCommand::new()
        .cwd(cwd)
        .args(["log", "-1", "--format=%T %P%x00%B"])
        .arg(copy.to_hex())
        .arg("--");
    let out = executor.run(read, cancel).await?;
    let text = out.stdout_utf8();
    let (ids, body) = text.split_once('\0').unwrap_or((&text, ""));
    let mut ids = ids
        .split(' ')
        .filter_map(|hex| Oid::from_hex_str(hex.trim()).ok());
    let tree = ids.next().ok_or_else(|| GitError::UnexpectedOutput {
        command: "git log".to_string(),
        message: "no tree came back for the copy put in the stash".to_string(),
    })?;
    let parents: Vec<Oid> = ids.collect();
    super::record::commit(executor, cwd, &tree, &parents, body.trim_end(), cancel).await
}

/// `stash store`: the entry goes to the top of the list with `message`.
async fn store(
    executor: &GitExecutor,
    cwd: &Path,
    commit: &Oid,
    message: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // No separator: `stash store` takes one commit and no pathspec.
    let cmd = GitCommand::new()
        .cwd(cwd)
        .args(["stash", "store", "-m", message])
        .arg(commit.to_hex());
    executor.run(cmd, cancel).await.map(drop)
}
