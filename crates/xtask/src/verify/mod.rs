//! `cargo xtask verify-ui` — one command from source to a judged
//! headless run.
//!
//! Wraps what the verify-ui skill prescribes: release build (QML is
//! embedded in the exe), offscreen QPA with an explicit font dir, the
//! PG_AUTO_* hooks, a bounded wait with a kill guard, and the
//! `screenshot saved=true` stderr line as the verdict.

mod options;
mod outcome;
mod ownership;
mod remote_verbs;
mod repos;
mod run;
mod shim;
mod verbs;

pub use run::run;
pub use shim::git_shim;
