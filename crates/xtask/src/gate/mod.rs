//! `cargo xtask gate` — the pre-merge tests, chosen by machine
//! (internal-docs/反映前テストの機械化.md).
//!
//! The branch's diff, everything that reads it ([`graph`]) and the tests
//! in that reach select the steps ([`plan`]; verbs by [`census`]). Each
//! step is cached by the object ids of what it reads ([`stamp`]); a commit
//! whose every owed step is green is stamped, and the
//! `reference-transaction` hook lets `refs/heads/main` move onto stamped
//! commits only ([`hooks`]).
//!
//! `--host-only` is the daily tier (CLAUDE.md「確認は 3 段」), whose stamps
//! the full run reuses. The first red stops the run ([`halt`]) unless
//! `--keep-going`, which `--all` implies.

mod census;
mod deps;
mod evidence;
mod execute;
mod graph;
mod halt;
mod hooks;
mod inputs;
mod plan;
mod record;
mod reuse;
mod run;
mod runner;
mod sides;
mod stamp;
mod standing;
mod step;
mod tiers;

use std::path::Path;

use halt::Halt;
use record::Waited;
pub(crate) use reuse::preserve_reader;
pub(crate) use run::for_landing;
pub use run::run;
use stamp::Store;
pub(crate) use standing::standing;

use crate::command::{self, Permission, Where};

/// 段 2, which `land` runs for itself.
pub(crate) static PREMERGE: command::Command = command::Command {
    id: "gate.premerge",
    call: "gate",
    purpose: "the pre-merge tests the branch's diff owes, both OSes",
    run_in: Where::Seat,
    needs: &["a committed branch — the reach is read off the diff against main"],
    permission: Permission::Plain,
};

/// 段 1.
pub(crate) static DAILY: command::Command = command::Command {
    id: "gate.daily",
    call: "gate --host-only",
    purpose: "the daily tier: the host half, no container",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

/// 段 3. Without `--fresh` a stamped tree runs almost nothing.
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
pub(crate) use census::Census;
pub(crate) use census::{FILE as CENSUS_FILE, names_in, page_settled_in, record};
pub(crate) use graph::qml_files;
pub(crate) use hooks::{SESSION, SKIP, install};
pub(crate) use plan::tested_on_linux;
pub(crate) use tiers::Tiers;

/// What a gate whose every step was green left behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gated {
    Stamped,
    /// Nothing is stamped: the host's verbs rewrote the census, so the tree
    /// that passed is not the commit. Commit the rewrite and gate again,
    /// which finds every step cached.
    CensusMoved,
}

/// What one side's steps stand on. `runner` is the copy its xtask steps
/// start from (`None` under the tests' faked steps).
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
    /// The run's name for the kept copies of its red steps' logs
    /// (`evidence::keep`).
    run: &'a str,
    /// Where this side's units tally their waits for room another gate
    /// held, for the run's line and record.
    waited: &'a Waited,
    /// When the sides started: every unit's ledger row is an offset from
    /// it, so rows of both sides compare.
    since: std::time::Instant,
    /// The machine's budget, which both sides and every seat draw on.
    pool: &'a crate::budget::Pool,
    /// The tree, which is what the fairness between equals is over.
    seat: &'a str,
    rank: crate::budget::Rank,
    /// `--fresh`: every step runs whether or not a stamp answers, so the
    /// one another tree wrote while this unit queued is not taken either.
    fresh: bool,
    /// The run's stop, which both sides watch ([`halt`]).
    halt: &'a Halt,
}
