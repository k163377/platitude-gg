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
mod measure;
mod options;
mod reading;
mod report;
mod sampler;

use measure::measure;
use options::{Options, parse};
use reading::Reading;
use report::{mb, report};

/// How often the samplers look, and how often the wait in
/// [`measure`] comes round.
const SAMPLE_MS: u64 = 100;

pub fn run(args: &[String]) -> Result<(), String> {
    // The samplers answer (0, 0) where they are not implemented, and a
    // 0MB working set must never pass for a reading.
    if !cfg!(any(windows, target_os = "linux")) {
        return Err("memory sampling is not implemented for this OS".into());
    }
    let opts = parse(args)?;
    let root = crate::tree::workspace_root();
    let path = crate::qt::path_with_qt()?;
    guard_the_window(&root)?;

    let mut extra = vec!["-p", "platitude-app"];
    if opts.breakdown {
        extra.extend(["--features", "memprobe"]);
    }
    let exe = crate::tree::app_exe(&root, &path, opts.build, &extra)?;
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
