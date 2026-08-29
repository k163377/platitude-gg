//! Whole files: staging, unstaging and the three ways of throwing a
//! file's changes away.
//!
//! Path arguments always come from git's own output and are passed after
//! `--`, each wrapped as `:(literal)` ([`crate::process::literal_pathspec`]),
//! so a file literally named `:(glob)x` cannot turn into a pathspec.

use std::collections::HashSet;
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::refs;
use crate::status::{self, StatusItem};

async fn run_over_paths(
    executor: &GitExecutor,
    cmd: GitCommand,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let cmd = cmd.args(paths.iter().map(|p| literal_pathspec(p)));
    executor.run(cmd, cancel).await.map(drop)
}

/// Whether HEAD names a commit yet.
///
/// Before the first one there is nothing to restore a path from, and the
/// commands that would take one back to HEAD have to empty it out of the
/// index instead.
async fn head_is_unborn(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    Ok(refs::head_state(executor, workdir, cancel)
        .await?
        .oid
        .is_none())
}

/// `git add -- <paths>`: stages modifications, additions and deletions.
pub async fn stage_paths(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir).args(["add", "--"]);
    run_over_paths(executor, cmd, paths, cancel).await
}

/// `git add --all`: stages modifications, additions and deletions, plus the
/// files git is not tracking yet. Staging all means all — the tracked-only
/// `--update` variant is deliberately not offered.
pub async fn stage_all(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir).args(["add", "--all"]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Empties the index back to HEAD, leaving the working tree alone.
pub async fn unstage_all(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = if head_is_unborn(executor, workdir, cancel).await? {
        // No HEAD to reset to; drop every entry instead. `--ignore-unmatch`
        // because an empty index matches nothing and `git rm` calls that
        // fatal — but "unstage nothing" has succeeded at its job (measured:
        // `rm --cached -r -- .` in a fresh `git init` exits 128).
        GitCommand::new().cwd(workdir).args([
            "rm",
            "--cached",
            "-r",
            "--ignore-unmatch",
            "--quiet",
            "--",
            ":(literal).",
        ])
    } else {
        GitCommand::new().cwd(workdir).args(["reset", "--quiet"])
    };
    executor.run(cmd, cancel).await.map(drop)
}

/// Removes staged changes for `paths`, keeping the working tree as-is.
///
/// `git restore --staged` needs a HEAD to restore from; on an unborn branch
/// the equivalent is dropping the entries from the index entirely.
pub async fn unstage_paths(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // Asked here as well as in `run_over_paths`: nothing to unstage must
    // not cost the spawn that reading HEAD takes.
    if paths.is_empty() {
        return Ok(());
    }
    let cmd = if head_is_unborn(executor, workdir, cancel).await? {
        GitCommand::new()
            .cwd(workdir)
            // `--ignore-unmatch` as in unstage_all: a path that is not in
            // the index is already unstaged, not a fatal error.
            .args(["rm", "--cached", "-r", "--ignore-unmatch", "--quiet", "--"])
    } else {
        GitCommand::new()
            .cwd(workdir)
            .args(["restore", "--staged", "--"])
    };
    run_over_paths(executor, cmd, paths, cancel).await
}

/// `git restore --worktree -- <paths>`: throws away unstaged modifications
/// of tracked files. Destructive — the caller confirms first.
pub async fn discard_worktree(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["restore", "--worktree", "--"]);
    run_over_paths(executor, cmd, paths, cancel).await
}

/// `git restore --staged --worktree -- <paths>`: throws away both sides at
/// once, back to HEAD — what is staged and what is on disk. With
/// `--staged` git restores from HEAD rather than from the index, so a path
/// HEAD does not have goes from disk with it: a file staged as new is
/// deleted, and so is the new name of a rename — whose old name must be
/// passed alongside it, or its staged deletion is left standing (measured).
///
/// Before the first commit there is no HEAD to restore from, and `git rm`
/// is the same journey: out of the index and off the disk.
///
/// Destructive — the caller confirms first.
pub async fn discard_to_head(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // As in unstage_paths: asked before HEAD is read, not only before the
    // command runs.
    if paths.is_empty() {
        return Ok(());
    }
    let cmd = if head_is_unborn(executor, workdir, cancel).await? {
        GitCommand::new()
            .cwd(workdir)
            .args(["rm", "-f", "-r", "--quiet", "--"])
    } else {
        GitCommand::new()
            .cwd(workdir)
            .args(["restore", "--staged", "--worktree", "--"])
    };
    run_over_paths(executor, cmd, paths, cancel).await
}

/// `git clean -f -d -- <paths>`: deletes untracked files. `status -uall`
/// hands us one path per file, and `-f` alone already deletes a file inside
/// an untracked directory; `-d` is kept so a directory pathspec still takes
/// the whole tree. An emptied parent directory stays on disk — git does not
/// track directories, so status stays clean. Destructive — the caller
/// confirms first.
pub async fn remove_untracked(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["clean", "-f", "-d", "--"]);
    run_over_paths(executor, cmd, paths, cancel).await
}

/// Which side of the working tree a chosen row was standing on. Carried
/// with the path rather than looked up again: a file changed on both
/// sides has a row in each bucket, and which of them was chosen decides
/// whether what is staged survives the discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardSide {
    /// An unstaged edit: the disk goes back to the index, what is staged
    /// stays.
    Unstaged,
    /// An untracked file: there the file itself is the change, so it goes.
    Untracked,
    /// A staged change: both sides go, back to HEAD.
    Staged,
}

/// Discards a chosen set of rows: each side goes by its own command — at
/// most three for the lot, however many rows were chosen (デザイン規約
/// §その他の操作) — and a side that stops the run stops it before the
/// next side is touched.
///
/// A staged rename is undone by both of its names at once (see
/// [`discard_to_head`]). Which staged paths are renames is read from
/// status here, in the same write as the commands, rather than gathered
/// by the caller: a list made in the UI predates whatever writes are
/// queued ahead of this one — the reasoning
/// [`stage_conflicted`](crate::session::RepoSession::stage_conflicted)
/// spells out.
///
/// Destructive — the caller confirms first.
pub async fn discard_chosen(
    executor: &GitExecutor,
    workdir: &Path,
    choices: &[(String, DiscardSide)],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();
    let mut staged = Vec::new();
    for (path, side) in choices {
        match side {
            DiscardSide::Unstaged => unstaged.push(path.clone()),
            DiscardSide::Untracked => untracked.push(path.clone()),
            DiscardSide::Staged => staged.push(path.clone()),
        }
    }
    if !staged.is_empty() {
        let current = status::load(executor, workdir, cancel).await?;
        let chosen: HashSet<&str> = staged.iter().map(String::as_str).collect();
        // Renames only: undoing `R` needs both of its names or the old one
        // stays staged as a deletion. A copy (`C`) carries `orig_path` too,
        // but its source is a live file with rows of its own — pulling it
        // in here would reset a file the user never chose.
        let old_names: Vec<String> = current
            .staged()
            .filter_map(|item| match item {
                StatusItem::Tracked {
                    path,
                    orig_path: Some(orig),
                    staged: 'R',
                    ..
                } if chosen.contains(path.as_str()) => Some(orig.clone()),
                _ => None,
            })
            .collect();
        staged.extend(old_names);
    }
    discard_worktree(executor, workdir, &unstaged, cancel).await?;
    remove_untracked(executor, workdir, &untracked, cancel).await?;
    discard_to_head(executor, workdir, &staged, cancel).await
}
