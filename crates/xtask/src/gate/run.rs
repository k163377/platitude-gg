//! What a gate is asked and what it says: the command line, `land`'s
//! own door onto the same run, and the block both of them keep.

use std::path::Path;

use super::Gated;
use super::census::{self, Shift};
use super::deps;
use super::execute::execute;
use super::hooks::{self, install};
use super::plan::{self, Plan, Required};
use super::record::{self, Spent};

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
    /// Run the rest after a red ([`super::halt`]) — `--keep-going`, or `--all`.
    keep_going: bool,
    verbs: Vec<String>,
    /// How many verify-ui verbs a side runs at once ([`super::sides::verbs`]).
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
        keep_going: false,
        verbs: Vec::new(),
        jobs: crate::budget::default_jobs(),
    };
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--host-only" => opts.host_only = true,
            // Stage 3 is asked for the whole picture, not the first red.
            "--all" => {
                opts.all = true;
                opts.keep_going = true;
            }
            "--keep-going" => opts.keep_going = true,
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
                     --keep-going, --dry-run, --dir <tree>, --main <ref>, --verb <line>…, \
                     --jobs <n>)"
                ));
            }
        }
        at += 1;
    }
    Ok(opts)
}

fn gate(args: &[String]) -> Result<(), String> {
    let opts = options(args)?;
    let mut spent = Spent::default();
    // waits(measured): the run's whole, for the record (`Spent::total`)
    let whole_run = std::time::Instant::now();
    // The tree's one gate, taken before the plan's seconds of reading. A
    // dry run runs nothing and holds nothing.
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
    print!("{}", plan::describe(&plan, opts.dry_run));
    let mut shift = Shift::default();
    if opts.dry_run {
        // The plan's own cost, said but not kept: a record would name a
        // run that did not happen.
        spent.total = whole_run.elapsed();
        let head = short(&plan.head);
        let run = stood(&plan, what.trim_end(), &head, opts.jobs, false, "dry run");
        print!("{}", record::render(&run, &spent, &shift));
        return Ok(());
    }
    let outcome = execute(
        &plan,
        opts.jobs,
        false,
        opts.keep_going,
        &mut spent,
        &mut shift,
    );
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
        Gated::Stamped => {
            // Green, and nothing else is running in this tree: where the
            // sweep belongs (反映前テストの機械化.md §世代の掃除).
            crate::sweep::at_a_tail(
                &plan.dir,
                &crate::sweep::Tail {
                    whatever_the_key_says: opts.all,
                    the_volume_too: !opts.host_only,
                },
            );
            Ok(())
        }
        // A failure as far as the stamp goes: it must name the commit
        // that holds the census. Said aloud, since a dirty tree stops the
        // next gate.
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
    print!("{}", plan::describe(&plan, false));
    let jobs = crate::budget::default_jobs();
    let mut shift = Shift::default();
    // A landing's red leaves main where it was whatever else would pass,
    // so the first one is the answer.
    let outcome = execute(&plan, jobs, true, false, &mut spent, &mut shift);
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
            plan.verbs_left,
        ),
        outcome,
    }
}

/// The run's block, said and kept ([`record`]), for a red run as much as
/// a green one.
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

pub(super) fn short(sha: &str) -> String {
    sha.chars().take(10).collect()
}

/// The line that says what a verb's rewrite means for whoever reads it.
pub(super) fn census_rewritten() -> String {
    format!(
        " The verbs rewrote {}: it is generated, so review the diff and commit it as it \
         stands, and the gate can stamp the commit that holds it.",
        census::FILE
    )
}

/// Where a tree's running gate leaves its note (`lanes::sole`): under
/// `target/`, so the note is no uncommitted change for the gate to refuse.
fn running_note(dir: &Path) -> std::path::PathBuf {
    dir.join("target").join("gate-running")
}
