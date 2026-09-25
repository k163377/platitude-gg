//! One measured run of the app: the process it starts, the memory and
//! host conditions sampled off it, and the wait that ends on the app's
//! own completion.
//!
//! The window is placed on one named screen through the run's
//! `state.toml`: screens need not share a rate, and the screen sets the
//! rate frames can arrive at (`perf::display`).

use std::process::{Command, Stdio};
use std::time::Duration;

use super::display::Screen;
use super::reading::{Reading, missing, read_app};
use super::{Options, SAMPLE_MS, artifacts, attribution, sampler};
use crate::wait::{Budget, Expired, Wait};

/// The least the app is left alone after `perf_done`, even with no
/// settling asked: one more sample, so the final reading is one the
/// sampler actually took.
const AFTER_DONE_MS: u64 = 250;

/// The most the scroll bench (twelve seconds of animation) is given.
/// Past it the bench is not slow but stopped: nothing draws the window,
/// so the animation does not advance. It sits just past the twenty-four
/// seconds a bench at half the screen's rate (the least the frame gate
/// publishes) takes; more only makes a covered window cost more.
const SCROLL_CEILING: Duration = Duration::from_secs(30);

fn scroll_deadline() -> Wait {
    Wait::new(
        "the scroll bench",
        Budget::whole(SCROLL_CEILING),
        Duration::ZERO,
    )
}

/// Why a run produced no reading, which decides whether taking it again
/// could help ([`Spoiled::budget`]).
#[derive(Debug)]
pub(super) enum Spoiled {
    /// The machine: a lock, a minimised window, a screen change, a busy
    /// machine.
    Host(String),
    /// A bench that stood still with nothing else wrong with the
    /// machine: the window was covered, or the screen was off.
    Covered(String),
    Run(String),
}

impl From<String> for Spoiled {
    /// Anything that failed before the machine could be asked (a
    /// directory, the spawn, the sampler) is the run's own.
    fn from(said: String) -> Self {
        Self::Run(said)
    }
}

impl Spoiled {
    pub(super) const CAUSES: usize = 3;

    /// Which cause this is, as an index below [`Self::CAUSES`].
    pub(super) fn cause(&self) -> usize {
        match self {
            Self::Host(_) => 0,
            Self::Covered(_) => 1,
            Self::Run(_) => 2,
        }
    }

    /// How many attempts a run spoiled this way is worth: the machine will
    /// not have spoiled the next one; what covers a window either passes
    /// or stays, so one more ceiling tells; the application will stop
    /// answering again.
    pub(super) fn budget(&self, retries: u32) -> u32 {
        match self {
            Self::Host(_) => retries,
            Self::Covered(_) => retries.min(1),
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
/// A build with no harness answers no `PGG_AUTO_*` knob: the repository
/// it opens comes from the `state.toml` in its config directory
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
        .env("PGG_CONFIG_DIR", config_dir)
        .env("PGG_LOG", &opts.log)
        // Qt names the adapter and backend it chose: runs on different
        // adapters are not each other's control.
        .env("QSG_INFO", "1")
        // A real window, deliberately: `verify-ui` runs offscreen, and
        // offscreen Qt builds no scene graph worth measuring.
        .env_remove("QT_QPA_PLATFORM")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // `--software`: the software scene graph's frames go through the
    // backing store, so it runs with the display off, where the D3D swap
    // chain never presents a frame. What its numbers mean is
    // `report::memory`'s to say.
    if opts.software {
        cmd.env("QT_QUICK_BACKEND", "software");
    }
    if !opts.harness {
        return cmd;
    }
    // The app says `perf_done` once every requested measurement has
    // answered; the parent, not the app, ends the process, so it takes the
    // last memory sample and reaps exactly the child it started.
    cmd.env("PGG_AUTO_PERF", "1")
        .env("PGG_PERF_SELECTION", &opts.selection)
        .env("PGG_PERF_OID", &opts.oid)
        .env("PGG_PERF_FILE", &opts.file)
        .env("PGG_PERF_CASES", super::cases::encode(&opts.cases))
        .env("PGG_PERF_CYCLES", opts.cycles.to_string())
        .env("PGG_PERF_COMPLETION", &opts.completion)
        .env(
            "PGG_PERF_DIFF_SCROLL",
            if opts.diff_scroll { "1" } else { "0" },
        )
        .env("PGG_PERF_DIFF", if opts.diff { "1" } else { "0" })
        .env(
            "PGG_PERF_TRACE_FRAMES",
            if opts.trace_frames { "1" } else { "0" },
        );
    for (asked, name, value) in [
        (opts.open, "PGG_AUTO_OPEN", opts.repo.display().to_string()),
        (opts.select, "PGG_AUTO_SELECT", "1".to_string()),
        (opts.scroll, "PGG_AUTO_SCROLL", "1".to_string()),
        (opts.breakdown, "PGG_MEM_REPORT", "1".to_string()),
        (opts.font_walk, "PGG_PERF_FONT_WALK", "1".to_string()),
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
    // Armed before the clock starts, so the sampler's and the attribution
    // script's compiles stay out of the timed window (`sampler::Armed`).
    // The sum cannot overflow (`options::settle` holds the two under a
    // day); the attribution's ceiling keeps the sampler watching through
    // the walk.
    let mut window = Duration::from_millis(opts.watchdog_ms + opts.settle_ms + AFTER_DONE_MS);
    let attributing = if opts.attribute {
        window += attribution::CEILING;
        Some(attribution::arm()?)
    } else {
        None
    };
    let armed = sampler::arm(window, samples, opts.software)?;
    // Started suspended: the sampler puts it in its job object before
    // letting it run, so every git the app spawns is inside the job. The
    // clock starts when the sampler resumes it.
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
            // Never resumed, so never running: ended here.
            if let Err(error) = child.kill() {
                println!("  note: could not end the app that never ran: {error}");
            }
            return Err(Spoiled::Run(said));
        }
    };
    let mut run = Wait::since(
        started,
        "the app",
        Budget::whole(Duration::from_millis(opts.watchdog_ms)),
        Duration::from_millis(SAMPLE_MS),
    );
    let stderr = child.stderr.take();
    let (done_rx, scroll, reader) = read_app(stderr, started, log, opts.harness);
    // `perf_done` is the success edge; the deadline only guards an app
    // that stopped answering. `--allow-noisy` lifts the bench ceiling too:
    // vsync-stepped animation finishes on time only at the screen's full
    // rate, so the ceiling would refuse the slow machine that flag exists
    // to publish.
    let bounded = !opts.limits.quiet_percent.is_infinite();
    let mut bench: Option<Wait> = None;
    let mut scroll_generation = 0;
    let ended = loop {
        if let Ok(success) = done_rx.try_recv() {
            break Ended::Done(success);
        }
        match child.try_wait() {
            Ok(Some(_)) => break Ended::Exited,
            Ok(None) => {}
            Err(e) => break Ended::WaitFailed(format!("waiting on the app failed: {e}")),
        }
        // The bench's own deadline, counted from the look that first saw
        // it running: without it a covered window waits out the whole
        // watchdog and is blamed on the app.
        if scroll.running() {
            if scroll.generation() != scroll_generation {
                scroll_generation = scroll.generation();
                bench = None;
            }
            let bench = bench.get_or_insert_with(scroll_deadline);
            if bounded && bench.check("a frame").is_err() {
                let _ = child.kill();
                break Ended::Covered;
            }
        }
        if let Err(expired) = run.look_again("perf_done") {
            let _ = child.kill();
            break Ended::TimedOut(expired);
        }
    };
    let done = matches!(ended, Ended::Done(true));
    // Held idle first, so the last reading is of a process that stopped
    // working; the same sampler still runs, so the peak and the settled
    // value are one series.
    if done {
        // waits(measured): the settling stretch the last reading is taken after — the
        // run is done and nothing is waited for; the stretch is a condition of the
        // reading (`settle_ms`)
        std::thread::sleep(Duration::from_millis(opts.settle_ms.max(AFTER_DONE_MS)));
    }
    let held = sampler.read();
    // After the settled reading and before the kill: the process the
    // reading was of, without the walk's own touches in that reading. A
    // run that never settled has nothing to attribute; dropping the armed
    // script ends it.
    let walked = done && attributing.is_some();
    let attributed = match attributing {
        Some(armed) if done => attribution::record(armed, child.id(), run_dir),
        _ => None,
    };
    let _ = child.kill();
    let _ = child.wait();
    let mut reading = reader.join().unwrap_or_default();
    let finished = sampler.finish()?;
    // The walk is instrumentation: where one ran, the peak and the
    // conditions are read off the series as it stood before it, and
    // memory.csv keeps what the walk itself did.
    let series = if walked { &held } else { &finished };
    reading.attribution = attributed;
    reading.perf_done |= done;
    reading.peak_working_set = series.peak_working_set;
    reading.os_peak_working_set = series.os_peak_working_set;
    reading.peak_private = series.peak_private;
    reading.settled_working_set = if done && opts.settle_ms > 0 {
        held.last.as_ref().map_or(0, |sample| sample.working_set)
    } else {
        0
    };
    reading.conditions = series.conditions.clone();
    // Against the same series (`fonts::FontWalk::weigh`).
    reading.font_walk = reading
        .font_walk
        .take()
        .map(|walk| walk.weigh(&series.history));
    // Written before the verdict: a run refused for the machine's state is
    // exactly the one whose numbers a later reader wants to see.
    let _ = std::fs::write(run_dir.join("reading.txt"), format!("{reading:#?}"));
    verdict(reading, opts, ended, screen.map(|s| s.name.as_str()))
}

/// How the wait ended.
enum Ended {
    /// The app reported `perf_done`, and whether it called the scenario
    /// a success. A false here is not an ending of its own — what went
    /// wrong is in the reading.
    Done(bool),
    /// The process left before saying anything.
    Exited,
    /// The scroll bench stood still past [`SCROLL_CEILING`].
    Covered,
    /// The app reported no end within its watchdog and was killed.
    TimedOut(Expired),
    WaitFailed(String),
}

/// Whether the run is a reading, and if not whose fault that was.
///
/// The host is asked first, whatever else went wrong: a dark screen or a
/// locked session freezes the scroll's animation and the run dies at its
/// deadline, which would otherwise read as an app that stopped answering.
fn verdict(
    reading: Reading,
    opts: &Options,
    ended: Ended,
    pinned: Option<&str>,
) -> Result<Reading, Spoiled> {
    let covered = matches!(ended, Ended::Covered);
    let ending = match ended {
        Ended::WaitFailed(said) => Some(said),
        Ended::TimedOut(expired) => Some(format!("{expired}, and the run was killed")),
        Ended::Exited => Some("the app exited before reporting perf_done".into()),
        Ended::Done(_) | Ended::Covered => None,
    };
    let host = reading
        .conditions
        .complaint(&opts.limits, pinned, opts.software);
    // A bench that stood still on an otherwise quiet machine: its own
    // kind, because it is retried its own way.
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

/// What to say about a bench that never advanced. With no frames, the
/// foreground share picks the sentence: a window that lost the front was
/// plausibly covered; calling one that held it covered sends a reader
/// looking for a window that was never there.
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

    /// The animation advances one vsync-step per presented frame, so at
    /// half the frames the bench takes twice as long. [`SCROLL_CEILING`]
    /// must sit past what the frame gate still publishes, or a merely
    /// slow run dies with no fps in it, blaming the screen.
    #[test]
    fn the_ceiling_cannot_refuse_a_run_the_frame_gate_would_publish() {
        let slowest = Duration::from_secs(12).div_f64(Limits::default().frame_share);
        assert!(
            slowest < SCROLL_CEILING,
            "{slowest:?} vs {SCROLL_CEILING:?}"
        );
    }

    /// A window nothing drew is `Covered`, however the application ended
    /// up looking.
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

    /// Without this every other test here passes with the `Ok` arm
    /// deleted.
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

    /// An early exit and a wait that could not be taken are both the
    /// run's own.
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

    /// A run nobody sampled says nothing about the machine.
    #[test]
    fn a_run_the_sampler_never_watched_is_the_runs_own() {
        let spoiled = verdict(Reading::default(), &defaults(), Ended::Done(true), None)
            .expect_err("nothing sampled and nothing reported");
        assert!(matches!(spoiled, Spoiled::Run(_)), "{spoiled:?}");
        let (Spoiled::Host(text) | Spoiled::Run(text) | Spoiled::Covered(text)) = &spoiled;
        assert!(text.contains("never sampled"), "{text}");
    }

    /// The bench ceiling is keyed off `quiet_percent`, so `--allow-noisy`
    /// must make it infinite (the ceiling's reason is in `measure`).
    #[test]
    fn the_bench_ceiling_opens_with_every_other_gate() {
        let open = options(&["--repo", ".", "--allow-noisy"]);
        assert!(open.limits.quiet_percent.is_infinite());
        assert!(defaults().limits.quiet_percent.is_finite());
    }

    /// A run that died at its deadline with the window minimised is a
    /// minimised window.
    #[test]
    fn the_machine_is_named_before_the_application() {
        let reading = Reading {
            conditions: Conditions {
                minimized: 12,
                ..quiet()
            },
            ..Reading::default()
        };
        let spoiled = verdict(reading, &defaults(), Ended::TimedOut(spent()), None)
            .expect_err("a minimised window is not a reading");
        let (Spoiled::Host(said) | Spoiled::Run(said) | Spoiled::Covered(said)) = spoiled;
        assert!(said.starts_with("the window was minimised"), "{said}");
        assert!(said.contains("still waiting for perf_done"), "{said}");
        assert!(said.ends_with("and the run was killed"), "{said}");
    }

    /// An expired run wait: a budget of nothing is spent at the first look.
    fn spent() -> Expired {
        Wait::new("the app", Budget::whole(Duration::ZERO), Duration::ZERO)
            .check("perf_done")
            .expect_err("a budget of nothing is spent at once")
    }
}
