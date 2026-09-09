//! `cargo xtask budget bench` — the two admission rules against each
//! other, on this machine, under the same offered load.
//!
//! What is being compared is a rule, so what has to be identical between
//! the two halves is everything else: the same units, in the same order,
//! each doing the same amount of work, on the same machine, in a ledger
//! of the run's own. A real gate cannot give that — its steps are cached
//! by what they read, its verbs depend on a release build, and running
//! three of them means three seats, which are not this session's to take
//! — so the load here is a **model of one gate**: the units a gate runs,
//! at the durations measured off real ones
//! (internal-docs/反映前テストの機械化.md §実測), scaled down by
//! `--scale`.
//!
//! The units are not sleeps. Each one burns real processor time —
//! calibrated once on the idle machine and handed to the children, so
//! that a unit under contention takes longer, exactly as a real one
//! does. A compiling unit burns it on four threads, which is what its
//! weight says it is. So the machine is real, the contention is real,
//! and the queueing is the real ledger's between real processes; the
//! only model is which units a gate runs and for how long.
//!
//! **What it therefore cannot show**: the reds that over-subscription
//! causes in the real thing (a verb starved past its watchdog is a
//! failure worth ten minutes — 反映前テストの機械化.md §動詞の天井), and
//! the memory and container I/O a real gate pays. Both fall on the old
//! rule's side, so the numbers here are the floor of the difference, not
//! the whole of it.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::ledger::{DIR, MODE, Pool};
use super::{Ask, COMPILE, LIGHT, Rank};

/// One unit of the modelled gate: what it is called, how long it took on
/// a real run, and what it takes of the machine.
struct Modelled {
    id: &'static str,
    secs: f64,
    weight: u32,
}

const fn unit(id: &'static str, secs: f64, weight: u32) -> Modelled {
    Modelled { id, secs, weight }
}

/// The always-steps: seconds each, no compiler, and ahead of everything
/// else (`gate::side`).
const ALWAYS: [Modelled; 4] = [
    unit("structure", 1.5, LIGHT),
    unit("waits", 2.5, LIGHT),
    unit("docs", 1.0, LIGHT),
    unit("fmt", 3.0, LIGHT),
];

/// The host's checks chain, in the plan's order. The wall clock of a
/// full gate is this chain (§実測: 115s, over half of it `test it`).
const HOST_CHECKS: [Modelled; 7] = [
    unit("deny", 5.0, LIGHT),
    unit("clippy platitude-core", 12.0, COMPILE),
    unit("clippy platitude-app", 20.0, COMPILE),
    unit("clippy xtask", 13.0, COMPILE),
    unit("test platitude-core it", 63.0, COMPILE),
    unit("test xtask", 13.0, COMPILE),
    unit("shipped", 10.0, COMPILE),
];

/// The container's, which is shorter and still minutes.
const LINUX_CHECKS: [Modelled; 5] = [
    unit("clippy platitude-core (linux)", 15.0, COMPILE),
    unit("clippy xtask (linux)", 12.0, COMPILE),
    unit("test platitude-core (linux)", 15.0, COMPILE),
    unit("test xtask (linux)", 34.0, COMPILE),
    unit("qmltest (linux)", 5.0, LIGHT),
];

/// The verbs of a side: how many, how long the median one takes, and
/// what the one that builds the release costs on top (§実測: 135 a side,
/// median 4s on the host and 6s in the container).
const HOST_VERBS: (usize, f64, f64) = (135, 4.0, 25.0);
const LINUX_VERBS: (usize, f64, f64) = (135, 6.0, 30.0);

/// `bare`, which reads the release the container's verbs built.
const BARE: Modelled = unit("bare (linux)", 20.0, COMPILE);

/// What one child says about its run.
#[derive(Default, Clone)]
struct Report {
    units: usize,
    waited: Duration,
    ran: Duration,
    failures: usize,
    /// The child's own wall clock, as the parent measured it.
    whole: Duration,
}

impl Report {
    fn render(&self) -> String {
        format!(
            "units {} waited_ms {} ran_ms {} failures {}\n",
            self.units,
            self.waited.as_millis(),
            self.ran.as_millis(),
            self.failures,
        )
    }

    fn parse(text: &str) -> Option<Report> {
        let mut words = text.split_whitespace();
        let mut report = Report::default();
        while let (Some(key), Some(value)) = (words.next(), words.next()) {
            match key {
                "units" => report.units = value.parse().ok()?,
                "waited_ms" => report.waited = Duration::from_millis(value.parse().ok()?),
                "ran_ms" => report.ran = Duration::from_millis(value.parse().ok()?),
                "failures" => report.failures = value.parse().ok()?,
                _ => {}
            }
        }
        Some(report)
    }
}

/// How much processor work a second of a modelled unit is, measured on
/// the machine before anything else runs and handed to every child, so
/// that all of them — and both halves of the comparison — spend the same
/// work on the same unit.
fn calibrate() -> u64 {
    // waits(measured): the calibration's own clock, which is the point of it
    let at = Instant::now();
    let mut spun = 1u64 << 20;
    loop {
        // waits(measured): the work's own clock, which is what calibration reads
        let started = Instant::now();
        burn(spun);
        let took = started.elapsed();
        if took >= Duration::from_millis(100) {
            let per_sec = (spun as f64 / took.as_secs_f64()) as u64;
            println!(
                "bench: calibrated at {per_sec} turns a second ({}ms spent measuring)",
                at.elapsed().as_millis()
            );
            return per_sec.max(1);
        }
        spun *= 2;
    }
}

/// The work a unit is made of: a hash chain the optimiser cannot drop.
fn burn(turns: u64) {
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    for turn in 0..turns {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(turn | 1);
    }
    std::hint::black_box(x);
}

/// One unit's worth of work, on as many threads as its weight says it
/// drives: a compiling unit is cargo and its rustc jobs, not one process.
fn spend(unit: &Modelled, rate: u64, scale: f64) {
    let turns = (unit.secs / scale * rate as f64) as u64;
    let threads = if unit.weight >= COMPILE { 4 } else { 1 };
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| burn(turns));
        }
    });
}

/// `budget bench …` — the driver.
pub(super) fn run(args: &[String]) -> Result<(), String> {
    let mut gates = 3usize;
    let mut scale = 5.0f64;
    let mut land_after = Duration::from_secs(5);
    let mut land = false;
    let mut every = false;
    let mut jobs = crate::gate::default_jobs();
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        let next = |at: &mut usize| -> Result<String, String> {
            *at += 1;
            args.get(*at)
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--gates" => {
                gates = next(&mut at)?
                    .parse()
                    .map_err(|_| "--gates takes a count")?
            }
            "--scale" => {
                scale = next(&mut at)?
                    .parse()
                    .map_err(|_| "--scale takes a number")?
            }
            "--jobs" => jobs = next(&mut at)?.parse().map_err(|_| "--jobs takes a count")?,
            "--land" => land = true,
            // The three the comparison owes: one gate alone (where a
            // rule can only make things worse), several at once, and a
            // landing arriving into several at once.
            "--scenarios" => every = true,
            "--land-after-ms" => {
                let ms = next(&mut at)?
                    .parse()
                    .map_err(|_| "--land-after-ms takes a count")?;
                land_after = Duration::from_millis(ms);
            }
            other => return Err(format!("unknown option {other:?}")),
        }
        at += 1;
    }
    if scale <= 0.0 {
        return Err("--scale takes a number above zero".into());
    }
    let rate = calibrate();
    let repo = make_repo()?;
    println!(
        "bench: scale 1/{scale}, jobs {jobs}, budget {}, {} thread(s) — in {}",
        super::demand(jobs),
        std::thread::available_parallelism().map_or(0, |n| n.get()),
        repo.display()
    );
    let scenarios: Vec<(usize, bool)> = if every {
        vec![(1, false), (gates, false), (gates, true)]
    } else {
        vec![(gates, land)]
    };
    for (gates, land) in scenarios {
        println!(
            "\n  {} gate(s){}",
            gates,
            if land {
                format!(", a landing after {}ms", land_after.as_millis())
            } else {
                String::new()
            }
        );
        println!(
            "  {:<8} {:>9} {:>11} {:>9} {:>11} {:>11} {:>7} {:>9}",
            "rule", "land", "land queued", "all", "waited", "ran", "units", "failures"
        );
        for mode in ["lanes", "budget"] {
            let round = one_round(&repo, mode, gates, land, land_after, jobs, scale, rate)?;
            println!("{round}");
        }
    }
    let _ = std::fs::remove_dir_all(&repo);
    Ok(())
}

/// One scenario under one rule.
#[expect(
    clippy::too_many_arguments,
    reason = "one scenario's parameters, each named at the command line"
)]
fn one_round(
    repo: &Path,
    mode: &str,
    gates: usize,
    land: bool,
    land_after: Duration,
    jobs: usize,
    scale: f64,
    rate: u64,
) -> Result<String, String> {
    // The ledger of the round before, gone: a rule is measured against
    // an empty machine, not against what the other one left.
    let _ = std::fs::remove_dir_all(repo.join(".git").join(DIR));
    let mut running: Vec<(String, Instant, Child, PathBuf)> = Vec::new();
    // waits(measured): the round's wall clock, which is what it measures
    let started = Instant::now();
    for gate in 0..gates {
        let seat = format!("g{gate}");
        running.push(shaped(repo, mode, &seat, false, jobs, scale, rate)?);
    }
    if land {
        // The landing arrives with the gates already under way, which is
        // the case the rule is for: it has to be let in past work that
        // was there first. A clock is what "midway" means here — the
        // scenario's own parameter, not a judgement on the answer.
        // waits(paced): the scenario's injection point
        std::thread::sleep(land_after);
        running.push(shaped(repo, mode, "land", true, jobs, scale, rate)?);
    }
    let mut land_took = Duration::ZERO;
    // The landing's own queueing, which is what the priority is for: the
    // rest of the table moves with the whole machine's speed, and this
    // column moves with the rule.
    let mut land_waited = Duration::ZERO;
    let mut all = Report::default();
    for (seat, at, mut child, report) in running {
        let status = child.wait().map_err(|e| e.to_string())?;
        let whole = at.elapsed();
        let mut mine = std::fs::read_to_string(&report)
            .ok()
            .and_then(|text| Report::parse(&text))
            .unwrap_or_default();
        mine.whole = whole;
        if !status.success() {
            mine.failures += 1;
        }
        if seat == "land" {
            land_took = whole;
            land_waited = mine.waited;
        }
        all.units += mine.units;
        all.waited += mine.waited;
        all.ran += mine.ran;
        all.failures += mine.failures;
        let _ = std::fs::remove_file(&report);
    }
    Ok(format!(
        "  {:<8} {:>9} {:>11} {:>9} {:>11} {:>11} {:>7} {:>9}",
        mode,
        clock(land_took),
        clock(land_waited),
        clock(started.elapsed()),
        clock(all.waited),
        clock(all.ran),
        all.units,
        all.failures,
    ))
}

/// Starts one modelled gate as a process of its own.
fn shaped(
    repo: &Path,
    mode: &str,
    seat: &str,
    landing: bool,
    jobs: usize,
    scale: f64,
    rate: u64,
) -> Result<(String, Instant, Child, PathBuf), String> {
    let report = repo.join(format!("report-{mode}-{seat}"));
    let _ = std::fs::remove_file(&report);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = Command::new(exe);
    command
        .args(["budget", "shaped", "--dir"])
        .arg(repo)
        .args(["--seat", seat])
        .args(["--jobs", &jobs.to_string()])
        .args(["--scale", &scale.to_string()])
        .args(["--rate", &rate.to_string()])
        .arg("--report")
        .arg(&report)
        .env(MODE, mode)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if landing {
        command.arg("--landing");
    }
    // waits(measured): this child's wall clock, which is the land column
    let at = Instant::now();
    let child = command.spawn().map_err(|e| e.to_string())?;
    Ok((seat.to_string(), at, child, report))
}

/// A repository of the bench's own, so the ledger under test is the
/// bench's and no seat's gate is measured into it — or disturbed by it.
fn make_repo() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("pg-bench-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut command = Command::new("git");
    command.arg("-C").arg(&dir).args(["init", "--quiet"]);
    let out = crate::subprocess::run_captured(&mut command)?;
    if !out.status.success() {
        return Err(format!(
            "could not make a repository in {}: {}",
            dir.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(dir)
}

fn clock(took: Duration) -> String {
    let secs = took.as_secs();
    format!("{}m{:02}s", secs / 60, secs % 60)
}

/// `budget shaped …` — one modelled gate, run as its own process.
pub(super) fn shaped_run(args: &[String]) -> Result<(), String> {
    let mut dir = crate::tree::workspace_root();
    let mut seat = "bench".to_string();
    let mut report = PathBuf::new();
    let (mut jobs, mut scale, mut rate, mut landing) =
        (crate::gate::default_jobs(), 5.0, 0u64, false);
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        let next = |at: &mut usize| -> Result<String, String> {
            *at += 1;
            args.get(*at)
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--dir" => dir = PathBuf::from(next(&mut at)?),
            "--seat" => seat = next(&mut at)?,
            "--report" => report = PathBuf::from(next(&mut at)?),
            "--jobs" => jobs = next(&mut at)?.parse().map_err(|_| "--jobs takes a count")?,
            "--scale" => {
                scale = next(&mut at)?
                    .parse()
                    .map_err(|_| "--scale takes a number")?
            }
            "--rate" => rate = next(&mut at)?.parse().map_err(|_| "--rate takes a count")?,
            "--landing" => landing = true,
            other => return Err(format!("unknown option {other:?}")),
        }
        at += 1;
    }
    let pool = Pool::of(&dir, jobs)?;
    // A landing takes its turn first, as `land` does, and only then asks
    // for any of the machine.
    let _turn = if landing {
        Some(pool.turn(&seat, &format!("land {seat}"))?)
    } else {
        None
    };
    let rank = if landing { Rank::Landing } else { Rank::Normal };
    let tally = Tally::default();
    let ground = Ground {
        pool: &pool,
        seat: &seat,
        rank,
        rate,
        scale,
        jobs,
        tally: &tally,
    };
    for unit in &ALWAYS {
        ground.one(unit, "host")?;
    }
    std::thread::scope(|scope| {
        scope.spawn(|| ground.side("host", &HOST_CHECKS, HOST_VERBS, None));
        scope.spawn(|| ground.side("linux", &LINUX_CHECKS, LINUX_VERBS, Some(&BARE)));
    });
    if !report.as_os_str().is_empty() {
        std::fs::write(&report, tally.report().render())
            .map_err(|e| format!("{}: {e}", report.display()))?;
    }
    Ok(())
}

/// What the modelled gate's units add up to, filled from every thread.
#[derive(Default)]
struct Tally {
    units: std::sync::atomic::AtomicUsize,
    waited: std::sync::atomic::AtomicU64,
    ran: std::sync::atomic::AtomicU64,
}

impl Tally {
    fn add(&self, waited: Duration, ran: Duration) {
        use std::sync::atomic::Ordering::Relaxed;
        self.units.fetch_add(1, Relaxed);
        self.waited.fetch_add(
            u64::try_from(waited.as_millis()).unwrap_or(u64::MAX),
            Relaxed,
        );
        self.ran
            .fetch_add(u64::try_from(ran.as_millis()).unwrap_or(u64::MAX), Relaxed);
    }

    fn report(&self) -> Report {
        use std::sync::atomic::Ordering::Relaxed;
        Report {
            units: self.units.load(Relaxed),
            waited: Duration::from_millis(self.waited.load(Relaxed)),
            ran: Duration::from_millis(self.ran.load(Relaxed)),
            failures: 0,
            whole: Duration::ZERO,
        }
    }
}

/// What one modelled gate's units run against.
struct Ground<'a> {
    pool: &'a Pool,
    seat: &'a str,
    rank: Rank,
    rate: u64,
    scale: f64,
    jobs: usize,
    tally: &'a Tally,
}

impl Ground<'_> {
    /// One unit: the machine's room taken for it, the work spent, the
    /// room given back.
    fn one(&self, unit: &Modelled, side: &str) -> Result<(), String> {
        let room = self.pool.admit(&Ask {
            weight: unit.weight,
            rank: self.rank,
            seat: self.seat,
            what: unit.id,
            side,
        })?;
        // waits(measured): the unit's own wall clock, for the report
        let at = Instant::now();
        spend(unit, self.rate, self.scale);
        self.tally.add(room.waited, at.elapsed());
        Ok(())
    }

    /// One side, shaped as the gate's is: the checks chain in order
    /// beside the block of verbs, and the trailing step after them.
    fn side(
        &self,
        side: &str,
        checks: &[Modelled],
        verbs: (usize, f64, f64),
        after: Option<&Modelled>,
    ) {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for unit in checks {
                    let _ = self.one(unit, side);
                }
            });
            scope.spawn(|| {
                let (count, each, build) = verbs;
                // The first verb builds the release and runs alone.
                let _ = self.one(&unit("verify (building)", build, COMPILE), side);
                let next = std::sync::atomic::AtomicUsize::new(1);
                std::thread::scope(|inner| {
                    for _ in 0..self.jobs.max(1) {
                        inner.spawn(|| {
                            while next.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < count {
                                let _ = self.one(&unit("verify", each, LIGHT), side);
                            }
                        });
                    }
                });
                if let Some(unit) = after {
                    let _ = self.one(unit, side);
                }
            });
        });
    }
}

/// The rule a child was told to run under, which the driver sets in the
/// environment ([`MODE`]) — named here so the module that owns
/// the switch is the one that spells it.
#[cfg(test)]
pub(super) fn mode_of(word: &str) -> super::ledger::Mode {
    match word {
        "lanes" => super::ledger::Mode::Lanes,
        _ => super::ledger::Mode::Whole,
    }
}

#[cfg(test)]
mod tests {
    use super::{ALWAYS, HOST_CHECKS, LINUX_CHECKS, Report, mode_of};
    use crate::budget::ledger::Mode;
    use crate::budget::{COMPILE, LIGHT};
    use std::time::Duration;

    #[test]
    fn a_child_s_report_goes_out_and_comes_back() {
        let text = Report {
            units: 12,
            waited: Duration::from_millis(1500),
            ran: Duration::from_millis(9000),
            failures: 1,
            whole: Duration::ZERO,
        }
        .render();
        let back = Report::parse(&text).expect("the report back");
        assert_eq!(back.units, 12);
        assert_eq!(back.waited, Duration::from_millis(1500));
        assert_eq!(back.ran, Duration::from_millis(9000));
        assert_eq!(back.failures, 1);
    }

    /// The modelled units carry the weights the real ones would be read
    /// as, or the load offered is not the load a gate offers.
    #[test]
    fn the_modelled_units_weigh_what_the_real_ones_do() {
        for unit in &ALWAYS {
            assert_eq!(unit.weight, LIGHT, "{} starts no compiler", unit.id);
        }
        let compiling = HOST_CHECKS
            .iter()
            .chain(LINUX_CHECKS.iter())
            .filter(|unit| unit.weight == COMPILE)
            .count();
        assert_eq!(
            compiling,
            HOST_CHECKS.len() + LINUX_CHECKS.len() - 2,
            "the chains are compiles but for deny and qmltest"
        );
    }

    #[test]
    fn the_driver_names_the_rule_its_children_run_under() {
        assert_eq!(mode_of("lanes"), Mode::Lanes);
        assert_eq!(mode_of("budget"), Mode::Whole);
    }
}
