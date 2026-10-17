//! Content lines of a unified (two-sided) hunk: one marker column, then
//! the text.

use super::{DiffHunk, DiffLine, DiffLineKind};

/// One content line of a unified hunk, and where each side gets to after it.
pub(super) fn read_unified_line(h: &mut DiffHunk, line: &str, old_no: &mut u32, new_no: &mut u32) {
    let mut chars = line.chars();
    let (kind, old, new, text) = match chars.next() {
        // An empty context line whose lone space some tool stripped.
        Some(' ') | None => (
            DiffLineKind::Context,
            Some(*old_no),
            Some(*new_no),
            chars.as_str(),
        ),
        Some('+') => (DiffLineKind::Addition, None, Some(*new_no), chars.as_str()),
        Some('-') => (DiffLineKind::Deletion, Some(*old_no), None, chars.as_str()),
        Some('\\') => (DiffLineKind::NoNewline, None, None, line),
        _ => {
            tracing::trace!(line, "unexpected line inside hunk");
            return;
        }
    };
    h.lines.push(DiffLine {
        kind,
        old_no: old,
        new_no: new,
        text: text.to_string(),
        markers: String::new(),
    });
    *old_no += u32::from(old.is_some());
    *new_no += u32::from(new.is_some());
}
