//! Both sides of a run at once, and within a side the checks group
//! and the built-app group with its blocks of verbs.

use std::path::Path;
use std::time::Duration;

use super::Ground;
use super::halt::Halt;
use super::plan::{Plan, Required, Side};
use super::record::{self, Spent, Waited};
use super::run::default_jobs;
use super::runner::{self, app_did_not_build, linux_runner};
use super::stamp::Store;
use super::step::{Ran, log_of, run_one};

/// Both sides at once, each on a thread of its own ([`side`]) and every
/// unit of both out of the machine's one budget: what came back red,
/// once the wall clock has been said.
///
/// **The always-steps go first, ahead of both sides.** They are the
/// host's and seconds long, and a red among them is what a person fixes
/// before anything else ([`halt`]) — a Linux side started beside them
/// would spend its minutes on greens that fix takes away, the app being
/// every verb's input. What that costs the Linux side is their seconds,
/// and it is not the side a gate waits on.
#[expect(
    clippy::too_many_arguments,
    reason = "the run's own pieces, each read by both sides"
)]
pub(super) fn run_sides(
    plan: &Plan,
    store: &Store,
    logs: &Path,
    runner: Option<&Path>,
    jobs: usize,
    landing: bool,
    halt: &Halt,
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
        halt,
    };
    let linux_ground = Ground {
        name: "linux",
        waited: &linux_waited,
        ..host_ground
    };
    // A red among them raises the halt whatever was asked, so the ones
    // after it, and every step of both sides, are filed at their own
    // door ([`run_one`]).
    let always = host.iter().take_while(|r| r.step.always).count();
    let mut failures: Vec<String> = Vec::new();
    for (index, required) in host.iter().enumerate().take(always) {
        if let Err(why) = run_one(&host_ground, index, required, false) {
            failures.push(why);
        }
    }
    failures.extend(std::thread::scope(|scope| {
        let host = scope.spawn(|| side(&host_ground, &host, always, jobs));
        // On this side and not ahead of both, so the host side starts
        // now: what the preparation owes is only that it is in before
        // the first step of *this* side (`runner::linux_runner`).
        let linux = scope.spawn(|| linux_side(&linux_ground, &linux, jobs));
        let mut failures = Vec::new();
        for handle in [host, linux] {
            match handle.join() {
                Ok(mut side_failures) => failures.append(&mut side_failures),
                Err(_) => failures.push("a side panicked".to_string()),
            }
        }
        failures
    }));
    if let Some(said) = halt.said(landing) {
        println!("{said}");
    }
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

/// The Linux side: its container and the task runner copy in it made
/// ready ([`runner::linux_runner`]), then its steps ([`side`]), then the
/// container taken down.
fn linux_side(ground: &Ground<'_>, steps: &[&Required], jobs: usize) -> Vec<String> {
    // A run already stopped has nothing to bring a container up for.
    if ground.halt.raised() {
        return side(ground, steps, 0, jobs);
    }
    // Named before the preparation, so that it is taken down
    // after the side however the preparation went
    // (`runner::container_expected`).
    let container = runner::container_expected(ground, steps);
    let failures = match linux_runner(ground, steps) {
        // The container is there for the verbs exactly when the
        // preparation that starts it came back with the copy.
        Ok(copy) => side(
            &Ground {
                copy: copy.as_deref(),
                container: copy.as_ref().and(container.as_deref()),
                ..*ground
            },
            steps,
            0,
            jobs,
        ),
        // A red of the side's own, ahead of all its steps: none of them
        // was reached.
        Err(why) => {
            ground.halt.red("the container's task runner", false);
            not_run(ground, steps.iter().map(|r| r.step.id.clone()));
            vec![why]
        }
    };
    if let Some(name) = &container {
        runner::dismiss_container(ground, name);
    }
    failures
}

/// Which tree a unit belongs to, as the queue names it: the seat's
/// letter, or the checkout's own name for the primary.
pub(crate) fn seat_of(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.display().to_string())
}

pub(super) fn rank(landing: bool) -> crate::budget::Rank {
    if landing {
        crate::budget::Rank::Landing
    } else {
        crate::budget::Rank::Normal
    }
}

/// One side's steps from `from` on (the host's always-steps before it
/// have run ahead of both sides — [`run_sides`]), as two groups that
/// share no build directory and so run beside each other: the checks —
/// clippy, the tests, shipped, deny, whatever else the plan owes — one at
/// a time in the plan's order, stopping at the first red (a build that
/// failed makes every later step of the group noise); and the built app —
/// the verify-ui verbs as one block through [`verbs`], then `bare` —
/// which alone read the release the first verb builds. Beside each other
/// they slow each other — every verb pays for the compile beside it, and
/// a cold checks chain can be the side's wall clock — and the side still
/// ends sooner than the two would as a sum (the numbers are in
/// internal-docs/反映前テストの機械化.md §実測).
fn side(ground: &Ground<'_>, steps: &[&Required], from: usize, jobs: usize) -> Vec<String> {
    let (built, checks): (Vec<_>, Vec<_>) = (from..steps.len())
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
/// the run ([`halt`]) — and under `--keep-going` none of the others: a
/// verb breaks nothing the next one reads, and every green is stamped, so
/// the run after the fix owes the reds alone.
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
    // nothing would leave the ledger with no answer for a step the plan
    // has — a number read off a file with holes in it, which
    // `record::Waited::unaccounted` refuses.
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
            // `--no-build` against a binary older than the sources. A
            // halted one built nothing either, and every one after it
            // is filed as halted at its own door.
            Ok(Ran::Step) => built = true,
            Ok(Ran::Stamped | Ran::Halted) => continue,
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
