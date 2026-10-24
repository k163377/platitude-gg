//! The fixture builder all three test modules use, and the line they ask
//! for by patch, hunk and line.

use crate::parse::diff::{DiffLineLocation, FilePatch, parse_patch};

pub(super) fn patches(text: &str) -> Vec<FilePatch> {
    parse_patch(text.as_bytes())
}

pub(super) fn at(patch: usize, hunk: usize, line: usize) -> DiffLineLocation {
    DiffLineLocation { patch, hunk, line }
}
