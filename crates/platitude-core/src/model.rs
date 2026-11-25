//! Domain model shared across parsers, the graph engine and sessions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::oid::Oid;

/// A short string that lives inline when it fits (24 bytes, which a typical
/// ref name does), and on the heap when it does not — resident data is
/// mostly names, and a `String` is one allocation each.
///
/// Readers use only `as_str`, `==` against `&str` and `Display`
/// (.claude/rules/core.md §セッション・実装の決定事項). Named as
/// `crate::Name` everywhere.
pub type Name = compact_str::CompactString;

/// How far a branch stands from the ref it is measured against, as of the
/// last fetch: the commits it has that the other lacks, and the other's
/// that it lacks.
///
/// The counts alone: 0/0 is level with that ref. Whether there is one to
/// measure against at all is the holder's to say (an `Option` of these, or
/// [`crate::refs::RefEntry::counts`]'s own rule).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AheadBehind {
    pub ahead: u32,
    pub behind: u32,
}

impl AheadBehind {
    /// Each side has commits the other lacks: neither a push nor a
    /// fast-forward brings the two together.
    pub fn is_diverged(self) -> bool {
        self.ahead > 0 && self.behind > 0
    }

    /// Neither side has a commit the other lacks: 0/0, nothing to say.
    pub fn is_level(self) -> bool {
        self.ahead == 0 && self.behind == 0
    }
}

/// Interning pool for strings that repeat heavily (author names): commits
/// store a `u32` id.
#[derive(Debug, Default)]
pub struct StrPool {
    map: HashMap<Arc<str>, u32>,
    items: Vec<Arc<str>>,
}

impl StrPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the id for `s`, inserting it on first sight.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(id) = self.map.get(s) {
            return *id;
        }
        let arc: Arc<str> = Arc::from(s);
        let id = self.items.len() as u32;
        self.items.push(Arc::clone(&arc));
        self.map.insert(arc, id);
        id
    }

    /// Resolves an id; empty string for unknown ids (never expected).
    pub fn get(&self, id: u32) -> &str {
        self.items.get(id as usize).map_or("", |s| s.as_ref())
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Metadata of one commit as parsed from `git log`.
///
/// Sized for 100k+ commit repositories: fixed-size id, interned author,
/// boxed text. Diffs are fetched lazily elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMeta {
    pub oid: Oid,
    /// Parent ids in order (first parent first).
    pub parents: Box<[Oid]>,
    /// Author name id in the session's [`StrPool`].
    pub author: u32,
    /// Author address id in the same pool, as the commit spells it — a locally
    /// assigned picture is looked up by its [`crate::avatar::key`].
    pub author_email: u32,
    /// `Co-authored-by` trailers as (name, address) pool ids, in message
    /// order. Not mailmapped — a trailer is message text, so the same
    /// person can be spelled differently here and in `author`.
    pub co_authors: Box<[(u32, u32)]>,
    /// Everything after the subject, with the `Co-authored-by` lines
    /// removed — what the row's hover reads out.
    pub body: Box<str>,
    /// Author timestamp (seconds since epoch).
    pub time: i64,
    /// First line of the commit message.
    pub subject: Box<str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_commits_on_both_sides_are_a_divergence() {
        let stands = |ahead, behind| AheadBehind { ahead, behind }.is_diverged();
        assert!(stands(1, 1));
        assert!(!stands(3, 0), "ahead alone is a push away");
        assert!(!stands(0, 3), "behind alone is a fast-forward away");
        assert!(!stands(0, 0));
    }

    #[test]
    fn only_nothing_on_either_side_is_level() {
        let level = |ahead, behind| AheadBehind { ahead, behind }.is_level();
        assert!(level(0, 0));
        assert!(!level(1, 0), "a commit to send");
        assert!(!level(0, 1), "a commit to take");
        assert!(!level(2, 3));
    }

    #[test]
    fn intern_deduplicates() {
        let mut pool = StrPool::new();
        let a = pool.intern("Alice");
        let b = pool.intern("Bob");
        let a2 = pool.intern("Alice");
        assert_eq!(a, a2);
        assert_ne!(a, b);
        assert_eq!(pool.get(a), "Alice");
        assert_eq!(pool.get(b), "Bob");
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn unknown_id_resolves_to_empty() {
        let pool = StrPool::new();
        assert_eq!(pool.get(42), "");
    }
}
