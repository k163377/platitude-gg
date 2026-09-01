//! One measured run of the app: the process it starts, the memory it
//! samples off it while it runs, and the wait that ends on the app's own
//! completion rather than on a clock.
//!
//! **A real window, deliberately.** `verify-ui` runs offscreen, and
//! offscreen Qt builds no scene graph worth measuring.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::reading::{Reading, missing, read_app};
use super::sampler::{sample_last, sample_memory, sample_once};
use super::{Options, SAMPLE_MS, artifacts};

pub(super) fn measure(
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
