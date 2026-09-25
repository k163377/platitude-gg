//! `cargo xtask perf` — the memory / startup / fps measurement, repeated
//! the same way every time. What each flag does is on the usage page
//! (`usage.rs`).
//!
//! The record in `ci/baseline/perf-windows-x64.md` is only worth anything
//! if the run behind it can be repeated exactly, so the conditions live
//! here: release build, a real window (offscreen reports neither memory
//! nor fps honestly) on one named screen, warm cache, the `PGG_AUTO_*`
//! hooks, WorkingSet sampled every 100ms for its maximum, causal
//! `perf_done`, and an outer kill guard.
//!
//! The machine is a condition too: a covered window, a locked session, a
//! sleeping screen and a parallel build all read exactly like a slower
//! application, so the run measures them and refuses
//! (`perf::sampler::Conditions`). So is the benchmark repository: a fetch
//! changes the rows, the ref tables and the commit the interaction opens
//! while `HEAD` holds still, so `perf::corpus` fingerprints it either side
//! of the runs.
//!
//! Every invocation starts with a calibration run that weighs the font
//! database's population, which the record reads the budget line net of
//! ([`fonts`]; `--no-font-walk` leaves it out).

mod artifacts;
mod attribution;
mod cases;
mod corpus;
mod display;
mod experiment;
mod fonts;
mod interactions;
mod measure;
mod options;
mod reading;
mod report;
mod rig;
mod sampler;
mod warmth;

use std::time::Duration;

use crate::command::{self, Permission, Where};
use measure::measure;
use options::{Options, parse};
use reading::Reading;
use report::{mb, report};

/// The escape is read by `guard_the_window`.
pub(crate) static PERF: command::Command = command::Command {
    id: "perf.measure",
    call: "perf --repo <path>",
    purpose: "the four budget numbers, measured against a repository",
    run_in: Where::Seat,
    needs: &[
        "a repository to measure against",
        "the machine still — the run refuses load that was not the application",
    ],
    permission: Permission::Escape(crate::hook::GUI_APPROVAL_FLAG),
};

pub(crate) static COMMANDS: &[&command::Command] = &[&PERF];

/// How often the samplers look, and how often the wait in
/// [`measure`] comes round.
const SAMPLE_MS: u64 = 100;

/// How long a run waits for the machine to go quiet: long enough to
/// outlast a lunch break (the session locked while nobody was there).
const QUIET_CEILING: Duration = Duration::from_secs(1_800);

pub fn run(args: &[String]) -> Result<(), String> {
    // The samplers answer (0, 0) where they are not implemented, so an
    // OS without them is refused here.
    if !cfg!(any(windows, target_os = "linux")) {
        return Err("memory sampling is not implemented for this OS".into());
    }
    let opts = parse(args)?;
    if !opts.compare.is_empty() {
        return experiment::compare(opts);
    }
    run_options(opts)
}

fn run_options(opts: Options) -> Result<(), String> {
    let root = crate::tree::workspace_root();
    let path = crate::qt::path_with_qt()?;
    guard_the_window(&root)?;

    // Taken before the build: a release build is minutes of exactly the
    // idle that darkens the screen. A software run holds only the
    // machine: its frames need no display.
    let _awake = sampler::keep_awake(!opts.software);

    // One announced build (`still`), covering the compiles inside it —
    // this tree's or the rig's.
    let built = {
        let _building = crate::still::busy(&root, "cargo xtask perf (building)")?;
        build(&root, &path, &opts)?
    };
    println!(
        "measured: {} of {}",
        built.short(),
        if opts.at.is_empty() {
            "this tree, edits included"
        } else {
            "the rig"
        }
    );

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
    let (output, exe_hash) = artifacts::prepare(
        &root,
        &built,
        &opts,
        screen.as_ref(),
        &corpus_line(corpus.as_ref()),
        &modes,
    )?;
    announce(&output, corpus.as_ref(), screen.as_ref(), &opts);

    // After the build, before the runs: the build can share the machine,
    // the runs cannot (`still`).
    let _still = crate::still::hold(&root, "cargo xtask perf")?;
    // Decided under the hold: a build the hold waited out is what would
    // have made the cache cold.
    let warmed = warmth::Warmed::now(
        &exe_hash,
        corpus.as_ref().map_or("-", |corpus| corpus.token.as_str()),
        &crate::seats::slashed(&opts.repo),
        &scenario(&opts),
    );
    let first = first_run(&root, &warmed, &opts);
    let bench = Bench {
        output: &output,
        exe: &built.exe,
        path: &path,
        root: &built.tree,
        opts: &opts,
        screen: screen.as_ref(),
    };
    let mut kept: Vec<Reading> = Vec::new();
    let mut retries = 0;
    let font_walk = calibrate(&bench, &opts, &mut retries)?;
    for run in first..=opts.runs {
        let discarded = run == 0;
        let note = if discarded { " (discarded)" } else { "" };
        let reading = bench.take(&run.to_string(), &opts, &mut retries)?;
        say(&run.to_string(), note, &reading);
        if !discarded {
            kept.push(reading);
        }
    }
    one_graphics_stack(&kept)?;
    unmoved_corpus(&opts, corpus.as_ref())?;
    if opts.cache == "warm" {
        warmth::note(&root, &warmed);
    }
    experiment::save(&output, &opts, &kept)?;
    report(
        &opts,
        &kept,
        &report::Context {
            screen: screen.as_ref(),
            corpus: corpus.as_ref(),
            built: &built,
            retries,
            font_walk: font_walk.as_ref(),
        },
    );
    Ok(())
}

fn announce(
    output: &std::path::Path,
    corpus: Option<&corpus::Corpus>,
    screen: Option<&display::Screen>,
    opts: &Options,
) {
    println!("evidence: {}", output.display());
    if let Some(corpus) = corpus {
        println!("corpus: {corpus}");
    }
    if let Some(screen) = screen {
        println!("screen: {} at {}Hz", screen.name, screen.hz);
    }
    if opts.software {
        println!(
            "software: the software scene graph draws, so the display may be on, off or \
             flipping; nothing holds it awake"
        );
    }
}

/// The rig's build of `--at`'s commit, or this tree's own — edits
/// included, which the evidence's `source.patch` records.
fn build(
    root: &std::path::Path,
    path: &std::ffi::OsStr,
    opts: &Options,
) -> Result<rig::Built, String> {
    if !opts.at.is_empty() {
        return rig::build_at(root, &opts.at, path, opts);
    }
    let exe = build_in(root, path, opts.build, opts)?;
    let commit = crate::subprocess::git_query(&crate::seats::slashed(root), &["rev-parse", "HEAD"])
        .unwrap_or_default();
    Ok(rig::Built {
        exe,
        commit,
        tree: root.to_path_buf(),
    })
}

/// The one place the harness / memprobe / shipped choice is spelled for a
/// build, so the shelf and the manifest name what was built
/// (`Options::features`).
fn build_in(
    tree: &std::path::Path,
    path: &std::ffi::OsStr,
    build: bool,
    opts: &Options,
) -> Result<std::path::PathBuf, String> {
    if !opts.harness {
        return crate::app_build::shipped_exe(tree, path, build);
    }
    let mut extra = vec!["-p", "platitude-app"];
    if opts.breakdown {
        extra.extend(["--features", "memprobe"]);
    }
    crate::app_build::app_exe(tree, path, build, &extra)
}

/// The scenario the runs drive, as the warm note keys it: what a stage
/// that drove less would have left cold (`warmth`).
fn scenario(opts: &Options) -> String {
    format!(
        "{}/{}/{}/{}/{}/{}/{}/{}/{}/{}",
        opts.selection,
        opts.diff,
        opts.scroll,
        opts.open,
        opts.oid,
        opts.file,
        cases::encode(&opts.cases)
            .replace('\n', "|")
            .replace('\t', "/"),
        opts.cycles,
        opts.completion,
        opts.diff_scroll
    )
}

/// The font-walk calibration run (`fonts::calibration`), before the
/// others. It does not replace the discarded run: it opens nothing a
/// `git show` reads, so the cold cache is still the first run's to pay
/// (`warmth`).
fn calibrate(
    bench: &Bench<'_>,
    opts: &Options,
    retries: &mut u32,
) -> Result<Option<fonts::FontWalk>, String> {
    if !opts.calibrate {
        return Ok(None);
    }
    let reading = bench.take("font-walk", &fonts::calibration(opts), retries)?;
    say("font-walk", " (calibration)", &reading);
    Ok(reading.font_walk)
}

/// Which run the kept ones start at: run 0 is discarded to pay for a
/// cold cache, unless an invocation minutes ago warmed exactly this and
/// nothing built since (`warmth`).
fn first_run(root: &std::path::Path, warmed: &warmth::Warmed, opts: &Options) -> u32 {
    if opts.cache != "warm" {
        println!(
            "cache: {} (no calibration or discarded launch; no automatic retry)",
            opts.cache
        );
        return 1;
    }
    if !opts.compare.is_empty() {
        println!("cache: warm (each A/B sample gets its own discarded warm-up)");
        return 0;
    }
    let (first, why) = match warmth::warm(root, warmed) {
        Some(age) => (
            1,
            format!(
                "none discarded — an invocation {}s ago warmed this exe, corpus and scenario, \
                 and no build ended since",
                age.as_secs()
            ),
        ),
        None => (
            0,
            "the first is a discarded warm-up; OS cache state is not inferred".to_string(),
        ),
    };
    println!(
        "repo: {} | runs: {} ({why})",
        opts.repo.display(),
        opts.runs
    );
    first
}

struct Bench<'a> {
    output: &'a std::path::Path,
    exe: &'a std::path::Path,
    path: &'a std::ffi::OsString,
    root: &'a std::path::Path,
    opts: &'a Options,
    screen: Option<&'a display::Screen>,
}

impl Bench<'_> {
    /// The gate focus cannot be: a window not in front still presents (the
    /// app does not always take the focus), and no API answers "was it
    /// covered" — the frames do. A bench that got no frames at all died at
    /// `measure::SCROLL_CEILING` and never reaches here.
    fn frames_delivered(&self, reading: &Reading) -> Option<String> {
        frames_delivered(
            self.screen.map(|screen| screen.hz),
            reading.fps,
            &self.opts.limits,
            self.opts.software,
        )
    }

    /// One run, taken again for as long as its cause's budget allows
    /// (`measure::Spoiled::budget`), each attempt after the machine went
    /// quiet. `opts` is the run's own: the calibration run drives less
    /// than the kept ones (`fonts::calibration`).
    fn take(&self, run: &str, opts: &Options, retries: &mut u32) -> Result<Reading, String> {
        let mut attempt = 0;
        let mut spent = [0u32; measure::Spoiled::CAUSES];
        loop {
            sampler::wait_for_quiet(&opts.limits, QUIET_CEILING)?;
            let run_dir = self.output.join(if attempt == 0 {
                format!("run-{run}")
            } else {
                format!("run-{run}-again-{attempt}")
            });
            std::fs::create_dir(&run_dir).map_err(|e| e.to_string())?;
            display::capture(&run_dir, "before")?;
            experiment::prepare_cold(opts, self.exe, &run_dir)?;
            let result = measure(self.exe, self.path, self.root, opts, &run_dir, self.screen);
            std::fs::write(run_dir.join("result.txt"), format!("{result:#?}"))
                .map_err(|e| e.to_string())?;
            display::capture(&run_dir, "after")?;
            let spoiled = match result {
                // An `Ok` already passed the host conditions; too few
                // frames is the machine's doing as much as a lock is.
                Ok(reading) => match self.frames_delivered(&reading) {
                    None => return Ok(reading),
                    Some(complaint) => measure::Spoiled::Host(complaint),
                },
                Err(spoiled) => spoiled,
            };
            attempt += 1;
            let taken = &mut spent[spoiled.cause()];
            *taken += 1;
            if *taken > spoiled.budget(opts.retries) {
                return Err(format!(
                    "run {run} was not a reading of the application after {} attempt(s) spoiled \
                     by {}: {}. {}",
                    *taken,
                    spoiled.named(),
                    spoiled.said(),
                    spoiled.advice()
                ));
            }
            *retries += 1;
            println!("  run {run}: {} — taking it again", spoiled.said());
        }
    }
}

/// The complaint when the scroll delivered under `limits.frame_share` of
/// the screen's rate. A software run is never read against the screen:
/// its frames reach none (`measure::command`).
fn frames_delivered(
    hz: Option<u32>,
    fps: Option<f64>,
    limits: &sampler::Limits,
    software: bool,
) -> Option<String> {
    if software {
        return None;
    }
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

/// Two runs that drew on different adapters are not each other's
/// control, and a machine can offer several (the rig's:
/// ci/baseline/perf-windows-x64.md §計測条件). `QT_D3D_ADAPTER_INDEX` is
/// the lever if Qt ever takes a different one.
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

/// `walk=` is the graph walk; the font walk, where the run paid one,
/// follows it by name.
fn say(run: &str, note: &str, reading: &Reading) {
    println!(
        "  run {run}{note}: ws={:.1}MB private={:.1}MB startup={} graph={} walk={} fps={}{}",
        mb(reading.peak_working_set),
        mb(reading.peak_private),
        reading.startup_ms.map_or("-".into(), |v| v.to_string()),
        reading.graph_ms.map_or("-".into(), |v| v.to_string()),
        reading.total_ms.map_or("-".into(), |v| v.to_string()),
        reading.fps.map_or("-".into(), |v| format!("{v:.1}")),
        reading
            .font_walk
            .as_ref()
            .map(fonts::FontWalk::line)
            .unwrap_or_default(),
    );
}

/// `cargo xtask hook pre-shell` cannot see this command (it names no app
/// binary), so `PGG_ALLOW_GUI=1` is checked here: a worktree window can
/// overlap another session's screenshot.
fn guard_the_window(root: &std::path::Path) -> Result<(), String> {
    let in_worktree = root
        .to_string_lossy()
        .replace('\\', "/")
        .contains("/.claude/worktrees/");
    if !in_worktree || std::env::var("PGG_ALLOW_GUI").as_deref() == Ok("1") {
        return Ok(());
    }
    Err(
        "this measurement needs a real window, and a worktree may not put one up on its own \
         (offscreen reports neither memory nor fps honestly, so there is no headless form of \
         it). Run it again with PGG_ALLOW_GUI=1 when the window was asked for."
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

    /// A floor under "was anything presented at all": half of what
    /// the screen it was on could have shown.
    #[test]
    fn a_scroll_nobody_could_have_seen_is_refused() {
        let limits = Limits::default();
        assert!(frames_delivered(Some(180), Some(176.0), &limits, false).is_none());
        assert!(frames_delivered(Some(180), Some(90.1), &limits, false).is_none());
        let complaint = frames_delivered(Some(180), Some(12.0), &limits, false)
            .expect("12fps on a 180Hz screen");
        assert!(complaint.contains("12 frames a second"), "{complaint}");
        assert!(complaint.contains("180Hz"), "{complaint}");
    }

    #[test]
    fn a_run_with_nothing_to_compare_against_is_not_refused() {
        let limits = Limits::default();
        assert!(frames_delivered(None, Some(1.0), &limits, false).is_none());
        assert!(frames_delivered(Some(0), Some(1.0), &limits, false).is_none());
        assert!(frames_delivered(Some(180), None, &limits, false).is_none());
        assert!(frames_delivered(Some(180), Some(12.0), &limits, true).is_none());
        // --allow-noisy publishes what a covered window did deliver; one
        // that delivered none still dies at `measure::SCROLL_CEILING`.
        assert!(frames_delivered(Some(180), Some(1.0), &Limits::OPEN, false).is_none());
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
