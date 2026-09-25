//! One step: the machine's room taken for it, run against its log,
//! filed in the ledger, and stamped when green.

use std::path::Path;
use std::time::Duration;

use super::Ground;
use super::evidence;
use super::plan::Required;
use super::record;
use super::runner::{FAKE_STAMP, Finished, execute_step};

pub(super) fn log_of(ground: &Ground<'_>, index: usize) -> std::path::PathBuf {
    ground.logs.join(format!("{}-{index:02}.log", ground.name))
}

/// What became of one step short of red. `Step` and `Stamped` differ to
/// the caller: a verb that ran here built the release the rest of its
/// block reuses; one a stamp answered for built nothing here ([`verbs`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ran {
    Step,
    Stamped,
    Halted,
}

/// One step, stamped when green (an always-step never is). `no_build` is a verb's
/// `--no-build`, the block's to hand out ([`verbs`]). A failure comes
/// back as the line to report.
///
/// The step is the unit the budget hands the machine out in, and nothing
/// is killed to make room, so a landing waits out whichever unit is
/// running: the longest are kept ([`Waited::ran`]), since a minutes-long
/// unit is the one worth splitting.
///
/// The ticket is taken before the step announces itself to a measurement
/// (`still::busy`, inside `check::run_step`) and let go after it ends, so
/// nothing holds room it is not using, or part of what it needs while
/// waiting for the rest.
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
    // No room is a red of the run like any other.
    let Some(room) =
        room_for(ground, id, weight).inspect_err(|_| ground.halt.red(id, required.step.always))?
    else {
        return Ok(Ran::Halted);
    };
    // Looked at again after the plan: another tree gating the same commit
    // may have stamped this key while this unit stood in the queue (the
    // stamps are the repository's). Two that miss at the same instant
    // both still run. `--fresh` skips the look.
    // The tests' switch for that window, which nothing driven from outside
    // can land in: it stamps this key here, as the other tree would have.
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
                // A green that cannot be stamped is a red.
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

/// The machine's room for one unit, or `None` when the run's halt stopped
/// it first (its row already filed). The halt is checked before, during
/// and after the queue, so a run gone red starts nothing and holds no place.
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

/// A red step's line for the gate's failure, its log kept first (the
/// re-run after a red gate writes over it in place). A failed keeping is
/// said, never a second failure.
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

/// A unit a stamp answered for, filed under `outcome`: `cached` (found at
/// planning) or `cached-late` (written by another tree while it queued).
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

/// A unit the run's halt stopped, filed as `halted`: neither red nor
/// green, so nothing is stamped and the next run owes it. `weight` is the
/// room it held (none short of running), `ran` how long it had run.
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

/// The `spent ` line of a step's own log, verbatim, or empty. Only the
/// step can split its wall clock — a verb's is build, fixture and window
/// (`verify::run::say_what_it_spent`).
fn spent_in(log: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(log) else {
        return String::new();
    };
    text.lines()
        .find(|line| line.starts_with("spent "))
        .map(|line| line.trim_start_matches("spent ").to_string())
        .unwrap_or_default()
}
