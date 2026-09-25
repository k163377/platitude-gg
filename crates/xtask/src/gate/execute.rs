//! A plan run and stamped: what no stamp could answer for, refused
//! before anything runs; the sides; the census read again; the stamp.

use super::Gated;
use super::census::{self, Census, Shift};
use super::halt::Halt;
use super::plan::Plan;
use super::record::Spent;
use super::run::{census_rewritten, short};
use super::runner::{FAKE_LOG, runner};
use super::sides::run_sides;
use super::stamp::{CommitStamp, Store};

/// Runs what is not cached, one thread per side and two groups within a
/// side ([`side`]), and stamps the commit when both sides came back with
/// nothing red. The first red stops the run on both sides ([`halt`]);
/// under `keep_going` a group stops at its first red step — except among
/// its verbs, which run to the end of their block `jobs` at a time
/// ([`verbs`]) — and everything else finishes, so its green steps are
/// stamped and need not run again. A `landing`'s units are handed the
/// machine ahead of any other gate's (`budget`).
pub(super) fn execute(
    plan: &Plan,
    jobs: usize,
    landing: bool,
    keep_going: bool,
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
    let census_before = census::bytes(&plan.dir)?;
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
    let halt = Halt::new(keep_going);
    let failures = run_sides(
        plan,
        &store,
        &logs,
        runner.as_deref(),
        jobs,
        landing,
        &halt,
        spent,
    )?;
    spent.sides = at.elapsed();
    let head = short(&plan.head);
    // A verb that passed rewrote its census line whether or not another
    // step went red, so this is said on both roads out.
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let census_now = census::bytes(&plan.dir)?;
    // The bytes answer whether the tree moved (a header the sources
    // changed moves it too, and the next gate would refuse to run over
    // that); the sets answer what moved, which is what a reader of a
    // whole-file diff cannot get at.
    let rewrote = census_now != census_before;
    *shift = Shift::between(&Census::read(&census_before)?, &Census::read(&census_now)?);
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
    // Asked of the census as committed, before anything runs: a
    // `--verb` of this gate would record its line too late to answer.
    if !plan.uncovered.is_empty() {
        return Err(format!(
            "no verb shows these components, so the gate cannot pass them:\n{}\nRun a verb \
             that brings each one up on its own (`cargo xtask verify-ui <verb> …`) — a passing \
             run records what it showed in {} — then review that diff, commit it, and gate \
             again: from then on the gate picks that verb by itself.",
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
