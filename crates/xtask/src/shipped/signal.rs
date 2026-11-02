//! A stop sent as a signal (the app's `app_backend/signals.rs`): an IDE's
//! SIGINT closes the window normally ([`super::stop`]). Then a window whose
//! launcher ignored SIGHUP, as `nohup` does, must stand through one and
//! close on SIGTERM ([`check_ignored_hangup`]).

use std::ffi::OsStr;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Child;
use std::time::Duration;

use crate::wait::{Budget, LOOK_AGAIN, Wait, stood};

const SIGHUP: i32 = 1;
pub(super) const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;
const SIG_IGN: usize = 1;
const SIG_ERR: usize = usize::MAX;

// SAFETY: transcribed from signal.h; integers in and out, and
// `sighandler_t` is pointer-sized.
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
    fn signal(signal: i32, handler: usize) -> usize;
}

pub(super) fn send(pid: u32, signal: i32) -> Result<(), String> {
    let pid = i32::try_from(pid).map_err(|e| e.to_string())?;
    // SAFETY: integers only, to the window this run started.
    if unsafe { kill(pid, signal) } != 0 {
        return Err(format!(
            "could not signal the shipped build: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// A launcher that ignored SIGHUP is obeyed: the window stands through
/// one, then closes on SIGTERM as on SIGINT.
pub(super) fn check_ignored_hangup(exe: &Path, path: &OsStr) -> Result<(), String> {
    let config = crate::keepsakes::keepsake_dir("shipped")?;
    let mut cmd = super::app_command(exe, path, &config);
    let ignore_hangup = || {
        // SAFETY: an integer and the ignore marker.
        if unsafe { signal(SIGHUP, SIG_IGN) } == SIG_ERR {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    };
    // SAFETY: between fork and exec the hook calls `signal` alone, which is
    // async-signal-safe.
    unsafe { cmd.pre_exec(ignore_hangup) };
    let mut wait = Wait::new(
        "the shipped build ignoring SIGHUP",
        Budget::whole(super::LOAD_CEILING),
        LOOK_AGAIN,
    );
    let mut window = super::Started::spawn(cmd, exe)?;
    let ending = super::wait_out(&mut window.child, &window.loaded, &mut wait);
    let verdict = match &ending {
        Ok(super::Ending::StillUp { .. }) => stand_through_hangup(&mut window.child),
        Ok(ending) => Err(format!(
            "the shipped build ignoring SIGHUP did not stand: {}",
            ending.account()
        )),
        Err(error) => Err(error.clone()),
    };
    let pid = window.child.id();
    let verdict = verdict.and_then(|stood_for| {
        super::close_after(&mut window.child, "SIGTERM", || send(pid, SIGTERM)).map(|()| stood_for)
    });
    let lines = window.finish(matches!(ending, Ok(super::Ending::LeftEarly { .. })));
    for line in &lines {
        println!("  | {line}");
    }
    let stood_for = verdict?;
    super::check_delivery(&lines)?;
    println!(
        "PASS: a SIGHUP its launcher ignored left the shipped build standing {:.1}s",
        stood_for.as_secs_f32()
    );
    Ok(())
}

fn stand_through_hangup(child: &mut Child) -> Result<Duration, String> {
    send(child.id(), SIGHUP)?;
    stood(super::STAND, LOOK_AGAIN, || {
        child.try_wait().map_err(|e| e.to_string()).transpose()
    })
    .map_err(|ended| match ended {
        Ok(status) => format!("a SIGHUP its launcher ignored ended the shipped build: {status}"),
        Err(error) => error,
    })
}
