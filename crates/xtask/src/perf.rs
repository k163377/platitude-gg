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

mod sampler;
#[cfg(test)]
mod tests;

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sampler::{sample_memory, sample_once};

const SAMPLE_MS: u64 = 100;

struct Options {
    repo: PathBuf,
    label: String,
    runs: u32,
    watchdog_ms: u64,
    scroll: bool,
    select: bool,
    breakdown: bool,
    build: bool,
    /// Start with no repository at all — the window and nothing in it.
    /// What it is for: subtracting this from a run that opened an empty
    /// repository leaves the cost of putting the page up, which is
    /// otherwise indistinguishable from the toolkit's own floor.
    open: bool,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        repo: PathBuf::new(),
        label: String::new(),
        runs: 3,
        watchdog_ms: 300_000,
        scroll: true,
        select: true,
        breakdown: false,
        build: true,
        open: true,
    };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let mut value = || {
            i += 1;
            args.get(i)
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg {
            "--repo" => opts.repo = PathBuf::from(value()?),
            "--label" => opts.label = value()?,
            "--runs" => {
                opts.runs = value()?
                    .parse()
                    .map_err(|_| "--runs takes a number".to_string())?;
            }
            "--watchdog-ms" => {
                opts.watchdog_ms = value()?
                    .parse()
                    .map_err(|_| "--watchdog-ms takes a number".to_string())?;
            }
            "--quit-ms" => return Err(
                "--quit-ms is not supported; use --watchdog-ms as the outer hang ceiling"
                    .into(),
            ),
            "--no-scroll" => opts.scroll = false,
            "--no-select" => opts.select = false,
            "--no-open" => opts.open = false,
            "--breakdown" => opts.breakdown = true,
            "--no-build" => opts.build = false,
            other => return Err(format!("unknown option: {other}")),
        }
        i += 1;
    }
    if opts.repo.as_os_str().is_empty() && opts.open {
        return Err("--repo <path> is required (or --no-open for the bare window)".into());
    }
    if opts.label.is_empty() {
        opts.label = opts
            .repo
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "no repository".to_string());
    }
    Ok(opts)
}

/// What one run reported.
#[derive(Default, Clone)]
struct Reading {
    peak_working_set: u64,
    peak_private: u64,
    /// Process start to the first rows being on the model — the budget's
    /// "startup to the graph's first display".
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
    /// The `mem report` line with the largest `rust_live`, verbatim.
    breakdown: Option<String>,
    breakdown_live: u64,
    perf_done: bool,
}
pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let root = crate::workspace_root();
    let path = crate::qt::path_with_qt()?;
    guard_the_window(&root)?;

    let mut extra = vec!["-p", "platitude-app"];
    if opts.breakdown {
        extra.extend(["--features", "memprobe"]);
    }
    let exe = crate::app_exe(&root, &path, opts.build, &extra)?;

    println!(
        "repo: {} | runs: {} (the first is discarded — cold cache)",
        opts.repo.display(),
        opts.runs
    );
    let mut kept: Vec<Reading> = Vec::new();
    for run in 0..=opts.runs {
        let reading = measure(&exe, &path, &root, &opts)?;
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
fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}
fn report(opts: &Options, kept: &[Reading]) {
    if kept.is_empty() {
        return;
    }
    let ws: Vec<f64> = kept.iter().map(|r| mb(r.peak_working_set)).collect();
    let private: Vec<f64> = kept.iter().map(|r| mb(r.peak_private)).collect();
    println!("\n== {} ==", opts.label);
    println!("  working set : {}", spread(&ws));
    println!("  private     : {}", spread(&private));
    let startups: Vec<f64> = kept.iter().filter_map(|r| r.startup_ms).map(f).collect();
    if !startups.is_empty() {
        println!(
            "  startup     : {} ms (to the first rows)",
            spread(&startups)
        );
    }
    let firsts: Vec<f64> = kept
        .iter()
        .filter_map(|r| r.first_chunk_ms)
        .map(f)
        .collect();
    if !firsts.is_empty() {
        println!("  of which walk: {} ms", spread(&firsts));
    }
    let fps: Vec<f64> = kept.iter().filter_map(|r| r.fps).collect();
    if !fps.is_empty() {
        println!("  scroll      : {} fps", spread(&fps));
    }
    let details: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.details_ms.clone())
        .map(f)
        .collect();
    if !details.is_empty() {
        println!("  details     : {} ms", spread(&details));
    }
    if let Some(line) = kept
        .iter()
        .max_by_key(|r| r.breakdown_live)
        .and_then(|r| r.breakdown.clone())
    {
        println!("\n  breakdown at the largest Rust heap of the kept runs:\n    {line}");
    }
}

fn f(v: u64) -> f64 {
    v as f64
}

/// `min–max` over the readings, or the single value when they agree.
fn spread(values: &[f64]) -> String {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    match (sorted.first(), sorted.last()) {
        (Some(lo), Some(hi)) if (hi - lo).abs() < 0.05 => format!("{lo:.1}"),
        (Some(lo), Some(hi)) => format!("{lo:.1}–{hi:.1}"),
        _ => "-".into(),
    }
}
fn measure(
    exe: &std::path::Path,
    path: &std::ffi::OsString,
    root: &std::path::Path,
    opts: &Options,
) -> Result<Reading, String> {
    // A config directory per process, so another perf process or a previous
    // run's restored state cannot decide what this one does.
    let config_dir = std::env::temp_dir().join(format!("pg-perf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&config_dir);
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
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
    let (done_rx, reader) = read_app(stderr, started);

    let deadline = started + Duration::from_millis(opts.watchdog_ms);
    let sampler = sample_memory(pid, deadline);
    // `perf_done`, not elapsed time, is the success edge. The deadline is
    // only an outer diagnostic guard for an app that stopped answering.
    let mut done = false;
    let mut exited = false;
    let mut timed_out = false;
    let mut wait_error = None;
    loop {
        if done_rx.try_recv().is_ok() {
            done = true;
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
    let final_sample = if done { sample_once(pid) } else { (0, 0) };
    let _ = child.kill();
    let _ = child.wait();
    let mut reading = reader.join().unwrap_or_default();
    let (ws, private) = sampler.join().unwrap_or_default();
    reading.perf_done |= done;
    reading.peak_working_set = ws.max(final_sample.0);
    reading.peak_private = private.max(final_sample.1);
    let _ = std::fs::remove_dir_all(&config_dir);
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

fn read_app(
    stderr: Option<std::process::ChildStderr>,
    started: Instant,
) -> (mpsc::Receiver<()>, std::thread::JoinHandle<Reading>) {
    let (done_tx, done_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut found = Reading::default();
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if found.startup_ms.is_none() && line.contains("graph first chunk") {
                    found.startup_ms = Some(started.elapsed().as_millis() as u64);
                }
                if line.contains("perf_done") {
                    let _ = done_tx.send(());
                }
                absorb(&line, &mut found);
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
        gaps.push("startup (no `graph first chunk`)");
    }
    if reading.first_chunk_ms.is_none() {
        gaps.push("the walk (no `first_chunk_ms=`)");
    }
    if reading.total_ms.is_none() {
        gaps.push("the finished graph (no `elapsed_ms=`)");
    }
    if opts.select && reading.details_ms.is_empty() {
        gaps.push("the interaction (no `details request round trip`)");
    }
    if opts.scroll && reading.fps.is_none() {
        gaps.push("fps (no `scroll_bench fps=`)");
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
    if let Some(rest) = line.split("scroll_bench fps=").nth(1) {
        found.fps = rest.split_whitespace().next().and_then(|v| v.parse().ok());
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
