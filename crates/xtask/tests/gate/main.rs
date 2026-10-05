//! The gate on throwaway repositories. Every test has its own repository
//! and git configuration, so none reads the user's identity or hooks.
//!
//! Steps are faked (`PGG_GATE_FAKE_LOG`): under test is which steps a
//! change owes and when they are asked again, on a miniature of this
//! workspace's shape.

mod support;

/// Read in by path because a binary crate's modules cannot be `use`d: the
/// suite waits with the runner's own budget (`wait::Budget::SUITE`).
#[expect(
    dead_code,
    reason = "the runner's module whole; the suite calls a part of it"
)]
#[path = "../../src/wait.rs"]
mod wait;

mod budget;
mod chips;
mod configuration;
mod halt;
mod hook;
mod landing;
mod leftovers;
mod permit;
mod pins;
mod seats;
mod selection;
mod shell;
mod stamps;
