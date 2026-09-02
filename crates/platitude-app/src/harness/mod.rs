//! The verification harness: what the app carries so that `cargo xtask
//! verify-ui`, `cargo xtask perf` and the screenshot runs can drive it and
//! read what came out. Nobody at a window ever reaches any of it.
//!
//! **The `PG_*` environment is read in this module and nowhere else in the
//! crate.** That is what makes the harness a thing a build can be without:
//! the app asks here what is driving, and a build without the `automation`
//! feature answers "nothing" from an idle record ([`knobs`]) instead of
//! from `std::env`. The QML half goes the same way — the verb files are a
//! module of their own (`src/auto`, `platitude.auto`), embedded under the
//! same feature, and nothing in `platitude.ui` names a type from it.
//!
//! A plain `cargo build --release` is the build without it. Every xtask
//! that drives the app asks for the feature back (`crate::app_exe`), so
//! the shipped binary is the only one that has to be remembered about.

mod faults;
mod knobs;
pub(crate) mod memprobe;
#[cfg(feature = "automation")]
mod perf_probe;

pub(crate) use faults::{fail_graph_pass, pass_hooks};
pub(crate) use knobs::knobs;

/// Starts the clock every measurement is taken against.
///
/// **First thing in `main`, before anything that can fail.** What it
/// measures is the whole of the process's life, and a later start would
/// quietly shorten every startup number by however much ran before it.
pub(crate) fn start_clock() {
    #[cfg(feature = "automation")]
    perf_probe::PerfProbe::start_clock();
}

/// Registers the harness's own QML types. Nothing in a build without it —
/// and nothing asks for them either, because the QML that names them is
/// the module the same feature leaves out.
#[cfg(feature = "automation")]
pub(crate) fn install(app: &mut qtbridge::QApp) {
    app.register::<perf_probe::PerfProbe>();
}

#[cfg(not(feature = "automation"))]
pub(crate) fn install(_app: &mut qtbridge::QApp) {}

/// The reporting channel the harness reads its answers off (QML →
/// tracing → `xtask`). A build without the harness has nobody to say
/// anything to, and says nothing.
#[cfg(feature = "automation")]
pub(crate) fn report(message: &str) {
    tracing::info!(target: "bench", "{message}");
}

#[cfg(not(feature = "automation"))]
pub(crate) fn report(_message: &str) {}
