//! Domain model shared across parsers, the graph engine and sessions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::oid::Oid;

/// Interning pool for strings that repeat heavily (author names).
///
/// Keeps per-commit metadata compact for very large repositories: commits
/// store a `u32` id instead of an owned string.
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
/// boxed subject. Message bodies and diffs are fetched lazily elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMeta {
    pub oid: Oid,
    /// Parent ids in order (first parent first).
    pub parents: Box<[Oid]>,
    /// Author name id in the session's [`StrPool`].
    pub author: u32,
    /// Author timestamp (seconds since epoch).
    pub time: i64,
    /// First line of the commit message.
    pub subject: Box<str>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
