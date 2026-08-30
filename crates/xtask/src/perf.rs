//! `cargo xtask perf` — the memory / startup / fps measurement, repeated
//! the same way every time.
//!
//! The record in `ci/baseline/perf-windows-x64.md` is only worth anything
//! if the run behind it can be repeated exactly, so the conditions live
//! here rather than in whatever shell somebody typed that day: release
//! build, a **real window** (offscreen reports neither memory nor fps
//! honestly), warm cache, the `PG_AUTO_*` hooks, WorkingSet sampled every
//! 100ms for its maximum, causal `perf_done`, and an outer kill guard.
//!
//! `--breakdown` adds `PG_MEM_REPORT=1` and prints the largest `mem
//! report` line the run produced, which is what says *where* the bytes
//! are. That needs a binary built with the `memprobe` feature; without it
//! the line still comes, with `counted=false` and no Rust-heap total.

mod artifacts;
mod display;
mod options;
mod report;
mod sampler;
#[cfg(test)]
mod tests;

use options::{Options, parse};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use report::{mb, report};
use sampler::{sample_last, sample_memory, sample_once};

const SAMPLE_MS: u64 = 100;

/// What one run reported.
#[derive(Default, Clone, Debug)]
struct Reading {
    peak_working_set: u64,
    peak_private: u64,
    /// What the process still held after `--settle-ms`, or 0 where the
    /// run was not asked to wait. Read beside the peak: the difference
    /// between them is work the process let go of once it was idle.
    ///
    /// The working set alone, because that is the side the budget is read
    /// against (ci/baseline/perf-windows-x64.md §判定).
    settled_working_set: u64,
    /// Process start to the first frame of the visible graph.
    ///
    /// Timed here rather than taken off the app's own `first_chunk_ms`,
    /// which starts counting when the walk starts and so leaves out
    /// everything before it: the runtime, the window, the QML engine and
    /// opening the repository. Those are most of what a person waits for.
    startup_ms: Option<u64>,
    first_chunk_ms: Option<u64>,
    total_ms: Option<u64>,
    fps: Option<f64>,
    details_ms: Vec<u64>,
    details_frame_ms: Vec<f64>,
    diff_frame_ms: Vec<f64>,
    frame_p95_ms: Option<f64>,
    frame_p99_ms: Option<f64>,
    frame_max_ms: Option<f64>,
    frame_count: Option<usize>,
    traced_frames: usize,
    scroll_visible: bool,
    scroll_framed: bool,
    selection: Option<String>,
    scenario_complete: Option<String>,
    rows: Option<usize>,
    failure: Option<String>,
    /// The `mem report` line with the largest `rust_live`, verbatim.
    breakdown: Option<String>,
    breakdown_live: u64,
    perf_done: bool,
}
pub fn run(args: &[String]) -> Result<(), String> {
    // The samplers answer (0, 0) where they are not implemented, and a
    // 0MB working set must never pass for a reading.
    if !cfg!(any(windows, target_os = "linux")) {
        return Err("memory sampling is not implemented for this OS".into());
    }
    let opts = parse(args)?;
    let root = crate::workspace_root();
    let path = crate::qt::path_with_qt()?;
    guard_the_window(&root)?;

    let mut extra = vec!["-p", "platitude-app"];
    if opts.breakdown {
        extra.extend(["--features", "memprobe"]);
    }
    let exe = crate::app_exe(&root, &path, opts.build, &extra)?;
    let output = artifacts::prepare(&root, &exe, &opts)?;
    println!("evidence: {}", output.display());

    println!(
        "repo: {} | runs: {} (the first is discarded — cold cache)",
        opts.repo.display(),
        opts.runs
    );
    let mut kept: Vec<Reading> = Vec::new();
    for run in 0..=opts.runs {
        let run_dir = output.join(format!("run-{run}"));
        std::fs::create_dir(&run_dir).map_err(|e| e.to_string())?;
        display::capture(&run_dir, "before")?;
        let result = measure(&exe, &path, &root, &opts, &run_dir);
        std::fs::write(run_dir.join("result.txt"), format!("{result:#?}"))
            .map_err(|e| e.to_string())?;
        display::capture(&run_dir, "after")?;
        let reading = result?;
        let discarded = run == 0;
        println!(
            "  run {}{}: ws={:.1}MB private={:.1}MB startup={} walk={} fps={}",
            run,
            if discarded { " (discarded)" } else { "" },
            mb(reading.peak_working_set),
            mb(reading.peak_private),
            reading.startup_ms.map_or("-".into(), |v| v.to_string()),
            reading.total_ms.map_or("-".into(), |v| v.to_string()),
            reading.fps.map_or("-".into(), |v| format!("{v:.1}")),
        );
        if !discarded {
            kept.push(reading);
        }
    }
    report(&opts, &kept);
    Ok(())
}

/// The one command in the task runner that opens a real window.
///
/// `cargo xtask hook pre-shell` cannot see this one because it names no app
/// binary. A worktree window can overlap another session's screenshot, so
/// the explicit `PG_ALLOW_GUI=1` remains the authorization boundary.
fn guard_the_window(root: &std::path::Path) -> Result<(), String> {
    let in_worktree = root
        .to_string_lossy()
        .replace('\\', "/")
        .contains("/.claude/worktrees/");
    if !in_worktree || std::env::var("PG_ALLOW_GUI").as_deref() == Ok("1") {
        return Ok(());
    }
    Err(
        "this measurement needs a real window, and a worktree may not put one up on its own \
         (offscreen reports neither memory nor fps honestly, so there is no headless form of \
         it). Run it again with PG_ALLOW_GUI=1 when the window was asked for."
            .into(),
    )
}
fn measure(
    exe: &std::path::Path,
    path: &std::ffi::OsString,
    root: &std::path::Path,
    opts: &Options,
    run_dir: &std::path::Path,
) -> Result<Reading, String> {
    // A config directory per process, so another perf process or a previous
    // run's restored state cannot decide what this one does.
    let (config_dir, log, samples) = artifacts::open_run(run_dir)?;
    let trace_frames = if opts.trace_frames { "1" } else { "0" };
    let mut cmd = Command::new(exe);
    crate::app_env::clear_automation(&mut cmd);
    cmd.current_dir(root)
        .env("PATH", path)
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PG_CONFIG_DIR", &config_dir)
        .env("PG_LOG", "info")
        // The app reports completion only after every requested measurement
        // has answered. The parent owns termination so it can take the last
        // process-memory sample and reap exactly the child it started.
        .env("PG_AUTO_PERF", "1")
        .env("PG_PERF_SELECTION", &opts.selection)
        .env("PG_PERF_OID", &opts.oid)
        .env("PG_PERF_FILE", &opts.file)
        .env("PG_PERF_DIFF", if opts.diff { "1" } else { "0" })
        .env("PG_PERF_TRACE_FRAMES", trace_frames)
        // A real window, deliberately: `verify-ui` runs offscreen, and
        // offscreen Qt builds no scene graph worth measuring.
        .env_remove("QT_QPA_PLATFORM")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if opts.open {
        cmd.env("PG_AUTO_OPEN", &opts.repo);
    }
    if opts.select {
        cmd.env("PG_AUTO_SELECT", "1");
    }
    if opts.scroll {
        cmd.env("PG_AUTO_SCROLL", "1");
    }
    if opts.breakdown {
        cmd.env("PG_MEM_REPORT", "1");
    }
    let started = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    let pid = child.id();
    let stderr = child.stderr.take();
    let (done_rx, reader) = read_app(stderr, started, log);

    let deadline = started + Duration::from_millis(opts.watchdog_ms);
    let sampling_end = deadline + Duration::from_millis(opts.settle_ms);
    let sampler = sample_memory(pid, sampling_end, started, samples);
    // `perf_done`, not elapsed time, is the success edge. The deadline is
    // only an outer diagnostic guard for an app that stopped answering.
    let mut done = false;
    let mut exited = false;
    let mut timed_out = false;
    let mut wait_error = None;
    loop {
        if let Ok(success) = done_rx.try_recv() {
            done = success;
            break;
        }
        match child.try_wait() {
            Ok(Some(_)) => {
                exited = true;
                break;
            }
            Ok(None) => {}
            Err(e) => {
                wait_error = Some(format!("waiting on the app failed: {e}"));
                break;
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(SAMPLE_MS));
    }
    // Held idle first when the run asked for it, so the last reading is
    // taken of a process that has stopped working rather than one caught
    // mid-frame. The background sampler is still running, so whatever the
    // hold costs still reaches the peak.
    let settled = if done && opts.settle_ms > 0 {
        hold_idle(pid, opts.settle_ms)
    } else {
        0
    };
    let final_sample = if done { sample_once(pid) } else { (0, 0) };
    let _ = child.kill();
    let _ = child.wait();
    let mut reading = reader.join().unwrap_or_default();
    let (ws, private) = sampler
        .join()
        .map_err(|_| "memory sampler panicked".to_string())??;
    reading.perf_done |= done;
    reading.peak_working_set = ws.max(final_sample.0);
    reading.peak_private = private.max(final_sample.1);
    reading.settled_working_set = settled;
    if let Some(error) = &reading.failure {
        return Err(error.clone());
    }
    if let Some(error) = wait_error {
        return Err(error);
    }
    if timed_out {
        return Err(format!(
            "the run did not report perf_done within {}ms and was killed — the reading is not usable",
            opts.watchdog_ms
        ));
    }
    if exited && !done {
        return Err("the app exited before reporting perf_done — the reading is not usable".into());
    }
    missing(&reading, opts)?;
    Ok(reading)
}

/// Leaves the app alone for `ms` and answers with the last working set
/// read off it. Sampled the whole way rather than once at the end so a
/// run that is still settling shows up as a value that is still moving.
///
/// **Outside the watchdog on purpose.** The ceiling is there to end a run
/// that stopped answering, and this hold begins after the run has already
/// answered — a wait the caller asked for by the second, not one the app
/// could stretch.
fn hold_idle(pid: u32, ms: u64) -> u64 {
    sample_last(pid, ms)
}

fn read_app(
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
                let completion = if found.failure.is_some() {
                    Some(false)
                } else if line.contains("perf_done") {
                    Some(true)
                } else {
                    None
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
fn missing(reading: &Reading, opts: &Options) -> Result<(), String> {
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
fn absorb(line: &str, found: &mut Reading) {
    if line.contains("perf_frame ") {
        found.traced_frames += 1;
    }
    if line.contains("perf_done") {
        found.perf_done = true;
    }
    if let Some(v) = field(line, "first_chunk_ms=") {
        found.first_chunk_ms = v.parse().ok();
    }
    if (line.contains("graph stream finished") || line.contains("graph replaced in place"))
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
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split(key)
        .nth(1)
        .map(|rest| rest.split_whitespace().next().unwrap_or(rest))
}
