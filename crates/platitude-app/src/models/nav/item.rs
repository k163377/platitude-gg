use super::*;

// A folder row is the only row held whole — no section's data arrives as
// one. Every other row is answered field by field out of `Source`.
//
// **The fifteen `#[derive(QModelItem)]` allows bind this struct** — Qt
// is handed `Role::ALL` (`qmodel.rs`), so a role a folder row has no
// field for is answered without one. What the derive buys here is the
// second spelling of the names: the test at the foot of `role.rs` holds
// these fields to the roles of the same numbers.
#[derive(QModelItem, Default)]
pub struct NavItem {
    /// Display text: the last path segment in tree mode, the full name in
    /// flat/filter mode.
    pub(super) name: String,
    /// Full ref/path (tooltips; folder rows carry their folder path here,
    /// which doubles as the toggle key).
    pub(super) full: String,
    pub(super) oid_hex: String,
    /// git's change code for a file row (`M`, `?`, `UU`). **A folder row
    /// carries its fold state here** (`FOLDED`, empty when open) —
    /// the derive allows this struct fifteen fields and they are all
    /// spoken for, so the slot is shared. Reads are guarded by `folder`,
    /// as the other shared fields are (`bucket` carries a branch on a
    /// worktree row, `full` a folder key).
    pub(super) change: String,
    pub(super) bucket: String,
    /// Display grouping of worktree rows (GitKraken-style): untracked
    /// files count as `unstaged` here while `bucket` keeps the real
    /// routing for diffs and staging.
    pub(super) group: String,
    /// The old path of a renamed working-tree file. **A folder row in
    /// the working tree's list carries its clean path here** — its
    /// `full` is the group-prefixed fold key, and the hover of an
    /// elided chain needs the path itself.
    pub(super) orig_path: String,
    /// The same source written the way the row writes names — what a
    /// delegate shows (`encode::rename_source`). A made row never carries
    /// one; the field is here so the derive spells the name a second
    /// time and the test at the foot of `role.rs` can hold the role of
    /// this number to it.
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
    /// endings. A flag — the words belong to the one row the pointer
    /// is on, so they are kept once on the model
    /// (`pointEol`).
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

/// What a worktree row puts in the same slot: the state of the checkout
/// itself, since it has no change code of its own. Read back by
/// `NameCell.seatMark`, which draws the mark the row opens with — the
/// seat a folder's arrow and a file's change letter share.
///
/// **Locked wins over prunable.** git can report both on one entry, and
/// they answer different questions: a lock is what somebody chose, a
/// prune is what happened to the folder. One seat holds one mark, and
/// the chosen one is the one a reader can act on.
pub const LOCKED: &str = "LOCKED";
/// `git worktree prune` would drop this entry — the folder it names is
/// gone from where git's administrative file says it is.
pub const PRUNABLE: &str = "PRUNABLE";
/// What a **branch** row puts in the slot: another working copy has it
/// checked out, so no move can land here (`BranchItem::held_elsewhere`).
/// The same seat, and the same one question the WORKTREES rows answer —
/// which is why the three share it.
pub const HELD: &str = "HELD";

pub(super) fn fold_state(expanded: bool) -> String {
    if expanded {
        String::new()
    } else {
        FOLDED.to_string()
    }
}
