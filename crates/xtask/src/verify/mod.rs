//! `cargo xtask verify-ui` — one command from source to a judged
//! headless run.
//!
//! Wraps what the verify-ui skill prescribes: release build (QML is
//! embedded in the exe), offscreen QPA with an explicit font dir, the
//! PGG_AUTO_* hooks, a bounded wait with a kill guard, and the
//! `screenshot saved=true` stderr line as the verdict.

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
