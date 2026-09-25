//! What a gate run cost, kept where the next run cannot take it away.
//!
//! Each phase (the gate lock, the plan, the runner, the two sides) is
//! timed on its own and said in one block at the end, also written under
//! `target/gate-runs/` so a before-and-after has both halves.
//!
//! Beside the block is the ledger ([`ledger`]), one row per unit of
//! either side. The block's `longest` is not a bill: which five it names
//! depends on this run's selection and order. What a group of verbs costs
//! is arithmetic over the ledger.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Under `target/`: anywhere git reads, a record would be an uncommitted
/// change the next gate refuses to run over.
const DIR: &str = "gate-runs";

/// How many records a tree keeps; a day of gating is a few dozen.
const KEEP: usize = 200;

/// How long one side's units waited for room another gate was holding,
/// how many did, and how long the units themselves ran. Filled by the
/// threads running them, read once at the end ([`super::step::run_one`]).
#[derive(Default)]
pub(crate) struct Waited {
    verbs: AtomicUsize,
    nanos: AtomicU64,
    /// The units that ran longest, longest first, kept to [`LONGEST`].
    longest: std::sync::Mutex<Vec<Unit>>,
    /// Every unit of this side, in the order they ended ([`Row`]).
    rows: std::sync::Mutex<Vec<Row>>,
}

/// One unit of work the gate ran, as the record names it.
pub(crate) struct Unit {
    pub id: String,
    pub weight: u32,
    pub ran: Duration,
}

/// One unit as the ledger names it.
pub(crate) struct Row {
    pub id: String,
    pub weight: u32,
    /// What became of it: `ran`, `cached` (a stamp the plan found),
    /// `cached-late` (one another tree wrote while this unit queued),
    /// `FAIL`, `halted` — kept from running or ended by the run's first
    /// red (`gate::halt`) — or `not-run` — a step a red earlier in its
    /// group stopped this run short of. The last two are rows, not
    /// absences: a missing row is the bookkeeping's fault
    /// ([`Waited::unaccounted`]).
    pub outcome: &'static str,
    /// Room the machine made it wait for, before it started.
    pub waited: Duration,
    /// From the first step of the run to this unit's start, so two rows
    /// say whether they overlapped.
    pub from_start: Duration,
    pub ran: Duration,
    /// The step's own `spent` line where it writes one
    /// (`verify::run::say_what_it_spent`), else empty.
    pub spent: String,
}

/// How many of the longest units a record names.
const LONGEST: usize = 5;

impl Waited {
    pub(crate) fn add(&self, waited: Duration) {
        if waited.is_zero() {
            return;
        }
        self.verbs.fetch_add(1, Ordering::Relaxed);
        let nanos = u64::try_from(waited.as_nanos()).unwrap_or(u64::MAX);
        self.nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    /// Notes that `id` held the machine for `ran`.
    pub(crate) fn ran(&self, id: &str, weight: u32, ran: Duration) {
        let mut longest = self
            .longest
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let at = longest.partition_point(|unit| unit.ran > ran);
        if at >= LONGEST {
            return;
        }
        longest.insert(
            at,
            Unit {
                id: id.to_string(),
                weight,
                ran,
            },
        );
        longest.truncate(LONGEST);
    }

    /// Notes what one unit came to, whether it ran or a stamp answered
    /// for it. Every unit of the side files one.
    pub(crate) fn filed(&self, row: Row) {
        self.rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(row);
    }

    /// Where the plan and the ledger disagree: a step the plan gave this
    /// side that no row answers for, or one two rows answer for.
    ///
    /// It checks the bookkeeping, not the run: stopped and cached steps
    /// have rows of their own (`super::sides::not_run`), so what is left
    /// is a path through the runner that files nothing.
    pub(crate) fn unaccounted(&self, planned: &[String]) -> Vec<String> {
        let rows = self
            .rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut filed: BTreeMap<&str, usize> = BTreeMap::new();
        for row in rows.iter() {
            *filed.entry(row.id.as_str()).or_default() += 1;
        }
        let mut notes = Vec::new();
        for id in planned {
            match filed.remove(id.as_str()) {
                Some(1) => {}
                Some(twice) => notes.push(format!("has {twice} rows for {id}")),
                None => notes.push(format!("has no row for {id}")),
            }
        }
        for (id, _) in filed {
            notes.push(format!("has a row for {id}, which the plan never gave it"));
        }
        notes
    }

    pub(crate) fn read(&self) -> (usize, Duration) {
        (
            self.verbs.load(Ordering::Relaxed),
            Duration::from_nanos(self.nanos.load(Ordering::Relaxed)),
        )
    }

    /// The longest units it saw, longest first.
    pub(crate) fn longest(&self) -> Vec<Unit> {
        self.longest
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|unit| Unit {
                id: unit.id.clone(),
                weight: unit.weight,
                ran: unit.ran,
            })
            .collect()
    }

    /// The two sides' lists as one, longest first: a landing waits out
    /// whichever unit of either side is running.
    pub(crate) fn longest_of(sides: [&Waited; 2]) -> Vec<Unit> {
        let mut units: Vec<Unit> = sides.iter().flat_map(|side| side.longest()).collect();
        units.sort_by_key(|unit| std::cmp::Reverse(unit.ran));
        units.truncate(LONGEST);
        units
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
    /// Whether that graph came off the shelf.
    pub graph_reused: bool,
    /// Why the graph was reused, rebuilt or kept out of the cache.
    pub graph_cache: String,
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
    /// How many units of the host side waited for room held elsewhere,
    /// and for how long in all.
    pub host_budget: (usize, Duration),
    /// The same for the container side.
    pub linux_budget: (usize, Duration),
    /// The longest units of the run, longest first: nothing is preempted,
    /// so a landing arriving mid-run waits one out.
    pub longest: Vec<Unit>,
    /// Every unit of both sides, as the table [`ledger`] writes
    /// ([`Row`]). Kept beside the record; empty for a run that started
    /// no side (a dry run, or one a refusal stopped).
    pub ledger: String,
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
    /// Verb steps selected, those of them not already stamped, and the
    /// census lines left to the full gate.
    pub verbs: (usize, usize, usize),
    pub outcome: &'a str,
}

/// `12.3s`, `1m02s`.
fn moment(spent: Duration) -> String {
    let secs = spent.as_secs();
    if secs < 60 {
        return format!("{:.1}s", spent.as_secs_f64());
    }
    format!("{}m{:02}s", secs / 60, secs % 60)
}

/// The block both the terminal and the record hold.
pub(crate) fn render(run: &Run<'_>, spent: &Spent, shift: &super::census::Shift) -> String {
    let (always, cached, to_run) = run.steps;
    let (verbs_selected, verbs_run, verbs_left) = run.verbs;
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
         {to_run}) / verbs {verbs_selected} selected, {verbs_run} to run, {verbs_left} left to \
         the full gate\n",
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
    if !spent.graph_cache.is_empty() {
        out.push_str(&format!("  graph cache {}\n", spent.graph_cache));
    }
    out.push_str(&format!(
        "  budget host {} unit(s) waited {} / linux {} unit(s) waited {}\n",
        spent.host_budget.0,
        moment(spent.host_budget.1),
        spent.linux_budget.0,
        moment(spent.linux_budget.1),
    ));
    if !spent.longest.is_empty() {
        let units: Vec<String> = spent
            .longest
            .iter()
            .map(|unit| format!("{} {} (weight {})", unit.id, moment(unit.ran), unit.weight))
            .collect();
        out.push_str(&format!("  longest {}\n", units.join(" / ")));
    }
    if !spent.ledger.is_empty() {
        out.push_str(&format!(
            "  ledger  {} unit(s) beside this block, in <this file>.units.tsv\n",
            spent.ledger.lines().count().saturating_sub(1),
        ));
    }
    // What the run's verbs did to the census, by name (`super::census::Shift`).
    out.push_str(&shift.block());
    out
}

/// Writes the block under `target/gate-runs/` and takes away all but the
/// newest [`KEEP`]. A record nobody could write is not worth a red gate.
pub(crate) fn keep(dir: &Path, run: &Run<'_>, spent: &Spent, shift: &super::census::Shift) {
    let records = dir.join("target").join(DIR);
    if std::fs::create_dir_all(&records).is_err() {
        return;
    }
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let name = format!("{at}-{}", std::process::id());
    let path = records.join(format!("{name}.txt"));
    let _ = std::fs::write(&path, render(run, spent, shift));
    if !spent.ledger.is_empty() {
        let _ = std::fs::write(records.join(format!("{name}.units.tsv")), &spent.ledger);
    }
    sweep(&records);
}

/// The header the ledger's columns are read by, and the order [`ledger`]
/// writes them in.
const COLUMNS: &str = "side\tid\toutcome\tweight\twaited_ms\tfrom_start_ms\tran_ms\tspent\n";

/// Every unit of both sides as one table.
pub(crate) fn ledger(sides: [(&str, &Waited); 2]) -> String {
    let mut out = String::from(COLUMNS);
    for (name, waited) in sides {
        let rows = waited
            .rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for row in rows.iter() {
            out.push_str(&format!(
                "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                row.id,
                row.outcome,
                row.weight,
                row.waited.as_millis(),
                row.from_start.as_millis(),
                row.ran.as_millis(),
                row.spent,
            ));
        }
    }
    out
}

/// All but the newest [`KEEP`] records, by the name they carry — the
/// epoch second sorts as it counts. Swept by run, not by file: a ledger
/// goes with its block.
fn sweep(records: &Path) {
    let Ok(entries) = std::fs::read_dir(records) else {
        return;
    };
    let mut runs: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter_map(|name| name.strip_suffix(".txt").map(str::to_string))
        .collect();
    if runs.len() <= KEEP {
        return;
    }
    runs.sort();
    let over = runs.len() - KEEP;
    for stale in runs.into_iter().take(over) {
        let _ = std::fs::remove_file(records.join(format!("{stale}.txt")));
        let _ = std::fs::remove_file(records.join(format!("{stale}.units.tsv")));
    }
}

#[cfg(test)]
mod tests {
    use super::{Row, Run, Spent, Unit, Waited, ledger, moment, render};
    use std::time::Duration;

    #[test]
    fn the_ledger_holds_every_unit_of_both_sides_with_what_each_one_came_to() {
        let host = Waited::default();
        let linux = Waited::default();
        host.filed(Row {
            id: "verify amend".to_string(),
            weight: 4,
            outcome: "ran",
            waited: Duration::from_millis(1200),
            from_start: Duration::from_millis(300),
            ran: Duration::from_millis(66_000),
            spent: "fixture=1673ms build=20874ms app=1812ms rest=402ms whole=24761ms".to_string(),
        });
        host.filed(Row {
            id: "fmt".to_string(),
            weight: 0,
            outcome: "cached",
            waited: Duration::ZERO,
            from_start: Duration::from_millis(10),
            ran: Duration::ZERO,
            spent: String::new(),
        });
        linux.filed(Row {
            id: "verify-linux amend".to_string(),
            weight: 4,
            outcome: "FAIL",
            waited: Duration::from_millis(50),
            from_start: Duration::from_millis(900),
            ran: Duration::from_millis(80_000),
            spent: String::new(),
        });
        let table = ledger([("host", &host), ("linux", &linux)]);
        let rows: Vec<&str> = table.lines().collect();
        assert_eq!(rows.len(), 4, "a header and one row per unit: {table}");
        assert!(rows[0].starts_with("side\tid\toutcome\t"));
        assert_eq!(
            rows[1],
            "host\tverify amend\tran\t4\t1200\t300\t66000\tfixture=1673ms build=20874ms \
             app=1812ms rest=402ms whole=24761ms"
        );
        // A stamp's row is still a row.
        assert_eq!(rows[2], "host\tfmt\tcached\t0\t0\t10\t0\t");
        assert_eq!(
            rows[3],
            "linux\tverify-linux amend\tFAIL\t4\t50\t900\t80000\t"
        );
    }

    fn filed(side: &Waited, id: &str, outcome: &'static str) {
        side.filed(Row {
            id: id.to_string(),
            weight: 0,
            outcome,
            waited: Duration::ZERO,
            from_start: Duration::ZERO,
            ran: Duration::ZERO,
            spent: String::new(),
        });
    }

    /// Keeps apart a step the run did not reach, which has a row of its
    /// own, and a step nobody filed.
    #[test]
    fn the_ledger_says_which_of_the_plans_steps_it_does_not_answer_for() {
        let side = Waited::default();
        let planned: Vec<String> = ["fmt", "clippy", "test it", "shipped"]
            .iter()
            .map(|id| (*id).to_string())
            .collect();
        filed(&side, "fmt", "ran");
        filed(&side, "clippy", "FAIL");
        // Not reached, and saying so: the group stopped at the red above.
        filed(&side, "test it", "not-run");
        assert_eq!(
            side.unaccounted(&planned),
            ["has no row for shipped"],
            "a step nothing filed is the one thing this is for"
        );

        // And the two ways a row can be wrong about the plan.
        filed(&side, "shipped", "ran");
        filed(&side, "shipped", "ran");
        filed(&side, "deny", "ran");
        assert_eq!(
            side.unaccounted(&planned),
            [
                "has 2 rows for shipped",
                "has a row for deny, which the plan never gave it"
            ]
        );
    }

    #[test]
    fn a_side_that_ran_nothing_still_answers_for_every_step() {
        let side = Waited::default();
        let planned: Vec<String> = ["fmt", "verify stash --preset basic"]
            .iter()
            .map(|id| (*id).to_string())
            .collect();
        for id in &planned {
            filed(&side, id, "cached");
        }
        assert!(side.unaccounted(&planned).is_empty());
    }

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
    fn the_tally_keeps_the_longest_units_of_both_sides() {
        let host = Waited::default();
        let linux = Waited::default();
        for (id, secs) in [("fmt", 1), ("clippy", 30), ("test it", 63), ("docs", 2)] {
            host.ran(id, 4, Duration::from_secs(secs));
        }
        linux.ran("test core", 4, Duration::from_secs(45));
        let longest: Vec<String> = Waited::longest_of([&host, &linux])
            .into_iter()
            .map(|unit| format!("{} {}", unit.id, unit.ran.as_secs()))
            .collect();
        assert_eq!(
            longest,
            ["test it 63", "test core 45", "clippy 30", "docs 2", "fmt 1"],
            "the longest of both sides, longest first"
        );
    }

    #[test]
    fn a_record_names_every_phase_it_timed() {
        let spent = Spent {
            graph: Duration::from_secs(1),
            graph_cache: "miss: invalid cache payload; saved".to_string(),
            sides: Duration::from_secs(90),
            total: Duration::from_secs(100),
            longest: vec![Unit {
                id: "test platitude-core it".to_string(),
                weight: 4,
                ran: Duration::from_secs(63),
            }],
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
            verbs: (353, 353, 0),
            outcome: "PASS",
        };
        let text = render(&run, &spent, &crate::gate::census::Shift::default());
        for word in [
            "gate lock",
            "graph",
            "graph cache miss: invalid cache payload; saved",
            "tree ids",
            "census",
            "plan",
            "prepare",
            "runner",
            "sides",
            "budget host",
            "longest test platitude-core it 1m03s (weight 4)",
            "1m30s",
            "1m40s",
            "steps 720",
        ] {
            assert!(text.contains(word), "{word:?} is not in {text}");
        }
    }
}
