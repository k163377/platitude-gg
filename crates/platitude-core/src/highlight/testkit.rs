//! The one fixture builder all three test modules use.

use crate::parse::diff::{FilePatch, parse_patch};

pub(super) fn patches(text: &str) -> Vec<FilePatch> {
    parse_patch(text.as_bytes())
}
