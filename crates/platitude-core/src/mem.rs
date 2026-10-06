//! Heap accounting for the structures that stay resident.
//!
//! Attribution only: the counting allocator in the app crate is the ground
//! truth, and what the named parts do not add up to is reported as a
//! remainder — so a container's overhead slightly off shows only there.

use std::collections::{BTreeMap, HashMap};

/// Heap bytes a value owns.
///
/// Excludes `size_of::<Self>()` — the caller adds that for a value it owns
/// outright, and a container already counts it inside its own buffer.
pub trait Footprint {
    fn heap_bytes(&self) -> usize;
}

/// Values with nothing behind them: the whole cost is `size_of`, which
/// whoever holds them counts.
macro_rules! flat {
    ($($t:ty),* $(,)?) => {
        $(impl Footprint for $t {
            fn heap_bytes(&self) -> usize { 0 }
        })*
    };
}

flat!(
    bool, u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, char, f32, f64
);
flat!(crate::Oid, crate::graph::Segment, crate::eol::Eol);
flat!(crate::session::LabelKind, crate::graph::SegmentKind);

impl Footprint for crate::eol::Scope {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Here(s) | Self::Ext(s) => s.heap_bytes(),
            Self::Repo => 0,
        }
    }
}

impl Footprint for crate::eol::Baseline {
    fn heap_bytes(&self) -> usize {
        self.scope.heap_bytes()
    }
}

impl Footprint for crate::eol::Notice {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Flipped { .. } | Self::Mixed { .. } => 0,
            Self::NewFile { baseline, .. } | Self::FirstEnding { baseline, .. } => {
                baseline.heap_bytes()
            }
        }
    }
}

impl Footprint for String {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}

/// Zero while the text fits inline — `capacity()` answers for the inline
/// buffer too, so this has to ask.
impl Footprint for crate::Name {
    fn heap_bytes(&self) -> usize {
        if self.is_heap_allocated() {
            self.capacity()
        } else {
            0
        }
    }
}

impl Footprint for str {
    fn heap_bytes(&self) -> usize {
        0
    }
}

impl<T: Footprint> Footprint for Vec<T> {
    fn heap_bytes(&self) -> usize {
        self.capacity() * size_of::<T>() + self.iter().map(Footprint::heap_bytes).sum::<usize>()
    }
}

impl<T: Footprint> Footprint for Option<T> {
    fn heap_bytes(&self) -> usize {
        self.as_ref().map_or(0, Footprint::heap_bytes)
    }
}

impl<T: Footprint + ?Sized> Footprint for Box<T> {
    fn heap_bytes(&self) -> usize {
        size_of_val(&**self) + (**self).heap_bytes()
    }
}

/// Counted in full at every holder — the one place this module can
/// double-count, so a report naming two holders of one `Arc` says so beside
/// the numbers.
impl<T: Footprint + ?Sized> Footprint for std::sync::Arc<T> {
    fn heap_bytes(&self) -> usize {
        // Two atomic counters sit in front of the value in the same box.
        2 * size_of::<usize>() + size_of_val(&**self) + (**self).heap_bytes()
    }
}

impl<T: Footprint> Footprint for [T] {
    fn heap_bytes(&self) -> usize {
        self.iter().map(Footprint::heap_bytes).sum()
    }
}

/// Needed explicitly, or the deref to `[T]` answers instead and a spilled
/// buffer goes uncounted.
impl<A: smallvec::Array> Footprint for smallvec::SmallVec<A>
where
    A::Item: Footprint,
{
    fn heap_bytes(&self) -> usize {
        let buffer = if self.spilled() {
            self.capacity() * size_of::<A::Item>()
        } else {
            0
        };
        buffer + self.iter().map(Footprint::heap_bytes).sum::<usize>()
    }
}

impl<A: Footprint, B: Footprint> Footprint for (A, B) {
    fn heap_bytes(&self) -> usize {
        self.0.heap_bytes() + self.1.heap_bytes()
    }
}

/// The buffer behind a `HashMap`/`HashSet` of this capacity.
///
/// hashbrown: power-of-two buckets at 7/8 load, one control byte each.
/// `capacity()` is already buckets × 7/8, so undoing the ratio lands on the
/// bucket count exactly — rounding up again would double every table.
fn table_bytes<T>(capacity: usize) -> usize {
    if capacity == 0 {
        return 0;
    }
    let buckets = (capacity * 8 / 7).max(4).next_power_of_two();
    buckets * (size_of::<T>() + 1)
}

impl<K: Footprint, V: Footprint, S> Footprint for HashMap<K, V, S> {
    fn heap_bytes(&self) -> usize {
        table_bytes::<(K, V)>(self.capacity())
            + self
                .iter()
                .map(|(k, v)| k.heap_bytes() + v.heap_bytes())
                .sum::<usize>()
    }
}

impl<T: Footprint, S> Footprint for std::collections::HashSet<T, S> {
    fn heap_bytes(&self) -> usize {
        table_bytes::<T>(self.capacity()) + self.iter().map(Footprint::heap_bytes).sum::<usize>()
    }
}

/// A `BTreeMap`'s nodes.
///
/// Charged by the node: a node is allocated whole with room for eleven
/// pairs, so charging entries would read a fleet of one-entry maps as
/// almost free.
///
/// The node count is an estimate — a tree grown by insertion uses about six
/// of the eleven slots — and exact only for `len <= 6`.
fn btree_bytes<K, V>(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    // `LeafNode`: a parent pointer, its index, the length, and room for
    // eleven pairs. Internal nodes (twelve more edge pointers) are few
    // enough to leave out.
    let node = 2 * size_of::<usize>() + 11 * (size_of::<K>() + size_of::<V>());
    let nodes = len.div_ceil(6).max(1);
    nodes * node
}

impl<K: Footprint, V: Footprint> Footprint for BTreeMap<K, V> {
    fn heap_bytes(&self) -> usize {
        btree_bytes::<K, V>(self.len())
            + self
                .iter()
                .map(|(k, v)| k.heap_bytes() + v.heap_bytes())
                .sum::<usize>()
    }
}

impl<T: Footprint> Footprint for std::collections::BTreeSet<T> {
    fn heap_bytes(&self) -> usize {
        btree_bytes::<T, ()>(self.len()) + self.iter().map(Footprint::heap_bytes).sum::<usize>()
    }
}

// ---------------------------------------------------------------------------
// Domain types that stay resident
// ---------------------------------------------------------------------------

impl Footprint for crate::session::RefLabel {
    fn heap_bytes(&self) -> usize {
        self.text.heap_bytes() + self.remote.heap_bytes()
    }
}

impl Footprint for crate::details::CoAuthor {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes() + self.email.heap_bytes()
    }
}

impl Footprint for crate::session::LogRow {
    fn heap_bytes(&self) -> usize {
        self.oid_hex.heap_bytes()
            + self.short_sha.heap_bytes()
            + self.author.heap_bytes()
            + self.author_email.heap_bytes()
            + self.co_authors.heap_bytes()
            + self.subject.heap_bytes()
            + self.body.heap_bytes()
            + self.segments.heap_bytes()
            + self.labels.heap_bytes()
            + self.stash_ref.heap_bytes()
            + self.parents.heap_bytes()
    }
}

impl Footprint for crate::session::BranchItem {
    fn heap_bytes(&self) -> usize {
        self.short.heap_bytes()
            + self.full.heap_bytes()
            + self.upstream.heap_bytes()
            + self.upstream_gone.heap_bytes()
            + self.tracked_by.heap_bytes()
    }
}

impl Footprint for crate::session::TagItem {
    fn heap_bytes(&self) -> usize {
        self.short.heap_bytes()
    }
}

impl Footprint for crate::session::TagDrift {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes() + self.remote.heap_bytes()
    }
}

impl Footprint for crate::refs::HeadState {
    fn heap_bytes(&self) -> usize {
        self.branch.heap_bytes()
    }
}

impl Footprint for crate::session::RefsSnapshot {
    fn heap_bytes(&self) -> usize {
        self.locals.heap_bytes()
            + self.remotes.heap_bytes()
            + self.tags.heap_bytes()
            + self.tags_by_name.heap_bytes()
            + self.tag_drifts.heap_bytes()
            // Not `remote_tags`: it points at the session's one index,
            // counted as `remote-tag-index` (`session::heap`).
            + self.head.heap_bytes()
            + self.remote_names.heap_bytes()
            + self.remote_urls.heap_bytes()
            + self.push_default.heap_bytes()
            + self.checkout_default.heap_bytes()
    }
}

impl Footprint for crate::remote::PushDefault {
    fn heap_bytes(&self) -> usize {
        self.remote.heap_bytes()
    }
}

impl Footprint for crate::session::EolMark {
    fn heap_bytes(&self) -> usize {
        self.path.heap_bytes() + self.notice.heap_bytes()
    }
}

impl Footprint for crate::remote::RemoteTag {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes()
    }
}

impl Footprint for crate::stash::StashEntry {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes() + self.message.heap_bytes()
    }
}

impl Footprint for crate::worktrees::WorktreeEntry {
    fn heap_bytes(&self) -> usize {
        self.path.heap_bytes()
            + self.branch.heap_bytes()
            + self.head_hex.heap_bytes()
            + self.lock_reason.heap_bytes()
            + self.prune_reason.heap_bytes()
    }
}

impl Footprint for crate::status::StatusItem {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Tracked {
                path, orig_path, ..
            } => path.heap_bytes() + orig_path.heap_bytes(),
            Self::Unmerged { path, .. } | Self::Untracked { path } => path.heap_bytes(),
        }
    }
}

impl Footprint for crate::status::WorkingTreeStatus {
    fn heap_bytes(&self) -> usize {
        self.branch_head.heap_bytes() + self.upstream.heap_bytes() + self.items.heap_bytes()
    }
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

/// One named part of a breakdown; `count` is how many things are in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: &'static str,
    pub bytes: usize,
    pub count: usize,
}

impl Part {
    pub fn new(name: &'static str, bytes: usize, count: usize) -> Self {
        Self { name, bytes, count }
    }

    /// A whole collection: its buffer, its elements' own heap, and its
    /// length.
    pub fn of<T: Footprint>(name: &'static str, items: &Vec<T>) -> Self {
        Self::new(name, items.heap_bytes(), items.len())
    }
}

pub fn total(parts: &[Part]) -> usize {
    parts.iter().map(|p| p.bytes).sum()
}

/// `name=bytes/count` for a log line, in the order given.
pub fn render(parts: &[Part]) -> String {
    parts
        .iter()
        .map(|p| format!("{}={}/{}", p.name, p.bytes, p.count))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;
