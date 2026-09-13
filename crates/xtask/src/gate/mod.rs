//! `cargo xtask gate` — the pre-merge tests, chosen by machine.
//!
//! What a branch owes main is read off its diff: the files it changed,
//! everything that reads them ([`graph`]), and the tests *in* that reach
//! — unit tests by module path, the integration binary by top-level
//! module, the verify-ui verbs by the census of what each one shows
//! ([`census`]) — plus the crate-wide checks (clippy, shipped, bare) for
//! the crates the reach enters ([`plan`]). Each step is cached by the
//! object ids of what it reads ([`stamp`]), so a second run of one commit
//! runs nothing, and a rebase reruns only what main's move touched. A
//! commit whose every owed step is green is stamped, and the
//! `reference-transaction` hook lets `refs/heads/main` move onto stamped
//! commits only ([`hooks`]). The host's verb runs rewrite their own
//! census lines as they go, so the file follows the change that moved it;
//! a gate that finds it rewritten stops before stamping, because the
//! commit that passed has to be the one holding it.
//!
//! `--host-only` is the daily tier (CLAUDE.md 確認は 3 段: no container);
//! its stamps are reused by the full run, which then owes the container
//! side alone. `--all` counts every file as changed — stage 2 in full,
//! what `check` used to be — and `--verb <line>` adds verify-ui lines to
//! run (and record) besides the census's.

mod census;
mod deps;
mod graph;
mod hooks;
mod inputs;
mod plan;
mod record;
mod reuse;
mod stamp;

use std::path::Path;

use plan::{Plan, Required, Side};
use record::{Spent, Waited};
pub(crate) use reuse::preserve_reader;
use stamp::{CommitStamp, Store};

use census::Shift;
// The census is read by the gate to choose verbs and by `verbs` to say
// which verbs it never chooses, so the type and the file lister under it
// stand where both can reach them.
pub(crate) use census::Census;
pub(crate) use census::{FILE as CENSUS_FILE, names_in, page_settled_in, record};
pub(crate) use graph::qml_files;
pub(crate) use hooks::{SESSION, SKIP, install};

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("verdict") => {
            let [old, new] = &args[1..] else {
                return Err("gate verdict takes <old> <new>".into());
            };
            let dir = std::env::current_dir().map_err(|e| e.to_string())?;
            hooks::verdict(&dir, old, new)
        }
        Some("install") => {
            let dir = match args.get(1..3) {
                Some([flag, path]) if flag == "--dir" => std::path::PathBuf::from(path),
                _ => std::env::current_dir().map_err(|e| e.to_string())?,
            };
            println!("{}", install(&dir)?);
            Ok(())
        }
        Some("deps") => deps::run(&args[1..]),
        _ => gate(args),
    }
}

/// What the command line asks of the gate.
struct Options {
    /// The tree to gate: this workspace unless `--dir` names another (a
    /// seat from the primary, a throwaway repository in the tests).
    dir: std::path::PathBuf,
    main_ref: String,
    host_only: bool,
    all: bool,
    fresh: bool,
    dry_run: bool,
    verbs: Vec<String>,
    /// How many verify-ui verbs a side runs at once ([`verbs`]).
    jobs: usize,
}

fn options(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        dir: crate::tree::workspace_root(),
        main_ref: "main".to_string(),
        host_only: false,
        all: false,
        fresh: false,
        dry_run: false,
        verbs: Vec::new(),
        jobs: default_jobs(),
    };
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--host-only" => opts.host_only = true,
            "--all" => opts.all = true,
            "--fresh" => opts.fresh = true,
            "--dry-run" => opts.dry_run = true,
            "--dir" => {
                at += 1;
                opts.dir = std::path::PathBuf::from(args.get(at).ok_or("--dir needs a path")?);
            }
            "--main" => {
                at += 1;
                opts.main_ref = args.get(at).ok_or("--main needs a ref")?.clone();
            }
            "--verb" => {
                at += 1;
                opts.verbs
                    .push(args.get(at).ok_or("--verb needs a verify-ui line")?.clone());
            }
            "--jobs" => {
                at += 1;
                let value = args.get(at).ok_or("--jobs needs a count")?;
                opts.jobs = value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n >= 1)
                    .ok_or_else(|| format!("--jobs takes a count of 1 or more, not {value:?}"))?;
            }
            other => {
                return Err(format!(
                    "unknown option {other:?} (gate takes --host-only, --all, --fresh, \
                     --dry-run, --dir <tree>, --main <ref>, --verb <line>…, --jobs <n>)"
                ));
            }
        }
        at += 1;
    }
    Ok(opts)
}

/// How many verbs a side runs at once unless `--jobs` says: a third of
/// the logical CPUs, one at least and eight at most. A verb is one app
/// (its QML engine and offscreen raster) plus the git it spawns, and both
/// sides run at once; the measurement behind the third is in
/// internal-docs/反映前テストの機械化.md §実測.
///
/// It is also what the machine's whole budget is computed from
/// (`budget::demand`), so every process on the machine names the same
/// pool without having to agree on anything but this.
pub(crate) fn default_jobs() -> usize {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    (cpus / 3).clamp(1, 8)
}

fn gate(args: &[String]) -> Result<(), String> {
    let opts = options(args)?;
    let mut spent = Spent::default();
    // waits(measured): the run's whole, for the record (`Spent::total`)
    let whole_run = std::time::Instant::now();
    // The tree's one gate, taken before the graph is read: a plan is
    // seconds of reading, and a second gate here has nothing to read. A
    // dry run holds nothing, because it runs nothing.
    let what = format!("gate {}", args.join(" "));
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let _sole = if opts.dry_run {
        None
    } else {
        Some(crate::lanes::sole(
            &running_note(&opts.dir),
            what.trim_end(),
        )?)
    };
    spent.sole = at.elapsed();
    let plan = plan::make(
        &opts.dir,
        &plan::Ask {
            main_ref: &opts.main_ref,
            host_only: opts.host_only,
            all: opts.all,
            fresh: opts.fresh,
            extra_verbs: &opts.verbs,
        },
        &mut spent,
    )?;
    print!("{}", plan::describe(&plan));
    let mut shift = Shift::default();
    if opts.dry_run {
        // The plan's own cost, said the way a run's is: what a dry run
        // is for is reading a selection, and how long the reading took
        // is half of what is asked of it here. Nothing is kept — no run
        // happened, and a record of one would be a run that did not.
        spent.total = whole_run.elapsed();
        let head = short(&plan.head);
        let run = stood(&plan, what.trim_end(), &head, opts.jobs, false, "dry run");
        print!("{}", record::render(&run, &spent, &shift));
        return Ok(());
    }
    let outcome = execute(&plan, opts.jobs, false, &mut spent, &mut shift);
    spent.total = whole_run.elapsed();
    report(
        &plan,
        what.trim_end(),
        opts.jobs,
        false,
        &spent,
        &outcome,
        &shift,
    );
    match outcome? {
        Gated::Stamped => Ok(()),
        // A tree left dirty without a word is the next gate refusing to
        // run over a change nobody made — which is the whole complaint
        // the recording answers. Said as the failure it is for the
        // stamp: the commit that passed has to be the one holding it.
        Gated::CensusMoved => Err(format!(
            "the verbs passed and the tree moved with them — nothing stamped for {}.{}",
            short(&plan.head),
            census_rewritten()
        )),
    }
}

/// The gate for `land`: the seat's tree, both sides, the census's verbs.
/// What it answers is the landing's to act on — a census the verbs moved
/// is committed there and gated again, rather than reported.
pub(crate) fn for_landing(seat: &Path, main_ref: &str) -> Result<Gated, String> {
    let mut spent = Spent::default();
    // waits(measured): the run's whole, for the record (`Spent::total`)
    let whole_run = std::time::Instant::now();
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let _sole = crate::lanes::sole(&running_note(seat), "land's gate")?;
    spent.sole = at.elapsed();
    let plan = plan::make(
        seat,
        &plan::Ask {
            main_ref,
            host_only: false,
            all: false,
            fresh: false,
            extra_verbs: &[],
        },
        &mut spent,
    )?;
    print!("{}", plan::describe(&plan));
    let jobs = default_jobs();
    let mut shift = Shift::default();
    let outcome = execute(&plan, jobs, true, &mut spent, &mut shift);
    spent.total = whole_run.elapsed();
    report(&plan, "land's gate", jobs, true, &spent, &outcome, &shift);
    outcome
}

/// What the plan chose, counted for the record.
fn stood<'a>(
    plan: &'a Plan,
    what: &'a str,
    head: &'a str,
    jobs: usize,
    landing: bool,
    outcome: &'a str,
) -> record::Run<'a> {
    let counted = |kept: fn(&Required) -> bool| plan.required.iter().filter(|r| kept(r)).count();
    record::Run {
        what,
        head,
        jobs,
        landing,
        changed: plan.changed.len(),
        reach: plan.reach.len(),
        steps: (
            counted(|r| r.step.always),
            counted(|r| r.cached),
            counted(|r| !r.step.always && !r.cached),
        ),
        verbs: (
            counted(|r| r.step.id.starts_with("verify")),
            counted(|r| r.step.id.starts_with("verify") && !r.cached),
        ),
        outcome,
    }
}

/// The run's block, said and kept ([`record`]). Both roads out of a gate
/// pass through here — a red run's phases are what a slow one is read
/// from as much as a green one's.
fn report(
    plan: &Plan,
    what: &str,
    jobs: usize,
    landing: bool,
    spent: &Spent,
    outcome: &Result<Gated, String>,
    shift: &Shift,
) {
    let head = short(&plan.head);
    let run = stood(
        plan,
        what,
        &head,
        jobs,
        landing,
        match outcome {
            Ok(Gated::Stamped) => "PASS",
            Ok(Gated::CensusMoved) => "census moved",
            Err(_) => "FAIL",
        },
    );
    print!("{}", record::render(&run, spent, shift));
    record::keep(&plan.dir, &run, spent, shift);
}

/// What a gate whose every step was green left behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gated {
    /// The commit is stamped.
    Stamped,
    /// Nothing is: the host's verbs rewrote the census, so the tree that
    /// passed is no longer the commit a stamp would name. The rewrite is
    /// generated — commit it and gate again, which finds every step
    /// cached.
    CensusMoved,
}

fn short(sha: &str) -> String {
    sha.chars().take(10).collect()
}

/// The line that says what a verb's rewrite means for whoever reads it.
fn census_rewritten() -> String {
    format!(
        " The verbs rewrote {}: it is generated, so review the diff and commit it (never by \
         hand), and the gate can stamp the commit that holds it.",
        census::FILE
    )
}

/// Where a tree's running gate leaves its note (`lanes::sole`): under
/// `target/`, which the tree's git does not read, so the note is not the
/// uncommitted change the gate refuses to run over.
fn running_note(dir: &Path) -> std::path::PathBuf {
    dir.join("target").join("gate-running")
}

/// Runs what is not cached, one thread per side and two groups within a
/// side ([`side`]), and stamps the commit when both sides came back with
/// nothing red. A group stops at its first red step — except among its
/// verbs, which run to the end of their block `jobs` at a time
/// ([`verbs`]) — and everything else finishes, so its green steps are
/// stamped and need not run again. A `landing`'s verbs are handed the
/// machine's lanes ahead of any other gate's (`lanes`).
fn execute(
    plan: &Plan,
    jobs: usize,
    landing: bool,
    spent: &mut Spent,
    shift: &mut Shift,
) -> Result<Gated, String> {
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    refuse_what_no_stamp_could_answer_for(plan)?;
    // Yesterday's runs go on their way out: a verb's repositories and
    // pictures are left where a person can look at them, and nothing
    // else ever takes them away (measured: sixty thousand of them, six
    // gigabytes, in three days).
    crate::verify::sweep_yesterdays_runs();
    // What the census said before the verbs ran, so that a line one of
    // them rewrote can be told from the file as it was committed.
    let census_before = std::fs::read(plan.dir.join(census::FILE)).unwrap_or_default();
    let store = Store::open(&plan.dir)?;
    let logs = plan.dir.join("target").join("gate-logs");
    std::fs::create_dir_all(&logs).map_err(|e| format!("{}: {e}", logs.display()))?;
    spent.prepare = at.elapsed();
    // The faked steps of the tests run nothing, in a repository that has
    // no task runner to build.
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let runner = if std::env::var_os(FAKE_LOG).is_some() {
        None
    } else {
        Some(runner(&plan.dir, &logs, jobs, landing)?)
    };
    spent.runner = at.elapsed();
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let failures = run_sides(plan, &store, &logs, runner.as_deref(), jobs, landing, spent)?;
    spent.sides = at.elapsed();
    let head = short(&plan.head);
    // A verb that passed rewrote its census line whether or not another
    // step went red, so this is said on both roads out.
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let census_now = std::fs::read(plan.dir.join(census::FILE)).unwrap_or_default();
    // The bytes answer whether the tree moved (a header the sources
    // changed moves it too, and the next gate would refuse to run over
    // that); the sets answer what moved, which is what a reader of a
    // whole-file diff cannot get at.
    let rewrote = census_now != census_before;
    *shift = Shift::between(
        &Census::parse(&String::from_utf8_lossy(&census_before)),
        &Census::parse(&String::from_utf8_lossy(&census_now)),
    );
    spent.census_after = at.elapsed();
    if !failures.is_empty() {
        return Err(format!(
            "gate failed: {} — nothing stamped for {head}.{}",
            failures.join(" / "),
            if rewrote {
                census_rewritten()
            } else {
                String::new()
            }
        ));
    }
    // Green, and the tree that passed is no longer the commit: a stamp
    // names one, so the file has to be in it before a stamp is written.
    if rewrote {
        return Ok(Gated::CensusMoved);
    }
    // A daily run over a commit the full gate already stamped keeps the
    // full stamp: what it ran is a part of what that one ran, and
    // writing `full=false` over it would send the next landing back
    // through a container side that has already answered.
    if plan.host_only
        && store
            .commit(&plan.head)
            .is_some_and(|found| found.full && found.main == plan.main && found.base == plan.base)
    {
        println!("gate: PASS — {head} keeps its full stamp (this run was host-only)");
        return Ok(Gated::Stamped);
    }
    let stamp = CommitStamp {
        main: plan.main.clone(),
        base: plan.base.clone(),
        onto_main: plan.onto_main,
        full: !plan.host_only,
        steps: plan
            .required
            .iter()
            .map(|r| {
                format!(
                    "{} {}",
                    r.step.id,
                    if r.key.is_empty() { "-" } else { &r.key }
                )
            })
            .collect(),
    };
    store.mark_commit(&plan.head, &stamp)?;
    println!(
        "gate: PASS — {head} stamped ({}, {})",
        if stamp.full { "full" } else { "host-only" },
        if stamp.onto_main {
            "on main"
        } else {
            "off main"
        }
    );
    Ok(Gated::Stamped)
}

/// Both sides at once, each on a thread of its own ([`side`]) and every
/// unit of both out of the machine's one budget: what came back red,
/// once the wall clock has been said.
fn run_sides(
    plan: &Plan,
    store: &Store,
    logs: &Path,
    runner: Option<&Path>,
    jobs: usize,
    landing: bool,
    spent: &mut Spent,
) -> Result<Vec<String>, String> {
    let host: Vec<&Required> = plan
        .required
        .iter()
        .filter(|r| r.step.side == Side::Host)
        .collect();
    let linux: Vec<&Required> = plan
        .required
        .iter()
        .filter(|r| r.step.side == Side::Linux)
        .collect();
    // The budget is the machine's — beside the repository's `.git`,
    // which every seat shares (`budget`). `jobs` above the machine's
    // count widens the pool, an explicit ask; below it, it narrows this
    // gate's share of it.
    let count = jobs.max(default_jobs());
    let pool = crate::budget::Pool::of(&plan.dir, count)?;
    let seat = seat_of(&plan.dir);
    // waits(measured): the sides' wall clock, said when both are in
    let started = std::time::Instant::now();
    println!(
        "gate: verbs {jobs} at a time per side, every unit of both sides out of the machine's \
         one budget of {}{}; a side's checks run beside its verbs",
        crate::budget::demand(count),
        if landing {
            " (a landing's units go ahead of the other gates')"
        } else {
            ""
        }
    );
    let host_waited = Waited::default();
    let linux_waited = Waited::default();
    let host_ground = Ground {
        name: "host",
        dir: &plan.dir,
        store,
        logs,
        runner,
        waited: &host_waited,
        pool: &pool,
        seat: &seat,
        rank: rank(landing),
        fresh: plan.fresh,
    };
    let linux_ground = Ground {
        name: "linux",
        waited: &linux_waited,
        ..host_ground
    };
    let failures: Vec<String> = std::thread::scope(|scope| {
        let host = scope.spawn(|| side(&host_ground, &host, jobs));
        let linux = scope.spawn(|| side(&linux_ground, &linux, jobs));
        let mut failures = Vec::new();
        for handle in [host, linux] {
            match handle.join() {
                Ok(mut side_failures) => failures.append(&mut side_failures),
                Err(_) => failures.push("a side panicked".to_string()),
            }
        }
        failures
    });
    let secs = started.elapsed().as_secs();
    println!("gate: {}m{:02}s wall clock", secs / 60, secs % 60);
    spent.host_budget = host_waited.read();
    spent.linux_budget = linux_waited.read();
    spent.longest = Waited::longest_of([&host_waited, &linux_waited]);
    Ok(failures)
}

/// A tree with uncommitted changes (a stamp names a commit, and this is
/// not one) and a component no verb shows: nothing a run could answer
/// for, said before anything runs.
fn refuse_what_no_stamp_could_answer_for(plan: &Plan) -> Result<(), String> {
    let here = plan.dir.display().to_string();
    let dirty = crate::subprocess::git_query(&here, &["status", "--porcelain"])
        .ok_or("git status failed")?;
    if !dirty.is_empty() {
        return Err(format!(
            "the tree has uncommitted changes, and a stamp names a commit — commit or stash \
             first:\n{dirty}"
        ));
    }
    // Not the change's fault, and not a thing a step could answer for:
    // an edge the graph failed to draw is one every selection through it
    // is short by, so a stamp written over this tree would be saying more
    // than the run knows (`graph::complaints`).
    if !plan.complaints.is_empty() {
        return Err(format!(
            "the graph cannot read this tree whole, so no selection off it can be stamped:\n{}",
            plan.complaints
                .iter()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !plan.uncovered.is_empty() {
        return Err(format!(
            "no verb shows these components, so the gate cannot pass them:\n{}\nRun a verb \
             that brings each one up (`cargo xtask verify-ui <verb> …`, or `gate --verb \
             <line>`) — a passing run records what it showed in {}, and from then on the gate \
             picks that verb by itself.",
            plan.uncovered
                .iter()
                .map(|f| format!("  {f}"))
                .collect::<Vec<_>>()
                .join("\n"),
            census::FILE
        ));
    }
    Ok(())
}

/// What one side's steps stand on: the side's name, the tree, the
/// stamps, the logs, and the runner copy its xtask steps start from
/// (`None` under the tests' faked steps).
#[derive(Clone, Copy)]
struct Ground<'a> {
    name: &'a str,
    dir: &'a Path,
    store: &'a Store,
    logs: &'a Path,
    runner: Option<&'a Path>,
    /// Where this side's units tally what they waited for room another
    /// gate was holding — said in a line here and kept in the run's
    /// record, beside the longest units, which are what a landing's
    /// wait is made of.
    waited: &'a Waited,
    /// The machine's budget, which both sides and every seat draw on.
    pool: &'a crate::budget::Pool,
    /// The tree, which is what the fairness between equals is over.
    seat: &'a str,
    rank: crate::budget::Rank,
    /// `--fresh`: every step runs whether or not a stamp answers, so the
    /// one another tree wrote while this unit queued is not taken either.
    fresh: bool,
}

/// Which tree a unit belongs to, as the queue names it: the seat's
/// letter, or the checkout's own name for the primary.
pub(crate) fn seat_of(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.display().to_string())
}

fn rank(landing: bool) -> crate::budget::Rank {
    if landing {
        crate::budget::Rank::Landing
    } else {
        crate::budget::Rank::Normal
    }
}

/// One side's steps, as two groups that share no build directory and so
/// run beside each other: the checks — clippy, the tests, shipped, deny,
/// whatever else the plan owes — one at a time in the plan's order,
/// stopping at the first red (a build that failed makes every later
/// step of the group noise); and the built app — the verify-ui verbs as
/// one block through [`verbs`], then `bare` — which alone read the
/// release the first verb builds. Beside each other they slow each
/// other — a verb costs half again as much at the median, and a cold
/// checks chain can be the side's wall clock — and the side still ends
/// a fifth sooner than the two would as a sum (the numbers are in
/// internal-docs/反映前テストの機械化.md §実測).
///
/// The always-steps go first and alone. They are seconds, and a red
/// among them is what a person fixes before anything else — a verb block
/// started beside them would run its minutes to greens that fix takes
/// away, the app being every verb's input.
fn side(ground: &Ground<'_>, steps: &[&Required], jobs: usize) -> Vec<String> {
    let mut at = 0;
    while at < steps.len() && steps[at].step.always {
        if let Err(why) = run_one(ground, at, steps[at], false) {
            return vec![why];
        }
        at += 1;
    }
    let (built, checks): (Vec<_>, Vec<_>) = (at..steps.len())
        .map(|i| (i, steps[i]))
        .partition(|(_, r): &(usize, &Required)| r.step.release);
    std::thread::scope(|scope| {
        let checks = scope.spawn(|| in_order(ground, &checks));
        let built = scope.spawn(|| against_the_build(ground, &built, jobs));
        let mut failures = Vec::new();
        for handle in [checks, built] {
            match handle.join() {
                Ok(mut group) => failures.append(&mut group),
                Err(_) => failures.push(format!("a group of the {} side panicked", ground.name)),
            }
        }
        failures
    })
}

/// The checks group: one at a time, stopping at the first red.
fn in_order(ground: &Ground<'_>, steps: &[(usize, &Required)]) -> Vec<String> {
    for (index, required) in steps {
        if let Err(why) = run_one(ground, *index, required, false) {
            return vec![why];
        }
    }
    Vec::new()
}

/// The built-app group: each run of verbs as one block ([`verbs`]) and
/// the steps between them as they come, stopping after a block with a
/// red in it or at a red step — `bare` after a red verb would be reading
/// a build the verbs have already answered for.
fn against_the_build(
    ground: &Ground<'_>,
    steps: &[(usize, &Required)],
    jobs: usize,
) -> Vec<String> {
    let mut at = 0;
    while at < steps.len() {
        if steps[at].1.step.builds_app {
            let end = steps[at..]
                .iter()
                .position(|(_, r)| !r.step.builds_app)
                .map_or(steps.len(), |n| at + n);
            let failures = verbs(ground, &steps[at..end], jobs);
            if !failures.is_empty() {
                return failures;
            }
            at = end;
            continue;
        }
        let (index, required) = steps[at];
        if let Err(why) = run_one(ground, index, required, false) {
            return vec![why];
        }
        at += 1;
    }
    Vec::new()
}

/// One side's verbs: the first uncached one runs alone and builds the
/// release, the rest reuse that build `jobs` at a time. A red verb stops
/// none of the others — a verb breaks nothing the next one reads, and
/// every green is stamped, so the run after the fix owes the reds alone.
/// Only a verb of this side that built in this very invocation earns the
/// others their `--no-build`: a cached verb's build happened in whatever
/// tree took the stamp, and the binary here may be older than the tree.
///
/// Every verb takes a ticket out of the machine's budget ([`run_one`]),
/// the first one included: what bounds the load is what is running, and
/// the building verb is one of them — the heavier one, since it is the
/// build.
///
/// A building verb that went red because the app did not build ends the
/// block: the next one alone would build the same sources to the same
/// error, and a block of a hundred verbs would spend its minutes saying
/// so a hundred times. The checks group's clippy fails on the same
/// source beside this block rather than ahead of it, so the block has to
/// stop itself.
fn verbs(ground: &Ground<'_>, block: &[(usize, &Required)], jobs: usize) -> Vec<String> {
    let name = ground.name;
    for (_, required) in block.iter().filter(|(_, r)| r.cached) {
        println!("[{name}] cached {}", required.step.id);
    }
    // What the side had waited before this block, so that the line below
    // says this block's own wait rather than the side's running total.
    let before = ground.waited.read();
    let mut queue = block.iter().filter(|(_, r)| !r.cached);
    let mut failures = Vec::new();
    // Alone until one is green: a red first verb may have left no build
    // for the others to reuse.
    let mut built = false;
    while !built {
        let Some((index, required)) = queue.next() else {
            return failures;
        };
        match run_one(ground, *index, required, false) {
            // Only a verb that actually ran here has left a release for
            // the others to reuse. One answered by a stamp another tree
            // wrote built nothing in this tree, and reading it as a
            // build is how the rest of the block would be handed
            // `--no-build` against a binary older than the sources.
            Ok(Ran::Step) => built = true,
            Ok(Ran::Stamped) => continue,
            Err(why) => {
                failures.push(why);
                if app_did_not_build(&log_of(ground, *index)) {
                    let left = queue.count();
                    println!("[{name}] the app did not build — {left} verb(s) not run");
                    failures.push(format!("{left} verb(s) not run: the app did not build"));
                    return failures;
                }
            }
        }
    }
    let rest: Vec<&(usize, &Required)> = queue.collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failed = std::sync::Mutex::new(failures);
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1).min(rest.len()) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some((index, required)) = rest.get(i) else {
                        break;
                    };
                    if let Err(why) = run_one(ground, *index, required, true) {
                        failed
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .push(why);
                    }
                }
            });
        }
    });
    let after = ground.waited.read();
    let (verbs_waited, in_all) = (after.0 - before.0, after.1 - before.1);
    if verbs_waited > 0 {
        println!(
            "[{name}] budget: {verbs_waited} verb(s) waited for room held elsewhere ({} in all)",
            crate::seats::format_age(Some(in_all))
        );
    }
    failed
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Where a side's step at `index` writes what it says.
fn log_of(ground: &Ground<'_>, index: usize) -> std::path::PathBuf {
    ground.logs.join(format!("{}-{index:02}.log", ground.name))
}

/// What became of one step: it ran here, or a stamp answered for it.
///
/// The two are not the same to the caller. A verb that ran here built
/// the release the rest of its block reuses; a verb a stamp answered for
/// built nothing here, whatever it built in the tree that took the stamp
/// ([`verbs`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ran {
    Step,
    Stamped,
}

/// One step against its log: the machine's room taken for it, then run,
/// timed, said as ok or FAIL, and stamped when green (never an
/// always-step, whose seconds are not worth one). `no_build` is a verb's
/// `--no-build`, the block's to hand out ([`verbs`]). A failure comes
/// back as the line to report.
///
/// **The step is the unit** the budget hands the machine out in
/// (`budget`), which is the granularity the runner already had. What
/// that costs is that a landing waits out whichever unit is running, so
/// the longest of them are kept and said ([`Waited::ran`]) — a unit that
/// is minutes long is a landing's wait, and the one worth splitting.
///
/// The ticket is taken before the step announces itself to a measurement
/// (`still::busy`, inside `check::run_step`) and let go after the step
/// has ended, so nothing holds room it is not using and nothing holds
/// part of what it needs while waiting for the rest.
fn run_one(
    ground: &Ground<'_>,
    index: usize,
    required: &Required,
    no_build: bool,
) -> Result<Ran, String> {
    let name = ground.name;
    let id = &required.step.id;
    if required.cached {
        println!("[{name}] cached {id}");
        return Ok(Ran::Stamped);
    }
    let weight = crate::budget::weight_of(&required.step.command, no_build);
    let room = ground
        .pool
        .admit_once_the_machine_is_free(&crate::budget::Ask {
            weight,
            rank: ground.rank,
            seat: ground.seat,
            what: id,
        })
        .map_err(|why| format!("{id}: {why}"))?;
    ground.waited.add(room.waited);
    // Looked at again now rather than only when the plan was made: a
    // unit that stood in the queue may have been answered while it stood
    // — another tree gating the same commit writes the same key, and the
    // stamps are the repository's rather than the tree's (`stamp`). It
    // takes duplicated work off a machine full of seats; what it cannot
    // do is stop two that miss at the same instant, which both then run.
    // Never under `--fresh`, which is the ask to run the step whatever
    // any stamp says.
    // The tests' switch for the window itself: the instant between the
    // plan and this look is another tree's to write in, and nothing a
    // test drives from outside can land in it. Named by step id, it
    // stamps this very key here — which is what the tree that took the
    // stamp would have left behind (`FAKE_LOG`).
    if std::env::var(FAKE_STAMP).is_ok_and(|named| named == *id) {
        ground
            .store
            .mark_step(&required.key, &format!("{id}\nstamped elsewhere\n"))
            .map_err(|why| format!("{id}: {why}"))?;
    }
    if !ground.fresh && !required.key.is_empty() && ground.store.step_green(&required.key) {
        println!("[{name}] cached {id} (stamped elsewhere while this waited)");
        return Ok(Ran::Stamped);
    }
    let log = log_of(ground, index);
    println!("[{name}] run    {id} … (log: {})", log.display());
    // waits(measured): the step's wall clock, said on its line and judged by nothing
    let at = std::time::Instant::now();
    let mut command = required.step.command.clone();
    if no_build {
        command.push("--no-build".to_string());
    }
    let outcome = execute_step(ground.dir, id, &command, &log, ground.runner, &room);
    let ran = at.elapsed();
    let secs = ran.as_secs();
    // How long this unit held the machine, kept because it is how long a
    // landing arriving behind it would have waited: nothing is killed to
    // make room (`budget`), so the longest unit is the interruption's
    // own ceiling.
    ground.waited.ran(id, weight, ran);
    match outcome {
        Ok(()) => {
            println!("[{name}] ok     {id} ({secs}s)");
            crate::check::print_shots(name, &log);
            if !required.step.always {
                ground
                    .store
                    .mark_step(
                        &required.key,
                        &format!("{id}\n{}\n", required.step.command.join(" ")),
                    )
                    .map_err(|why| format!("{id}: green but not stamped: {why}"))?;
            }
            Ok(Ran::Step)
        }
        Err(why) => {
            println!("[{name}] FAIL   {id} ({secs}s): {why}");
            Err(id.clone())
        }
    }
}

/// The tests' switch: with it set no step runs at all (`execute_step`).
const FAKE_LOG: &str = "PGG_GATE_FAKE_LOG";

/// The tests' switch for a step another tree stamped while this one
/// waited for room ([`run_one`]).
const FAKE_STAMP: &str = "PGG_GATE_FAKE_STAMP";

/// Where a faked run records the steps it was handed `--no-build`, so a
/// test can see which of a block's verbs were told to reuse a release
/// and which were left to build one ([`verbs`]). Beside the fake log,
/// whose own lines are the ids and nothing else — every test reads that
/// one as a set of ids, and widening it would rewrite all of them.
fn no_build_log(fake: &str) -> String {
    format!("{fake}.no-build")
}

/// One step, through `check`'s watched runner, its xtask launcher
/// swapped for the runner copy ([`launched`]). With `PGG_GATE_FAKE_LOG`
/// set the step is not run at all: its id is appended to that file and
/// it passes, unless `PGG_GATE_FAKE_FAIL` names it — which is how the
/// tests watch selection and caching without a toolchain in the
/// throwaway repository. `PGG_GATE_FAKE_REWRITE` names a step that
/// rewrites the census the way a passing verb does: one line put in,
/// once, so the run after the commit of it finds nothing to move.
fn execute_step(
    dir: &Path,
    id: &str,
    command: &[String],
    log: &Path,
    runner: Option<&Path>,
    room: &crate::budget::Admitted,
) -> Result<(), String> {
    if let Ok(fake) = std::env::var(FAKE_LOG) {
        use std::io::Write;
        // Both sides append from their own thread: one write per line,
        // under one lock, or the ids interleave mid-word.
        static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _turn = ONE_AT_A_TIME.lock().map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&fake)
            .map_err(|e| format!("{fake}: {e}"))?;
        file.write_all(format!("{id}\n").as_bytes())
            .map_err(|e| e.to_string())?;
        if command.iter().any(|word| word == "--no-build") {
            let mut told = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(no_build_log(&fake))
                .map_err(|e| format!("{fake}: {e}"))?;
            told.write_all(format!("{id}\n").as_bytes())
                .map_err(|e| e.to_string())?;
        }
        let failing = std::env::var("PGG_GATE_FAKE_FAIL").unwrap_or_default();
        if failing.split(',').any(|f| f == id) {
            return Err("failed on purpose (PGG_GATE_FAKE_FAIL)".into());
        }
        if let Some(line) = id.strip_prefix("verify ")
            && std::env::var("PGG_GATE_FAKE_REWRITE").is_ok_and(|step| step == id)
        {
            // The verb's own line with one more name on it — the shape
            // of a run that met a component it had not before.
            let path = dir.join(census::FILE);
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let rewritten: String = text
                .lines()
                .map(|held| {
                    if held.starts_with(&format!("{line}\t")) && !held.ends_with(" Theme") {
                        format!("{held} Theme\n")
                    } else {
                        format!("{held}\n")
                    }
                })
                .collect();
            if rewritten != text {
                std::fs::write(&path, rewritten).map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
        return Ok(());
    }
    match crate::check::run_step(dir, &launched(command, runner), log, room) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "exited non-zero (log: {})\n{}",
            log.display(),
            crate::check::log_tail(log)
        )),
        Err(why) => Err(why),
    }
}

/// What a runner copy is called, beside the logs; the pid follows.
const RUNNER: &str = "xtask-runner-";

/// The task runner the steps start from: this program, built from the
/// tree once and copied beside the logs.
///
/// A step that is one of xtask's own verbs is spelled `cargo run -p
/// xtask -- …` in the plan, and cargo holds `target/debug` for the
/// length of any build there — so beside the checks ([`side`]) every
/// verb's launcher would wait out whatever `cargo test` was compiling,
/// and even with nothing beside them but each other a fifth of a gate's
/// host verb logs showed the wait on the build directory (none from the
/// copy). Built with cargo all the same, so that it is the
/// tree's code — under `land` the rebase has just brought sources in —
/// and copied rather than run in place, because the landing has renamed
/// the slot away from under this very process and the slot is what cargo
/// rebuilds. What earlier gates left is taken away first; a copy a
/// process of theirs still holds stays, its name carrying their pid.
fn runner(
    dir: &Path,
    logs: &Path,
    jobs: usize,
    landing: bool,
) -> Result<std::path::PathBuf, String> {
    let exe = format!("xtask{}", std::env::consts::EXE_SUFFIX);
    // A compile like any other, and out of the same budget: this is the
    // one the gate runs before its sides, so a machine full of seats
    // would otherwise start every gate with an uncounted cargo.
    let pool = crate::budget::Pool::of(dir, jobs.max(default_jobs()))?;
    let room = pool.admit_once_the_machine_is_free(&crate::budget::Ask {
        weight: crate::budget::COMPILE,
        rank: rank(landing),
        seat: &seat_of(dir),
        what: "the task runner's build",
    })?;
    // Through the same road every step takes (`check::run_step`): the
    // announcement to a measurement, the ceiling, and the tree kill at it.
    // A build with no ceiling would sit on a rustc holding this tree's
    // build lock with the gate's own lock held, and the next gate here
    // would be refused by a live pid saying nothing.
    let build_log = logs.join(format!("{RUNNER}build-{}.log", std::process::id()));
    let build = ["cargo", "build", "--locked", "-p", "xtask"].map(String::from);
    match crate::check::run_step(dir, &build, &build_log, &room) {
        Ok(true) => {}
        Ok(false) => {
            return Err(format!(
                "the task runner did not build in {}:\n{}",
                dir.display(),
                crate::check::log_tail(&build_log)
            ));
        }
        // A ceiling, a cargo already announced on this tree, or a spawn
        // that failed: the step's own words say which.
        Err(why) => {
            return Err(format!(
                "the task runner's build in {} did not run to its end: {why}",
                dir.display()
            ));
        }
    }
    if let Ok(entries) = std::fs::read_dir(logs) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(RUNNER) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let built = dir.join("target").join("debug").join(&exe);
    let copy = logs.join(format!(
        "{RUNNER}{}{}",
        std::process::id(),
        std::env::consts::EXE_SUFFIX
    ));
    std::fs::copy(&built, &copy).map_err(|e| {
        format!(
            "could not copy {} to {}: {e}",
            built.display(),
            copy.display()
        )
    })?;
    Ok(copy)
}

/// The command as it is started: a step that is one of this program's
/// own verbs — `cargo run --locked -p xtask -- <verb>…`, which the plan
/// keeps spelling so that a stamp's key names one line on every machine
/// — starts from the runner copy, and any other step as spelled.
fn launched(command: &[String], runner: Option<&Path>) -> Vec<String> {
    const THROUGH_CARGO: [&str; 6] = ["cargo", "run", "--locked", "-p", "xtask", "--"];
    let through_cargo = command.len() >= THROUGH_CARGO.len()
        && command
            .iter()
            .zip(THROUGH_CARGO)
            .all(|(word, spelled)| word == spelled);
    match runner {
        Some(runner) if through_cargo => std::iter::once(runner.display().to_string())
            .chain(command[THROUGH_CARGO.len()..].iter().cloned())
            .collect(),
        _ => command.to_vec(),
    }
}

/// Whether a red verb's log says the app itself did not build — cargo's
/// own line, or the runner's when it reports the build (`tree::app_exe`).
fn app_did_not_build(log: &Path) -> bool {
    let text = String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).into_owned();
    text.contains("could not compile") || text.contains("cargo build --release failed")
}

/// Stop hook: where the seat's gate stands, for the user's eyes. A seat
/// ahead of main whose tip carries no full stamp is work reported before
/// it was gated — said as a system message, not a block, so a turn that
/// ends mid-work is not held to a gate it was never claiming.
pub(crate) fn standing(cwd: &str) -> Option<String> {
    let root = crate::seats::worktree_root(cwd)?;
    let head = crate::subprocess::git_query(&root, &["rev-parse", "HEAD"])?;
    let ahead = crate::seats::commits_in(&root, "main..HEAD")?;
    if ahead == 0 {
        return None;
    }
    let store = Store::open(Path::new(&root)).ok()?;
    let seat = root.rsplit('/').next().unwrap_or_default();
    Some(match store.commit(&head) {
        None => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main and its tip is not gated — \
             `cargo xtask gate` before 「マージ可」."
        ),
        Some(found) if !found.full => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main; its tip is gated on the host \
             side only — the full `cargo xtask gate` still owes the container side."
        ),
        Some(found) if !found.onto_main => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main; its tip was gated off main — \
             `land` will rebase and gate again."
        ),
        Some(_) => {
            format!("gate: seat {seat} is {ahead} commit(s) ahead of main, tip gated in full.")
        }
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{app_did_not_build, launched};

    fn words(line: &[&str]) -> Vec<String> {
        line.iter().map(|w| (*w).to_string()).collect()
    }

    #[test]
    fn an_xtask_step_starts_from_the_runner_and_a_cargo_step_as_spelled() {
        let runner = Path::new("C:/x/target/gate-logs/xtask-runner-7.exe");
        assert_eq!(
            launched(
                &words(&[
                    "cargo",
                    "run",
                    "--locked",
                    "-p",
                    "xtask",
                    "--",
                    "verify-ui",
                    "wip",
                    "--no-build"
                ]),
                Some(runner)
            ),
            words(&[
                "C:/x/target/gate-logs/xtask-runner-7.exe",
                "verify-ui",
                "wip",
                "--no-build"
            ])
        );
        let test = words(&["cargo", "test", "--locked", "-p", "xtask", "--lib"]);
        assert_eq!(launched(&test, Some(runner)), test);
        let verb = words(&["cargo", "run", "--locked", "-p", "xtask", "--", "structure"]);
        assert_eq!(
            launched(&verb, None),
            verb,
            "without a copy the plan's spelling stands"
        );
        // An older spelling, without the lock, is not this runner's line
        // any more: it is started as spelled, and cargo answers for it.
        let unlocked = words(&["cargo", "run", "-p", "xtask", "--", "structure"]);
        assert_eq!(launched(&unlocked, Some(runner)), unlocked);
    }

    #[test]
    fn a_build_that_failed_is_read_off_the_verbs_log() {
        let dir = std::env::temp_dir().join(format!("pgg-gate-build-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let log = dir.join("host-08.log");
        std::fs::write(
            &log,
            "building (release, automation)…\nerror[E0425]: cannot find value\n\
             error: could not compile `platitude-app` (bin \"platitude-gg\") due to 1 previous error\n",
        )
        .expect("a log");
        assert!(app_did_not_build(&log));
        std::fs::write(
            &log,
            "building (release, automation)…\nFAIL: wip in 2.1s (exit 1)\n",
        )
        .expect("a log");
        assert!(
            !app_did_not_build(&log),
            "a verb red on its own account is not a build that failed"
        );
        assert!(
            !app_did_not_build(&dir.join("host-99.log")),
            "no log, no build to have failed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
