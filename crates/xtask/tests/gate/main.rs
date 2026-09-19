//! The gate on throwaway repositories: selection follows the reach of a
//! diff, stamps follow the inputs, main refuses what is not stamped,
//! land rebases → gates → fast-forwards, and the chip guard reads what a
//! seat is holding out of the repository. Every test builds its own
//! repository under the OS temp directory with a git configuration of
//! its own, so they run in parallel and read nobody's identity or hooks.
//!
//! Steps are faked (`PGG_GATE_FAKE_LOG`): what is under test is which
//! steps a change owes and when they are asked again. The repository is
//! a miniature of this workspace's shape —
//! enough for the dependency graph to have edges to follow.

mod support;

/// The runner's own wait module, read in by path: this crate cannot `use`
/// a binary crate's modules, and what the suite waits with has to be the
/// one budget the runner's own tests wait with (`wait::Budget::SUITE`).
/// What the suite does not call is still the runner's.
#[expect(
    dead_code,
    reason = "the runner's module whole; the suite calls a part of it"
)]
#[path = "../../src/wait.rs"]
mod wait;

mod budget;
mod chips;
mod hook;
mod landing;
mod permit;
mod seats;
mod selection;
mod stamps;
