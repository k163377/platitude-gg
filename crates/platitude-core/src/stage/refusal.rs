//! What a partial stage refuses: before git is asked, the fingerprint did
//! not match, the hunk the selection indexes is no longer in the diff, or
//! the file is conflicted and has no single old side to cut a patch
//! against; after, git turned the patch down and the file had moved.

use crate::details;
use crate::error::GitError;
use crate::patch::{self, PatchSide};
use crate::process::GitOutput;
use crate::report::{ReportKind, WriteReport};

/// Refuses a conflicted file's diff, which has more than one old side and
/// so cannot be cut into a patch that applies (`platitude_core::patch`).
/// The pane withholds the pieces on such a file; left to git, a write that
/// got here anyway would fail on a patch fragment nobody wrote.
pub(super) fn refuse_combined(raw: &[u8]) -> Result<(), GitError> {
    if patch::is_combined(raw) {
        return Err(stale(ReportKind::ConflictedPart));
    }
    Ok(())
}

/// Refuses a diff whose bytes are not the ones the selection indexed.
pub(super) fn verify_fingerprint(raw: &[u8], seen: u64, kind: ReportKind) -> Result<(), GitError> {
    if details::fingerprint(raw) == seen {
        return Ok(());
    }
    Err(stale(kind))
}

/// Which way a part was being taken, which is what the heading turns on:
/// "nothing was staged" for a part taken off the staged side would name
/// the wrong direction. Discarding never comes through here.
pub(super) fn taking(side: PatchSide) -> ReportKind {
    match side {
        PatchSide::Reverse => ReportKind::StaleUnstage,
        PatchSide::Forward => ReportKind::StaleStage,
    }
}

/// git turning down a rebuilt patch whose file moved after the fingerprint
/// check: [`stale`] found one step later. The screen says it the same way;
/// the log keeps git's words.
pub(super) fn moved(kind: ReportKind, command: String, out: &GitOutput) -> GitError {
    crate::report::reported(WriteReport::local(kind), command, out)
}

/// git turning a stage, an unstage or a discard down because another git
/// held the index's lock (`index.lock': File exists`, the same at the
/// minimum and 2.55) — a report, as a commit's is; any other failure goes
/// through as it was. `kind` is the lock's report for that verb
/// ([`ReportKind::StageLocked`] and the two beside it).
pub(super) fn locked_out(kind: ReportKind) -> impl Fn(GitError) -> GitError {
    move |error| match error {
        GitError::Failed {
            command,
            code,
            stderr,
        } if crate::report::lock_taken(&stderr) => GitError::Reported {
            command,
            code,
            stderr,
            report: Box::new(WriteReport::local(kind)),
        },
        other => other,
    }
}

/// The lock's report for the verb a part was taken in ([`taking`]'s or the
/// discard's stale kind).
pub(super) fn locked_kind(stale: ReportKind) -> ReportKind {
    match stale {
        ReportKind::StaleUnstage => ReportKind::UnstageLocked,
        ReportKind::StaleDiscard => ReportKind::DiscardLocked,
        _ => ReportKind::StageLocked,
    }
}

/// A refusal this end made before git was asked. Nothing ran, so the
/// screen says both lines in its own language (デザイン規約
/// §答えの要らない報せ).
pub(super) fn stale(kind: ReportKind) -> GitError {
    GitError::Reported {
        // No command ran, but the write log still prints this error
        // (`session::write`), so the kind is what it says.
        command: "stage".to_string(),
        code: 0,
        stderr: format!("{kind:?}"),
        report: Box::new(WriteReport::local(kind)),
    }
}
