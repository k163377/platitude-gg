//! One content line of a unified hunk: a single marker column, then the
//! text — the ordinary two-sided diff, beside `combined` for the merges.

use super::{DiffHunk, DiffLine, DiffLineKind};

/// One content line of a unified hunk, and where each side gets to after it.
pub(super) fn read_unified_line(h: &mut DiffHunk, line: &str, old_no: &mut u32, new_no: &mut u32) {
    let mut chars = line.chars();
    match chars.next() {
        Some(' ') => {
            h.lines.push(DiffLine {
                kind: DiffLineKind::Context,
                old_no: Some(*old_no),
                new_no: Some(*new_no),
                text: chars.as_str().to_string(),
                markers: String::new(),
            });
            *old_no += 1;
            *new_no += 1;
        }
        Some('+') => {
            h.lines.push(DiffLine {
                kind: DiffLineKind::Addition,
                old_no: None,
                new_no: Some(*new_no),
                text: chars.as_str().to_string(),
                markers: String::new(),
            });
            *new_no += 1;
        }
        Some('-') => {
            h.lines.push(DiffLine {
                kind: DiffLineKind::Deletion,
                old_no: Some(*old_no),
                new_no: None,
                text: chars.as_str().to_string(),
                markers: String::new(),
            });
            *old_no += 1;
        }
        Some('\\') => {
            h.lines.push(DiffLine {
                kind: DiffLineKind::NoNewline,
                old_no: None,
                new_no: None,
                text: line.to_string(),
                markers: String::new(),
            });
        }
        None => {
            // A completely empty line inside a hunk is a context line
            // whose content is empty (git prints a lone space, but some
            // tools strip trailing whitespace; tolerate).
            h.lines.push(DiffLine {
                kind: DiffLineKind::Context,
                old_no: Some(*old_no),
                new_no: Some(*new_no),
                text: String::new(),
                markers: String::new(),
            });
            *old_no += 1;
            *new_no += 1;
        }
        _ => tracing::trace!(line, "unexpected line inside hunk"),
    }
}
