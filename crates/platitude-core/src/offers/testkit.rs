//! What the rules' tests share.

use super::{RefKind, RefTarget};

/// A row by its kind and name; every case names its row.
pub(super) fn target(kind: RefKind, full: &str) -> RefTarget<'_> {
    RefTarget::new(kind, full).expect("the case names its row")
}
