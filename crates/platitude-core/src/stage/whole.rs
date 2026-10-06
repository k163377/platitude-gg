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

/// Whether HEAD names a commit yet. Before the first one, the commands
/// that would restore from HEAD have to empty the index instead.
async fn head_is_unborn(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    Ok(refs::head_tip(executor, workdir, cancel).await?.is_none())
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

/// `git add --all`: untracked files included — never the tracked-only
/// `--update`.
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
        // No HEAD to reset to; drop every entry instead. `--ignore-unmatch`:
        // `git rm` calls an empty index matching nothing fatal, but
        // "unstage nothing" has succeeded.
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
/// On an unborn branch (no HEAD to restore from) the entries are dropped
/// from the index instead.
pub async fn unstage_paths(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // Before HEAD is read, so an empty list costs no spawn.
    if paths.is_empty() {
        return Ok(());
    }
    let cmd = if head_is_unborn(executor, workdir, cancel).await? {
        GitCommand::new()
            .cwd(workdir)
            // `--ignore-unmatch` as in unstage_all: a path that is not in
            // the index is already unstaged.
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
pub async fn discard_working_tree(
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
/// once, back to HEAD. A path HEAD does not have goes from disk too: a
/// file staged as new is deleted, and so is the new name of a rename —
/// whose old name must be passed alongside it, or its staged deletion is
/// left standing. Before the first commit, `git rm` does the same.
///
/// Destructive — the caller confirms first.
pub async fn discard_to_head(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // As in unstage_paths.
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

/// `git clean -f -d -- <paths>`: deletes untracked files. `-d` so a
/// directory pathspec still takes the whole tree. An emptied parent
/// directory stays on disk, which status does not show. Destructive — the
/// caller confirms first.
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
/// with the path: a file changed on both sides has a row in each bucket,
/// and which was chosen decides whether what is staged survives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardSide {
    /// An unstaged edit: the disk goes back to the index, what is staged
    /// stays.
    Unstaged,
    /// An untracked file: the file itself goes.
    Untracked,
    /// A staged change: both sides go, back to HEAD.
    Staged,
}

/// Discards a chosen set of rows: [`with_old_names`], then
/// [`discard_rows`].
///
/// Destructive — the caller confirms first.
pub async fn discard_chosen(
    executor: &GitExecutor,
    workdir: &Path,
    choices: &[(String, DiscardSide)],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let rows = with_old_names(executor, workdir, choices, cancel).await?;
    discard_rows(executor, workdir, &rows, cancel).await
}

/// The chosen rows, and each staged rename's old name as a staged row of
/// its own: a staged rename is undone by both of its names (see
/// [`discard_to_head`]). Which staged paths are renames is read from
/// status here, in the write: a list made in the UI predates the writes
/// queued ahead of this one (as in
/// [`stage_conflicted`](crate::session::RepoSession::stage_conflicted)).
pub async fn with_old_names(
    executor: &GitExecutor,
    workdir: &Path,
    choices: &[(String, DiscardSide)],
    cancel: &CancellationToken,
) -> Result<Vec<(String, DiscardSide)>, GitError> {
    let mut rows = choices.to_vec();
    let chosen: HashSet<&str> = choices
        .iter()
        .filter(|(_, side)| *side == DiscardSide::Staged)
        .map(|(path, _)| path.as_str())
        .collect();
    if chosen.is_empty() {
        return Ok(rows);
    }
    let current = status::load_tracked(executor, workdir, cancel).await?;
    // Renames only: a copy (`C`) carries `orig_path` too, but its source is
    // a live file with rows of its own — pulling it in would reset a file
    // the user never chose.
    rows.extend(current.staged().filter_map(|item| match item {
        StatusItem::Tracked {
            path,
            orig_path: Some(orig),
            staged: 'R',
            ..
        } if chosen.contains(path.as_str()) => Some((orig.clone(), DiscardSide::Staged)),
        _ => None,
    }));
    Ok(rows)
}

/// Discards rows whose staged renames carry their old names already
/// ([`with_old_names`]): each side goes by its own command — at most three
/// for the lot (デザイン規約 §その他の操作) — and a side that fails stops
/// the run before the next side is touched.
///
/// Destructive — the caller confirms first.
pub async fn discard_rows(
    executor: &GitExecutor,
    workdir: &Path,
    rows: &[(String, DiscardSide)],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();
    let mut staged = Vec::new();
    for (path, side) in rows {
        match side {
            DiscardSide::Unstaged => unstaged.push(path.clone()),
            DiscardSide::Untracked => untracked.push(path.clone()),
            DiscardSide::Staged => staged.push(path.clone()),
        }
    }
    discard_working_tree(executor, workdir, &unstaged, cancel).await?;
    remove_untracked(executor, workdir, &untracked, cancel).await?;
    discard_to_head(executor, workdir, &staged, cancel).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;

    /// Not even the read of HEAD: every command here reads a missing
    /// pathspec as the whole work tree, and `git clean -f -d --` on its own
    /// would delete every untracked file.
    #[tokio::test]
    async fn an_empty_selection_runs_nothing() {
        let (exec, asked) = refusing::git();
        let (nowhere, cancel) = (refusing::nowhere(), CancellationToken::new());
        let workdir = nowhere.as_path();
        let none: [String; 0] = [];

        stage_paths(&exec, workdir, &none, &cancel)
            .await
            .expect("stage");
        unstage_paths(&exec, workdir, &none, &cancel)
            .await
            .expect("unstage");
        discard_working_tree(&exec, workdir, &none, &cancel)
            .await
            .expect("discard worktree");
        discard_to_head(&exec, workdir, &none, &cancel)
            .await
            .expect("discard to head");
        remove_untracked(&exec, workdir, &none, &cancel)
            .await
            .expect("remove untracked");
        discard_chosen(&exec, workdir, &[], &cancel)
            .await
            .expect("discard chosen");
        assert_eq!(asked.count(), 0, "git was asked anyway");

        // The same observer hears a call that has something to do, so the
        // silence above is the guard.
        stage_paths(&exec, workdir, &["a.txt".to_string()], &cancel)
            .await
            .expect_err("there is no git here to stage with");
        assert_eq!(asked.count(), 1);
    }
}
