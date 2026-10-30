//! `cargo xtask verify-ui` — one command from source to a judged
//! headless run: release build (QML is embedded in the exe), offscreen
//! QPA with an explicit font dir, the `PGG_AUTO_*` hooks, a bounded wait
//! with a kill guard, and a verdict off the run's own lines (`outcome`).

mod child;
mod contract;
mod coverage;
mod faults;
pub(crate) mod git_shim;
mod look;
mod observed;
mod options;
mod outcome;
mod ownership;
mod perf;
mod records_check;
mod repos;
mod run;
mod seed;
mod shim;
mod shot_check;
mod stages;
mod verbs;
mod wedge;

pub use coverage::run as verbs;
pub(crate) use faults::run as wedge_check;
pub(crate) use look::look_into;
pub(crate) use options::{canonical_line, suite_words};
pub(crate) use ownership::{ResourceClaim, claim_dir, claim_resource, keep, sweep_yesterdays_runs};
pub(crate) use records_check::{lines as records_lines, run as records_check};
pub use run::run;
pub use shim::git_shim;
pub(crate) use shot_check::run as shot_check;
pub(crate) use verbs::judgment_of;

use crate::command::{self, Permission, Where};

pub(crate) static UI: command::Command = command::Command {
    id: "verify.ui",
    call: "verify-ui <verb>",
    purpose: "one headless run from source to a judged screenshot",
    run_in: Where::Seat,
    needs: &["a verb the harness answers to (.claude/skills/verify-ui/verbs.md)"],
    permission: Permission::Plain,
};

pub(crate) static VERBS: command::Command = command::Command {
    id: "verify.verbs",
    call: "verbs",
    purpose: "the registered lines against the harness, the judgment rows and the sources",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static WEDGE: command::Command = command::Command {
    id: "verify.wedge",
    call: "wedge-check",
    purpose: "whether the harness still catches a run that only looks finished",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static SHOT: command::Command = command::Command {
    id: "verify.shot",
    call: "shot-check",
    purpose: "whether a picture a read landed in is taken again from its source",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static RECORDS: command::Command = command::Command {
    id: "verify.records",
    call: "records-check",
    purpose: "whether a completion waits for its write's records in the command log",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&UI, &VERBS, &WEDGE, &SHOT, &RECORDS];
