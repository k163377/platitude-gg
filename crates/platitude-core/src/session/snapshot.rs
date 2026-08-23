//! What the sidebar is handed: one sorted reading of the repository's
//! refs, and the rows it lists them as.

use super::*;

/// Sidebar-ready refs snapshot (sorted).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefsSnapshot {
    pub locals: Vec<BranchItem>,
    pub remotes: Vec<BranchItem>,
    pub tags: Vec<TagItem>,
    /// Tags this repository holds that a remote carries on some other
    /// commit. A run of its own rather than a field on every
    /// [`TagItem`]: a name standing on two commits is rare, and 45,901
    /// tags would each pay for the field (CLAUDE.md 性能予算 — refs の
    /// 本数に比例させない).
    pub tag_drifts: Vec<TagDrift>,
    pub head: Option<HeadState>,
    /// Names of the configured remotes, sorted. A branch with no upstream
    /// has to be told where to go, and this is the list to offer.
    pub remote_names: Vec<String>,
    /// Their fetch URLs, in the same order — what the form that corrects
    /// one opens with already filled in.
    pub remote_urls: Vec<String>,
    /// Which of them a push goes to when no branch says otherwise, and
    /// whether this repository's own config is what says so. `None` where
    /// nothing is marked — then a branch's own upstream decides, and a
    /// branch with none falls back to whichever remote is called `origin`.
    pub push_default: Option<crate::remote::PushDefault>,
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
    /// Another working copy has this branch checked out, so git refuses
    /// both `switch` and `branch --delete` for it (2026-08-21 実測) —
    /// whether or not that copy is **locked**, which stops a different
    /// set of commands. A `bool` and not the folder: this rides one per
    /// branch, and the reference repository has fifty thousand refs.
    ///
    /// Local branches only. A remote row lands on the local branch of
    /// the same name, and whether *that* one is held is a question the
    /// menu asks of the worktree list by name.
    pub held_elsewhere: bool,
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

/// One remote holding one tag on a commit this repository does not have
/// it on.
///
/// What the menu's push row is decided by: a plain push to a name the
/// remote already has elsewhere is refused outright, so that row comes up
/// as the leased overwrite instead — and [`Self::commit`] is the commit
/// the lease is pinned to, which is the one the reader was being shown
/// (デザイン規約 §相手の履歴を置き換える).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDrift {
    pub name: crate::Name,
    pub remote: crate::Name,
    /// Where that remote has it, peeled the same way the local reading is.
    pub commit: Oid,
}
