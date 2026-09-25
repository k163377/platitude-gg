//! What the process is holding on to, and where. Two halves, read
//! together:
//!
//! * **the counting allocator** is ground truth for live Rust heap. Qt
//!   allocates through C++ `operator new`, never through Rust's
//!   `GlobalAlloc`, so this number splits "the data this application
//!   built" from "the toolkit under it" exactly;
//! * **the registry** attributes bytes *inside* that number to the models
//!   holding them; the report prints what is left over as a signed
//!   remainder.
//!
//! The counting is behind the `memprobe` feature; the per-model walks,
//! O(rows), also need `PGG_MEM_REPORT=1` ([`enabled`]).
//!
//! The process's allocator is chosen here too: there is one
//! `#[global_allocator]` slot and the counter has to sit in front of
//! whatever fills it.
//!
//! The report half's one caller is `harness::singleton::Harness`, not
//! compiled without the harness. `allow`, not `expect`: the test keeps
//! some of the names live, which would leave the expectation unfulfilled.
#![cfg_attr(not(feature = "automation"), allow(dead_code))]

use std::collections::BTreeMap;
use std::sync::Mutex;
#[cfg(feature = "memprobe")]
use std::sync::atomic::Ordering;

use platitude_core::mem::{Footprint, Part};

// ---------------------------------------------------------------------------
// The allocator
// ---------------------------------------------------------------------------

/// What every allocation goes to: the platform's own
/// (rules-refs/core.md「mimalloc は不採用」). A measuring build puts the
/// counter in front of the same one.
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

    /// Live bytes by size class (bucket *n* = allocations under 2^n bytes
    /// and at least 2^(n-1)). When the named parts do not add up to the
    /// live total, the shape of the remainder narrows the search: millions
    /// of small allocations are a collection of nodes, a handful of
    /// megabyte blocks a buffer somebody is holding.
    pub(super) static CLASSES: [AtomicUsize; 32] = [const { AtomicUsize::new(0) }; 32];

    /// Live allocations by the same size class: bytes alone cannot tell one
    /// big buffer from thousands of small nodes.
    pub(super) static COUNTS: [AtomicUsize; 32] = [const { AtomicUsize::new(0) }; 32];

    pub(super) fn class_of(size: usize) -> usize {
        // Saturated: 2^30 bytes and up are all filed in the top bucket.
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

/// Whether the per-model walks run at all: only with the counting
/// allocator (parts with no counted total are half a report) and
/// `PGG_MEM_REPORT=1`. The `cfg!` takes the walk out of a shipped build
/// at compile time.
pub fn enabled() -> bool {
    cfg!(feature = "memprobe") && super::knobs().mem_report
}

/// What each model last reported, keyed by kind and tab; sorted, so two
/// reports of the same shape line up. A value, so a test can file into a
/// registry of its own; the process's is [`REGISTRY`].
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

    /// Files a collection's footprint under `kind` for tab `tab`. Call it
    /// where the collection settles — the end of a drain — and only under
    /// [`enabled`]: it walks the items.
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

/// One report line for where the run has got to. Called on the Qt main
/// thread, where the sessions live (`Harness::note_memory`).
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
/// left. `session_parts` is the core side (`RepoSession::heap_report`).
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
        // Signed: under zero is an `Arc` counted on both sides (the refs
        // snapshot, held by the session and every sidebar section), over
        // it is what nothing named.
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
