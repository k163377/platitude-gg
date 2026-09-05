//! The gate on throwaway repositories: selection follows the reach of a
//! diff, stamps follow the inputs, main refuses what is not stamped, and
//! land rebases → gates → fast-forwards. Every test builds its own
//! repository under the OS temp directory with a git configuration of
//! its own, so they run in parallel and read nobody's identity or hooks.
//!
//! Steps are faked (`PG_GATE_FAKE_LOG`): what is under test is which
//! steps a change owes and when they are asked again, not whether cargo
//! passes. The repository is a miniature of this workspace's shape —
//! enough for the dependency graph to have edges to follow.

mod support;

mod hook;
mod landing;
mod selection;
mod stamps;
