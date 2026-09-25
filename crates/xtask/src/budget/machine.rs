//! The machine as every process on it names it the same: how many
//! units fit at once, and the name a tree queues under. Read by the
//! gate, the budget, `check` and `land` alike, so it reads nothing of
//! theirs (反映前テストの機械化.md §依存木).

use std::path::Path;

/// How many verbs a side runs at once unless `--jobs` says (why a third:
/// internal-docs/反映前テストの機械化.md §実測). Also what the machine's
/// whole budget is computed from (`budget::demand`), so every process
/// names the same pool.
pub(crate) fn default_jobs() -> usize {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    (cpus / 3).clamp(1, 8)
}

/// Which tree a unit belongs to, as the queue names it: the seat's
/// letter, or the checkout's own name for the primary.
pub(crate) fn seat_of(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.display().to_string())
}
