//! What a partial stage refuses before git is asked: the fingerprint did
//! not match, the hunk the selection indexes is no longer in the diff, or
//! the file is conflicted and has no single old side to cut a patch
//! against.

use crate::details;
use crate::error::GitError;
use crate::patch::{self, PatchSide};
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
        report: Box::new(WriteReport::local(kind, String::new())),
    }
}
