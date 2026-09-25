//! Hunk and line subsets, applied as patches rebuilt from the diff the
//! selection was made on. Each carries that diff's fingerprint and refuses
//! outright where the file has moved on ([`super::refusal`]).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::details::{self, DiffTarget};
use crate::error::GitError;
use crate::patch::{self, HunkSelect, PatchSide};
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::repo::RepoInfo;
use crate::report::ReportKind;
use crate::scratch::ScratchFile;

use super::refusal::{refuse_combined, stale, taking, verify_fingerprint};
use super::whole::unstage_paths;

/// Stages or unstages part of one file's diff.
///
/// The diff is re-run here and `seen` — the fingerprint of the diff the
/// selection was made on ([`details::file_diff_with_fingerprint`]) — is
/// checked against it; any drift is a refusal. `git apply` cannot be the
/// net: a patch rebuilt from drifted bytes always fits them.
///
/// [`DiffTarget::Untracked`] is marked intent-to-add first (git's own
/// requirement for partially staging a new file), then treated as unstaged.
/// Its fingerprint is checked *before* the mark: marking first would change
/// which command the diff even is.
pub async fn apply_partial(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    seen: u64,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if selects.is_empty() {
        return Ok(());
    }
    let workdir = repo.workdir.as_path();

    let (target, side, verify, intent_path) = match target {
        DiffTarget::Unstaged { .. } => (target.clone(), PatchSide::Forward, Some(seen), None),
        DiffTarget::Staged { .. } => (target.clone(), PatchSide::Reverse, Some(seen), None),
        DiffTarget::Untracked { path } => {
            let raw = details::file_diff_raw(executor, workdir, target, cancel).await?;
            verify_fingerprint(&raw, seen, taking(PatchSide::Forward))?;
            let cmd = GitCommand::new()
                .cwd(workdir)
                .args(["add", "--intent-to-add", "--"])
                .arg(literal_pathspec(path));
            executor.run(cmd, cancel).await?;
            (
                DiffTarget::Unstaged { path: path.clone() },
                PatchSide::Forward,
                None,
                Some(path.clone()),
            )
        }
        DiffTarget::Commit { .. } | DiffTarget::Range { .. } | DiffTarget::Choice { .. } => {
            return Err(GitError::UnexpectedOutput {
                command: "git apply --cached".to_string(),
                message: "a committed diff cannot be staged".to_string(),
            });
        }
    };

    let result = apply_prepared(executor, repo, &target, selects, side, verify, cancel).await;
    if result.is_err() {
        // The intent-to-add mark already moved the file out of the
        // untracked bucket, so a failure puts it back (best-effort).
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

/// The staging half of [`apply_partial`]. `verify` is `None` when the
/// untracked arm already checked the fingerprint.
async fn apply_prepared(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    side: PatchSide,
    verify: Option<u64>,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let workdir = repo.workdir.as_path();
    let raw = details::file_diff_raw(executor, workdir, target, cancel).await?;
    if let Some(seen) = verify {
        verify_fingerprint(&raw, seen, taking(side))?;
    }
    refuse_combined(&raw)?;
    let Some(built) = patch::build_partial(&raw, selects, side) else {
        // The file changed under the open diff.
        return Err(stale(taking(side)));
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
        // the patch lands byte-for-byte.
        .args(["apply", "--cached", "--whitespace=nowarn"]);
    if side == PatchSide::Reverse {
        cmd = cmd.arg("--reverse");
    }
    cmd = cmd.arg(scratch.path());
    executor.run(cmd, cancel).await.map(drop)
}

/// Throws away part of one file's unstaged diff: built like an unstaging
/// selection (the post-image side stays whole, since that is the side the
/// patch has to fit) and applied in reverse to the working tree alone, so
/// what is staged survives.
///
/// Only the unstaged side gets here: a staged hunk is unstaged first, and
/// an untracked file has no pre-image to restore part of.
///
/// Destructive — the caller confirms first.
pub async fn discard_partial(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    seen: u64,
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
    // Destructive and index-addressed: bytes that drifted since the
    // selection was made would throw away the wrong lines.
    verify_fingerprint(&raw, seen, ReportKind::StaleDiscard)?;
    refuse_combined(&raw)?;
    let Some(built) = patch::build_partial(&raw, selects, PatchSide::Reverse) else {
        return Err(stale(ReportKind::StaleDiscard));
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
        // Pinned against `apply.whitespace`, as when staging.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Oid;
    use crate::refusing;

    fn an_oid() -> Oid {
        Oid::from_hex_str("0123456789abcdef0123456789abcdef01234567").expect("40 hex digits")
    }

    /// The three targets whose diff is of history.
    fn committed_targets() -> [DiffTarget; 3] {
        [
            DiffTarget::Commit {
                oid: an_oid(),
                parent: None,
                path: "f.txt".to_string(),
                orig_path: None,
            },
            DiffTarget::Range {
                from: an_oid(),
                to: an_oid(),
                path: "f.txt".to_string(),
                orig_path: None,
            },
            DiffTarget::Choice {
                oids: vec![an_oid(), an_oid()],
                path: "f.txt".to_string(),
                orig_path: None,
            },
        ]
    }

    /// A committed diff has no index entry to be written into. The pane
    /// offers no pieces on such a target; this is the floor under that.
    #[tokio::test]
    async fn no_diff_of_history_can_be_staged_and_none_of_them_reaches_git() {
        for target in committed_targets() {
            let (exec, asked) = refusing::git();
            let (repo, cancel) = (refusing::repo(), CancellationToken::new());
            let err = apply_partial(&exec, &repo, &target, &[HunkSelect::whole(0)], 0, &cancel)
                .await
                .expect_err("committed diffs are not stageable");
            assert!(
                err.to_string().contains("cannot be staged"),
                "{target:?}: {err}"
            );
            assert_eq!(asked.count(), 0, "{target:?}: nothing was asked of git");
        }
    }

    #[tokio::test]
    async fn only_the_unstaged_side_can_be_thrown_away_in_pieces() {
        let others = [
            DiffTarget::Staged {
                path: "f.txt".to_string(),
                orig_path: None,
            },
            DiffTarget::Untracked {
                path: "f.txt".to_string(),
            },
        ];
        for target in committed_targets().into_iter().chain(others) {
            let (exec, asked) = refusing::git();
            let (repo, cancel) = (refusing::repo(), CancellationToken::new());
            let err = discard_partial(&exec, &repo, &target, &[HunkSelect::whole(0)], 0, &cancel)
                .await
                .expect_err("only unstaged changes go piecemeal");
            assert!(
                err.to_string().contains("only unstaged changes"),
                "{target:?}: {err}"
            );
            assert_eq!(asked.count(), 0, "{target:?}: nothing was asked of git");
        }
    }
}
