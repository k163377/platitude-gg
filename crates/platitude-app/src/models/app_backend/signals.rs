//! A stop sent as a signal goes through the close gate the window's ✕
//! uses (`WindowQuitGate.qml`), as a launcher's stop through its console
//! does on Windows (`console.rs`).
//!
//! An IDE's Stop sends SIGINT (IntelliJ's `KillableProcessHandler`: to the
//! process tree it started, children first), a terminal's Ctrl+C sends it
//! to its foreground job, `kill` and a session's end send SIGTERM, and a
//! terminal that closes sends SIGHUP. `cargo run` execs the binary in its
//! own place, so each reaches the window itself; left at its default, each
//! ends the process where it stands, past the gate and the settings flush.
//!
//! - Each queues a close request; the gate waits a running write out, and
//!   the close saves the settings on its way.
//! - An IDE's SIGINT and a terminal's Ctrl+C reach git's children too (the
//!   same tree, the same job): git ends the write where it stands and
//!   takes its lock files back, and the gate passes once it has. Git stays
//!   in the window's job: one of its own would leave an ssh that asks on
//!   the terminal stopped (SIGTTIN). A `kill` of the window alone leaves
//!   git be.
//! - A signal the launcher set to be ignored stays ignored (`nohup`'s
//!   SIGHUP, a non-interactive shell's `&` and its SIGINT), as Unix
//!   programs keep it.
//! - No launcher is followed: a terminal signals its foreground job only,
//!   which a launcher that left (`cargo xtask launch`) no longer is.
//! - The handler only writes a byte down a socket; a thread of its own
//!   reads it and queues the request (`Feed` takes a lock).
//!
//! Headless: `cargo xtask shipped` (SIGINT and SIGTERM close the window
//! normally; a SIGHUP its launcher ignored leaves it standing).
#![expect(
    unsafe_code,
    reason = "signal dispositions have no std binding, and four signatures do not earn a dependency"
)]

use std::ffi::c_void;
use std::io::{self, Read};
use std::os::fd::IntoRawFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::{AppMsg, Feed};

/// The stops, by the numbers Linux and macOS share: SIGHUP, SIGINT, SIGTERM.
const STOPS: [i32; 3] = [1, 2, 15];
const SIG_DFL: usize = 0;
const SIG_IGN: usize = 1;
const SIG_ERR: usize = usize::MAX;

static DESTINATION: Mutex<Option<Arc<Feed<AppMsg>>>> = Mutex::new(None);

/// The handler's end of the socket [`relay`] reads: made once for the
/// process, -1 until then.
static WAKE: AtomicI32 = AtomicI32::new(-1);

/// `struct sigaction`, as far as [`disposition`] reads it: the handler
/// comes first on Linux and macOS alike, and the rest is room for what the
/// system writes after it (the whole struct is 152 bytes on glibc, 16 on
/// macOS).
#[repr(C)]
struct Action {
    handler: usize,
    rest: [u64; 32],
}

// SAFETY: transcribed from signal.h, unistd.h and errno.h; `sighandler_t`
// is pointer-sized, as the handler's address passed in it.
unsafe extern "C" {
    fn signal(signal: i32, handler: usize) -> usize;
    fn sigaction(signal: i32, action: *const Action, old: *mut Action) -> i32;
    fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    #[cfg_attr(target_os = "macos", link_name = "__error")]
    fn errno() -> *mut i32;
}

fn destination() -> MutexGuard<'static, Option<Arc<Feed<AppMsg>>>> {
    DESTINATION.lock().unwrap_or_else(|e| e.into_inner())
}

/// The stops this one caught, given back to their default when it drops.
pub(super) struct Listener {
    caught: Vec<i32>,
}

/// Catches the stops its launcher left on. `None`: it ignored them all.
pub(super) fn listen(feed: Arc<Feed<AppMsg>>) -> io::Result<Option<Listener>> {
    let mut destination = destination();
    if destination.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "stop listener already installed",
        ));
    }
    awaken()?;
    *destination = Some(feed);
    let mut caught = Vec::new();
    for stop in STOPS {
        let installed = disposition(stop).and_then(|now| {
            if now == SIG_IGN {
                return Ok(false);
            }
            // SAFETY: a static handler that only writes to a socket kept
            // open for the process.
            if unsafe { signal(stop, handler()) } == SIG_ERR {
                return Err(io::Error::last_os_error());
            }
            Ok(true)
        });
        match installed {
            Ok(true) => caught.push(stop),
            Ok(false) => {}
            Err(error) => {
                give_back(&caught);
                *destination = None;
                return Err(error);
            }
        }
    }
    if caught.is_empty() {
        *destination = None;
        return Ok(None);
    }
    Ok(Some(Listener { caught }))
}

fn give_back(caught: &[i32]) {
    for &stop in caught {
        // SAFETY: gives back the default this module replaced.
        unsafe { signal(stop, SIG_DFL) };
    }
}

/// How `stop` is handled now, read without changing it.
fn disposition(stop: i32) -> io::Result<usize> {
    let mut old = Action {
        handler: SIG_DFL,
        rest: [0; 32],
    };
    // SAFETY: no new action, and the old one into room past the size of
    // the system's `struct sigaction`.
    if unsafe { sigaction(stop, std::ptr::null(), &raw mut old) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(old.handler)
}

/// Makes the socket and its reader, once for the process.
fn awaken() -> io::Result<()> {
    if WAKE.load(Ordering::SeqCst) >= 0 {
        return Ok(());
    }
    let (wake, woken) = UnixStream::pair()?;
    // A full socket holds a wake already; the handler must never wait.
    wake.set_nonblocking(true)?;
    std::thread::Builder::new()
        .name("stop-signals".into())
        .spawn(move || relay(woken))?;
    WAKE.store(wake.into_raw_fd(), Ordering::SeqCst);
    Ok(())
}

/// Queues a close request for every wake, on a thread that may take a lock.
/// A relay that ends gives the stops it served their default back: caught
/// with nobody reading, they would end nothing.
fn relay(mut woken: UnixStream) {
    let mut bytes = [0_u8; 16];
    loop {
        match woken.read(&mut bytes) {
            Ok(0) => break,
            Ok(_) => {
                if let Some(feed) = destination().as_ref() {
                    feed.push_coalescing(AppMsg::Stop);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                tracing::warn!(%error, "stopped listening for stop signals");
                break;
            }
        }
    }
    for stop in STOPS {
        if disposition(stop).is_ok_and(|now| now == handler()) {
            give_back(&[stop]);
        }
    }
}

/// [`stopped`] as `signal` takes it.
fn handler() -> usize {
    stopped as extern "C" fn(i32) as usize
}

extern "C" fn stopped(_stop: i32) {
    let wake = WAKE.load(Ordering::SeqCst);
    if wake < 0 {
        return;
    }
    // SAFETY: the calling thread's errno, kept across the write so the
    // code this interrupted reads its own.
    let kept = unsafe { *errno() };
    let byte = 0_u8;
    // SAFETY: write is async-signal-safe; an open socket and one byte.
    unsafe { write(wake, (&raw const byte).cast(), 1) };
    // SAFETY: as above.
    unsafe { *errno() = kept };
}

impl Drop for Listener {
    fn drop(&mut self) {
        give_back(&self.caught);
        *destination() = None;
    }
}
