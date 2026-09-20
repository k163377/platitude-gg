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
mod evidence;
mod graph;
mod hooks;
mod inputs;
mod plan;
mod record;
mod reuse;
mod runner;
mod stamp;

use std::path::Path;
use std::time::Duration;

use plan::{Plan, Required, Side};
use record::{Spent, Waited};
pub(crate) use reuse::preserve_reader;
use runner::{FAKE_LOG, FAKE_STAMP, app_did_not_build, execute_step, linux_runner, runner};
use stamp::{CommitStamp, Store};

use census::Shift;

use crate::command::{self, Permission, Where};

/// 段 2: what a branch owes main, chosen by machine. The tier `land`
/// runs for itself, and the one a session runs before reporting.
pub(crate) static PREMERGE: command::Command = command::Command {
    id: "gate.premerge",
    call: "gate",
    purpose: "the pre-merge tests the branch's diff owes, both OSes",
    run_in: Where::Seat,
    needs: &["a committed branch — the reach is read off the diff against main"],
    permission: Permission::Plain,
};

/// 段 1: the same selection with no container behind it.
pub(crate) static DAILY: command::Command = command::Command {
    id: "gate.daily",
    call: "gate --host-only",
    purpose: "the daily tier: the host half, no container",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

/// 段 3: every file counted as changed, and the stamps ignored. Without
/// `--fresh` a stamped tree runs almost nothing.
pub(crate) static FULL: command::Command = command::Command {
    id: "gate.full",
    call: "gate --all --fresh",
    purpose: "stage 3: every step owed, stamps ignored",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static VERDICT: command::Command = command::Command {
    id: "gate.verdict",
    call: "gate verdict <old> <new>",
    purpose: "the reference-transaction hook's question: may main move here",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static INSTALL: command::Command = command::Command {
    id: "gate.install",
    call: "gate install",
    purpose: "point core.hooksPath at the gate's reference-transaction hook",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static DEPS: command::Command = command::Command {
    id: "gate.deps",
    call: "gate deps",
    purpose: "the reach graph itself: what a change reaches, and how",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] =
    &[&PREMERGE, &DAILY, &FULL, &VERDICT, &INSTALL, &DEPS];
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
                    .ok_or_else(|| format!("--jobs takes a count of 1 or more; got {value:?}"))?;
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
/// is committed there and gated again.
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
        " The verbs rewrote {}: it is generated, so review the diff and commit it as it \
         stands, and the gate can stamp the commit that holds it.",
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
    // This run's own name, for what a red step leaves behind
    // (`evidence`). The second it started and the process that ran it,
    // as the record beside it is named (`record::keep`) — one pair of
    // eyes reading a red gate has both.
    let run = format!(
        "{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
        std::process::id()
    );
    let host_ground = Ground {
        name: "host",
        dir: &plan.dir,
        store,
        logs,
        runner,
        copy: None,
        container: None,
        run: &run,
        waited: &host_waited,
        since: started,
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
    let mut failures: Vec<String> = std::thread::scope(|scope| {
        let host = scope.spawn(|| side(&host_ground, &host, jobs));
        // On this side and not ahead of both, so the host side starts
        // now: what the preparation owes is only that it is in before
        // the first step of *this* side (`runner::linux_runner`).
        let linux = scope.spawn(|| {
            // Named before the preparation, so that it is taken down
            // after the side however the preparation went
            // (`runner::container_expected`).
            let container = runner::container_expected(&linux_ground, &linux);
            let failures = match linux_runner(&linux_ground, &linux) {
                // The container is there for the verbs exactly when the
                // preparation that starts it came back with the copy.
                Ok(copy) => side(
                    &Ground {
                        copy: copy.as_deref(),
                        container: copy.as_ref().and(container.as_deref()),
                        ..linux_ground
                    },
                    &linux,
                    jobs,
                ),
                Err(why) => vec![why],
            };
            if let Some(name) = &container {
                runner::dismiss_container(&linux_ground, name);
            }
            failures
        });
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
    spent.ledger = record::ledger([("host", &host_waited), ("linux", &linux_waited)]);
    // The ledger against the plan, before either is read as a number.
    // Every step has a row whichever way the run went — cached, run,
    // failed, or not reached — so a step with none is a path through the
    // runner that files nothing, and the arithmetic somebody does over
    // this table would be short by exactly the steps nobody can see are
    // missing. A red for the gate, because it is the gate's own books.
    for (name, waited, steps) in [
        ("host", &host_waited, &host),
        ("linux", &linux_waited, &linux),
    ] {
        let planned: Vec<String> = steps.iter().map(|r| r.step.id.clone()).collect();
        for note in waited.unaccounted(&planned) {
            failures.push(format!("the {name} side's ledger {note}"));
        }
    }
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
    /// The Linux side's own copy of the task runner, by the name
    /// `runner::linux_runner` prepared it under — `None` on the host
    /// side, and on a Linux side whose every step a stamp answers for.
    copy: Option<&'a str>,
    /// The gate's own container the copy's verbs go into, by name —
    /// `None` wherever `copy` is, and on a Linux host, where the verbs
    /// run where they stand (`linux::container::exec_in`).
    container: Option<&'a str>,
    /// What this run is called where its red steps' logs are kept
    /// (`evidence::keep`): a step's log is named by its index, so the
    /// next gate in this tree writes over it.
    run: &'a str,
    /// Where this side's units tally what they waited for room another
    /// gate was holding — said in a line here and kept in the run's
    /// record, beside the longest units, which are what a landing's
    /// wait is made of.
    waited: &'a Waited,
    /// When the sides started, which every unit's row is an offset from
    /// — two rows of the ledger say whether they overlapped, and a side
    /// says where in its own run the time went.
    since: std::time::Instant,
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
            not_run(ground, steps[at + 1..].iter().map(|r| r.step.id.clone()));
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
    for (n, (index, required)) in steps.iter().enumerate() {
        if let Err(why) = run_one(ground, *index, required, false) {
            not_run(
                ground,
                steps[n + 1..].iter().map(|(_, r)| r.step.id.clone()),
            );
            return vec![why];
        }
    }
    Vec::new()
}

/// The steps a side stopped short of, filed as what they are.
///
/// **A step the run did not reach and a step the ledger lost are two
/// different things**, and only one of them is a fault of the runner:
/// a red stops the group it is in, and the plan's other steps are then
/// answered for by a row saying so. Anything the ledger still has no row
/// for after that is a path through here that files nothing, which is
/// what `record::Waited::unaccounted` refuses.
fn not_run(ground: &Ground<'_>, ids: impl IntoIterator<Item = String>) {
    for id in ids {
        ground.waited.filed(record::Row {
            id,
            weight: 0,
            outcome: "not-run",
            waited: Duration::ZERO,
            from_start: ground.since.elapsed(),
            ran: Duration::ZERO,
            spent: String::new(),
        });
    }
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
                not_run(ground, steps[end..].iter().map(|(_, r)| r.step.id.clone()));
                return failures;
            }
            at = end;
            continue;
        }
        let (index, required) = steps[at];
        if let Err(why) = run_one(ground, index, required, false) {
            not_run(
                ground,
                steps[at + 1..].iter().map(|(_, r)| r.step.id.clone()),
            );
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
/// source beside this block, so the block has to
/// stop itself.
fn verbs(ground: &Ground<'_>, block: &[(usize, &Required)], jobs: usize) -> Vec<String> {
    let name = ground.name;
    // **Through the same door as every other unit**, cached or not: the
    // line a stamped verb prints and the row it files are one path
    // ([`run_one`]), and a block that said `cached` here and filed
    // nothing left the ledger with no answer for a step the plan had —
    // which is a number read off a file with holes in it
    // (`record::Waited::unaccounted` is what refuses that now).
    for (index, required) in block.iter().filter(|(_, r)| r.cached) {
        let _ = run_one(ground, *index, required, false);
    }
    // What the side had waited before this block, so that the line below
    // says this block's own wait.
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
                    let left: Vec<String> = queue.map(|(_, r)| r.step.id.clone()).collect();
                    println!(
                        "[{name}] the app did not build — {} verb(s) not run",
                        left.len()
                    );
                    failures.push(format!(
                        "{} verb(s) not run: the app did not build",
                        left.len()
                    ));
                    not_run(ground, left);
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
/// timed, said as ok or FAIL, and stamped when green (an always-step's
/// seconds are not worth one). `no_build` is a verb's
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
        return Ok(stamp_answered(ground, id, "cached", 0, Duration::ZERO));
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
    // Looked at again now, after the plan was made: a unit that stood
    // in the queue may have been answered while it stood — another tree
    // gating the same commit writes the same key, and the stamps are the
    // repository's, whichever tree wrote them (`stamp`). It takes
    // duplicated work off a machine full of seats; what it cannot do is
    // stop two that miss at the same instant, which both then run.
    // Skipped under `--fresh`, which is the ask to run the step whatever
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
        return Ok(stamp_answered(
            ground,
            id,
            "cached-late",
            weight,
            room.waited,
        ));
    }
    let log = log_of(ground, index);
    println!("[{name}] run    {id} … (log: {})", log.display());
    let from_start = ground.since.elapsed();
    // waits(measured): the step's wall clock, said on its line and judged by nothing
    let at = std::time::Instant::now();
    let mut command = required.step.command.clone();
    if no_build {
        command.push("--no-build".to_string());
    }
    let outcome = execute_step(ground, id, &command, &log, &room);
    let ran = at.elapsed();
    let secs = ran.as_secs();
    // How long this unit held the machine, kept because it is how long a
    // landing arriving behind it would have waited: nothing is killed to
    // make room (`budget`), so the longest unit is the interruption's
    // own ceiling.
    ground.waited.ran(id, weight, ran);
    ground.waited.filed(record::Row {
        id: id.clone(),
        weight,
        outcome: if outcome.is_ok() { "ran" } else { "FAIL" },
        waited: room.waited,
        from_start,
        ran,
        // Off the step's own log, where the step wrote one: a verb says
        // what its fixture, its build and its window took, and the wall
        // clock above says nothing about which of the three it was
        // (`verify::run::say_what_it_spent`).
        spent: spent_in(&log),
    });
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
            // The log is kept before anything else looks at this tree:
            // the re-run that follows a red gate writes over it in place
            // (`evidence`), and a keeping that failed is said here and
            // is never a second failure of the step.
            match evidence::keep(
                ground.logs,
                ground.run,
                ground.dir,
                &log,
                &required.step.command,
            ) {
                Ok(None) => Err(id.clone()),
                Ok(Some(note)) => {
                    println!("[{name}] {note}");
                    Err(format!("{id} — {note}"))
                }
                Err(why) => {
                    println!("[{name}] the red step's log was not kept: {why}");
                    Err(id.clone())
                }
            }
        }
    }
}

/// A unit a stamp answered for, filed in the ledger under the word that
/// says which stamp it was: one found when the plan was made, or one
/// another tree wrote while this unit stood in the queue. It ran
/// nothing, and the room it waited for is the only machine it took.
fn stamp_answered(
    ground: &Ground<'_>,
    id: &str,
    outcome: &'static str,
    weight: u32,
    waited: Duration,
) -> Ran {
    ground.waited.filed(record::Row {
        id: id.to_string(),
        weight,
        outcome,
        waited,
        from_start: ground.since.elapsed(),
        ran: Duration::ZERO,
        spent: String::new(),
    });
    Ran::Stamped
}

/// What a step said it spent, off its own log, or empty where it said
/// nothing.
///
/// **The step is the one that can split its own time.** From out here a
/// unit is one wall clock, and a verb's is a release build plus a
/// fixture plus a window in whatever proportion the block's ordering
/// gave it. The line is the step's words verbatim, so nothing is
/// invented on the way into the ledger.
fn spent_in(log: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(log) else {
        return String::new();
    };
    text.lines()
        .find(|line| line.starts_with("spent "))
        .map(|line| line.trim_start_matches("spent ").to_string())
        .unwrap_or_default()
}

/// Stop hook: where the seat's gate stands, for the user's eyes. A seat
/// ahead of main whose tip carries no full stamp is work reported before
/// it was gated — said as a system message, so a turn that ends
/// mid-work is not held to a gate it was never claiming.
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
