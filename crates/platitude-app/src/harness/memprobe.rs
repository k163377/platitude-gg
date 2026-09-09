//! What the process is holding on to, and where.
//!
//! Built to answer the memory budget's question without guessing, so it has
//! two halves that are read together:
//!
//! * **the counting allocator** is ground truth for how many bytes of Rust
//!   heap are live. Qt allocates through C++ `operator new` and never
//!   through Rust's `GlobalAlloc`, so the line between "the data this
//!   application built" and "the toolkit under it" falls out of this
//!   number exactly rather than by apportionment;
//! * **the registry** attributes bytes *inside* that number to the models
//!   holding them. Each model reports its own footprint as it changes;
//!   the report sums them, and prints what is left over as a remainder
//!   rather than pretending the named parts are everything.
//!
//! Off unless asked for, and a shipped build cannot be asked. The counting
//! is behind the `memprobe` feature, and the per-model walks need that
//! feature *and* `PGG_MEM_REPORT=1` (`harness::knobs`, the crate's only
//! reader of one) — they are O(rows), and a measurement must not pay for
//! itself on every drain of a normal run.
//!
//! The process's allocator is chosen here too, because there is only one
//! `#[global_allocator]` slot and the counter has to sit in front of
//! whatever fills it.
//!
//! **The report half is reached from one place** — the slot on
//! `harness::singleton::Harness` — and that type is not compiled into a
//! build without the harness, so nothing there calls any of it. `allow`
//! rather than `expect`: the test at the foot keeps some of the same names
//! live whenever tests are compiled, and an expectation that goes
//! unfulfilled is a warning of its own.
#![cfg_attr(not(feature = "automation"), allow(dead_code))]

use std::collections::BTreeMap;
use std::sync::Mutex;
// Only the counting half reads a counter, and a build without it must not
// carry an import nothing uses.
#[cfg(feature = "memprobe")]
use std::sync::atomic::Ordering;

use platitude_core::mem::{Footprint, Part};

// ---------------------------------------------------------------------------
// The allocator
// ---------------------------------------------------------------------------

/// What every allocation actually goes to.
///
/// **The platform's own, measured against the alternative.** mimalloc was
/// tried here and is worse for this workload — the numbers and the reason
/// are in `ci/baseline/perf-windows-x64.md`, so the next reader does not
/// have to re-run it.
///
/// A shipped build links it straight into the `#[global_allocator]` slot
/// below; a measuring build has the counter in front of it, so both are
/// reporting the same allocator.
type Base = std::alloc::System;
const BASE: Base = std::alloc::System;

#[cfg(not(feature = "memprobe"))]
#[global_allocator]
static ALLOCATOR: Base = BASE;

#[cfg(feature = "memprobe")]
mod counting {
    use std::alloc::{GlobalAlloc, Layout};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::BASE;

    pub(super) static LIVE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static PEAK: AtomicUsize = AtomicUsize::new(0);

    /// Live bytes by size class (bucket *n* = allocations of 2^n bytes and
    /// up, to 2^(n+1)).
    ///
    /// What it is for: when the named parts do not add up to the live
    /// total, the shape of the remainder narrows the search before any
    /// code is read. A remainder made of millions of 32-byte allocations
    /// is a collection of small nodes; one made of a handful of megabyte
    /// blocks is a buffer somebody is holding.
    pub(super) static CLASSES: [AtomicUsize; 32] = [const { AtomicUsize::new(0) }; 32];

    /// Live allocations by the same size class. Bytes alone cannot tell a
    /// single thirty-megabyte buffer from thirty thousand small nodes, and
    /// those two lead to opposite places.
    pub(super) static COUNTS: [AtomicUsize; 32] = [const { AtomicUsize::new(0) }; 32];

    pub(super) fn class_of(size: usize) -> usize {
        // Saturated, not wrapped: a 2^31-byte-and-up allocation belongs in
        // the top bucket, not relabelled as a small one.
        ((usize::BITS - size.leading_zeros()) as usize).min(31)
    }

    /// Adds relaxed counters in front of [`super::BASE`] and nothing else —
    /// no side table, no per-allocation header, so what it measures is what
    /// the process would have without it.
    pub struct Counting;

    impl Counting {
        fn took(bytes: usize) {
            let now = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
            PEAK.fetch_max(now, Ordering::Relaxed);
            let class = class_of(bytes);
            CLASSES[class].fetch_add(bytes, Ordering::Relaxed);
            COUNTS[class].fetch_add(1, Ordering::Relaxed);
        }

        fn gave_back(bytes: usize) {
            LIVE.fetch_sub(bytes, Ordering::Relaxed);
            let class = class_of(bytes);
            CLASSES[class].fetch_sub(bytes, Ordering::Relaxed);
            COUNTS[class].fetch_sub(1, Ordering::Relaxed);
        }
    }

    // SAFETY: every method forwards to `BASE` with the layout it was given
    // and returns its pointer unchanged; the counters are plain atomics
    // that allocate nothing.
    #[expect(unsafe_code)]
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarded unchanged.
            let p = unsafe { BASE.alloc(layout) };
            if !p.is_null() {
                Self::took(layout.size());
            }
            p
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarded unchanged.
            let p = unsafe { BASE.alloc_zeroed(layout) };
            if !p.is_null() {
                Self::took(layout.size());
            }
            p
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            Self::gave_back(layout.size());
            // SAFETY: forwarded unchanged.
            unsafe { BASE.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: forwarded unchanged.
            let p = unsafe { BASE.realloc(ptr, layout, new_size) };
            if !p.is_null() {
                Self::gave_back(layout.size());
                Self::took(new_size);
            }
            p
        }
    }

    #[global_allocator]
    static ALLOCATOR: Counting = Counting;
}

/// Live Rust heap in bytes, or `None` in a build without the feature.
pub fn live_bytes() -> Option<usize> {
    #[cfg(feature = "memprobe")]
    {
        Some(counting::LIVE.load(Ordering::Relaxed))
    }
    #[cfg(not(feature = "memprobe"))]
    {
        None
    }
}

/// The high-water mark of [`live_bytes`] since the process started.
pub fn peak_bytes() -> Option<usize> {
    #[cfg(feature = "memprobe")]
    {
        Some(counting::PEAK.load(Ordering::Relaxed))
    }
    #[cfg(not(feature = "memprobe"))]
    {
        None
    }
}

/// Live heap by allocation size class, as `2^n=bytes/count` for the
/// classes holding anything. Empty in a build without the feature.
pub fn size_classes() -> String {
    #[cfg(feature = "memprobe")]
    {
        counting::CLASSES
            .iter()
            .zip(counting::COUNTS.iter())
            .enumerate()
            .map(|(n, (bytes, count))| {
                (
                    n,
                    bytes.load(Ordering::Relaxed),
                    count.load(Ordering::Relaxed),
                )
            })
            .filter(|(_, bytes, _)| *bytes > 0)
            .map(|(n, bytes, count)| format!("2^{n}={bytes}/{count}"))
            .collect::<Vec<_>>()
            .join(",")
    }
    #[cfg(not(feature = "memprobe"))]
    {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// Whether the per-model walks run at all.
///
/// Two things, and both have to hold: the counting allocator has to be in
/// this build, because attributed parts with no counted total to hold them
/// against are half a report; and the run has to have asked
/// (`PGG_MEM_REPORT=1`, through `harness::knobs`, which is the only place
/// in the crate that reads one). A shipped build has neither, and the
/// `cfg!` is what takes the walk out of it rather than a flag that
/// happens to be off.
pub fn enabled() -> bool {
    cfg!(feature = "memprobe") && super::knobs().mem_report
}

/// What each model last reported, keyed by kind and tab. Sorted, so two
/// reports of the same shape read the same way down the line.
///
/// A value rather than a bare static so a test can file into a registry
/// of its own; the process keeps one ([`REGISTRY`]) for the real models,
/// reached through the free functions below.
pub struct Registry(Mutex<BTreeMap<(String, i32), (usize, usize)>>);

impl Registry {
    pub const fn new() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    fn parts(&self) -> std::sync::MutexGuard<'_, BTreeMap<(String, i32), (usize, usize)>> {
        // A poisoned lock only means a holder panicked mid-report; the map
        // is plain numbers and stays usable.
        match self.0.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// Files a collection's footprint under `kind` for tab `tab`.
    ///
    /// Call it where the collection settles — the end of a drain — and
    /// only under [`enabled`]. Costs one walk of the items.
    pub fn note<T: Footprint>(&self, kind: &str, tab: i32, items: &Vec<T>) {
        self.note_bytes(kind, tab, items.heap_bytes(), items.len());
    }

    /// Files a number somebody else worked out.
    pub fn note_bytes(&self, kind: &str, tab: i32, bytes: usize, count: usize) {
        self.parts().insert((kind.to_string(), tab), (bytes, count));
    }

    /// Drops everything filed for a tab that has closed.
    pub fn forget(&self, tab: i32) {
        self.parts().retain(|(_, t), _| *t != tab);
    }

    /// Everything filed so far, tab numbers folded into the names.
    pub fn model_parts(&self) -> Vec<(String, usize, usize)> {
        self.parts()
            .iter()
            .map(|((kind, tab), (bytes, count))| (format!("{kind}#{tab}"), *bytes, *count))
            .collect()
    }
}

/// The process's registry — what the running models file into.
static REGISTRY: Registry = Registry::new();

/// [`Registry::note`], on the process's registry.
pub fn note<T: Footprint>(kind: &str, tab: i32, items: &Vec<T>) {
    REGISTRY.note(kind, tab, items);
}

/// [`Registry::note_bytes`], on the process's registry.
pub fn note_bytes(kind: &str, tab: i32, bytes: usize, count: usize) {
    REGISTRY.note_bytes(kind, tab, bytes, count);
}

/// [`Registry::forget`], on the process's registry.
pub fn forget(tab: i32) {
    REGISTRY.forget(tab);
}

/// [`Registry::model_parts`], on the process's registry.
pub fn model_parts() -> Vec<(String, usize, usize)> {
    REGISTRY.model_parts()
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

/// One report line for where the run has got to, gathered and written.
///
/// The hub is read here rather than at the QML slot that asks: the slot's
/// whole part in this is being on the Qt main thread, which is where the
/// sessions live, so the moment is its business and the reading is not
/// (`AppBackend::note_memory`). Off unless the run asked for it.
pub(crate) fn note_now(label: &str) {
    if !enabled() {
        return;
    }
    let (session_parts, waiting) = crate::hub::Hub::with(|hub| {
        (
            hub.sessions()
                .into_iter()
                .flat_map(|(_, session)| session.heap_report())
                .collect::<Vec<_>>(),
            hub.feed_depths(),
        )
    })
    .unwrap_or_default();
    report(label, &session_parts, &waiting);
}

/// Writes one report line: the live heap, the named parts, and what is
/// left.
///
/// `session_parts` is the core side (see `RepoSession::heap_report`), which
/// the caller fetches because only it can reach the open sessions.
pub fn report(label: &str, session_parts: &[Part], waiting: &str) {
    let models = model_parts();
    let named: usize = models.iter().map(|(_, b, _)| *b).sum::<usize>()
        + session_parts.iter().map(|p| p.bytes).sum::<usize>();
    let live = live_bytes();
    let rendered_models = models
        .iter()
        .map(|(name, bytes, count)| format!("{name}={bytes}/{count}"))
        .collect::<Vec<_>>()
        .join(" ");
    tracing::info!(
        label,
        rust_live = live.unwrap_or(0),
        rust_peak = peak_bytes().unwrap_or(0),
        counted = live.is_some(),
        named,
        // Negative when the same `Arc` is counted on both sides — the refs
        // snapshot is held by the session and by every sidebar section —
        // so it is a signed remainder rather than a subtraction that
        // cannot be trusted to stay positive.
        unattributed = live.map_or(0i64, |l| l as i64 - named as i64),
        models = rendered_models,
        session = platitude_core::mem::render(session_parts),
        waiting,
        classes = size_classes(),
        "mem report"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_noted_collection_comes_back_named_by_tab() {
        // A registry of this test's own: what the process one would show
        // depends on which other tests have filed into it by now.
        let registry = Registry::new();
        registry.note("rows", 7, &vec![String::from("abc")]);
        let parts = registry.model_parts();
        let row = parts.iter().find(|(name, _, _)| name == "rows#7");
        assert!(row.is_some(), "the note should be filed under kind#tab");
        registry.forget(7);
        assert!(
            !registry
                .model_parts()
                .iter()
                .any(|(name, _, _)| name == "rows#7"),
            "a closed tab leaves nothing behind"
        );
    }
}
