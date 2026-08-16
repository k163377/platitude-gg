use super::*;

// A folder row is the only row held whole — no section's data arrives as
// one. Every other row is answered field by field out of `Source`; the
// fields below are then only the declaration — `#[derive(QModelItem)]`
// turns them into the role names the delegate resolves by, and `Role`
// answers under those names (held to this list by the test at the foot of
// the file).
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
    /// carries its fold state here instead** (`FOLDED`, empty when open) —
    /// qtbridge's `QModelItem` allows fifteen fields, so the slot is
    /// shared. Reads are guarded by `folder`, as the other shared fields
    /// are (`bucket` carries a branch on a worktree row, `full` a folder
    /// key).
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
    /// one; the field is here because **the view's role table is one role
    /// per field of this struct** (the test at the foot of this file), and
    /// a role no field stands for cannot be asked for by name.
    pub(super) orig_name: String,
    pub(super) is_head: bool,
    pub(super) has_remote: bool,
    /// This repository does not hold the ref, so the name greys. Only
    /// tags are ever listed that way. Written as the negative of core's
    /// `here` so every other kind of row keeps it off by default.
    pub(super) only_remote: bool,
    /// PR-state badge. Real data arrives in Phase 4 (ls-remote refs/pull
    /// matching); until then PG_FAKE_PR previews the look.
    pub(super) has_pr: bool,
    /// This file's pending change has something to say about its line
    /// endings. A flag, not the sentence — a `QModelItem` holds fifteen
    /// fields and this struct uses all fifteen, so the words for the row
    /// the pointer is on are kept once on the model instead (`pointEol`).
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

pub(super) fn fold_state(expanded: bool) -> String {
    if expanded {
        String::new()
    } else {
        FOLDED.to_string()
    }
}
