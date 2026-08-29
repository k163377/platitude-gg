//! What a partial stage refuses, and how it says so.
//!
//! **This end is the one that says no here**, before git is asked: the
//! bytes carry a fingerprint and it did not match, the hunk the selection
//! indexes is no longer in the diff, or the file is conflicted and has no
//! single old side to cut a patch against. Nothing ran, so there is no
//! command to name and no words to quote — the screen says both lines in
//! its own language (デザイン規約 §答えの要らない報せ).

use crate::details;
use crate::error::GitError;
use crate::patch::{self, PatchSide};
use crate::report::{ReportKind, WriteReport};

/// Refuses a conflicted file's diff, which has more than one old side and
/// so cannot be cut into a patch that applies (`platitude_core::patch`).
///
/// The pane withholds the pieces on such a file, so nothing should ask —
/// but a write that got here anyway must say why in this app's words. Left
/// to git, the same refusal arrives as `git apply` complaining about a
/// patch fragment nobody wrote.
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

/// Which way a part was being taken, which is the whole of what the
/// heading turns on: a part taken off the staged side is being unstaged,
/// and saying "nothing was staged" for it would name the wrong direction
/// (デザイン規約 §答えの要らない報せ). Discarding never comes through here.
pub(super) fn taking(side: PatchSide) -> ReportKind {
    match side {
        PatchSide::Reverse => ReportKind::StaleUnstage,
        PatchSide::Forward => ReportKind::StaleStage,
    }
}

/// A refusal this end made before git was asked. Nothing ran, so there is
/// no command to name and no words to quote: the screen says both lines in
/// its own language (デザイン規約 §答えの要らない報せ).
pub(super) fn stale(kind: ReportKind) -> GitError {
    GitError::Reported {
        command: String::new(),
        code: 0,
        stderr: String::new(),
        report: Box::new(WriteReport::local(kind, String::new())),
    }
}
