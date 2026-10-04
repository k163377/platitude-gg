//! `cargo xtask demo-repo` — throwaway repositories in known states.
//!
//! Every invocation builds a fresh repository, isolated from the
//! developer's git configuration.

mod authorship;
mod basic;
mod conflict;
mod deep;
mod discards;
mod lfs;
mod pictures;
mod presets;
mod remote;
mod repo;
mod scale;
mod signing;
mod stack;
mod tags;
mod template;
mod worktrees;

pub(crate) use presets::{claim_root, claimed_roots};
pub use presets::{create, create_named, run};
pub(crate) use worktrees::NESTED_COPY;

use crate::command::{self, Permission, Where};

pub(crate) static DEMO_REPO: command::Command = command::Command {
    id: "demo.repo",
    call: "demo-repo <preset>",
    purpose: "a throwaway repository in a known state, printed as its path",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&DEMO_REPO];
pub(crate) use scale::pasted;
