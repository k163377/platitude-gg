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
//! side alone. `--all` counts every file as changed — stage 2 in full —
//! and `--verb <line>` adds verify-ui lines to run (and record) besides
//! the census's. The first red stops the run ([`halt`]) unless
//! `--keep-going` — which `--all` implies — says to run the rest.

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
pub use run::run;
pub(crate) use run::{default_jobs, for_landing};
pub(crate) use sides::seat_of;
use stamp::Store;
pub(crate) use standing::standing;

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
pub(crate) use plan::tested_on_linux;
pub(crate) use tiers::Tiers;

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
    /// The run's stop, which both sides watch ([`halt`]).
    halt: &'a Halt,
}
