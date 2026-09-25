//! One step: the machine's room taken for it, run against its log,
//! filed in the ledger, and stamped when green.

use std::path::Path;
use std::time::Duration;

use super::Ground;
use super::evidence;
use super::plan::Required;
use super::record;
use super::runner::{FAKE_STAMP, Finished, execute_step};

/// Where a side's step at `index` writes what it says.
pub(super) fn log_of(ground: &Ground<'_>, index: usize) -> std::path::PathBuf {
    ground.logs.join(format!("{}-{index:02}.log", ground.name))
}

/// What became of one step short of red: it ran here, a stamp answered
/// for it, or the run's halt stopped it ([`halt`]).
///
/// The first two are not the same to the caller. A verb that ran here
/// built the release the rest of its block reuses; a verb a stamp
/// answered for built nothing here, whatever it built in the tree that
/// took the stamp ([`verbs`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ran {
    Step,
    Stamped,
    Halted,
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
pub(super) fn run_one(
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
    // A unit that could not be given room is a red of the run like any
    // other, and stops it like any other.
    let Some(room) =
        room_for(ground, id, weight).inspect_err(|_| ground.halt.red(id, required.step.always))?
    else {
        return Ok(Ran::Halted);
    };
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
    // own ceiling. A unit the halt ended held it until then.
    ground.waited.ran(id, weight, ran);
    let outcome = match outcome {
        Ok(Finished::Halted) => {
            println!("[{name}] halted {id} ({secs}s) — the run went red elsewhere");
            return Ok(halted(ground, id, weight, room.waited, ran));
        }
        Ok(Finished::Green) => Ok(()),
        Err(why) => Err(why),
    };
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
            if !required.step.always
                && let Err(why) = ground.store.mark_step(
                    &required.key,
                    &format!("{id}\n{}\n", required.step.command.join(" ")),
                )
            {
                // Green, and still a run that cannot say so for next time:
                // red, and stopped as any red is.
                ground.halt.red(id, required.step.always);
                return Err(format!("{id}: green but not stamped: {why}"));
            }
            Ok(Ran::Step)
        }
        Err(why) => {
            println!("[{name}] FAIL   {id} ({secs}s): {why}");
            ground.halt.red(id, required.step.always);
            Err(kept_red(ground, required, &log))
        }
    }
}

/// The machine's room for one unit, or `None` for a unit the run's halt
/// stopped first — its row already filed ([`halted`]). The halt is looked
/// at before the unit queues, while it waits, and once it is admitted, so
/// a run gone red starts nothing more and holds no place in the queue.
fn room_for(
    ground: &Ground<'_>,
    id: &str,
    weight: u32,
) -> Result<Option<crate::budget::Admitted>, String> {
    if ground.halt.raised() {
        halted(ground, id, 0, Duration::ZERO, Duration::ZERO);
        return Ok(None);
    }
    let asked = ground.since.elapsed();
    let room = ground
        .pool
        .admit_unless(
            &crate::budget::Ask {
                weight,
                rank: ground.rank,
                seat: ground.seat,
                what: id,
            },
            &|| ground.halt.raised(),
        )
        .map_err(|why| format!("{id}: {why}"))?;
    let Some(room) = room else {
        let waited = ground.since.elapsed() - asked;
        ground.waited.add(waited);
        halted(ground, id, 0, waited, Duration::ZERO);
        return Ok(None);
    };
    ground.waited.add(room.waited);
    if ground.halt.raised() {
        halted(ground, id, 0, room.waited, Duration::ZERO);
        return Ok(None);
    }
    Ok(Some(room))
}

/// A red step's line for the gate's failure, its log kept first: the
/// re-run that follows a red gate writes over it in place (`evidence`),
/// and a keeping that failed is said here and is never a second failure
/// of the step.
fn kept_red(ground: &Ground<'_>, required: &Required, log: &Path) -> String {
    let (name, id) = (ground.name, &required.step.id);
    match evidence::keep(
        ground.logs,
        ground.run,
        ground.dir,
        log,
        &required.step.command,
    ) {
        Ok(None) => id.clone(),
        Ok(Some(note)) => {
            println!("[{name}] {note}");
            format!("{id} — {note}")
        }
        Err(why) => {
            println!("[{name}] the red step's log was not kept: {why}");
            id.clone()
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

/// A unit the run's halt stopped ([`halt`]) — at its door, while it
/// waited for room, or while it ran — filed as `halted`: not a red of its
/// own, and not a green either, so nothing is stamped for it and the next
/// run owes it. `weight` is the room it held (none short of running) and
/// `ran` how long it had run when it was ended.
fn halted(ground: &Ground<'_>, id: &str, weight: u32, waited: Duration, ran: Duration) -> Ran {
    ground.waited.filed(record::Row {
        id: id.to_string(),
        weight,
        outcome: "halted",
        waited,
        from_start: ground.since.elapsed(),
        ran,
        spent: String::new(),
    });
    Ran::Halted
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
