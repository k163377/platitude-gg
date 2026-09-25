//! A unit as the rest of the runner meets it: its weight, the ticket it
//! holds while it runs, and the process it started. The ledger is
//! [`super::ledger`]; which unit starts next is [`super::queue`].

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::ledger::{Pool, decide_in, parse, render};
use super::queue::Rank;
use crate::locks::Locked;

/// A unit that starts no compiler: a verify-ui verb reusing a build, or
/// one of this runner's reading verbs.
pub(crate) const LIGHT: u32 = 1;

/// A unit that starts a compiler or a test binary — one process driving
/// many. Four is the ratio the default verb count was chosen against
/// (`budget::default_jobs`); it is admission control, not a measurement.
pub(crate) const COMPILE: u32 = 4;

/// The whole budget is one gate at its widest — both sides, each a
/// compiling chain beside `jobs` verbs — so a gate alone never waits for
/// itself and a second gate shares it. Every process computes it from the
/// same `jobs`, so they agree without talking; a gate told a larger
/// `--jobs` widens the pool for everyone while it runs
/// ([`queue::budget_of`]).
pub(crate) fn demand(jobs: usize) -> u32 {
    let verbs = u32::try_from(jobs).unwrap_or(u32::MAX);
    2 * (COMPILE + verbs.saturating_mul(LIGHT))
}

/// A step's weight, read off its words: the runner's reading verbs,
/// `cargo fmt` and a verb told `--no-build` start no compiler; everything
/// else starts one.
pub(crate) fn weight_of(command: &[String], no_build: bool) -> u32 {
    let mut words = command.iter().map(String::as_str);
    let first = words.next().unwrap_or_default();
    // The plan spells steps `cargo run --locked -p xtask -- <verb>` and the
    // gate starts them from a copy of the runner (`gate::runner::launched`),
    // so the verb follows the `--` or the program.
    let verb = if first == "cargo" {
        match words.next().unwrap_or_default() {
            "run" => words.find(|word| *word == "--").and(words.next()),
            // The alias, as `linux::command_line` builds it and a person
            // types it.
            "xtask" => words.next(),
            "fmt" => return LIGHT,
            _ => return COMPILE,
        }
    } else {
        words.next()
    };
    // `linux <verb>` is the same verb inside the container.
    let verb = match verb {
        Some("linux") => words.next(),
        other => other,
    };
    match verb {
        Some("structure" | "waits" | "docs" | "verbs" | "deny" | "qmltest") => LIGHT,
        Some("verify-ui") if no_build => LIGHT,
        _ => COMPILE,
    }
}

/// What one unit asks for.
pub(crate) struct Ask<'a> {
    pub weight: u32,
    pub rank: Rank,
    /// The unit's tree, which fairness between equals is over
    /// ([`queue::order`]).
    pub seat: &'a str,
    /// Named by `cargo xtask budget` and by a wait that ran out.
    pub what: &'a str,
}

impl<'a> Ask<'a> {
    /// A landing's turn: none of the machine, at the landings' own rank.
    pub(crate) fn turn(seat: &'a str, what: &'a str) -> Ask<'a> {
        Ask {
            weight: 0,
            rank: Rank::Landing,
            seat,
            what,
        }
    }
}

/// A ticket held while this stands, and how long it waited for other
/// units (zero when there was room at the first look).
#[derive(Debug)]
pub(crate) struct Admitted {
    /// The ticket's two files, removed in this order; `None` for a unit
    /// under its parent's ticket, which holds nothing.
    pub(super) mine: Option<(PathBuf, PathBuf)>,
    pub(super) _lock: Option<Locked>,
    pub waited: Duration,
}

impl Drop for Admitted {
    fn drop(&mut self) {
        // Removed while `_lock` is still held, as a dead ticket is swept:
        // released first, the name could be read between the two.
        if let Some((ticket, lock)) = &self.mine {
            let _ = std::fs::remove_file(ticket);
            let _ = std::fs::remove_file(lock);
        }
    }
}

impl Admitted {
    /// A unit that takes nothing: one under its parent's ticket.
    pub(super) fn carried() -> Self {
        Self {
            mine: None,
            _lock: None,
            waited: Duration::ZERO,
        }
    }

    /// Records `program` at pid `child` in the ticket, so a ledger reading
    /// it after the owner is gone knows the machine is still in use
    /// ([`Pool::leftover`]).
    pub(crate) fn started(&self, child: u32, program: &str) {
        if let Some((ticket, _)) = &self.mine {
            write_child(ticket, child, program);
        }
    }
}

/// A standalone command's ticket: it holds exactly one for its whole life
/// ([`standalone`]), so a spawn site deep in another module can record
/// what it started without a parameter threaded down to it.
///
/// Only a command that is itself one unit sets it. A runner of steps (the
/// gate, `check`) holds many tickets from a [`Pool`] and records through
/// each step's own (`check::run_step`); through [`standalone`] its later
/// steps would write at a ticket already taken down.
static MINE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Records `program` at `child` in this process's own ticket (`MINE`);
/// does nothing where it holds none — under a gate, under a parent's
/// ticket, or before one was taken.
pub(crate) fn child_started(child: u32, program: &str) {
    if let Some(ticket) = MINE.get() {
        write_child(ticket, child, program);
    }
}

/// Writes `child` and its program into `ticket`, under the ledger's lock
/// so nobody reads it half written.
///
/// Pid and name go together: a pid alone is a claim on whoever inherits
/// it (`Pool::leftover`). The name is the caller's spelling, not a probe
/// (a probe costs a process on Windows, on every gate step), put in the
/// probe's vocabulary so the reader can compare (`subprocess::as_probed`,
/// `subprocess::image_still_at`).
///
/// Best effort: a ticket that could not be written falls back to its lock
/// alone, and the step still runs. The ticket is read first, so one
/// already taken down is not written back as a unit nobody holds.
fn write_child(ticket: &Path, child: u32, program: &str) {
    let Some(dir) = ticket.parent() else {
        return;
    };
    let Ok(_decision) = decide_in(dir) else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(ticket) else {
        return;
    };
    let Some(mut held) = parse(&text) else {
        return;
    };
    held.child = child;
    held.child_name = crate::subprocess::as_probed(program);
    let _ = std::fs::write(ticket, render(&held));
}

/// `Command::status`, plus telling the ledger which process does the work
/// ([`child_started`]): a container outlives its killed launcher and a
/// cargo the runner that spawned it, which the launcher's lock alone
/// would not count.
pub(crate) fn watched(command: &mut Command) -> std::io::Result<std::process::ExitStatus> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command.spawn()?;
    child_started(child.id(), &program);
    child.wait()
}

/// Set for an admitted unit's children: a nested ask is answered with a
/// pass ([`under`]).
pub(crate) const HELD: &str = "PGG_BUDGET_HELD";

/// The room for a command that is itself one unit (a session's verb, a
/// container command, `shipped`).
///
/// Tickets are taken only at a runner's steps (`gate::step::run_one`,
/// `gate::runner`, `check::run_side`) and at the top of a standalone
/// command; everything below carries its parent's ([`under`]). That is
/// answered before looking for a repository: inside a container there may
/// be no road to the ledger. Nothing nests inside a standalone ticket, so
/// no unit is counted twice.
pub(crate) fn standalone(
    tree: &Path,
    weight: u32,
    rank: Rank,
    what: &str,
) -> Result<Admitted, String> {
    let room = marked(tree, weight, rank, what, std::env::var_os(HELD).is_some())?;
    if !room.waited.is_zero() {
        println!(
            "waited {} for room on the machine ({what}) — `cargo xtask budget` says who has it",
            crate::seats::format_age(Some(room.waited))
        );
    }
    Ok(room)
}

pub(super) fn marked(
    tree: &Path,
    weight: u32,
    rank: Rank,
    what: &str,
    carried: bool,
) -> Result<Admitted, String> {
    if carried {
        return Ok(Admitted::carried());
    }
    let room =
        Pool::of(tree, crate::budget::default_jobs())?.admit_once_the_machine_is_free(&Ask {
            weight,
            rank,
            seat: &crate::budget::seat_of(tree),
            what,
        })?;
    // From here on, whatever this process spawns is this unit's work. Set
    // once: with a second unit, a child could not say which it belongs to.
    if let Some((ticket, _)) = &room.mine {
        let _ = MINE.set(ticket.clone());
    }
    Ok(room)
}

/// Marks `command` as running under this process's ticket, so nothing in
/// it takes a second one for the same work.
pub(crate) fn under(command: &mut Command) {
    command.env(HELD, "1");
}
