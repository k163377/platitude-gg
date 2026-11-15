//! The open diff drawn with the parts pressed on it still in it (デザイン規約
//! §diff の中のステージ): a hunk staged, unstaged or discarded leaves the
//! rows at the press, a line turns into context or leaves, and the numbers
//! after it move as git's next read will number them. Which parts, and for
//! how long, is `ops::StageAhead`'s (`diff_parts`); the layout is
//! `platitude_core::parts::after`'s — the one rule core places a part
//! pressed on this drawing by (`stage::PressedOn`), so a press on a row of
//! it lands where the row stood.

use platitude_core::parse::diff::{DiffLine, DiffLineKind, FilePatch};
use platitude_core::parts::{After, after};

use crate::ops::DiffPart;

/// The read laid out with `parts` in it; `None` where none stands.
pub(super) fn laid(read: &[FilePatch], parts: &[DiffPart]) -> Option<After> {
    if parts.is_empty() {
        return None;
    }
    let parts: Vec<_> = parts.iter().map(|p| p.part).collect();
    Some(after(read, &parts))
}

/// Whether a press of `hunk` (or of its `line`; negative for the whole hunk)
/// takes every change `patches` shows, in a diff a part can empty: one
/// file's text edit under one name — no file made or taken away (a part
/// leaves the entry), no rename (the name stays where its content went), no
/// mode change (no part moves it, `parse::diff::FilePatch::mode_change`).
pub(super) fn takes_all(patches: &[FilePatch], hunk: i32, line: i32) -> bool {
    let [patch] = patches else {
        return false;
    };
    if patch.is_binary
        || patch.is_combined
        || patch.unmerged
        || patch.gitlink
        || patch.lfs
        || patch.mode_change.is_some()
        || patch.old_path.is_none()
        || patch.old_path != patch.new_path
    {
        return false;
    }
    let changed =
        |line: &&DiffLine| matches!(line.kind, DiffLineKind::Addition | DiffLineKind::Deletion);
    let Ok(hunk) = usize::try_from(hunk) else {
        return false;
    };
    match usize::try_from(line) {
        Err(_) => hunk == 0 && patch.hunks.len() == 1,
        Ok(line) => {
            let changes = patch
                .hunks
                .iter()
                .flat_map(|h| &h.lines)
                .filter(changed)
                .count();
            changes == 1
                && patch
                    .hunks
                    .get(hunk)
                    .and_then(|h| h.lines.get(line))
                    .is_some_and(|l| changed(&l))
        }
    }
}
