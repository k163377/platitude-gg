//! One measured run of the app: the process it starts, the memory and the
//! host conditions it samples off it while it runs, and the wait that ends
//! on the app's own completion rather than on a clock.
//!
//! **A real window, deliberately.** `verify-ui` runs offscreen, and
//! offscreen Qt builds no scene graph worth measuring.
//!
//! **On one named screen, deliberately.** The window's place is written
//! into the run's own `state.toml` rather than left to the platform: this
//! machine has monitors at 100, 180 and 100Hz, and the screen a window
//! lands on sets the rate its frames can possibly arrive at
//! (`perf::display`).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::display::Screen;
use super::reading::{Reading, missing, read_app};
use super::{Options, SAMPLE_MS, artifacts, sampler};

/// How long the app is left alone after `perf_done` even when the run
/// asked for no settling: enough for at least one more sample, so the
/// final reading is one the sampler actually took rather than whatever
/// it happened to hold when completion arrived.
const AFTER_DONE_MS: u64 = 250;

/// How long the scroll bench is given past the twelve seconds it asks
/// for. Past this it is not slow, it is not running: the window it draws
/// into is covered, and the animation that advances it stopped with the
/// frames. Thirty seconds, against the twenty-four a bench at half the
/// screen's rate takes — the least the frame gate publishes, so a run
/// slower than that is refused by the frame gate anyway, and every
/// second past it only makes a covered window cost more.
const SCROLL_CEILING: Duration = Duration::from_secs(30);

/// Why a run produced no reading, and whether taking it again could
/// help. The difference matters: a machine that spoiled a run will not
/// have spoiled the next one, and an application that stopped answering
/// will stop answering again.
#[derive(Debug)]
pub(super) enum Spoiled {
    /// The machine: a lock, a minimised window, a screen change, a busy
    /// machine. Worth taking again for as long as `--retries` allows.
    Host(String),
    /// A bench that stood still with nothing else wrong with the
    /// machine: the window was covered, or the screen was off. Worth one
    /// more attempt — what covers a window either passes or stays, and a
    /// stayer is not worth two more ceilings of saying so
    /// (`perf::Bench::take`).
    Covered(String),
    Run(String),
}

impl From<String> for Spoiled {
    /// Anything that went wrong before the machine could be asked is
    /// the run's own — a directory that would not open, a process
    /// that would not start, a sampler that panicked.
    fn from(said: String) -> Self {
        Self::Run(said)
    }
}

impl Spoiled {
    /// How many causes there are, for a count per cause.
    pub(super) const CAUSES: usize = 3;

    /// Which cause this is, as an index into a count per cause.
    pub(super) fn cause(&self) -> usize {
        match self {
            Self::Host(_) => 0,
            Self::Covered(_) => 1,
            Self::Run(_) => 2,
        }
    }

    /// How many attempts a run spoiled this way is worth. The machine,
    /// `--retries` — it will not have spoiled the next one. A covered
    /// window, one: what covers it either passes or stays, and a stayer
    /// is not worth two more ceilings of saying so. The application, none:
    /// it will stop answering again.
    pub(super) fn budget(&self, retries: u32) -> u32 {
        match self {
            Self::Host(_) => retries,
            Self::Covered(_) => 1,
            Self::Run(_) => 0,
        }
    }

    pub(super) fn named(&self) -> &'static str {
        match self {
            Self::Host(_) => "the machine",
            Self::Covered(_) => "a covered window",
            Self::Run(_) => "the application",
        }
    }

    pub(super) fn said(&self) -> &str {
        match self {
            Self::Host(said) | Self::Covered(said) | Self::Run(said) => said,
        }
    }

    /// What to do about it, once the budget is spent.
    pub(super) fn advice(&self) -> &'static str {
        match self {
            Self::Host(_) => {
                "Measure it on a machine nobody else is using, or pass --allow-noisy to publish \
                 what a busy one produced."
            }
            Self::Covered(_) => "Whatever covers the window stays — clear it and measure again.",
            Self::Run(_) => "",
        }
    }
}

/// The process this run measures, and everything it is told.
///
/// A build with no harness in it is told nothing at all beyond where its
/// two files are: it answers no `PG_AUTO_*` knob, and the repository it
/// opens comes out of the `state.toml` written beside them
/// (`artifacts::state_file`).
fn command(
    exe: &std::path::Path,
    path: &std::ffi::OsString,
    root: &std::path::Path,
    opts: &Options,
    config_dir: &std::path::Path,
) -> Command {
    let mut cmd = Command::new(exe);
    crate::app_env::clear_automation(&mut cmd);
    cmd.current_dir(root)
        .env("PATH", path)
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PG_CONFIG_DIR", config_dir)
        .env("PG_LOG", "info")
        // Which graphics device and backend Qt chose, said by Qt itself.
        // Two runs on different adapters are not each other's control, and
        // this machine has more than one (ci/baseline/perf-windows-x64.md).
        .env("QSG_INFO", "1")
        // A real window, deliberately: `verify-ui` runs offscreen, and
        // offscreen Qt builds no scene graph worth measuring.
        .env_remove("QT_QPA_PLATFORM")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if !opts.harness {
        return cmd;
    }
    // The app reports completion only after every requested measurement
    // has answered. The parent owns termination so it can take the last
    // process-memory sample and reap exactly the child it started.
    cmd.env("PG_AUTO_PERF", "1")
        .env("PG_PERF_SELECTION", &opts.selection)
        .env("PG_PERF_OID", &opts.oid)
        .env("PG_PERF_FILE", &opts.file)
        .env("PG_PERF_DIFF", if opts.diff { "1" } else { "0" })
        .env(
            "PG_PERF_TRACE_FRAMES",
            if opts.trace_frames { "1" } else { "0" },
        );
    for (asked, name, value) in [
        (opts.open, "PG_AUTO_OPEN", opts.repo.display().to_string()),
        (opts.select, "PG_AUTO_SELECT", "1".to_string()),
        (opts.scroll, "PG_AUTO_SCROLL", "1".to_string()),
        (opts.breakdown, "PG_MEM_REPORT", "1".to_string()),
    ] {
        if asked {
            cmd.env(name, value);
        }
    }
    cmd
}

pub(super) fn measure(
    exe: &std::path::Path,
    path: &std::ffi::OsString,
    root: &std::path::Path,
    opts: &Options,
    run_dir: &std::path::Path,
    screen: Option<&Screen>,
) -> Result<Reading, Spoiled> {
    // A config directory per process, so another perf process or a previous
    // run's restored state cannot decide what this one does.
    let (config_dir, log, samples) = artifacts::open_run(run_dir, opts, screen)?;
    let mut cmd = command(exe, path, root, opts, &config_dir);
    // Armed before the clock starts: the sampler's compile stays out of
    // the timed window (`sampler::Armed`). The sum cannot overflow —
    // `options::settle` holds the two under a day.
    let window = Duration::from_millis(opts.watchdog_ms + opts.settle_ms + AFTER_DONE_MS);
    let armed = sampler::arm(window, samples)?;
    // Started with nothing executed yet: the sampler puts the process in
    // its job object and only then lets it run, so the first git the app
    // spawns is inside the job with everything after it. The clock starts
    // when the sampler says the process is running.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(sampler::CREATE_SUSPENDED);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    let (sampler, started) = match armed.watch(child.id()) {
        Ok(watching) => watching,
        Err(said) => {
            // Never resumed, so never running: ended here rather than
            // left suspended for the outer kill guard.
            if let Err(error) = child.kill() {
                println!("  note: could not end the app that never ran: {error}");
            }
            return Err(Spoiled::Run(said));
        }
    };
    let deadline = started + Duration::from_millis(opts.watchdog_ms);
    let stderr = child.stderr.take();
    let (done_rx, scroll, reader) = read_app(stderr, started, log, opts.harness);
    // `perf_done`, not elapsed time, is the success edge. The deadline is
    // only an outer diagnostic guard for an app that stopped answering.
    // **`--allow-noisy` opens this one too.** The ceiling is a rate in
    // disguise: twelve seconds of vsync-stepped animation take twelve
    // seconds only at the screen's full rate, and past this they have
    // not finished — so an absolute ceiling refuses everything below
    // about a quarter of that rate. That is a slow machine, which is
    // the case `--allow-noisy` exists to publish rather than refuse.
    let ceiling = (!opts.limits.quiet_percent.is_infinite()).then_some(SCROLL_CEILING);
    let mut began = Instant::now();
    let mut watching = false;
    let ended = loop {
        if let Ok(success) = done_rx.try_recv() {
            break Ended::Done(success);
        }
        match child.try_wait() {
            Ok(Some(_)) => break Ended::Exited,
            Ok(None) => {}
            Err(e) => break Ended::WaitFailed(format!("waiting on the app failed: {e}")),
        }
        // The bench's own deadline. A window nothing is drawing advances
        // no animation, so the bench that should end in twelve seconds
        // ends never — and waiting the whole watchdog out to say so
        // costs five minutes and names the wrong culprit.
        match scroll.stalled_for(began) {
            Some(_) if !watching => {
                watching = true;
                began = Instant::now();
            }
            Some(waited) if ceiling.is_some_and(|ceiling| waited > ceiling) => {
                let _ = child.kill();
                break Ended::Covered;
            }
            _ => {}
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            break Ended::TimedOut;
        }
        std::thread::sleep(Duration::from_millis(SAMPLE_MS));
    };
    let done = matches!(ended, Ended::Done(true));
    // Held idle first, so the last reading is taken of a process that has
    // stopped working rather than one caught mid-frame. The one sampler is
    // still running, which is what makes both the peak and the settled
    // value readings of the same series rather than of two more processes
    // started beside the one being measured.
    if done {
        std::thread::sleep(Duration::from_millis(opts.settle_ms.max(AFTER_DONE_MS)));
    }
    let held = sampler.read();
    let _ = child.kill();
    let _ = child.wait();
    let mut reading = reader.join().unwrap_or_default();
    let series = sampler.finish()?;
    reading.perf_done |= done;
    reading.peak_working_set = series.peak_working_set;
    reading.peak_private = series.peak_private;
    reading.settled_working_set = if done && opts.settle_ms > 0 {
        held.last.as_ref().map_or(0, |sample| sample.working_set)
    } else {
        0
    };
    reading.conditions = series.conditions;
    // Written before the verdict, because the verdict may be that this
    // is not a reading — and a run refused for the state of the machine
    // is exactly the one whose numbers a later reader wants to see.
    let _ = std::fs::write(run_dir.join("reading.txt"), format!("{reading:#?}"));
    verdict(reading, opts, ended, screen.map(|s| s.name.as_str()))
}

/// How the wait ended: exactly one of these, because the loop leaves
/// through exactly one edge.
enum Ended {
    /// The app reported `perf_done`, and whether it called the scenario
    /// a success. A false here is not an ending of its own — what went
    /// wrong is in the reading.
    Done(bool),
    /// The process left before saying anything.
    Exited,
    /// The scroll bench stood still past [`SCROLL_CEILING`].
    Covered,
    TimedOut,
    WaitFailed(String),
}

/// Whether the run is a reading, and if not whose fault that was.
///
/// **The host first, whatever else went wrong.** A dark screen or a
/// locked session stops the compositor presenting, which freezes the
/// animation the scroll is driven by, and the run then dies at its
/// deadline — reported as an application that stopped answering unless
/// the machine is asked about first. Anything the host spoiled is worth
/// taking again; nothing else is (`Spoiled`).
fn verdict(
    reading: Reading,
    opts: &Options,
    ended: Ended,
    pinned: Option<&str>,
) -> Result<Reading, Spoiled> {
    let covered = matches!(ended, Ended::Covered);
    let ending = match ended {
        Ended::WaitFailed(said) => Some(said),
        Ended::TimedOut => Some(format!(
            "the run did not report perf_done within {}ms and was killed",
            opts.watchdog_ms
        )),
        Ended::Exited => Some("the app exited before reporting perf_done".into()),
        Ended::Done(_) | Ended::Covered => None,
    };
    let host = reading.conditions.complaint(&opts.limits, pinned);
    // A bench that stood still on a machine with nothing else to say
    // about itself: its own kind, because it is retried its own way.
    let stood = (host.is_none() && covered).then(|| stood_still(&reading.conditions));
    let failed = reading
        .failure
        .clone()
        .or_else(|| reading.conditions.unwatched())
        .or(ending)
        .or_else(|| missing(&reading, opts).err());
    let and_so = |said: String| match &failed {
        Some(failed) => format!("{said} — and so {failed}"),
        None => said,
    };
    if let Some(host) = host {
        return Err(Spoiled::Host(and_so(host)));
    }
    if let Some(stood) = stood {
        return Err(Spoiled::Covered(and_so(stood)));
    }
    match failed {
        Some(failed) => Err(Spoiled::Run(failed)),
        None => Ok(reading),
    }
}

/// What to say about a bench that never advanced.
///
/// **Only the frames know, and there are none — so the evidence has to
/// pick the sentence.** A window that lost the front was plausibly
/// covered; one that held it for every tick was not, and saying so
/// anyway sends a reader looking for a window that was never there.
/// What is left in that case is the screen, the desktop in front of it,
/// and the application itself standing still.
fn stood_still(conditions: &super::sampler::Conditions) -> String {
    let held_the_front = conditions
        .foreground_share()
        .is_some_and(|share| share > 0.99);
    let cause = if held_the_front {
        "nothing presented a window that was in front the whole time — the screen was off, \
         another desktop was in front of it, or the application stopped drawing"
    } else {
        "the screen was off, or the window was covered"
    };
    let front = conditions.foreground_share().map_or_else(
        || "no window ever appeared".to_string(),
        |share| format!("in front for {:.0}% of the ticks it had one", share * 100.0),
    );
    format!(
        "the scroll bench produced nothing for {}s — an animation nothing draws never advances. \
         {cause} ({front})",
        SCROLL_CEILING.as_secs(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::options;
    use crate::perf::sampler::{Conditions, Limits};

    fn defaults() -> Options {
        options(&["--repo", "."])
    }

    fn options(args: &[&str]) -> Options {
        options::parse(&args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>())
            .expect("a valid option line")
    }

    /// A machine with nothing to say about itself, so the verdict turns
    /// on the run alone.
    fn quiet() -> Conditions {
        Conditions {
            samples: 20,
            windowed: 20,
            foreground: 20,
            interactive: 20,
            ..Conditions::default()
        }
    }

    /// The bench's animation advances one vsync-step per presented
    /// frame, so twelve seconds of animation takes twelve seconds only
    /// at full rate: at half the frames it takes twice as long.
    /// [`SCROLL_CEILING`] must sit past what the frame gate still
    /// publishes, or a run refused here would have been reported as
    /// merely slow — and it dies with no fps in it, blaming the screen.
    #[test]
    fn the_ceiling_cannot_refuse_a_run_the_frame_gate_would_publish() {
        let slowest = Duration::from_secs(12).div_f64(Limits::default().frame_share);
        assert!(
            slowest < SCROLL_CEILING,
            "{slowest:?} vs {SCROLL_CEILING:?}"
        );
    }

    /// A window nothing drew is the machine's fault and worth another
    /// run, however the application ended up looking.
    #[test]
    fn a_covered_window_is_the_hosts_fault() {
        let reading = Reading {
            conditions: Conditions {
                foreground: 8,
                ..quiet()
            },
            ..Reading::default()
        };
        let spoiled = verdict(reading, &defaults(), Ended::Covered, None)
            .expect_err("a covered window is not a reading");
        assert!(matches!(spoiled, Spoiled::Covered(_)), "{spoiled:?}");
        let (Spoiled::Host(said) | Spoiled::Run(said) | Spoiled::Covered(said)) = spoiled;
        assert!(said.contains("covered"), "{said}");
    }

    /// **A window that held the front was not covered**, and saying so
    /// sends a reader looking for a window that was never there. With
    /// no frames there is no evidence either way, so the sentence has
    /// to follow what the conditions do say.
    #[test]
    fn a_window_that_held_the_front_is_not_called_covered() {
        let reading = Reading {
            conditions: quiet(),
            ..Reading::default()
        };
        let spoiled = verdict(reading, &defaults(), Ended::Covered, None)
            .expect_err("a bench that produced nothing is not a reading");
        let (Spoiled::Host(said) | Spoiled::Run(said) | Spoiled::Covered(said)) = spoiled;
        assert!(!said.contains("was covered"), "{said}");
        assert!(said.contains("in front the whole time"), "{said}");
    }

    /// An application that answered nothing, on a machine with no
    /// complaint against it, is its own fault — and taking it again
    /// would produce the same nothing.
    #[test]
    fn an_application_that_said_nothing_is_its_own_fault() {
        let reading = Reading {
            conditions: quiet(),
            ..Reading::default()
        };
        let spoiled = verdict(reading, &defaults(), Ended::Done(true), None)
            .expect_err("a reading with no numbers in it");
        assert!(matches!(spoiled, Spoiled::Run(_)), "{spoiled:?}");
    }

    /// A complete reading on a quiet machine is a reading. Without
    /// this every other test here passes with the `Ok` arm deleted, and
    /// a verdict that never returns one refuses every measurement.
    #[test]
    fn a_complete_reading_on_a_quiet_machine_is_one() {
        let mut reading = Reading {
            conditions: quiet(),
            peak_working_set: 100,
            peak_private: 100,
            startup_ms: Some(10),
            first_chunk_ms: Some(5),
            total_ms: Some(8),
            ..Reading::default()
        };
        for line in [
            "perf_selection mode=none oid=none",
            "perf_complete selection=none details=false diff=false graph=true scrolled=false",
            "perf_done",
        ] {
            super::super::reading::absorb(line, &mut reading);
        }
        let opts = options(&["--repo", ".", "--no-select", "--no-scroll"]);
        verdict(reading, &opts, Ended::Done(true), None).expect("a reading with its numbers in it");
    }

    /// An application that left before saying anything is its own
    /// fault, and so is one whose wait could not be taken — neither is
    /// worth another four runs.
    #[test]
    fn an_application_that_left_is_not_retried() {
        for (ending, said) in [
            (Ended::Exited, "exited before"),
            (
                Ended::WaitFailed("waiting on the app failed: gone".into()),
                "waiting on the app failed",
            ),
        ] {
            let reading = Reading {
                conditions: quiet(),
                ..Reading::default()
            };
            let spoiled =
                verdict(reading, &defaults(), ending, None).expect_err("an app that said nothing");
            let (Spoiled::Host(text) | Spoiled::Run(text) | Spoiled::Covered(text)) = &spoiled;
            assert!(matches!(spoiled, Spoiled::Run(_)), "{spoiled:?}");
            assert!(text.contains(said), "{text}");
        }
    }

    /// A run nobody sampled says nothing about the machine, so taking
    /// it again would produce the same nothing. It is the run's own.
    #[test]
    fn a_run_the_sampler_never_watched_is_the_runs_own() {
        let spoiled = verdict(Reading::default(), &defaults(), Ended::Done(true), None)
            .expect_err("nothing sampled and nothing reported");
        assert!(matches!(spoiled, Spoiled::Run(_)), "{spoiled:?}");
        let (Spoiled::Host(text) | Spoiled::Run(text) | Spoiled::Covered(text)) = &spoiled;
        assert!(text.contains("never sampled"), "{text}");
    }

    /// `--allow-noisy` publishes what a slow machine produced, and the
    /// bench ceiling is a rate in disguise: an absolute one refuses
    /// every run below about a quarter of the screen's frame rate,
    /// which is the case the flag exists for.
    #[test]
    fn the_bench_ceiling_opens_with_every_other_gate() {
        let open = options(&["--repo", ".", "--allow-noisy"]);
        assert!(open.limits.quiet_percent.is_infinite());
        assert!(defaults().limits.quiet_percent.is_finite());
    }

    /// The machine is named first: a run that died at its deadline with
    /// the window minimised is a minimised window, not an application
    /// that stopped answering.
    #[test]
    fn the_machine_is_named_before_the_application() {
        let reading = Reading {
            conditions: Conditions {
                minimized: 12,
                ..quiet()
            },
            ..Reading::default()
        };
        let spoiled = verdict(reading, &defaults(), Ended::TimedOut, None)
            .expect_err("a minimised window is not a reading");
        let (Spoiled::Host(said) | Spoiled::Run(said) | Spoiled::Covered(said)) = spoiled;
        assert!(said.starts_with("the window was minimised"), "{said}");
        assert!(said.contains("did not report perf_done"), "{said}");
    }
}
