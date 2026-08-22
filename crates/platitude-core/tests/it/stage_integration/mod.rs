//! Staging against real repositories: whole files, hunks and single lines.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod partial;
mod refusals;
mod whole;
