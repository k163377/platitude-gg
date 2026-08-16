//! What the sidebar is handed: one sorted reading of the repository's
//! refs, and the rows it lists them as.

use super::*;

/// Sidebar-ready refs snapshot (sorted).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefsSnapshot {
    pub locals: Vec<BranchItem>,
    pub remotes: Vec<BranchItem>,
    pub tags: Vec<TagItem>,
    pub head: Option<HeadState>,
    /// Names of the configured remotes, sorted. A branch with no upstream
    /// has to be told where to go, and this is the list to offer.
    pub remote_names: Vec<String>,
}

/// One sidebar branch row.
///
/// The commit is kept as an [`Oid`] and spelled out where it is shown.
/// Forty hex characters is a string allocation per row for something no
/// reader ever sees in full — the graph is what it is for — and a
/// repository's branches and tags together made 2.6MB of them
/// (`JetBrains/kotlin`, 53,724 refs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchItem {
    pub short: crate::Name,
    pub full: crate::Name,
    pub oid: Oid,
    pub has_remote: bool,
    pub is_head: bool,
    /// For a local branch, the remote branch it speaks for (`origin/main`),
    /// wherever the two stand — the one its badge is about, and the one a
    /// rename offers to carry over. Empty when it speaks for none.
    pub upstream: crate::Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagItem {
    pub short: crate::Name,
    /// Peeled commit id (what the graph row is keyed on). Binary, for the
    /// reason [`BranchItem::oid`] is.
    pub oid: Oid,
    pub annotated: bool,
    /// Creator date (unix seconds); the sidebar sorts tags newest-first.
    /// Zero for a tag only a remote has: an advertisement carries the name
    /// and the commit, and no date to sort by.
    pub created_unix: i64,
    /// A remote carries this name too (cloud badge).
    pub has_remote: bool,
    /// Whether this repository holds the tag. False lists a name only a
    /// remote has — the sidebar is where it can be read at all, since no
    /// local ref puts it on a graph row.
    pub here: bool,
}
