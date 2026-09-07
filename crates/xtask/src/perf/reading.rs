//! What one measured run reported, and whether it reported enough.
//!
//! The app says its numbers on stderr as `key=value` (`platitude_gg::init_tracing`),
//! so a reading is assembled a line at a time as the run talks, and refused as a
//! whole if a number this run was asked to take never arrived.

use std::io::Write;
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
    /// Process start to the graph stream saying it finished — a data
    /// event, one frame short of anything being on screen.
    ///
    /// Why it is here beside [`Reading::startup_ms`]: this line is
    /// ordinary application logging, so it is the one startup number a
    /// build with no harness in it can also answer. It is what makes the
    /// two builds comparable at all (`Options::harness`).
    pub(super) graph_ms: Option<u64>,
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
    /// What the settled process held, read from outside it under
    /// `--attribute`; the whole text is `attribution.txt` in the run.
    pub(super) attribution: Option<super::attribution::Attribution>,
    /// The font walk the run was asked to pay before `perf_done`, as its
    /// three lines arrived, and — once `measure` has read the sampler
    /// back at them — what the process weighed either side
    /// (`perf::fonts`). `None` where the run never said the lines.
    pub(super) font_walk: Option<super::fonts::FontWalk>,
    pub(super) perf_done: bool,
}

/// What the parent watches while the app talks: whether the scroll bench
/// has begun, and whether it has ended.
///
/// The bench is the one phase with a deadline of its own. It is driven by
/// an animation, the animation is advanced by the render loop, and a
/// window nothing is drawing — covered by another window, or behind a
/// locked session — advances neither. Nothing then arrives until the
/// outer watchdog fires minutes later, which says "the app stopped
/// answering" about a machine that simply covered it up.
#[derive(Default)]
pub(super) struct Scroll {
    began: std::sync::atomic::AtomicBool,
    ended: std::sync::atomic::AtomicBool,
}

impl Scroll {
    /// Whether the bench has begun and not yet ended: the stretch a frame
    /// is owed in, under a deadline of the bench's own (`measure`).
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
                // the walk is read back against runs on that clock.
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
                    watched
                        .began
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if line.contains("scroll_bench") {
                    watched
                        .ended
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                absorb(&line, &mut found);
                // A build with no harness never says `perf_done`, so the
                // graph having finished streaming is the whole of what
                // ends it: the last thing it says that this measurement
                // was waiting for.
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

/// Refuses a reading that lost a number this run was asked to take: a
/// run that measured nothing must not read as one that measured well.
/// The app's log picking up colour is one way to lose every `key=value`
/// at once (`platitude_gg::init_tracing`).
pub(super) fn missing(reading: &Reading, opts: &Options) -> Result<(), String> {
    let mut gaps = Vec::new();
    if reading.peak_working_set == 0 || reading.peak_private == 0 {
        gaps.push("nonzero process memory samples");
    }
    // Asked of both builds alike: the walk needs nothing from the process.
    if opts.attribute && reading.attribution.is_none() {
        gaps.push("the memory attribution (attribution.txt in the run says what the walk said)");
    }
    if !opts.harness {
        // The whole of what a build with no harness can be asked for.
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

/// What the calibration run is missing when it lost its one number: the
/// walk, said in order and sampled either side. Asked of both its
/// shapes — with a repository and without one — and of no other run.
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

/// The line every build says when the graph has finished streaming —
/// ordinary application logging, not the harness, which is what makes it
/// the one edge the two builds share (`Reading::graph_ms`).
pub(super) fn graph_finished(line: &str) -> bool {
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
        // The software scene graph, which `--software` draws with
        // (`measure::command`): it names no adapter, and two runs that
        // drew with different scene graphs are not each other's control
        // either.
        "Loading backend",
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
