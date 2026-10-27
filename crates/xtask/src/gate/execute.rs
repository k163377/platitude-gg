//! A plan run and stamped: what no stamp could answer for, refused
//! before anything runs; the sides; the stamp.

use super::halt::Halt;
use super::plan::Plan;
use super::record::Spent;
use super::run::short;
use super::runner::{FAKE_LOG, runner};
use super::sides::run_sides;
use super::stamp::{CommitStamp, Store};

/// Runs what is not cached, one thread per side and two groups within a
/// side (`sides::side`), and stamps the commit when both sides came back
/// with nothing red. The first red stops both sides ([`Halt`]); under
/// `keep_going` a group stops at its first red step — its verbs still run
/// to the end of their block, `jobs` at a time (`sides::verbs`) — and the
/// rest finishes, so its green steps are stamped and need not run again.
/// A `landing`'s units get the machine ahead of other gates' (`budget`).
///
/// Nothing a step runs writes the tree: what passed is the commit, so the
/// stamp names it as it stands.
pub(super) fn execute(
    plan: &Plan,
    jobs: usize,
    landing: bool,
    keep_going: bool,
    spent: &mut Spent,
) -> Result<(), String> {
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    refuse_what_no_stamp_could_answer_for(plan)?;
    // A verb's repositories and pictures are left for a person to look
    // at, and nothing else ever takes them away.
    crate::verify::sweep_yesterdays_runs();
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
    if !failures.is_empty() {
        return Err(format!(
            "gate failed: {} — nothing stamped for {head}.",
            failures.join(" / "),
        ));
    }
    // A host-only run keeps a both-sides stamp already on the commit:
    // writing `container=false` over it would send the next landing back
    // through a container side that has already answered.
    if plan.host_only
        && store.commit(&plan.head).is_some_and(|found| {
            found.container && found.main == plan.main && found.base == plan.base
        })
    {
        println!("gate: PASS — {head} keeps its both-sides stamp (this run was host-only)");
        return Ok(());
    }
    let stamp = CommitStamp {
        main: plan.main.clone(),
        base: plan.base.clone(),
        onto_main: plan.onto_main,
        container: !plan.host_only,
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
        if stamp.container {
            "both sides"
        } else {
            "host-only"
        },
        if stamp.onto_main {
            "on main"
        } else {
            "off main"
        }
    );
    Ok(())
}

/// A tree with uncommitted changes, a graph that cannot read it whole,
/// and a component no registered line shows: nothing a run could answer
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
    // An edge the graph failed to draw is one every selection through it
    // is short by (`graph::complaints`).
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
    // Asked of the table as committed, before anything runs: a run that
    // showed the component would not register it.
    if !plan.uncovered.is_empty() {
        return Err(format!(
            "no registered line brings these components up, so the gate cannot pass them:\n{}\n\
             Name each one on the row of the line whose run stands it up in {} (the popup or \
             the part itself, or the holder that opens it), run that line once to see it met \
             (`cargo xtask verify-ui <line>` says what the run met beside its row), commit, and \
             gate again.",
            plan.uncovered
                .iter()
                .map(|f| format!("  {f}"))
                .collect::<Vec<_>>()
                .join("\n"),
            super::lines::FILE
        ));
    }
    Ok(())
}
