//! `cargo xtask perf` — the memory / startup / fps measurement, repeated
//! the same way every time.
//!
//! The record in `ci/baseline/perf-windows-x64.md` is only worth anything
//! if the run behind it can be repeated exactly, so the conditions live
//! here rather than in whatever shell somebody typed that day: release
//! build, a **real window** (offscreen reports neither memory nor fps
//! honestly), **on one named screen**, warm cache, the `PG_AUTO_*` hooks,
//! WorkingSet sampled every 100ms for its maximum, causal `perf_done`,
//! and an outer kill guard.
//!
//! **Three of those conditions are the machine, not the application.** A
//! window somebody covered, a session somebody locked, a screen that went
//! to sleep and a parallel build all answer with numbers that read
//! exactly like a slower application, so the run measures them too and
//! refuses rather than publishes (`perf::sampler::Conditions`). The
//! benchmark repository is the fourth: it is a live clone, and a `git
//! fetch` changes the rows, the ref tables and the commit the interaction
//! opens while `HEAD` holds still (`perf::corpus`).
//!
//! `--breakdown` adds `PG_MEM_REPORT=1` and prints the largest `mem
//! report` line the run produced, which is what says *where* the bytes
//! are. That needs a binary built with the `memprobe` feature; without it
//! the line still comes, with `counted=false` and no Rust-heap total.
//!
//! `--shipped` measures the other build — `cargo build --release` with no
//! features, which is what a person installs. It answers memory and the
//! time to a finished graph and nothing else, because every `perf_*` line
//! belongs to the harness the shipped build leaves out; run it beside the
//! ordinary one to say what carrying the harness costs.

mod artifacts;
mod corpus;
mod display;
mod measure;
mod options;
mod reading;
mod report;
mod sampler;

use std::time::Duration;

use measure::measure;
use options::{Options, parse};
use reading::Reading;
use report::{mb, report};

/// How often the samplers look, and how often the wait in
/// [`measure`] comes round.
const SAMPLE_MS: u64 = 100;

/// How long a run will wait for the machine to go quiet before giving up
/// on it. Long enough to outlast a lunch break, since that is exactly the
/// case: somebody walked away, the session locked, and the measurement
/// should be taken when they are back rather than thrown away.
const QUIET_CEILING: Duration = Duration::from_secs(1_800);

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

    let exe = if opts.harness {
        let mut extra = vec!["-p", "platitude-app"];
        if opts.breakdown {
            extra.extend(["--features", "memprobe"]);
        }
        crate::tree::app_exe(&root, &path, opts.build, &extra)?
    } else {
        crate::tree::shipped_exe(&root, &path, opts.build)?
    };

    let corpus = if opts.open {
        let found = corpus::describe(&opts.repo)?;
        if let Some(complaint) = corpus::mismatch(&found, &opts.corpus) {
            return Err(complaint);
        }
        Some(found)
    } else {
        None
    };

    let (modes, screens) = display::survey();
    let screen = match display::choose(&screens, &opts.screen) {
        Ok(screen) => Some(screen.clone()),
        Err(complaint) if opts.screen.is_empty() => {
            println!("  {complaint}; the platform will place the window.");
            None
        }
        Err(complaint) => return Err(complaint),
    };
    let output = artifacts::prepare(
        &root,
        &exe,
        &opts,
        screen.as_ref(),
        &corpus_line(corpus.as_ref()),
        &modes,
    )?;
    println!("evidence: {}", output.display());
    if let Some(corpus) = &corpus {
        println!("corpus: {corpus}");
    }
    if let Some(screen) = &screen {
        println!("screen: {} at {}Hz", screen.name, screen.hz);
    }

    println!(
        "repo: {} | runs: {} (the first is discarded — cold cache)",
        opts.repo.display(),
        opts.runs
    );
    let bench = Bench {
        output: &output,
        exe: &exe,
        path: &path,
        root: &root,
        opts: &opts,
        screen: screen.as_ref(),
    };
    let mut kept: Vec<Reading> = Vec::new();
    let mut retries = 0;
    for run in 0..=opts.runs {
        let discarded = run == 0;
        let reading = bench.take(run, &mut retries)?;
        say(run, discarded, &reading);
        if !discarded {
            kept.push(reading);
        }
    }
    one_graphics_stack(&kept)?;
    unmoved_corpus(&opts, corpus.as_ref())?;
    report(
        &opts,
        &kept,
        &report::Context {
            screen: screen.as_ref(),
            corpus: corpus.as_ref(),
            retries,
        },
    );
    Ok(())
}

/// Everything one run needs, held once so a run is `take(n)`.
struct Bench<'a> {
    output: &'a std::path::Path,
    exe: &'a std::path::Path,
    path: &'a std::ffi::OsString,
    root: &'a std::path::Path,
    opts: &'a Options,
    screen: Option<&'a display::Screen>,
}

impl Bench<'_> {
    /// Whether the scroll bench was presented at all, read against what
    /// the screen it was on could have shown.
    ///
    /// This is the gate that focus cannot be. A window that is not in
    /// front is still composited and still presents — the app does not
    /// always take the focus off the shell that started it — but a window
    /// that was covered, on a screen that slept, or behind a locked
    /// session stops receiving frames entirely, and there is no API that
    /// answers "was it covered". The frames themselves answer it.
    fn frames_delivered(&self, reading: &Reading) -> Option<String> {
        frames_delivered(
            self.screen.map(|screen| screen.hz),
            reading.fps,
            &self.opts.limits,
        )
    }

    /// One run, taken again for as long as the machine keeps spoiling it.
    ///
    /// A run spoiled by the host is not a slow application and must not
    /// be published as one; it is also not a failure of the application,
    /// so it is not fatal either. Between attempts the runner waits for
    /// the machine rather than firing straight into the same noise —
    /// which is what makes "somebody walked away and the session locked"
    /// end in a measurement instead of in a wasted afternoon.
    fn take(&self, run: u32, retries: &mut u32) -> Result<Reading, String> {
        let mut attempt = 0;
        loop {
            sampler::wait_for_quiet(&self.opts.limits, QUIET_CEILING)?;
            let run_dir = self.output.join(if attempt == 0 {
                format!("run-{run}")
            } else {
                format!("run-{run}-again-{attempt}")
            });
            std::fs::create_dir(&run_dir).map_err(|e| e.to_string())?;
            display::capture(&run_dir, "before")?;
            let result = measure(
                self.exe,
                self.path,
                self.root,
                self.opts,
                &run_dir,
                self.screen,
            );
            std::fs::write(run_dir.join("result.txt"), format!("{result:#?}"))
                .map_err(|e| e.to_string())?;
            display::capture(&run_dir, "after")?;
            let reading = result?;
            let Some(complaint) = reading
                .conditions
                .complaint(&self.opts.limits)
                .or_else(|| self.frames_delivered(&reading))
            else {
                return Ok(reading);
            };
            attempt += 1;
            *retries += 1;
            if attempt > self.opts.retries {
                return Err(format!(
                    "run {run} was not a reading of the application after {attempt} attempts: \
                     {complaint}. Measure it on a machine nobody else is using, or pass \
                     --allow-noisy to publish what a busy one produced."
                ));
            }
            println!("  run {run}: {complaint} — taking it again");
        }
    }
}

/// Whether the scroll bench was presented at all, read against what the
/// screen it was on could have shown. `None` where there is nothing to
/// read it against — a run with no scroll, or a screen whose mode table
/// owned no refresh rate.
fn frames_delivered(hz: Option<u32>, fps: Option<f64>, limits: &sampler::Limits) -> Option<String> {
    let hz = f64::from(hz.filter(|hz| *hz > 0)?);
    let fps = fps?;
    let floor = hz * limits.frame_share;
    (fps < floor).then(|| {
        format!(
            "the scroll delivered {fps:.0} frames a second on a {hz:.0}Hz screen, under the \
             {floor:.0} this reads as drawn at all — the window was covered, the screen slept, \
             or the machine was doing something else"
        )
    })
}

/// One graphics stack across the kept runs. Two runs that drew on
/// different adapters are not each other's control, and this machine
/// offers three — an NVIDIA discrete, an AMD integrated and the basic
/// render driver (`QT_D3D_ADAPTER_INDEX` is the lever if Qt ever takes a
/// different one).
fn one_graphics_stack(kept: &[Reading]) -> Result<(), String> {
    let Some(first) = kept.first() else {
        return Ok(());
    };
    let Some(odd) = kept.iter().find(|run| run.graphics != first.graphics) else {
        return Ok(());
    };
    Err(format!(
        "the kept runs did not all draw on the same graphics stack, so they are not each other's \
         control.\n  {}\n  {}",
        first.graphics.join(" | "),
        odd.graphics.join(" | ")
    ))
}

/// The corpus again at the end: a fetch that landed between the first run
/// and the last one leaves every earlier reading measuring a different
/// repository from the later ones.
fn unmoved_corpus(opts: &Options, before: Option<&corpus::Corpus>) -> Result<(), String> {
    let Some(before) = before.filter(|_| opts.open) else {
        return Ok(());
    };
    let after = corpus::describe(&opts.repo)?;
    if after == *before {
        return Ok(());
    }
    Err(format!(
        "the benchmark repository changed while it was being measured.\n  started {before}\n  \
         ended   {after}\nThe readings are of two different repositories and cannot be published \
         together."
    ))
}

fn corpus_line(corpus: Option<&corpus::Corpus>) -> String {
    corpus.map_or_else(|| "no repository".to_string(), ToString::to_string)
}

fn say(run: u32, discarded: bool, reading: &Reading) {
    println!(
        "  run {}{}: ws={:.1}MB private={:.1}MB startup={} graph={} walk={} fps={}",
        run,
        if discarded { " (discarded)" } else { "" },
        mb(reading.peak_working_set),
        mb(reading.peak_private),
        reading.startup_ms.map_or("-".into(), |v| v.to_string()),
        reading.graph_ms.map_or("-".into(), |v| v.to_string()),
        reading.total_ms.map_or("-".into(), |v| v.to_string()),
        reading.fps.map_or("-".into(), |v| format!("{v:.1}")),
    );
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

#[cfg(test)]
mod tests {
    use super::sampler::Limits;
    use super::{Reading, frames_delivered, one_graphics_stack};

    fn drew(lines: &[&str]) -> Reading {
        Reading {
            graphics: lines.iter().map(|line| (*line).to_string()).collect(),
            ..Reading::default()
        }
    }

    /// A floor under "was anything presented at all", not a performance
    /// budget: half of what the screen it was on could have shown.
    #[test]
    fn a_scroll_nobody_could_have_seen_is_refused() {
        let limits = Limits::default();
        assert!(frames_delivered(Some(180), Some(176.0), &limits).is_none());
        assert!(frames_delivered(Some(180), Some(90.1), &limits).is_none());
        let complaint =
            frames_delivered(Some(180), Some(12.0), &limits).expect("12fps on a 180Hz screen");
        assert!(complaint.contains("12 frames a second"), "{complaint}");
        assert!(complaint.contains("180Hz"), "{complaint}");
    }

    /// Nothing to read the frames against is not a spoiled run: a screen
    /// whose mode table owned no rate, and a run that did not scroll.
    #[test]
    fn a_run_with_nothing_to_compare_against_is_not_refused() {
        let limits = Limits::default();
        assert!(frames_delivered(None, Some(1.0), &limits).is_none());
        assert!(frames_delivered(Some(0), Some(1.0), &limits).is_none());
        assert!(frames_delivered(Some(180), None, &limits).is_none());
        // --allow-noisy publishes what a covered window produced.
        assert!(frames_delivered(Some(180), Some(1.0), &Limits::OPEN).is_none());
    }

    #[test]
    fn kept_runs_that_drew_on_different_adapters_are_not_a_reading() {
        let nvidia = drew(&["Adapter 0: NVIDIA", "using this adapter"]);
        let amd = drew(&["Adapter 0: AMD", "using this adapter"]);
        assert!(one_graphics_stack(&[]).is_ok());
        assert!(one_graphics_stack(&[nvidia.clone(), nvidia.clone()]).is_ok());
        let complaint = one_graphics_stack(&[nvidia, amd]).expect_err("two adapters");
        assert!(complaint.contains("NVIDIA"), "{complaint}");
        assert!(complaint.contains("AMD"), "{complaint}");
    }
}
