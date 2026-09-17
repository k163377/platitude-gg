//! What a unit is, as everything outside this module meets it: what it
//! weighs, what it holds while it runs, and what it says about the
//! process it started.
//!
//! The ledger those tickets stand in is [`super::ledger`]; the rule that
//! decides which of them starts is [`super::queue`].

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::ledger::{Pool, decide_in, parse, render};
use super::queue::Rank;
use crate::locks::Locked;

/// What a unit that runs one process without a compiler in it takes: a
/// verify-ui verb that reuses a build (one app, its offscreen raster and
/// the git it spawns), or one of this runner's own reading verbs.
pub(crate) const LIGHT: u32 = 1;

/// What a unit that starts a compiler or a test binary takes. Four,
/// because such a unit is one process driving many: cargo drives rustc
/// jobs, a test binary drives its own threads, and the container's half
/// drives both inside a VM. The number is the ratio the default verb
/// count was already chosen against — a third of the machine's threads
/// for the verbs of a side (`gate::default_jobs`) — and it is admission
/// control.
pub(crate) const COMPILE: u32 = 4;

/// One gate's worth of machine, which is the whole budget: both sides at
/// once, each running its checks chain (one compiling unit) beside its
/// block of `jobs` verbs.
///
/// Written this way so that a gate running alone never waits for itself
/// — the budget is exactly what one gate asks for at its widest — and so
/// that a second gate shares that one. Every process on the machine
/// computes it from the same `jobs`, so they agree without talking; a
/// gate told `--jobs` above the machine's count says so in its tickets
/// and widens the pool for everyone while it runs
/// ([`queue::budget_of`]).
pub(crate) fn demand(jobs: usize) -> u32 {
    let verbs = u32::try_from(jobs).unwrap_or(u32::MAX);
    2 * (COMPILE + verbs.saturating_mul(LIGHT))
}

/// What a step's command asks of the machine, read off the words it is
/// spelled with: the runner's own reading verbs and `cargo fmt` start no
/// compiler, a verb told `--no-build` reuses one, and everything else
/// here — clippy, the tests, `shipped`, `bare`, the container's half of
/// each, and the one verb of a side that builds the release — starts one.
pub(crate) fn weight_of(command: &[String], no_build: bool) -> u32 {
    let mut words = command.iter().map(String::as_str);
    let first = words.next().unwrap_or_default();
    // The plan spells this runner's steps `cargo run --locked -p xtask --
    // <verb>` and the gate starts them from a copy of the built runner
    // (`gate::launched`), so the verb is what follows either spelling —
    // found by the `--`, so the flags between are not counted.
    let verb = if first == "cargo" {
        match words.next().unwrap_or_default() {
            "run" => words.find(|word| *word == "--").and(words.next()),
            // The alias, which is what the container's command line is
            // built as (`linux::command_line`) and what a person types.
            "xtask" => words.next(),
            // A cargo that is not this runner: `fmt` reads, the rest
            // compile.
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
    /// The tree the unit belongs to, which is what the fairness between
    /// equals is over ([`queue::order`]).
    pub seat: &'a str,
    /// What the unit is, for `cargo xtask budget` and for the line a
    /// wait that ran out says.
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

/// A ticket held for as long as this stands, and how long it took to get
/// — the wait for other units to finish, zero when there was room at the
/// first look (the microseconds of reading the ledger are not a wait).
#[derive(Debug)]
pub(crate) struct Admitted {
    /// The ticket's two files, taken down in this order; `None` for a
    /// unit that runs under its parent's ticket or under no budget at
    /// all, which holds nothing and gives nothing back.
    pub(super) mine: Option<(PathBuf, PathBuf)>,
    pub(super) _lock: Option<Locked>,
    pub waited: Duration,
}

impl Drop for Admitted {
    fn drop(&mut self) {
        // Under the lock this still holds, as a dead ticket is swept
        // under its own: a name let go of before it is taken away is one
        // another process can be reading between the two.
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

    /// Says that this unit has started `program` at `child`, so that a
    /// ledger reading this ticket after its owner is gone knows the
    /// machine is still being used ([`Pool::leftover`]) — and knows it
    /// by what is at that number.
    pub(crate) fn started(&self, child: u32, program: &str) {
        if let Some((ticket, _)) = &self.mine {
            write_child(ticket, child, program);
        }
    }
}

/// The ticket this process holds for its own work, where it holds one:
/// a standalone command takes exactly one for its whole life and
/// nothing nests inside it ([`standalone`]), so "this process's ticket"
/// is a thing that means something — which is what lets a spawn site
/// deep in another module say what it started without every function
/// between here and there carrying a parameter for it.
///
/// **Only a command that is itself one unit ever sets it.** A runner
/// that drives steps — the gate, and `check` — holds many tickets at
/// once and takes them straight from a [`Pool`]; each of its steps says
/// what it started through the ticket it was handed
/// (`check::run_step`). A runner that went through [`standalone`] would
/// name its first step here and leave every later one writing at a
/// ticket that had already come down.
static MINE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Says that this process's own unit has started `program` at `child` —
/// the cargo a build spawns, the app a verb runs, the docker a container
/// command waits on. Does nothing where this process holds no ticket of
/// its own: under a gate, under a parent's ticket, or before one was
/// taken.
pub(crate) fn child_started(child: u32, program: &str) {
    if let Some(ticket) = MINE.get() {
        write_child(ticket, child, program);
    }
}

/// Writes `child` and the program at it into the ticket at `path`, under
/// the ledger's lock so nobody reads it half written.
///
/// The two go down together because a number without a name is a claim
/// on whoever inherits that number (`Pool::leftover`). The name is the
/// caller's spelling of the program — a probe costs a process on
/// Windows, and this is on the road of every one of a gate's seven
/// hundred steps — written in the probe's own vocabulary so that the
/// reader can compare the two (`subprocess::as_probed`,
/// `subprocess::image_still_at`).
///
/// Best effort on purpose: what this protects against is the owner
/// dying, and a ticket that could not be written simply falls back
/// to what the ledger did before — the room comes back with the
/// lock. A failure here leaves the step to run, which is the thing
/// the whole gate is actually for. A ticket already taken down is
/// read first, so a name with nothing at it stays that way rather
/// than coming back as a unit nobody holds.
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

/// Runs `command` to its end as this unit's work, telling the ledger
/// which process is doing it ([`child_started`]).
///
/// **What `Command::status` does, plus the one thing the budget needs.**
/// A container the launcher started keeps running when the launcher is
/// killed, and a cargo outlives the runner that spawned it; a ledger
/// that handed the room out on the strength of the launcher's lock
/// alone would be counting neither.
pub(crate) fn watched(command: &mut Command) -> std::io::Result<std::process::ExitStatus> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command.spawn()?;
    child_started(child.id(), &program);
    child.wait()
}

/// Set for the children of an admitted unit: they run under their
/// parent's ticket, and a nested ask is answered with a pass
/// ([`under`]).
pub(crate) const HELD: &str = "PGG_BUDGET_HELD";

/// The room for one command that is itself a unit — a verb a session
/// ran, a container command, a `shipped` build, one of `check`'s steps.
///
/// **The two places a ticket is taken are the runner's steps
/// (`gate::run_one`, `gate::runner`, `check::run_side`) and the top of a
/// standalone command.** Below either, everything is a child and carries
/// its parent's ([`under`]) — which is what this answers first, before
/// it goes looking for a repository: inside a container there may be no
/// road to the ledger at all, and what runs there is under the ticket
/// the host-side command took.
///
/// Nothing nests inside a standalone command's own ticket, so no unit is
/// counted twice: the build a verb does is the verb's, and the build
/// `shipped` does is `shipped`'s.
pub(crate) fn standalone(
    tree: &Path,
    weight: u32,
    rank: Rank,
    what: &str,
) -> Result<Admitted, String> {
    let room = marked(tree, weight, rank, what, std::env::var_os(HELD).is_some())?;
    // Said where it happened: a command a person is waiting
    // on has to say that what it is waiting for is the
    // machine.
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
        Pool::of(tree, crate::gate::default_jobs())?.admit_once_the_machine_is_free(&Ask {
            weight,
            rank,
            seat: &crate::gate::seat_of(tree),
            what,
        })?;
    // From here on, whatever this process spawns is this unit's work
    // ([`MINE`]). Set once: a standalone command takes one ticket, and a
    // second call would be a second unit in a process that has no way to
    // say which of them a child belongs to.
    if let Some((ticket, _)) = &room.mine {
        let _ = MINE.set(ticket.clone());
    }
    Ok(room)
}

/// Marks `command` as running under this process's ticket, so nothing in
/// it takes a second one for the same work. Every caller of
/// `check::run_step` is itself one unit — the gate's step under its own
/// ticket, `check`'s step under no budget at all — so the mark goes on
/// beside the measurement's ([`still::step`]).
pub(crate) fn under(command: &mut Command) {
    command.env(HELD, "1");
}
