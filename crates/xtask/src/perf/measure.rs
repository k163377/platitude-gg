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
use super::sampler::sample_memory;
use super::{Options, SAMPLE_MS, artifacts};

/// How long the app is left alone after `perf_done` even when the run
/// asked for no settling: enough for at least one more sample, so the
/// final reading is one the sampler actually took rather than whatever
/// it happened to hold when completion arrived.
const AFTER_DONE_MS: u64 = 250;

/// The process this run measures, and everything it is told.
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
) -> Result<Reading, String> {
    // A config directory per process, so another perf process or a previous
    // run's restored state cannot decide what this one does.
    let (config_dir, log, samples) = artifacts::open_run(run_dir, screen)?;
    let mut cmd = command(exe, path, root, opts, &config_dir);
    let started = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    let pid = child.id();
    let stderr = child.stderr.take();
    let (done_rx, reader) = read_app(stderr, started, log);

    let deadline = started + Duration::from_millis(opts.watchdog_ms);
    let sampling_end = deadline + Duration::from_millis(opts.settle_ms + AFTER_DONE_MS);
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
