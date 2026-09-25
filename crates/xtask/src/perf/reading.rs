//! What one measured run reported, and whether it reported enough.
//!
//! The app says its numbers on stderr as `key=value` (`platitude_gg::init_tracing`):
//! a reading is assembled a line at a time, and refused whole if a number this
//! run was asked to take never arrived.

use std::io::Write;
use std::sync::mpsc;
use std::time::Instant;

use super::Options;

#[derive(Default, Clone, Debug)]
pub(super) struct Reading {
    pub(super) peak_working_set: u64,
    pub(super) os_peak_working_set: Option<u64>,
    pub(super) peak_private: u64,
    /// What the process still held after `--settle-ms`, or 0 where the
    /// run was not asked to wait; the peak minus this is what it let go
    /// of once idle. The working set alone: the budget is read against it
    /// (ci/baseline/perf-windows-x64.md §判定).
    pub(super) settled_working_set: u64,
    /// Process start to the first frame of the visible graph. Timed here
    /// because the app's `first_chunk_ms` starts at the walk and leaves out
    /// the runtime, the window, the QML engine and opening the repository.
    pub(super) startup_ms: Option<u64>,
    /// Process start to the graph stream saying it finished — a data
    /// event, one frame short of the screen. Ordinary application logging,
    /// so it is the one startup number a build with no harness also
    /// answers (`Options::harness`).
    pub(super) graph_ms: Option<u64>,
    pub(super) first_chunk_ms: Option<u64>,
    pub(super) total_ms: Option<u64>,
    pub(super) fps: Option<f64>,
    /// Frame intervals the scroll bench measured over 16.7ms: a fixed
    /// stutter threshold, so screens of any rate answer the same question.
    pub(super) over_16_ms: Option<usize>,
    pub(super) details_ms: Vec<u64>,
    /// The same request, to where the answer is in the model and its
    /// signals are out — so the wait divides into the read, this, and the
    /// painting the frame below ends.
    pub(super) details_applied_ms: Vec<u64>,
    pub(super) details_frame_ms: Vec<f64>,
    /// The file's own round trip. The frame below is the whole wait, so
    /// the two split the longest wait into the read and the drawing.
    pub(super) diff_ms: Vec<u64>,
    /// And where the rows of it are in the model (`details_applied_ms`).
    pub(super) diff_applied_ms: Vec<u64>,
    pub(super) diff_frame_ms: Vec<f64>,
    pub(super) events: Vec<super::interactions::Event>,
    pub(super) diff_scrolls: Vec<String>,
    pub(super) frame_p95_ms: Option<f64>,
    pub(super) frame_p99_ms: Option<f64>,
    pub(super) frame_max_ms: Option<f64>,
    pub(super) frame_count: Option<usize>,
    pub(super) traced_frames: usize,
    pub(super) scroll_visible: bool,
    pub(super) scroll_framed: bool,
    pub(super) selection: Option<String>,
    /// The commit the run actually selected: the default `--selection
    /// first` takes the newest ref-reachable one, which a `git fetch`
    /// replaces (`perf::corpus`).
    pub(super) selected_oid: Option<String>,
    pub(super) scenario_complete: Option<String>,
    pub(super) rows: Option<usize>,
    /// What Qt said about the graphics device it chose, from `QSG_INFO`:
    /// Qt does not always take the same adapter
    /// (ci/baseline/perf-windows-x64.md §この記録の読み方 1).
    pub(super) graphics: Vec<String>,
    /// What the machine around the process was doing while it ran.
    pub(super) conditions: super::sampler::Conditions,
    pub(super) failure: Option<String>,
    /// The `mem report` line with the largest `rust_live`, verbatim.
    pub(super) breakdown: Option<String>,
    pub(super) breakdown_live: u64,
    /// What the settled process held, read from outside it under
    /// `--attribute`; the whole text is `attribution.txt` in the run.
    pub(super) attribution: Option<super::attribution::Attribution>,
    /// The font walk's three lines as they arrived and, once `measure` has
    /// weighed them against the sampler, what the process weighed either
    /// side (`perf::fonts`). `None` where the run never said the lines.
    pub(super) font_walk: Option<super::fonts::FontWalk>,
    pub(super) perf_done: bool,
}

/// What the parent watches while the app talks: the scroll bench, the one
/// phase with a deadline of its own (`measure::SCROLL_CEILING`).
#[derive(Default)]
pub(super) struct Scroll {
    began: std::sync::atomic::AtomicBool,
    ended: std::sync::atomic::AtomicBool,
    generation: std::sync::atomic::AtomicUsize,
}

impl Scroll {
    pub(super) fn generation(&self) -> usize {
        self.generation.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn begin(&self) {
        use std::sync::atomic::Ordering::Relaxed;
        self.ended.store(false, Relaxed);
        self.generation.fetch_add(1, Relaxed);
        self.began.store(true, Relaxed);
    }
    pub(super) fn running(&self) -> bool {
        use std::sync::atomic::Ordering::Relaxed;
        self.began.load(Relaxed) && !self.ended.load(Relaxed)
    }
}

pub(super) fn read_app(
    stderr: Option<std::process::ChildStderr>,
    started: Instant,
    mut log: std::fs::File,
    harness: bool,
) -> (
    mpsc::Receiver<bool>,
    std::sync::Arc<Scroll>,
    std::thread::JoinHandle<Reading>,
) {
    let scroll = std::sync::Arc::new(Scroll::default());
    let watched = std::sync::Arc::clone(&scroll);
    let (done_tx, done_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut found = Reading::default();
        if let Some(pipe) = stderr {
            for line in crate::app_out::lines(pipe) {
                if let Err(error) = writeln!(log, "{} {line}", started.elapsed().as_micros()) {
                    found.failure = Some(format!("could not preserve app log: {error}"));
                }
                if found.startup_ms.is_none() && line.contains("perf_graph_frame") {
                    found.startup_ms = Some(started.elapsed().as_millis() as u64);
                }
                if found.graph_ms.is_none() && graph_finished(&line) {
                    found.graph_ms = Some(started.elapsed().as_millis() as u64);
                }
                // On the parent's clock, like the two above: the sampler
                // the walk is weighed against runs on it.
                if let Some(mark) = super::fonts::mark(&line) {
                    let at_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
                    found
                        .font_walk
                        .get_or_insert_with(Default::default)
                        .note(mark, at_us);
                }
                if line.contains("perf_failed") {
                    found.failure = Some(line.clone());
                }
                if line.contains("perf_scroll_begin") {
                    watched.begin();
                }
                if line.contains("scroll_bench") {
                    watched
                        .ended
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                absorb(&line, &mut found);
                // A build with no harness never says `perf_done`; the
                // graph finishing its stream is what ends it.
                let ended = if harness {
                    line.contains("perf_done")
                } else {
                    graph_finished(&line)
                };
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
    (done_rx, scroll, reader)
}

/// Refuses a reading that lost a number this run was asked to take. The
/// app's log picking up colour loses every `key=value` at once
/// (`platitude_gg::init_tracing`).
pub(super) fn missing(reading: &Reading, opts: &Options) -> Result<(), String> {
    super::interactions::validate(reading, opts)?;
    let mut gaps = Vec::new();
    if reading.peak_working_set == 0 || reading.peak_private == 0 {
        gaps.push("nonzero process memory samples");
    }
    // Asked of both builds alike: the walk needs nothing from the process.
    if opts.attribute && reading.attribution.is_none() {
        gaps.push("the memory attribution (attribution.txt in the run says what the walk said)");
    }
    if !opts.harness {
        if reading.graph_ms.is_none() {
            gaps.push("a finished graph (no `graph stream finished`)");
        }
        if reading.first_chunk_ms.is_none() {
            gaps.push("the walk (no `first_chunk_ms=`)");
        }
        return if gaps.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "the shipped build ended without {} — it reports no `perf_*` line of its own, so \
                 these ordinary log lines are the whole of the reading",
                gaps.join(", ")
            ))
        };
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
    gaps.extend(font_walk_gap(reading, opts));
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
    gaps.extend(scenario_gaps(reading, opts));
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

/// What the scenario this run was asked to drive had to say about itself,
/// both ways round: the numbers a page holds are the ones its scenario
/// asked for.
fn scenario_gaps(reading: &Reading, opts: &Options) -> Vec<&'static str> {
    let expected = format!(
        "selection={} details={} diff={} graph={} scrolled={}",
        opts.selection,
        opts.select,
        opts.select && opts.diff,
        opts.scroll || !opts.select || !opts.diff,
        opts.scroll
    );
    let mut gaps = Vec::new();
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
    if opts.select && opts.diff && (reading.diff_frame_ms.is_empty() || reading.diff_ms.is_empty())
    {
        gaps.push("the requested diff (round trip and frame)");
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
    gaps
}

/// What the calibration run is missing when it lost its one number. Asked
/// of both its shapes (with a repository and without) and of no other run.
fn font_walk_gap(reading: &Reading, opts: &Options) -> Option<&'static str> {
    let weighed = reading
        .font_walk
        .as_ref()
        .is_some_and(super::fonts::FontWalk::weighed);
    (opts.font_walk && !weighed).then_some(
        "the font walk, said in order and sampled either side (`perf_font_walk_begin` / \
         `done` / `settled`, with a memory tick before the first and before the last)",
    )
}

/// The line every build says when the graph has finished streaming
/// (`Reading::graph_ms`).
pub(super) fn graph_finished(line: &str) -> bool {
    line.contains("graph stream finished") || line.contains("graph replaced in place")
}

/// What Qt says about the device it is drawing on and the rate it thinks
/// it has, under `QSG_INFO=1`. Kept verbatim: two runs on different
/// adapters are not each other's control.
fn graphics_note(line: &str) -> bool {
    [
        "Creating QRhi with backend",
        "Adapter ",
        "using this adapter",
        "using vsync:",
        // The software scene graph (`--software`), which names no adapter.
        "Loading backend",
    ]
    .iter()
    .any(|mark| line.contains(mark))
}

pub(super) fn absorb(line: &str, found: &mut Reading) {
    if let Some(event) = super::interactions::read(line) {
        found.events.push(event);
    }
    let diff_surface = token(line, "surface=").is_some_and(|v| v.trim_matches('"') == "diff");
    if line.contains("scroll_bench") && diff_surface {
        found.diff_scrolls.push(line.to_owned());
    }
    if line.contains("perf_frame ") && !diff_surface {
        found.traced_frames += 1;
    }
    if line.contains("perf_done") {
        found.perf_done = true;
    }
    if graphics_note(line) && found.graphics.len() < 12 {
        // Qt says it once per window, naming the window by an address new
        // every process; cut off, the facts compare across runs.
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
    interaction_marks(line, found);
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
    if line.contains("scroll_bench") && !diff_surface {
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
/// The marks one driven point leaves, in the order the wait divides:
/// the app says when the answer arrived and when the rows of it were in
/// the model, and the harness says when the frame that drew them came.
fn interaction_marks(line: &str, found: &mut Reading) {
    for (mark, into) in [
        ("details request round trip", &mut found.details_ms),
        ("details rows applied", &mut found.details_applied_ms),
        ("diff request round trip", &mut found.diff_ms),
        ("diff rows applied", &mut found.diff_applied_ms),
    ] {
        if line.contains(mark)
            && let Some(v) = field(line, "elapsed_ms=")
            && let Ok(ms) = v.parse()
        {
            into.push(ms);
        }
    }
    for (mark, into) in [
        ("perf_details_frame", &mut found.details_frame_ms),
        ("perf_diff_frame", &mut found.diff_frame_ms),
    ] {
        if line.contains(mark)
            && let Some(ms) = field(line, "elapsed_ms=").and_then(|v| v.parse().ok())
        {
            into.push(ms);
        }
    }
}

/// One `key=value` off a line whose keys are whole words: the tables the
/// runner writes for itself (`perf::display`, `perf::sampler`). Not
/// [`field`], which finds the key anywhere: `y=` on a line carrying
/// `primary=false` answers `false`.
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
