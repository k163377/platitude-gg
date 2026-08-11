//! `cargo xtask perf` — the memory / startup / fps measurement, repeated
//! the same way every time.
//!
//! The record in `ci/baseline/perf-windows-x64.md` is only worth anything
//! if the run behind it can be repeated exactly, so the conditions live
//! here rather than in whatever shell somebody typed that day: release
//! build, a **real window** (offscreen reports neither memory nor fps
//! honestly), warm cache, the `PG_AUTO_*` hooks, WorkingSet sampled every
//! 100ms for its maximum, and a deadline with a kill guard — a run that
//! meets a locked screen must end by itself.
//!
//! `--breakdown` adds `PG_MEM_REPORT=1` and prints the largest `mem
//! report` line the run produced, which is what says *where* the bytes
//! are. That needs a binary built with the `memprobe` feature; without it
//! the line still comes, with `counted=false` and no Rust-heap total.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How often the process is asked how big it is.
const SAMPLE_MS: u64 = 100;

/// Grace on top of `PG_AUTO_QUIT_MS` before the run is killed.
const GRACE_MS: u64 = 25_000;

struct Options {
    repo: PathBuf,
    label: String,
    runs: u32,
    quit_ms: u64,
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
        quit_ms: 40_000,
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
            "--quit-ms" => {
                opts.quit_ms = value()?
                    .parse()
                    .map_err(|_| "--quit-ms takes a number".to_string())?;
            }
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
}

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let root = crate::workspace_root();
    let path = crate::qt::path_with_qt()?;
    guard_the_window(&root)?;

    if opts.build {
        println!("building (release{})…", feature_note(opts.breakdown));
        let mut cmd = Command::new("cargo");
        cmd.args(["build", "--release", "-p", "platitude-app"]);
        if opts.breakdown {
            cmd.args(["--features", "memprobe"]);
        }
        let status = cmd
            .current_dir(&root)
            .env("PATH", &path)
            .status()
            .map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build --release failed".into());
        }
    }

    let exe = root.join("target").join("release").join(if cfg!(windows) {
        "platitude-gg.exe"
    } else {
        "platitude-gg"
    });
    if !exe.is_file() {
        return Err(format!("{} not found — build first", exe.display()));
    }

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
/// `cargo xtask hook pre-shell` cannot see this one — it names no binary,
/// the way `verify-ui` names none — so the rule it enforces is written here
/// instead of being quietly skipped: a window put up from a worktree covers
/// whatever is on the screen and lands in the middle of another session's
/// grab. A person asking for this measurement in so many words is the
/// exception the hook already spells, and `PG_ALLOW_GUI=1` is how they say
/// it (CLAUDE.md ビルド・テスト).
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

fn feature_note(breakdown: bool) -> &'static str {
    if breakdown { ", memprobe" } else { "" }
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
    // A config directory per run, so nothing a previous run remembered
    // (restored tabs, a window size) decides what this one does.
    let config_dir = std::env::temp_dir().join(format!("pg-perf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&config_dir);
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(exe);
    cmd.current_dir(root)
        .env("PATH", path)
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PG_CONFIG_DIR", &config_dir)
        .env("PG_LOG", "info")
        .env("PG_AUTO_QUIT_MS", opts.quit_ms.to_string())
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

    let reader = std::thread::spawn(move || {
        let mut found = Reading::default();
        if let Some(pipe) = stderr {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if found.startup_ms.is_none() && line.contains("graph first chunk") {
                    found.startup_ms = Some(started.elapsed().as_millis() as u64);
                }
                absorb(&line, &mut found);
            }
        }
        found
    });

    let deadline = started + Duration::from_millis(opts.quit_ms + GRACE_MS);
    let sampler = sample_memory(pid, deadline);

    // Bounded wait with a kill guard — never an unbounded one.
    let mut killed = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => return Err(format!("waiting on the app failed: {e}")),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            killed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(SAMPLE_MS));
    }
    let _ = child.wait();

    let mut reading = reader.join().unwrap_or_default();
    let (ws, private) = sampler.join().unwrap_or_default();
    reading.peak_working_set = ws;
    reading.peak_private = private;
    let _ = std::fs::remove_dir_all(&config_dir);
    if killed {
        return Err(format!(
            "the run did not end within {}ms and was killed — the reading is not usable",
            opts.quit_ms + GRACE_MS
        ));
    }
    missing(&reading, opts)?;
    Ok(reading)
}

/// Refuses a reading that lost a number this run was asked to take.
///
/// A gap used to print as `-` and the report simply left the row out, so
/// a run that measured nothing looked like a run that measured well. That
/// is how the app's log picking up colour went unnoticed until the
/// interaction budget needed re-measuring: every `key=value` in it stopped
/// being findable, and three of the five numbers quietly became `-`
/// (`platitude_gg::init_tracing`).
fn missing(reading: &Reading, opts: &Options) -> Result<(), String> {
    // The bare window (`--no-open`) has no repository, so it has no graph
    // to walk, no row to select and nothing to scroll: it takes the memory
    // floor and nothing else.
    if !opts.open {
        return Ok(());
    }
    let mut gaps = Vec::new();
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

/// Picks the numbers this measurement is about out of one stderr line.
fn absorb(line: &str, found: &mut Reading) {
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

/// The value of `key` in a `tracing` line, up to the next space.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split(key)
        .nth(1)
        .map(|rest| rest.split_whitespace().next().unwrap_or(rest))
}

/// Samples the process every [`SAMPLE_MS`] and returns
/// `(peak working set, peak private)` in bytes.
///
/// Two implementations because there is no portable way to ask: Windows
/// has one PowerShell loop for the whole run (spawning one per sample
/// would cost more than the thing being measured), Linux reads `/proc`
/// directly, and anything else reports nothing rather than a guess.
fn sample_memory(pid: u32, deadline: Instant) -> std::thread::JoinHandle<(u64, u64)> {
    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            windows_sampler(pid, deadline)
        }
        #[cfg(target_os = "linux")]
        {
            linux_sampler(pid, deadline)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = (pid, deadline);
            (0, 0)
        }
    })
}

#[cfg(windows)]
fn windows_sampler(pid: u32, deadline: Instant) -> (u64, u64) {
    let seconds = deadline.saturating_duration_since(Instant::now()).as_secs() + 5;
    // `Refresh()` is what makes a held Process object re-read its counters;
    // without it the loop would report the first sample forever.
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue';\
         $p=Get-Process -Id {pid};\
         $ws=0;$pv=0;$end=(Get-Date).AddSeconds({seconds});\
         while($p -ne $null -and -not $p.HasExited -and (Get-Date) -lt $end){{\
           $p.Refresh();\
           if($p.WorkingSet64 -gt $ws){{$ws=$p.WorkingSet64}};\
           if($p.PrivateMemorySize64 -gt $pv){{$pv=$p.PrivateMemorySize64}};\
           Start-Sleep -Milliseconds {SAMPLE_MS};\
         }};\
         Write-Output \"$ws $pv\""
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();
    let Ok(out) = out else { return (0, 0) };
    parse_pair(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(target_os = "linux")]
fn linux_sampler(pid: u32, deadline: Instant) -> (u64, u64) {
    let status = format!("/proc/{pid}/status");
    let (mut ws, mut pv) = (0u64, 0u64);
    while Instant::now() < deadline {
        let Ok(text) = std::fs::read_to_string(&status) else {
            break; // the process is gone
        };
        for line in text.lines() {
            let kb = |l: &str| -> u64 {
                l.split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0)
                    * 1024
            };
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                ws = ws.max(kb(rest));
            } else if let Some(rest) = line.strip_prefix("VmData:") {
                pv = pv.max(kb(rest));
            }
        }
        std::thread::sleep(Duration::from_millis(SAMPLE_MS));
    }
    (ws, pv)
}

/// `"123 456"` → `(123, 456)`; anything else → zeros.
fn parse_pair(text: &str) -> (u64, u64) {
    let mut numbers = text.split_whitespace().filter_map(|v| v.parse().ok());
    (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tracing_field_reads_up_to_the_next_space() {
        let line = "INFO first_chunk_ms=873 graph first chunk";
        assert_eq!(field(line, "first_chunk_ms="), Some("873"));
        assert_eq!(field(line, "missing="), None);
    }

    #[test]
    fn the_bench_line_gives_its_fps() {
        let mut found = Reading::default();
        absorb("INFO report: scroll_bench fps=178.3 rows=2000", &mut found);
        assert_eq!(found.fps, Some(178.3));
    }

    #[test]
    fn the_largest_heap_is_the_breakdown_that_is_kept() {
        let mut found = Reading::default();
        absorb("mem report rust_live=100 models=a", &mut found);
        absorb("mem report rust_live=900 models=b", &mut found);
        absorb("mem report rust_live=300 models=c", &mut found);
        assert_eq!(found.breakdown_live, 900);
        assert!(found.breakdown.is_some_and(|l| l.contains("models=b")));
    }

    #[test]
    fn a_spread_of_one_value_is_printed_once() {
        assert_eq!(spread(&[3.0, 3.0]), "3.0");
        assert_eq!(spread(&[3.0, 5.0]), "3.0–5.0");
        assert_eq!(spread(&[]), "-");
    }

    #[test]
    fn the_sampler_pair_survives_junk() {
        assert_eq!(parse_pair("123 456\r\n"), (123, 456));
        assert_eq!(parse_pair(""), (0, 0));
    }
}
