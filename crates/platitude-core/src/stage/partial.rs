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

use super::endings;
use super::refusal::{
    locked_kind, locked_out, moved, refuse_combined, stale, taking, verify_fingerprint,
};
use super::whole::unstage_paths;

/// Stages or unstages part of one file's diff.
///
/// The diff is re-run here and `seen` — the fingerprint of the diff the
/// selection was made on ([`details::file_diff_with_fingerprint`]) — is
/// checked against it; any drift is a refusal. `git apply` cannot be the
/// net: a patch rebuilt from drifted bytes always fits them. A file that
/// moves after this read so the patch no longer fits is git's to find, and
/// the same refusal once a second read shows the move.
///
/// [`DiffTarget::Untracked`] is marked intent-to-add first (git's own
/// requirement for partially staging a new file), then treated as unstaged.
/// Its fingerprint is checked *before* the mark: marking first would change
/// which command the diff even is. The diff read after the mark, which the
/// patch is built from, has to hold the same hunks as the one checked — a
/// file moved in between would have the selection counted in other lines.
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
        DiffTarget::Unstaged { .. } => (
            target.clone(),
            PatchSide::Forward,
            Verify::Print(seen),
            None,
        ),
        DiffTarget::Staged { .. } => (
            target.clone(),
            PatchSide::Reverse,
            Verify::Print(seen),
            None,
        ),
        DiffTarget::Untracked { path } => {
            let raw = details::file_diff_raw(executor, workdir, target, cancel).await?;
            verify_fingerprint(&raw, seen, taking(PatchSide::Forward))?;
            let cmd = GitCommand::new()
                .cwd(workdir)
                .args(["add", "--intent-to-add", "--"])
                .arg(literal_pathspec(path));
            executor
                .run(cmd, cancel)
                .await
                .map_err(locked_out(ReportKind::StageLocked))?;
            (
                DiffTarget::Unstaged { path: path.clone() },
                PatchSide::Forward,
                Verify::Hunks(raw),
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

/// What the diff read for the patch is checked against: the fingerprint the
/// selection was made on, or — for an untracked file, whose fingerprint was
/// checked on the `--no-index` read before the intent-to-add mark — that
/// read's hunks.
enum Verify {
    Print(u64),
    Hunks(Vec<u8>),
}

/// The staging half of [`apply_partial`].
async fn apply_prepared(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    side: PatchSide,
    verify: Verify,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let workdir = repo.workdir.as_path();
    let raw = details::file_diff_raw(executor, workdir, target, cancel).await?;
    match &verify {
        Verify::Print(seen) => verify_fingerprint(&raw, *seen, taking(side))?,
        Verify::Hunks(checked) if hunks_of(checked) != hunks_of(&raw) => {
            return Err(stale(taking(side)));
        }
        Verify::Hunks(_) => {}
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
    apply_built(executor, workdir, target, &raw, cmd, taking(side), cancel).await
}

/// Runs git's apply of a patch built from `raw`. Exit 1 is `apply` finding
/// the patch does not fit what is there (`patch does not apply`, `does not
/// exist in index`, `No such file or directory`) with nothing written; when
/// the diff reads differently now, the file moved after the fingerprint
/// check and it is `kind`'s refusal. Another git holding the index (128 —
/// only `--cached` takes its lock) is the lock's report for the same verb
/// ([`locked_kind`]). A diff that reads the same was turned down for a
/// reason of git's own, and git's words are the answer.
async fn apply_built(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    raw: &[u8],
    cmd: GitCommand,
    kind: ReportKind,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    // A read that fails says nothing about a move; git's words stand.
    if out.code == 1
        && details::file_diff_raw(executor, workdir, target, cancel)
            .await
            .is_ok_and(|now| now != raw)
    {
        return Err(moved(kind, command, &out));
    }
    Err(locked_out(locked_kind(kind))(GitError::Failed {
        command,
        code: out.code,
        stderr: out.failure_message(),
    }))
}

/// A diff's hunks, its file header left off — what a selection's indexes
/// count in. The `--no-index` read of an untracked file and the read of the
/// same file marked intent-to-add differ in the header alone.
fn hunks_of(raw: &[u8]) -> &[u8] {
    if raw.starts_with(b"@@") {
        return raw;
    }
    raw.windows(3)
        .position(|w| w == b"\n@@")
        .map_or(&[], |at| &raw[at + 1..])
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
    let DiffTarget::Unstaged { path } = target else {
        return Err(GitError::UnexpectedOutput {
            command: "git apply --reverse".to_string(),
            message: "only unstaged changes can be discarded piecemeal".to_string(),
        });
    };
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
    let mut cmd = GitCommand::new().cwd(workdir);
    if let Some(setting) = endings::kept(executor, workdir, path, &built, cancel).await? {
        cmd = cmd.args(["-c", setting]);
    }
    let cmd = cmd
        // Pinned against `apply.whitespace`, as when staging.
        .args(["apply", "--whitespace=nowarn", "--reverse"])
        .arg(scratch.path());
    let kind = ReportKind::StaleDiscard;
    apply_built(executor, workdir, target, &raw, cmd, kind, cancel).await
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
