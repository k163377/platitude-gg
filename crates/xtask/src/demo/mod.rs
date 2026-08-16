//! `cargo xtask demo-repo` — throwaway repositories in known states.
//!
//! Every invocation builds a fresh repository, isolated from the
//! developer's git configuration; nothing is ever reused.

mod authorship;
mod basic;
mod conflict;
mod presets;
mod remote;
mod repo;
mod scale;
mod signing;
mod tags;

pub use presets::{create, create_named, run};
pub(crate) use scale::pasted;
