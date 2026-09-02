//! What one measured run reported, and whether it reported enough.
//!
//! The app says its numbers on stderr as `key=value` (`platitude_gg::init_tracing`),
//! so a reading is assembled a line at a time as the run talks, and refused as a
//! whole if a number this run was asked to take never arrived.

use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc;
use std::time::Instant;

use super::Options;

/// What one run reported.
#[derive(Default, Clone, Debug)]
pub(super) struct Reading {
    pub(super) peak_working_set: u64,
    pub(super) peak_private: u64,
    /// What the process still held after `--settle-ms`, or 0 where the
    /// run was not asked to wait. Read beside the peak: the difference
    /// between them is work the process let go of once it was idle.
    ///
    /// The working set alone, because that is the side the budget is read
    /// against (ci/baseline/perf-windows-x64.md §判定).
    pub(super) settled_working_set: u64,
    /// Process start to the first frame of the visible graph.
    ///
    /// Timed here rather than taken off the app's own `first_chunk_ms`,
    /// which starts counting when the walk starts and so leaves out
    /// everything before it: the runtime, the window, the QML engine and
    /// opening the repository. Those are most of what a person waits for.
    pub(super) startup_ms: Option<u64>,
    pub(super) first_chunk_ms: Option<u64>,
    pub(super) total_ms: Option<u64>,
    pub(super) fps: Option<f64>,
    /// Frame intervals the scroll bench measured over 16.7ms — the count
    /// of frames a person would have seen as a stutter, which is the same
    /// question on a 100Hz screen and a 180Hz one.
    pub(super) over_16_ms: Option<usize>,
    pub(super) details_ms: Vec<u64>,
    pub(super) details_frame_ms: Vec<f64>,
    pub(super) diff_frame_ms: Vec<f64>,
    pub(super) frame_p95_ms: Option<f64>,
    pub(super) frame_p99_ms: Option<f64>,
    pub(super) frame_max_ms: Option<f64>,
    pub(super) frame_count: Option<usize>,
    pub(super) traced_frames: usize,
    pub(super) scroll_visible: bool,
    pub(super) scroll_framed: bool,
    pub(super) selection: Option<String>,
    /// The commit the run actually selected. Recorded because the default
    /// `--selection first` names no commit — it takes the newest
    /// ref-reachable one, which a `git fetch` replaces (`perf::corpus`).
    pub(super) selected_oid: Option<String>,
    pub(super) scenario_complete: Option<String>,
    pub(super) rows: Option<usize>,
    /// What Qt said about the graphics device it chose, from `QSG_INFO`.
    /// This machine has more than one adapter and Qt does not always take
    /// the same one (ci/baseline/perf-windows-x64.md §この記録の読み方 3).
    pub(super) graphics: Vec<String>,
    /// What the machine around the process was doing while it ran.
    pub(super) conditions: super::sampler::Conditions,
    pub(super) failure: Option<String>,
    /// The `mem report` line with the largest `rust_live`, verbatim.
    pub(super) breakdown: Option<String>,
    pub(super) breakdown_live: u64,
    pub(super) perf_done: bool,
}

pub(super) fn read_app(
    stderr: Option<std::process::ChildStderr>,
    started: Instant,
    mut log: std::fs::File,
) -> (mpsc::Receiver<bool>, std::thread::JoinHandle<Reading>) {
    let (done_tx, done_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut found = Reading::default();
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if let Err(error) = writeln!(log, "{} {line}", started.elapsed().as_micros()) {
                    found.failure = Some(format!("could not preserve app log: {error}"));
                }
                if found.startup_ms.is_none() && line.contains("perf_graph_frame") {
                    found.startup_ms = Some(started.elapsed().as_millis() as u64);
                }
                if line.contains("perf_failed") {
                    found.failure = Some(line.clone());
                }
                absorb(&line, &mut found);
                let ended = line.contains("perf_done");
                let completion = match (found.failure.is_some(), ended) {
                    (true, _) => Some(false),
                    (false, true) => Some(true),
                    (false, false) => None,
                };
                if let Some(done) = completion
                    && done_tx.send(done).is_err()
                {
                    found.failure = Some("measurement receiver disconnected".into());
                    break;
                }
            }
        }
        found
    });
    (done_rx, reader)
}

/// Refuses a reading that lost a number this run was asked to take: a
/// run that measured nothing must not read as one that measured well.
/// The app's log picking up colour is one way to lose every `key=value`
/// at once (`platitude_gg::init_tracing`).
pub(super) fn missing(reading: &Reading, opts: &Options) -> Result<(), String> {
    let mut gaps = Vec::new();
    if reading.peak_working_set == 0 || reading.peak_private == 0 {
        gaps.push("nonzero process memory samples");
    }
    if opts.breakdown
        && !reading
            .breakdown
            .as_ref()
            .is_some_and(|s| s.contains("counted=true"))
    {
        gaps.push("a counted Rust heap (build with memprobe)");
    }
    if !reading.perf_done {
        gaps.push("perf completion (no `perf_done`)");
    }
    if !opts.open {
        return if gaps.is_empty() {
            Ok(())
        } else {
            Err(format!("the run ended without {}", gaps.join(", ")))
        };
    }
    if reading.startup_ms.is_none() {
        gaps.push("startup (no `perf_graph_frame`)");
    }
    if reading.first_chunk_ms.is_none() && reading.rows != Some(0) {
        gaps.push("the walk (no `first_chunk_ms=`)");
    }
    if reading.total_ms.is_none() {
        gaps.push("the finished graph (no `elapsed_ms=`)");
    }
    let expected = format!(
        "selection={} details={} diff={} graph={} scrolled={}",
        opts.selection,
        opts.select,
        opts.select && opts.diff,
        opts.scroll || !opts.select || !opts.diff,
        opts.scroll
    );
    if reading.selection.as_deref() != Some(opts.selection.as_str())
        || !reading
            .scenario_complete
            .as_ref()
            .is_some_and(|line| line.contains(&expected))
    {
        gaps.push("the requested selection and completed scenario");
    }
    if opts.select && (reading.details_ms.is_empty() || reading.details_frame_ms.is_empty()) {
        gaps.push("the interaction (data and rendered frame)");
    }
    if !opts.select && (!reading.details_ms.is_empty() || !reading.details_frame_ms.is_empty()) {
        gaps.push("an actually unselected page");
    }
    if opts.select && opts.diff && reading.diff_frame_ms.is_empty() {
        gaps.push("the requested diff frame");
    }
    if opts.scroll
        && (!reading.fps.is_some_and(|fps| fps.is_finite() && fps > 0.0)
            || !reading.scroll_visible
            || !reading.scroll_framed)
    {
        gaps.push("fps of a visible, moving graph");
    }
    if opts.scroll
        && opts.trace_frames
        && (reading.traced_frames == 0 || reading.frame_count != Some(reading.traced_frames))
    {
        gaps.push("the complete timestamped frame trace");
    }
    if gaps.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the run ended without {} — the app's log did not say what this \
         measurement reads, so the numbers it did take cannot be published \
         as a whole reading",
        gaps.join(", ")
    ))
}
/// The line the graph stream says when it has finished — ordinary
/// application logging, not the harness.
fn graph_finished(line: &str) -> bool {
    line.contains("graph stream finished") || line.contains("graph replaced in place")
}

/// What Qt says about the device it is drawing on and the rate it thinks
/// it has, under `QSG_INFO=1`. Kept verbatim: two runs on different
/// adapters are not each other's control, and this machine offers three
/// (an NVIDIA discrete, an AMD integrated, and the basic render driver).
fn graphics_note(line: &str) -> bool {
    [
        "Creating QRhi with backend",
        "Adapter ",
        "using this adapter",
        "using vsync:",
    ]
    .iter()
    .any(|mark| line.contains(mark))
}

pub(super) fn absorb(line: &str, found: &mut Reading) {
    if line.contains("perf_frame ") {
        found.traced_frames += 1;
    }
    if line.contains("perf_done") {
        found.perf_done = true;
    }
    if graphics_note(line) && found.graphics.len() < 12 {
        // Qt says the same thing once per window it builds, and names the
        // window it is saying it about — an address that is new every
        // process. The record wants the set of facts, which is what two
        // runs can be held to having in common.
        let said = line
            .split(" for window")
            .next()
            .unwrap_or(line)
            .trim()
            .to_string();
        if !found.graphics.contains(&said) {
            found.graphics.push(said);
        }
    }
    if let Some(v) = field(line, "first_chunk_ms=") {
        found.first_chunk_ms = v.parse().ok();
    }
    if graph_finished(line)
        && let Some(v) = field(line, "elapsed_ms=")
    {
        found.total_ms = v.parse().ok();
    }
    if line.contains("details request round trip")
        && let Some(v) = field(line, "elapsed_ms=")
        && let Ok(ms) = v.parse()
    {
        found.details_ms.push(ms);
    }
    if line.contains("perf_selection") {
        found.selection = field(line, "mode=").map(str::to_string);
        found.selected_oid = field(line, "oid=").map(str::to_string);
    }
    if line.contains("perf_complete") {
        found.scenario_complete = Some(line.to_string());
        found.rows = field(line, "rows=").and_then(|v| v.parse().ok());
    }
    if line.contains("perf_scroll_frame") {
        found.scroll_framed = field(line, "visible=") == Some("true")
            && field(line, "row=")
                .and_then(|v| v.parse::<usize>().ok())
                .is_some();
    }
    if line.contains("perf_details_frame")
        && let Some(ms) = field(line, "elapsed_ms=").and_then(|v| v.parse().ok())
    {
        found.details_frame_ms.push(ms);
    }
    if line.contains("perf_diff_frame")
        && let Some(ms) = field(line, "elapsed_ms=").and_then(|v| v.parse().ok())
    {
        found.diff_frame_ms.push(ms);
    }
    if line.contains("scroll_bench") {
        found.frame_count = field(line, "frame_count=").and_then(|v| v.parse().ok());
        found.fps = field(line, "fps=").and_then(|v| v.parse().ok());
        found.scroll_visible = field(line, "visible=") == Some("true")
            && field(line, "moved=")
                .and_then(|v| v.parse::<f64>().ok())
                .is_some_and(|v| v > 0.0)
            && field(line, "frame_count=")
                .and_then(|v| v.parse::<u64>().ok())
                .is_some_and(|v| v > 0);
        found.over_16_ms = field(line, "over_16_ms=").and_then(|v| v.parse().ok());
        found.frame_p95_ms = field(line, "frame_p95_ms=").and_then(|v| v.parse().ok());
        found.frame_p99_ms = field(line, "frame_p99_ms=").and_then(|v| v.parse().ok());
        found.frame_max_ms = field(line, "frame_max_ms=").and_then(|v| v.parse().ok());
    }
    if line.contains("mem report") {
        let live: u64 = field(line, "rust_live=")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        // The largest heap the run reached is the one worth breaking down;
        // an early tick would name a graph that had not finished loading.
        if live >= found.breakdown_live {
            found.breakdown_live = live;
            found.breakdown = Some(line.to_string());
        }
    }
}
/// One `key=value` off a line whose keys are whole words: the tables the
/// runner writes for itself (`perf::display`, `perf::sampler`).
///
/// Not [`field`], which finds the key anywhere in the line — that answers
/// `false` for `y=` on a line carrying `primary=false`, and a screen whose
/// coordinate would not parse is a screen the window cannot be placed on.
pub(super) fn token<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split_whitespace()
        .find_map(|word| word.strip_prefix(key))
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split(key)
        .nth(1)
        .map(|rest| rest.split_whitespace().next().unwrap_or(rest))
}

#[cfg(test)]
mod tests;
