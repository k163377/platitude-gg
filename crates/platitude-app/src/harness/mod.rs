//! The verification harness: what the app carries so that `cargo xtask
//! verify-ui`, `cargo xtask perf` and the screenshot runs can drive it and
//! read what came out. Behind the `automation` feature
//! (.claude/rules/app-ui.md): a build without it answers "nothing is
//! driving" from an idle record ([`knobs`]).
//!
//! A plain `cargo build --release` is the build without it; every xtask
//! that drives the app asks for the feature back (`xtask::tree::app_exe`).

mod deadline;
mod faults;
mod knobs;
pub(crate) mod memprobe;
#[cfg(feature = "automation")]
mod perf_probe;
#[cfg(feature = "automation")]
mod singleton;

pub(crate) use deadline::{Station, at as station, watch as watch_deadline};
pub(crate) use faults::{
    fail_graph_pass, held_save, hold_the_working_tree_row, let_the_working_tree_row_through,
    pass_hooks, walk_again_while_held,
};
pub(crate) use knobs::knobs;

/// A line for the verdict a run is read off, said from the product's own
/// path: the moment a step nobody at a window can see is over, such as
/// the shutdown's join of the saves.
#[cfg(feature = "automation")]
pub(crate) fn said(message: &str) {
    tracing::info!(target: "bench", "{message}");
}

#[cfg(not(feature = "automation"))]
pub(crate) fn said(_message: &str) {}

/// Starts the clock every measurement is taken against. First thing in
/// `main`: a later start would quietly shorten every startup number.
pub(crate) fn start_clock() {
    #[cfg(feature = "automation")]
    perf_probe::PerfProbe::start_clock();
}

/// Registers the harness's own QML types; nothing without the feature,
/// which leaves out the QML naming them too (why `Harness` is a type of
/// its own: `singleton`).
#[cfg(feature = "automation")]
pub(crate) fn install(app: &mut qtbridge::QApp) {
    app.register::<perf_probe::PerfProbe>()
        .register::<singleton::Harness>();
}

#[cfg(not(feature = "automation"))]
pub(crate) fn install(_app: &mut qtbridge::QApp) {}

/// The reporting channel the harness reads its answers off (QML →
/// tracing → `xtask`). No arm without the feature: the only caller is
/// [`singleton::Harness`].
#[cfg(feature = "automation")]
pub(crate) fn report(message: &str) {
    tracing::info!(target: "bench", "{message}");
    // The loop's last known turn ([`deadline`]).
    deadline::heard(message);
}
