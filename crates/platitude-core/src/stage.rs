//! Staging: whole files, and hunk/line subsets via rebuilt partial patches.
//!
//! Path arguments always come from git's own output and are passed after
//! `--`, each wrapped as `:(literal)` ([`crate::process::literal_pathspec`]),
//! so a file literally named `:(glob)x` cannot turn into a pathspec.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::details::{self, DiffTarget};
use crate::error::GitError;
use crate::patch::{self, HunkSelect, PatchSide};
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::refs;
use crate::repo::RepoInfo;
use crate::scratch::ScratchFile;

/// `git add -- <paths>`: stages modifications, additions and deletions.
pub async fn stage_paths(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["add", "--"])
        .args(paths.iter().map(|p| literal_pathspec(p)));
    executor.run(cmd, cancel).await.map(drop)
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
    let unborn = refs::head_state(executor, workdir, cancel)
        .await?
        .oid
        .is_none();
    let cmd = if unborn {
        // No HEAD to reset to; drop every entry instead. `--ignore-unmatch`
        // because an empty index matches nothing and `git rm` calls that
        // fatal — but "unstage nothing" has succeeded at its job (実測:
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
    if paths.is_empty() {
        return Ok(());
    }
    let unborn = refs::head_state(executor, workdir, cancel)
        .await?
        .oid
        .is_none();
    let cmd = if unborn {
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
    let cmd = cmd.args(paths.iter().map(|p| literal_pathspec(p)));
    executor.run(cmd, cancel).await.map(drop)
}

/// `git restore --worktree -- <paths>`: throws away unstaged modifications
/// of tracked files. Destructive — the caller confirms first.
pub async fn discard_worktree(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["restore", "--worktree", "--"])
        .args(paths.iter().map(|p| literal_pathspec(p)));
    executor.run(cmd, cancel).await.map(drop)
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
    if paths.is_empty() {
        return Ok(());
    }
    let unborn = refs::head_state(executor, workdir, cancel)
        .await?
        .oid
        .is_none();
    let cmd = if unborn {
        GitCommand::new()
            .cwd(workdir)
            .args(["rm", "-f", "-r", "--quiet", "--"])
    } else {
        GitCommand::new()
            .cwd(workdir)
            .args(["restore", "--staged", "--worktree", "--"])
    };
    let cmd = cmd.args(paths.iter().map(|p| literal_pathspec(p)));
    executor.run(cmd, cancel).await.map(drop)
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
    if paths.is_empty() {
        return Ok(());
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["clean", "-f", "-d", "--"])
        .args(paths.iter().map(|p| literal_pathspec(p)));
    executor.run(cmd, cancel).await.map(drop)
}

/// Stages or unstages part of one file's diff.
///
/// The diff is re-run here rather than taken from the caller so the bytes
/// the selection indexes into are exactly the bytes being rebuilt. A
/// concurrent edit therefore changes what the indices mean; the session
/// serializes writes and refreshes afterwards, and `git apply` rejects a
/// patch that no longer fits.
///
/// [`DiffTarget::Untracked`] is staged with intent-to-add first (git's own
/// requirement for partially staging a new file), then treated as unstaged.
pub async fn apply_partial(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if selects.is_empty() {
        return Ok(());
    }
    let workdir = repo.workdir.as_path();

    let (target, side, intent_path) = match target {
        DiffTarget::Unstaged { .. } => (target.clone(), PatchSide::Forward, None),
        DiffTarget::Staged { .. } => (target.clone(), PatchSide::Reverse, None),
        DiffTarget::Untracked { path } => {
            let cmd = GitCommand::new()
                .cwd(workdir)
                .args(["add", "--intent-to-add", "--"])
                .arg(literal_pathspec(path));
            executor.run(cmd, cancel).await?;
            (
                DiffTarget::Unstaged { path: path.clone() },
                PatchSide::Forward,
                Some(path.clone()),
            )
        }
        DiffTarget::Commit { .. } => {
            return Err(GitError::UnexpectedOutput {
                command: "git apply --cached".to_string(),
                message: "a committed diff cannot be staged".to_string(),
            });
        }
    };

    let result = apply_prepared(executor, repo, &target, selects, side, cancel).await;
    if result.is_err() {
        // The intent-to-add mark has already moved the file out of the
        // untracked bucket; a failure must not leave it half-staged with
        // nothing actually staged. Best-effort — the failure itself is
        // what the caller surfaces.
        if let Some(path) = intent_path
            && let Err(undo) = unstage_paths(executor, workdir, &[path], cancel).await
        {
            tracing::warn!(
                %undo,
                "could not undo intent-to-add after a failed partial stage"
            );
        }
    }
    result
}

/// The staging half of [`apply_partial`], once the target is one a diff
/// can be built from.
async fn apply_prepared(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    side: PatchSide,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let workdir = repo.workdir.as_path();
    let raw = details::file_diff_raw(executor, workdir, target, cancel).await?;
    let Some(built) = patch::build_partial(&raw, selects, side) else {
        // The selection indexes a diff that no longer holds it — the file
        // changed under the open diff. Doing nothing must not read as the
        // write having landed.
        return Err(GitError::Rejected {
            message: "the file changed on disk; the selected part is no longer \
                      in its diff"
                .to_string(),
        });
    };

    let scratch = ScratchFile::create(&repo.git_dir, "stage.patch", &built).map_err(|source| {
        GitError::Io {
            command: "git apply --cached".to_string(),
            source,
        }
    })?;

    let mut cmd = GitCommand::new()
        .cwd(workdir)
        // `--whitespace=nowarn` pins behavior against `apply.whitespace`:
        // the patch must land byte-for-byte, never "fixed".
        .args(["apply", "--cached", "--whitespace=nowarn"]);
    if side == PatchSide::Reverse {
        cmd = cmd.arg("--reverse");
    }
    cmd = cmd.arg(scratch.path());
    executor.run(cmd, cancel).await.map(drop)
}

/// Throws away part of one file's unstaged diff: the selection is built
/// the way [`apply_partial`] builds an unstaging one — the post-image side
/// stays whole, since that is the side the patch has to fit — and applied
/// in reverse to the working tree alone. Without `--cached` the index is
/// not touched, so what is staged survives.
///
/// Only the unstaged side gets here. A staged hunk is unstaged first (that
/// is what the staged side's affordance does) and thrown away from the
/// unstaged side afterwards; an untracked file has no pre-image to restore
/// part of, so it goes whole or not at all.
///
/// Destructive — the caller confirms first.
pub async fn discard_partial(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if selects.is_empty() {
        return Ok(());
    }
    if !matches!(target, DiffTarget::Unstaged { .. }) {
        return Err(GitError::UnexpectedOutput {
            command: "git apply --reverse".to_string(),
            message: "only unstaged changes can be discarded piecemeal".to_string(),
        });
    }
    let workdir = repo.workdir.as_path();
    let raw = details::file_diff_raw(executor, workdir, target, cancel).await?;
    let Some(built) = patch::build_partial(&raw, selects, PatchSide::Reverse) else {
        // As in apply_partial: a vanished selection is a refusal, not a
        // discard that quietly did nothing.
        return Err(GitError::Rejected {
            message: "the file changed on disk; the selected part is no longer \
                      in its diff"
                .to_string(),
        });
    };

    let scratch =
        ScratchFile::create(&repo.git_dir, "discard.patch", &built).map_err(|source| {
            GitError::Io {
                command: "git apply --reverse".to_string(),
                source,
            }
        })?;
    let cmd = GitCommand::new()
        .cwd(workdir)
        // Same pinning as the staging path: the patch lands byte-for-byte
        // or not at all.
        .args(["apply", "--whitespace=nowarn", "--reverse"])
        .arg(scratch.path());
    executor.run(cmd, cancel).await.map(drop)
}

/// Number of hunks in the diff a target currently produces (the range the
/// UI may address with [`HunkSelect`]).
pub async fn hunk_count(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Result<usize, GitError> {
    let raw = details::file_diff_raw(executor, workdir, target, cancel).await?;
    Ok(patch::hunk_count(&raw))
}
