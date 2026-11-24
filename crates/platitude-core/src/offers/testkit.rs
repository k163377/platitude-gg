//! What the rules' tests share.

use super::{Leads, RefKind, RefTarget, RowStanding, TreeStanding};

/// A row by its kind and name; every case names its row.
pub(super) fn target(kind: RefKind, full: &str) -> RefTarget<'_> {
    RefTarget::new(kind, full).expect("the case names its row")
}

/// The commit a case's row points at: not HEAD's (`TreeStanding::on`'s
/// `abc123` names none).
pub(super) fn row_commit() -> crate::Oid {
    crate::Oid::from_hex(b"1111111111111111111111111111111111111111").expect("a full object name")
}

/// `target` on `tree` in an open tab, on a commit and leading nowhere else;
/// the cases bend one field at a time.
pub(super) fn standing<'a>(
    target: Option<RefTarget<'a>>,
    tree: &'a TreeStanding,
) -> RowStanding<'a> {
    RowStanding {
        target,
        commit: Some(row_commit()),
        open: true,
        tree,
        leads: Leads::Nowhere,
    }
}
