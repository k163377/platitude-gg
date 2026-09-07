//! What a gate run cost, kept where the next run cannot take it away.
//!
//! The wall clock alone says a gate was slow, never where the time went.
//! A gate is four things in a row: the tree's one gate lock, the plan (a
//! dependency graph read off the sources, one listing of the tree for the
//! cache keys, the census and who wears whom), the task runner built and
//! copied, and the two sides — inside which a verb can be waiting for a
//! lane another seat's gate is holding. Each is timed on its own and said
//! in one block at the end.
//!
//! The same block is written under `target/gate-runs/`, one file per run.
//! A number printed to a terminal is gone by the next tool call, and a
//! before-and-after wants both halves; these files are what the second
//! half is compared against. They are under a kilobyte, and the newest
//! [`KEEP`] stay.

use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Where a run's record goes: under `target/`, which the tree's git does
/// not read, so a record is not an uncommitted change the next gate
/// refuses to run over.
const DIR: &str = "gate-runs";

/// How many records a tree keeps. What a comparison ever reaches back
/// for is the last handful, and a day of gating is a few dozen.
const KEEP: usize = 200;

/// How long the verbs of one side waited for a lane another gate was
/// holding, and how many did. Filled by the threads running the verbs,
/// read once at the end ([`super::verbs`]).
#[derive(Default)]
pub(crate) struct Waited {
    verbs: AtomicUsize,
    nanos: AtomicU64,
}

impl Waited {
    pub(crate) fn add(&self, waited: Duration) {
        if waited.is_zero() {
            return;
        }
        self.verbs.fetch_add(1, Ordering::Relaxed);
        let nanos = u64::try_from(waited.as_nanos()).unwrap_or(u64::MAX);
        self.nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    pub(crate) fn read(&self) -> (usize, Duration) {
        (
            self.verbs.load(Ordering::Relaxed),
            Duration::from_nanos(self.nanos.load(Ordering::Relaxed)),
        )
    }
}

/// The phases of one gate run, in the order they happen.
#[derive(Default)]
pub(crate) struct Spent {
    /// Waiting for the tree's one gate (`lanes::sole`).
    pub sole: Duration,
    /// Reading the dependency graph off the sources, or off the shelf
    /// (`super::reuse`).
    pub graph: Duration,
    /// Whether that graph came off the shelf rather than the sources.
    pub graph_reused: bool,
    /// The one `git ls-tree` the cache keys are made of.
    pub ids: Duration,
    /// The census, and the wearers the components are read through.
    pub census: Duration,
    /// The rest of the plan: the diff, the reach, the steps, the stamps.
    pub plan_rest: Duration,
    /// The tree's status and the stamp store, before anything runs.
    pub prepare: Duration,
    /// Building this program from the tree and copying it beside the logs.
    pub runner: Duration,
    /// Both sides, from the first step to the last.
    pub sides: Duration,
    /// Reading the census back to see whether a verb moved it.
    pub census_after: Duration,
    /// The whole run.
    pub total: Duration,
    /// How many verbs of the host side waited for a lane held elsewhere,
    /// and for how long in all.
    pub host_lanes: (usize, Duration),
    /// The same for the container side.
    pub linux_lanes: (usize, Duration),
}

/// What a run was asked to do and what came of it, beside its timings.
pub(crate) struct Run<'a> {
    pub what: &'a str,
    pub head: &'a str,
    pub jobs: usize,
    pub landing: bool,
    pub changed: usize,
    pub reach: usize,
    /// always, cached, to run.
    pub steps: (usize, usize, usize),
    /// Verb steps selected, and those of them not already stamped.
    pub verbs: (usize, usize),
    pub outcome: &'a str,
}

/// `12.3s`, `1m02s` — a phase of a gate is seconds to minutes, and a
/// tenth is the smallest difference worth reading.
fn moment(spent: Duration) -> String {
    let secs = spent.as_secs();
    if secs < 60 {
        return format!("{:.1}s", spent.as_secs_f64());
    }
    format!("{}m{:02}s", secs / 60, secs % 60)
}

/// The block both the terminal and the record hold.
pub(crate) fn render(run: &Run<'_>, spent: &Spent) -> String {
    let (always, cached, to_run) = run.steps;
    let (verbs_selected, verbs_run) = run.verbs;
    let mut out = format!(
        "gate: {} — {} ({}), HEAD {}, jobs {}{}\n",
        run.outcome,
        run.what,
        moment(spent.total),
        run.head,
        run.jobs,
        if run.landing { ", landing" } else { "" }
    );
    out.push_str(&format!(
        "  chose  changed {} / reach {} / steps {} (always {always}, cached {cached}, run \
         {to_run}) / verbs {verbs_selected} selected, {verbs_run} to run\n",
        run.changed,
        run.reach,
        always + cached + to_run,
    ));
    out.push_str(&format!(
        "  spent  gate lock {} / graph {}{} / tree ids {} / census {} / plan {} / prepare {} / \
         runner {} / sides {} / census back {}\n",
        moment(spent.sole),
        moment(spent.graph),
        if spent.graph_reused { " (kept)" } else { "" },
        moment(spent.ids),
        moment(spent.census),
        moment(spent.plan_rest),
        moment(spent.prepare),
        moment(spent.runner),
        moment(spent.sides),
        moment(spent.census_after),
    ));
    out.push_str(&format!(
        "  lanes  host {} verb(s) waited {} / linux {} verb(s) waited {}\n",
        spent.host_lanes.0,
        moment(spent.host_lanes.1),
        spent.linux_lanes.0,
        moment(spent.linux_lanes.1),
    ));
    out
}

/// Writes the block under `target/gate-runs/`, named by the second it
/// ended and the process that ran it, and takes away all but the newest
/// [`KEEP`]. A record nobody could write is not worth a red gate: the run
/// itself has already answered, and this is the note beside it.
pub(crate) fn keep(dir: &Path, run: &Run<'_>, spent: &Spent) {
    let records = dir.join("target").join(DIR);
    if std::fs::create_dir_all(&records).is_err() {
        return;
    }
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let path = records.join(format!("{at}-{}.txt", std::process::id()));
    let _ = std::fs::write(&path, render(run, spent));
    sweep(&records);
}

/// All but the newest [`KEEP`] records, by the name they carry — the
/// epoch second sorts as it counts, so the oldest names come first.
fn sweep(records: &Path) {
    let Ok(entries) = std::fs::read_dir(records) else {
        return;
    };
    let mut names: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "txt"))
        .collect();
    if names.len() <= KEEP {
        return;
    }
    names.sort();
    let over = names.len() - KEEP;
    for stale in names.into_iter().take(over) {
        let _ = std::fs::remove_file(stale);
    }
}

#[cfg(test)]
mod tests {
    use super::{Run, Spent, Waited, moment, render};
    use std::time::Duration;

    #[test]
    fn a_moment_reads_in_tenths_under_a_minute_and_in_minutes_over_one() {
        assert_eq!(moment(Duration::from_millis(1234)), "1.2s");
        assert_eq!(moment(Duration::from_millis(50)), "0.1s");
        assert_eq!(moment(Duration::from_secs(62)), "1m02s");
        assert_eq!(moment(Duration::from_secs(3600)), "60m00s");
    }

    #[test]
    fn a_lane_tally_counts_only_the_verbs_that_waited() {
        let waited = Waited::default();
        waited.add(Duration::ZERO);
        assert_eq!(waited.read(), (0, Duration::ZERO));
        waited.add(Duration::from_secs(2));
        waited.add(Duration::from_secs(3));
        assert_eq!(waited.read(), (2, Duration::from_secs(5)));
    }

    #[test]
    fn a_record_names_every_phase_it_timed() {
        let spent = Spent {
            graph: Duration::from_secs(1),
            sides: Duration::from_secs(90),
            total: Duration::from_secs(100),
            ..Spent::default()
        };
        let run = Run {
            what: "gate",
            head: "0123456789",
            jobs: 8,
            landing: false,
            changed: 1,
            reach: 46,
            steps: (4, 2, 714),
            verbs: (353, 353),
            outcome: "PASS",
        };
        let text = render(&run, &spent);
        for word in [
            "gate lock",
            "graph",
            "tree ids",
            "census",
            "plan",
            "prepare",
            "runner",
            "sides",
            "lanes",
            "1m30s",
            "1m40s",
            "steps 720",
        ] {
            assert!(text.contains(word), "{word:?} is not in {text}");
        }
    }
}
