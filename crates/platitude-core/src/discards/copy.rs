//! Copies of uncommitted work a write throws away (破棄記録仕様.md §2.1):
//! what the paths held before the write, kept over what the write left
//! there. The copy is a stash whose base is the state the write left — the
//! index and the working tree right after — so its difference from its
//! base is what went and nothing else: a hunk thrown out of a file whose
//! other hunks stayed comes back alone, beside them or without them.
//!
//! Read in temporary indexes begun from a copy of the working copy's own
//! index file — its bytes, where `read-tree` would write every entry anew
//! (ci/baseline/code-costs-windows-x64.md) — so neither `refs/stash`, the
//! index nor the working tree is touched.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::record::{
    PATHS_PER_CALL, append, commit, commit_of, empty_tree, io_error, isolated, message,
    moved_value, nonce, oid_out, worktree_id, write_tree,
};
use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::scratch::ScratchFile;

/// What a copy is a copy of (§2.1 `Operation:`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyOperation {
    /// Whole files, staged or not, and untracked ones with them.
    Discard,
    /// Hunks of one file.
    Hunk,
    /// Lines of one file.
    Lines,
    /// Untracked files alone.
    Untracked,
    /// What `reset --hard` throws away with the commits.
    ResetHard,
}

impl CopyOperation {
    pub(super) fn word(self) -> &'static str {
        match self {
            CopyOperation::Discard => "discard",
            CopyOperation::Hunk => "hunk",
            CopyOperation::Lines => "lines",
            CopyOperation::Untracked => "untracked",
            CopyOperation::ResetHard => "reset --hard",
        }
    }

    /// Whether the write takes part of one file's unstaged diff, leaving
    /// the index as it is.
    fn takes_part(self) -> bool {
        matches!(self, CopyOperation::Hunk | CopyOperation::Lines)
    }
}

/// The uncommitted work a write is about to throw away, and why.
#[derive(Debug, Clone, Copy)]
pub struct CopyOf<'a> {
    pub workdir: &'a Path,
    /// The copy's own git directory: its index, and the temporary ones.
    pub git_dir: &'a Path,
    /// Tracked paths, as the index and the working tree hold them.
    pub tracked: &'a [String],
    /// Untracked paths — a folder takes everything under it.
    pub untracked: &'a [String],
    pub operation: CopyOperation,
    /// Whether the write takes the tracked paths back in the index as well
    /// as in the working tree — a staged row's discard, back to HEAD — so
    /// what it left is read off the index afterwards.
    pub touches_index: bool,
    /// For `reset --hard`: the move it goes with — the ref (`main`, or
    /// `HEAD` while detached) and its old and new tips — so the list tells
    /// the two as one (§2).
    pub moved: Option<(&'a str, Oid, Oid)>,
}

/// A copy begun: what the paths held before the write ran, waiting for
/// what the write left ([`record_copy`]).
#[derive(Debug)]
pub struct Copied {
    head: Oid,
    head_tree: Oid,
    branch: Option<String>,
    /// None without tracked paths.
    before: Option<Before>,
    /// The untracked files, as a tree of their own; none where there are
    /// none to take.
    untracked: Option<Oid>,
    not_copied: Vec<String>,
    tracked: Vec<String>,
    untracked_paths: Vec<String>,
    operation: CopyOperation,
    touches_index: bool,
    moved: Option<(String, Oid, Oid)>,
    summary: String,
}

/// What the tracked paths held before the write.
#[derive(Debug)]
enum Before {
    /// Whole files: both sides.
    Whole(Sides),
    /// Part of one file's unstaged diff: the working tree's side alone,
    /// and the temporary index holding it — what the write leaves is taken
    /// into it again, the index being the same before and after.
    Part { work: Oid, index: IndexCopy },
}

/// The two sides of a copy's state (§2.1): the index's tree, and the
/// working tree's — the index with the tracked paths as the working tree
/// holds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sides {
    index: Oid,
    work: Oid,
}

/// Reads what `of`'s paths hold before the write throws it away (§2.1), to
/// go on the record against what the write leaves ([`record_copy`]). `None`
/// while HEAD has no commit (§2.1) and for no paths at all. A path git
/// cannot take (a required filter missing, an encoding it cannot convert)
/// is left as the index has it and named under `Not-copied:`; whether it
/// could is read off the exit code alone.
pub async fn copy_work(
    executor: &GitExecutor,
    of: &CopyOf<'_>,
    cancel: &CancellationToken,
) -> Result<Option<Copied>, GitError> {
    if of.tracked.is_empty() && of.untracked.is_empty() {
        return Ok(None);
    }
    let Some((head, head_tree, branch)) = head_of(executor, of.workdir, cancel).await? else {
        return Ok(None);
    };
    let mut not_copied = Vec::new();
    let before = if of.tracked.is_empty() {
        None
    } else {
        let index = IndexCopy::now(executor, of, &head, cancel).await?;
        let index_tree = if of.operation.takes_part() {
            None
        } else {
            Some(index.tree(executor, of, &head, cancel).await?)
        };
        index
            .take(executor, of, of.tracked, &mut not_copied, cancel)
            .await?;
        let work = index.tree(executor, of, &head, cancel).await?;
        Some(match index_tree {
            Some(index_tree) => Before::Whole(Sides {
                index: index_tree,
                work,
            }),
            None => Before::Part { work, index },
        })
    };
    let untracked = untracked_tree(executor, of, of.untracked, &mut not_copied, cancel).await?;
    let count = of.tracked.len() + of.untracked.len();
    let summary = format!(
        "{} {count} {} on {}",
        of.operation.word(),
        if count == 1 { "path" } else { "paths" },
        branch.as_deref().unwrap_or("a detached HEAD")
    );
    Ok(Some(Copied {
        head,
        head_tree,
        branch,
        before,
        untracked,
        not_copied,
        tracked: of.tracked.to_vec(),
        untracked_paths: of.untracked.to_vec(),
        operation: of.operation,
        touches_index: of.touches_index,
        moved: of
            .moved
            .map(|(reference, old, new)| (reference.to_string(), old, new)),
        summary,
    }))
}

/// Puts a copy on the record once the write it was made for answered:
/// what the paths held before, over what the write left — the index and
/// the working tree as the write leaves them where it landed, read as they
/// are now where it failed part-way. Nothing goes on where the write left
/// it all as it was. Whether a line went on.
///
/// The copy is stash-shaped (§2.1), as `git stash` reads one: its tree the
/// working tree's side before, its first parent the base — what the write
/// left in the working tree — its second the index's side over the same
/// base, its third the untracked files that went. The base is HEAD itself
/// where it holds what HEAD does, else a commit made on HEAD for it, which
/// `Stands-on:` names so the graph draws the copy on HEAD. A `reset
/// --hard`'s copy stands on the commit it moved off: what it took is all
/// the work over that commit.
pub async fn record_copy(
    executor: &GitExecutor,
    workdir: &Path,
    git_dir: &Path,
    copied: &Copied,
    landed: bool,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let of = CopyOf {
        workdir,
        git_dir,
        tracked: &copied.tracked,
        untracked: &copied.untracked_paths,
        operation: copied.operation,
        touches_index: copied.touches_index,
        moved: None,
    };
    let head_tree = copied.head_tree;
    let reset = copied.operation == CopyOperation::ResetHard;
    // Whether tracked work went, and the trees: the base's (what the write
    // left in the working tree), the index side's and the working tree's.
    let (tracked_taken, base_tree, index_side, work) = match &copied.before {
        None => (false, head_tree, head_tree, head_tree),
        Some(Before::Whole(before)) => {
            let left = if reset && landed {
                None
            } else {
                Some(left_by(executor, &of, copied, before, landed, cancel).await?)
            };
            let taken = left.is_none_or(|left| left != *before);
            match left {
                Some(left) if !reset => {
                    (taken, left.work, index_side_of(before, &left), before.work)
                }
                _ => (taken, head_tree, before.index, before.work),
            }
        }
        Some(Before::Part { work, index }) => {
            let mut not_copied = Vec::new();
            index
                .take(executor, &of, &copied.tracked, &mut not_copied, cancel)
                .await?;
            let left = index.tree(executor, &of, &copied.head, cancel).await?;
            (left != *work, left, left, *work)
        }
    };
    let untracked = if landed {
        copied.untracked
    } else {
        untracked_taken(executor, &of, copied.untracked, cancel).await?
    };
    let unnamed_taken = landed && !copied.not_copied.is_empty();
    if !tracked_taken && untracked.is_none() && !unnamed_taken {
        return Ok(false);
    }

    let base = base_of(executor, workdir, copied, &base_tree, cancel).await?;
    let on = copied.branch.as_deref().unwrap_or("(no branch)");
    let index = commit(
        executor,
        workdir,
        &index_side,
        &[base],
        &format!("index on {on}"),
        cancel,
    )
    .await?;
    let mut parents = vec![base, index];
    if let Some(tree) = &untracked {
        let made = format!("untracked files on {on}");
        parents.push(commit(executor, workdir, tree, &[], &made, cancel).await?);
    }
    let mut trailers = vec![
        ("Operation", copied.operation.word().to_string()),
        ("Working-copy", workdir.to_string_lossy().into_owned()),
        ("Worktree", worktree_id(git_dir)),
    ];
    if let Some(branch) = &copied.branch {
        trailers.push(("Branch", branch.clone()));
    }
    trailers.extend(
        copied
            .not_copied
            .iter()
            .map(|path| ("Not-copied", path.clone())),
    );
    if let Some((reference, old, new)) = &copied.moved {
        trailers.push(("Moved", moved_value(reference, old, new)));
    }
    if base != copied.head {
        trailers.push(("Stands-on", copied.head.to_hex()));
    }
    trailers.push(("Nonce", nonce()));
    let copy = commit(
        executor,
        workdir,
        &work,
        &parents,
        &message(&copied.summary, &trailers),
        cancel,
    )
    .await?;
    append(executor, workdir, git_dir, &copy, &copied.summary, cancel).await?;
    Ok(true)
}

/// What the write left at the copy's tracked paths, whole files. A write
/// that landed says so itself: an unstaged row's discard takes the working
/// tree back to the index and leaves the index; a staged row's takes both
/// back to HEAD, read off the index now. A write that failed part-way is
/// read as it is now.
async fn left_by(
    executor: &GitExecutor,
    of: &CopyOf<'_>,
    copied: &Copied,
    before: &Sides,
    landed: bool,
    cancel: &CancellationToken,
) -> Result<Sides, GitError> {
    let head = &copied.head;
    if landed && !copied.touches_index {
        return Ok(Sides {
            index: before.index,
            work: before.index,
        });
    }
    let now = IndexCopy::now(executor, of, head, cancel).await?;
    let index = now.tree(executor, of, head, cancel).await?;
    if landed {
        return Ok(Sides { index, work: index });
    }
    let mut not_copied = Vec::new();
    now.take(executor, of, of.tracked, &mut not_copied, cancel)
        .await?;
    let work = now.tree(executor, of, head, cancel).await?;
    Ok(Sides { index, work })
}

/// The commit a copy stands on: HEAD where `tree` (what the write left in
/// the working tree) is HEAD's own, else a commit of it made on HEAD.
async fn base_of(
    executor: &GitExecutor,
    workdir: &Path,
    copied: &Copied,
    tree: &Oid,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    if *tree == copied.head_tree {
        return Ok(copied.head);
    }
    let made = format!("left by: {}", copied.summary);
    commit(executor, workdir, tree, &[copied.head], &made, cancel).await
}

/// HEAD's commit, its tree and the branch it has out (none while
/// detached); none while HEAD has no commit.
async fn head_of(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<(Oid, Oid, Option<String>)>, GitError> {
    let Some(head) = commit_of(executor, workdir, "HEAD", cancel).await? else {
        return Ok(None);
    };
    // The tree by the commit's id, so both are the same HEAD's; the branch
    // by HEAD's full name, which is `HEAD` itself while detached.
    let cmd = GitCommand::new()
        .cwd(workdir)
        .arg("rev-parse")
        .arg(format!("{}^{{tree}}", head.to_hex()))
        .args(["--symbolic-full-name", "HEAD"]);
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let mut lines = text.lines();
    let tree = oid_out(lines.next().unwrap_or_default(), "git rev-parse")?;
    let branch = lines
        .next()
        .and_then(|name| name.trim().strip_prefix("refs/heads/"))
        .map(str::to_string);
    Ok(Some((head, tree, branch)))
}

/// The copy's index side, over the same base as its working tree's: what
/// the write took from the index, put on what it left in the working tree.
/// The index untouched (an unstaged row's discard), that is the base
/// itself; the index left as the working tree was (a
/// staged row's discard, back to HEAD in both), the index as it was. A
/// write that failed part-way, leaving the two apart where it changed the
/// index, keeps the index as it was.
fn index_side_of(before: &Sides, left: &Sides) -> Oid {
    if left.index == before.index {
        left.work
    } else {
        before.index
    }
}

/// The untracked files a write that failed part-way did take: the copy's
/// untracked tree less what still stands. None where nothing went.
async fn untracked_taken(
    executor: &GitExecutor,
    of: &CopyOf<'_>,
    untracked: Option<Oid>,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    let Some(tree) = untracked else {
        return Ok(None);
    };
    let cmd = GitCommand::new()
        .cwd(of.workdir)
        .args(["ls-tree", "-r", "-z", "--name-only", "--end-of-options"])
        .arg(tree.to_hex());
    let out = executor.run(cmd, cancel).await?;
    let standing: Vec<String> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).into_owned())
        .filter(|path| std::fs::symlink_metadata(of.workdir.join(path)).is_ok())
        .collect();
    if standing.is_empty() {
        return Ok(Some(tree));
    }
    let index = ScratchFile::fresh(of.git_dir, "copy-index").map_err(io_error)?;
    let read = isolated(of.workdir, of.git_dir, index.path())
        .arg("read-tree")
        .arg(tree.to_hex());
    executor.run(read, cancel).await?;
    for chunk in standing.chunks(PATHS_PER_CALL) {
        let cmd = isolated(of.workdir, of.git_dir, index.path())
            .args(["update-index", "--force-remove", "--"])
            .args(chunk);
        executor.run(cmd, cancel).await?;
    }
    let left = write_tree(executor, of.workdir, of.git_dir, index.path(), cancel).await?;
    let empty = empty_tree(executor, of.workdir, of.git_dir, cancel).await?;
    Ok((left != empty).then_some(left))
}

/// The untracked `paths` as a tree of their own, from an empty index; none
/// where nothing of them is taken.
async fn untracked_tree(
    executor: &GitExecutor,
    of: &CopyOf<'_>,
    paths: &[String],
    not_copied: &mut Vec<String>,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    if paths.is_empty() {
        return Ok(None);
    }
    let index = IndexCopy {
        file: ScratchFile::fresh(of.git_dir, "copy-index").map_err(io_error)?,
    };
    index.take(executor, of, paths, not_copied, cancel).await?;
    let tree = write_tree(executor, of.workdir, of.git_dir, index.file.path(), cancel).await?;
    let empty = empty_tree(executor, of.workdir, of.git_dir, cancel).await?;
    Ok((tree != empty).then_some(tree))
}

/// A temporary index (§2.1), dropped with the handle.
#[derive(Debug)]
struct IndexCopy {
    file: ScratchFile,
}

impl IndexCopy {
    /// One begun from what the working copy's own index holds now: a copy
    /// of its file, dated as the file is. With no index file yet, HEAD's
    /// tree is read in.
    ///
    /// The date is part of what the index says: an entry stamped in the
    /// same instant the index was written may hide a later change of the
    /// same size, and git looks into such a file rather than trust its stat
    /// ("racy git"). A copy dated when it was made would trust it, and
    /// copy the file as the index has it. Read before the bytes, so a
    /// newer index under it only makes more entries looked into.
    async fn now(
        executor: &GitExecutor,
        of: &CopyOf<'_>,
        head: &Oid,
        cancel: &CancellationToken,
    ) -> Result<Self, GitError> {
        let file = ScratchFile::fresh(of.git_dir, "copy-index").map_err(io_error)?;
        let index = of.git_dir.join("index");
        let copied = std::fs::metadata(&index)
            .and_then(|meta| meta.modified())
            .and_then(|written| {
                std::fs::copy(&index, file.path())?;
                std::fs::File::options()
                    .write(true)
                    .open(file.path())?
                    .set_modified(written)
            });
        match copied {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let read = isolated(of.workdir, of.git_dir, file.path())
                    .arg("read-tree")
                    .arg(head.to_hex());
                executor.run(read, cancel).await?;
            }
            Err(error) => return Err(io_error(error)),
        }
        Ok(Self { file })
    }

    /// The tree the index holds. A path in conflict takes HEAD's version
    /// (gone where HEAD has none) — the work is the working tree's — since
    /// `write-tree` writes nothing over an unmerged entry (exit 128).
    async fn tree(
        &self,
        executor: &GitExecutor,
        of: &CopyOf<'_>,
        head: &Oid,
        cancel: &CancellationToken,
    ) -> Result<Oid, GitError> {
        let cmd = isolated(of.workdir, of.git_dir, self.file.path())
            .arg("write-tree")
            .answers_by_code(128);
        let out = executor.run_unchecked(cmd, cancel).await?;
        if out.code == 0 {
            return oid_out(&out.stdout_utf8(), "git write-tree");
        }
        self.settle_conflicts(executor, of, head, cancel).await?;
        write_tree(executor, of.workdir, of.git_dir, self.file.path(), cancel).await
    }

    async fn settle_conflicts(
        &self,
        executor: &GitExecutor,
        of: &CopyOf<'_>,
        head: &Oid,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        let unmerged = isolated(of.workdir, of.git_dir, self.file.path()).args([
            "ls-files",
            "--unmerged",
            "-z",
        ]);
        let out = executor.run(unmerged, cancel).await?;
        let mut paths: Vec<String> = out
            .stdout
            .split(|b| *b == 0)
            .filter_map(|record| {
                let record = String::from_utf8_lossy(record);
                record.split_once('\t').map(|(_, path)| path.to_string())
            })
            .collect();
        paths.sort();
        paths.dedup();
        for chunk in paths.chunks(PATHS_PER_CALL) {
            let cmd = isolated(of.workdir, of.git_dir, self.file.path())
                .args(["update-index", "--force-remove", "--"])
                .args(chunk);
            executor.run(cmd, cancel).await?;
            let listed = GitCommand::new()
                .cwd(of.workdir)
                .args(["ls-tree", "-z", "--full-tree", "--end-of-options"])
                .arg(head.to_hex())
                .arg("--")
                .args(chunk.iter().map(|path| literal_pathspec(path)));
            let out = executor.run(listed, cancel).await?;
            let mut put =
                isolated(of.workdir, of.git_dir, self.file.path()).args(["update-index", "--add"]);
            let mut any = false;
            for record in out.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
                let record = String::from_utf8_lossy(record);
                let Some((meta, path)) = record.split_once('\t') else {
                    continue;
                };
                if let [mode, _, object] = meta.split(' ').collect::<Vec<_>>()[..] {
                    put = put.args(["--cacheinfo", &format!("{mode},{object},{path}")]);
                    any = true;
                }
            }
            if any {
                executor.run(put, cancel).await?;
            }
        }
        Ok(())
    }

    /// Takes `paths` from the working tree as `add --all` takes them — a
    /// path no longer there goes from the index. A path git will not take
    /// is left as it was and goes into `not_copied`: the paths are tried
    /// together, and one by one only when that fails.
    async fn take(
        &self,
        executor: &GitExecutor,
        of: &CopyOf<'_>,
        paths: &[String],
        not_copied: &mut Vec<String>,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        let index = self.file.path();
        // `add` refuses a pathspec that matches nothing, and a path gone from
        // the disk and the index alike matches nothing.
        let (present, missing): (Vec<&String>, Vec<&String>) = paths
            .iter()
            .partition(|path| std::fs::symlink_metadata(of.workdir.join(path.as_str())).is_ok());
        for chunk in missing.chunks(PATHS_PER_CALL) {
            let cmd = isolated(of.workdir, of.git_dir, index)
                .args(["update-index", "--force-remove", "--"])
                .args(chunk.iter().map(|path| path.as_str()));
            executor.run(cmd, cancel).await?;
        }
        // `--sparse`: in a sparse checkout `add` refuses a path outside its
        // definition, which a temporary index has no reason to.
        let add = |paths: &[&String]| {
            isolated(of.workdir, of.git_dir, index)
                .args(["add", "--sparse", "--all", "--force", "--"])
                .args(paths.iter().map(|path| literal_pathspec(path)))
                .answers_by_code(128)
        };
        for chunk in present.chunks(PATHS_PER_CALL) {
            if executor.run_unchecked(add(chunk), cancel).await?.code == 0 {
                continue;
            }
            for path in chunk {
                if executor.run_unchecked(add(&[*path]), cancel).await?.code != 0
                    && !not_copied.contains(*path)
                {
                    not_copied.push((*path).clone());
                }
            }
        }
        Ok(())
    }
}
