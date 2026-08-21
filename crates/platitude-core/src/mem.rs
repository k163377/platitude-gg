//! Heap accounting for the structures that stay resident.
//!
//! Not a profiler. It answers one question — *how many bytes does this
//! collection own* — for the handful of things that are still alive when
//! the window is sitting idle, which is what a memory budget is spent
//! against.
//!
//! Attribution only. The ground truth for "how much Rust heap is live" is
//! the counting allocator in the app crate; what this module produces is a
//! breakdown of it, and whatever the named parts do not add up to is
//! reported as a remainder rather than assumed to be zero. Getting a
//! container's own overhead slightly wrong therefore shows up as a bigger
//! remainder, never as a wrong total.

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

/// Nothing at all while the text fits inline, which is the whole point of
/// [`crate::Name`] — so this has to ask rather than read `capacity()`,
/// which answers for the inline buffer too.
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

/// **Counted in full at every site that holds one.** Shared handles are the
/// one place this module can double-count, so a report that names two
/// holders of the same `Arc` says so beside the numbers.
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

/// **Needed explicitly**, or the deref to `[T]` answers instead and a
/// spilled buffer goes uncounted — the one case this type exists to make
/// rare is also the one that would then be invisible.
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
/// hashbrown holds a power-of-two number of buckets at 7/8 load, with one
/// control byte beside each. `capacity()` is already the usable figure —
/// buckets × 7/8 — so undoing that ratio lands back on the bucket count
/// exactly, and rounding it up again would report every table at twice its
/// size.
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
/// **Charged by the node, not by the entry.** A node holds room for eleven
/// pairs whether or not eleven are in it, and it is allocated whole — so a
/// map with one entry in it costs the same as a map with eleven. Charging
/// entries instead reads a fleet of one-entry maps as almost free, which is
/// the opposite of what it is (measured: `remote_tag_index`, one inner map
/// per tag).
///
/// The node count is where this stays an estimate: a tree grown by
/// insertion settles around six of the eleven slots used, and only the
/// single-node case (`len <= 11`) is exact.
fn btree_bytes<K, V>(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    // `LeafNode`: a parent pointer, its index, the length, and room for
    // eleven pairs. An internal node adds twelve edge pointers, and there
    // are far fewer of those than leaves.
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
    }
}

impl Footprint for crate::session::BranchItem {
    fn heap_bytes(&self) -> usize {
        self.short.heap_bytes() + self.full.heap_bytes() + self.upstream.heap_bytes()
    }
}

impl Footprint for crate::session::TagItem {
    fn heap_bytes(&self) -> usize {
        self.short.heap_bytes()
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
            + self.head.heap_bytes()
            + self.remote_names.heap_bytes()
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
            Self::Unmerged { path, .. } | Self::Untracked { path } | Self::Ignored { path } => {
                path.heap_bytes()
            }
        }
    }
}

impl Footprint for crate::status::WorkTreeStatus {
    fn heap_bytes(&self) -> usize {
        self.branch_head.heap_bytes() + self.upstream.heap_bytes() + self.items.heap_bytes()
    }
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

/// One named part of a breakdown: what it is, how many bytes it owns, and
/// how many things are in it.
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

/// Sum of a breakdown's parts.
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
mod tests {
    use super::*;

    #[test]
    fn a_string_owns_its_buffer() {
        let mut s = String::with_capacity(64);
        s.push_str("abc");
        assert_eq!(s.heap_bytes(), 64);
        assert_eq!(String::new().heap_bytes(), 0);
    }

    #[test]
    fn a_vec_counts_its_buffer_and_its_elements() {
        let items: Vec<String> = vec!["a".repeat(40), "b".repeat(40)];
        // Two Strings inline (capacity is exact for `repeat`) plus their
        // buffers.
        assert_eq!(
            items.heap_bytes(),
            items.capacity() * size_of::<String>() + 80
        );
    }

    #[test]
    fn an_empty_container_owns_nothing() {
        assert_eq!(Vec::<String>::new().heap_bytes(), 0);
        assert_eq!(HashMap::<u32, String>::new().heap_bytes(), 0);
        assert_eq!(BTreeMap::<u32, String>::new().heap_bytes(), 0);
        assert_eq!(Option::<String>::None.heap_bytes(), 0);
    }

    #[test]
    fn a_map_counts_its_table_and_its_entries() {
        let mut map: HashMap<u32, String> = HashMap::new();
        map.insert(1, "x".repeat(10));
        // The table is sized off capacity, not length, and the value's
        // buffer is on top of it.
        assert!(map.heap_bytes() >= table_bytes::<(u32, String)>(map.capacity()) + 10);
    }

    #[test]
    fn nested_rows_add_up() {
        let row = crate::session::LogRow {
            row: 0,
            oid_hex: "0".repeat(40),
            short_sha: String::new(),
            author: String::new(),
            author_email: String::new(),
            co_authors: Vec::new(),
            time: 0,
            subject: "s".repeat(30),
            body: String::new(),
            node_lane: 0,
            node_color: 0,
            width: 1,
            segments: Vec::new(),
            labels: Vec::new(),
            stash_ref: String::new(),
        };
        assert_eq!(row.heap_bytes(), 70);
        let rows = vec![row];
        assert_eq!(
            rows.heap_bytes(),
            rows.capacity() * size_of::<crate::session::LogRow>() + 70
        );
    }

    #[test]
    fn a_one_entry_btree_still_costs_a_whole_node() {
        let mut one: BTreeMap<u64, u64> = BTreeMap::new();
        one.insert(1, 1);
        let node = 2 * size_of::<usize>() + 11 * 16;
        assert_eq!(one.heap_bytes(), node);
        // Eleven entries still fit in that one node, so the two agree —
        // which is what makes a fleet of small maps visible as the cost
        // it is rather than as eleven times less.
        let full: BTreeMap<u64, u64> = (0..11).map(|i| (i, i)).collect();
        assert_eq!(full.heap_bytes(), 2 * node);
    }

    #[test]
    fn parts_render_and_total() {
        let parts = vec![Part::new("rows", 100, 2), Part::new("labels", 50, 3)];
        assert_eq!(total(&parts), 150);
        assert_eq!(render(&parts), "rows=100/2 labels=50/3");
    }
}
