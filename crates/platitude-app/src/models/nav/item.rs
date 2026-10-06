use super::*;

// A folder row is the only row held whole; every other row is answered
// field by field out of `Source`. `#[derive(QModelItem)]` caps this at
// fifteen fields, but Qt is handed `Role::ALL` (`qmodel.rs`), so roles
// past them need no field. The test at the foot of `role.rs` holds these
// fields to the roles of the same numbers.
#[derive(QModelItem, Default)]
pub struct NavItem {
    /// Display text: the last path segment in tree mode, the full name in
    /// flat/filter mode.
    pub(super) name: String,
    /// Full ref/path (tooltips; folder rows carry their folder path here,
    /// which doubles as the toggle key).
    pub(super) full: String,
    pub(super) oid_hex: String,
    /// git's change code for a file row (`M`, `?`, `UU`). A folder row
    /// carries its fold state here (`FOLDED`, empty when open) — all
    /// fifteen fields are spoken for, so slots are shared and reads are
    /// guarded by `folder`.
    pub(super) change: String,
    pub(super) bucket: String,
    /// Display grouping of working-tree rows: untracked files count as
    /// `unstaged` here while `bucket` keeps the real routing for diffs
    /// and staging.
    pub(super) group: String,
    /// The old path of a renamed working-tree file. A folder row in the
    /// working tree's list carries its clean path here — its `full` is
    /// the group-prefixed fold key, and the hover of an elided chain
    /// needs the path itself.
    pub(super) orig_path: String,
    /// The rename source cut the way the row cuts names
    /// (`encode::rename_source`). A made row never carries one; the field
    /// exists so the derive spells this role's name for the test in
    /// `role.rs`.
    pub(super) orig_name: String,
    pub(super) is_head: bool,
    pub(super) has_remote: bool,
    /// This repository does not hold the ref, so the name greys. Only
    /// tags are ever listed that way. Written as the negative of core's
    /// `here` so every other kind of row keeps it off by default.
    pub(super) only_remote: bool,
    /// PR-state badge. Real data arrives in Phase 4 (ls-remote refs/pull
    /// matching); until then PGG_FAKE_PR previews the look.
    pub(super) has_pr: bool,
    /// This file's pending change has something to say about its line
    /// endings. A flag: the words belong to the one row the pointer is
    /// on and are kept once on the model (`pointEol`).
    pub(super) eol_mark: bool,
    pub(super) depth: i32,
    pub(super) folder: bool,
}

impl platitude_core::mem::Footprint for NavItem {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes()
            + self.full.heap_bytes()
            + self.oid_hex.heap_bytes()
            + self.change.heap_bytes()
            + self.bucket.heap_bytes()
            + self.group.heap_bytes()
            + self.orig_path.heap_bytes()
            + self.orig_name.heap_bytes()
    }
}

/// What a folder row puts in `change` while it is closed.
pub const FOLDED: &str = "FOLDED";

/// What a worktree row puts in the same slot: the state of the checkout,
/// drawn by `NameCell.seatMark` in the seat a folder's arrow and a file's
/// change letter share.
///
/// Locked wins over prunable when git reports both: the lock is what
/// somebody chose, so it is the one a reader can act on.
pub const LOCKED: &str = "LOCKED";
/// `git worktree prune` would drop this entry — the folder it names is
/// gone from where git's administrative file says it is.
pub const PRUNABLE: &str = "PRUNABLE";
/// The repository's own worktree (`WorktreeEntry::main`). Last of
/// the three, and the two above never land on it: git refuses
/// `worktree lock` on it, and `worktree prune` looks only at linked worktrees.
pub const MAIN: &str = "MAIN";
/// What a **branch** row puts in the slot: another worktree has it
/// checked out, so no move can land here (`BranchItem::held_elsewhere`) —
/// the question the WORKTREES rows' marks answer, so it shares their seat.
pub const HELD: &str = "HELD";

pub(super) fn fold_state(expanded: bool) -> String {
    if expanded {
        String::new()
    } else {
        FOLDED.to_string()
    }
}
