//! The verification harness: what the app carries so that `cargo xtask
//! verify-ui`, `cargo xtask perf` and the screenshot runs can drive it and
//! read what came out. Nobody at a window ever reaches any of it.
//!
//! **The `PG_*` environment is read here and nowhere else in the crate.**
//! That is what makes the harness a thing a build can be without: the app
//! asks this module what is driving, and a build that left the harness out
//! answers "nothing" from a constant instead of from `std::env`.

pub(crate) mod memprobe;
mod perf_probe;

pub use perf_probe::PerfProbe;
