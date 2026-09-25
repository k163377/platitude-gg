//! `cargo xtask verify-ui` — one command from source to a judged
//! headless run: release build (QML is embedded in the exe), offscreen
//! QPA with an explicit font dir, the `PGG_AUTO_*` hooks, a bounded wait
//! with a kill guard, and a verdict off the run's own lines (`outcome`).

mod child;
mod coverage;
mod faults;
mod look;
mod options;
mod outcome;
mod ownership;
mod perf;
mod repos;
mod run;
mod seed;
mod shim;
mod verbs;
mod wedge;

pub use coverage::run as verbs;
pub(crate) use faults::run as wedge_check;
pub(crate) use options::suite_words;
pub(crate) use ownership::{ResourceClaim, claim_dir, claim_resource, keep, sweep_yesterdays_runs};
pub use run::run;
pub use shim::git_shim;

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
    purpose: "which verbs exist, and what each one is for",
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

pub(crate) static COMMANDS: &[&command::Command] = &[&UI, &VERBS, &WEDGE];
