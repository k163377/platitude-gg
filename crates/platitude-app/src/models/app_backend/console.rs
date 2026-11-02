//! A launcher's stop through its console goes through the close gate the
//! window's ✕ uses (`WindowQuitGate.qml`); elsewhere the stop is a signal
//! (`signals.rs`).
//!
//! A launcher stops what it started through that process's console: an
//! IDE's Stop (IntelliJ's `KillableProcessHandler` on Windows 10 and later:
//! WinP's Ctrl+C to the console of the process it started), a terminal's
//! Ctrl+C. `cargo run` waits for its child with Ctrl+C handled as nothing,
//! and a release build is GUI-subsystem with no console of its own, so the
//! stop would reach nobody and both processes would stay up.
//!
//! The window joins its parent's console only when that parent is `cargo`
//! (an IDE's run, a terminal's `cargo run`; user decision — ask before
//! widening), and only while cargo lives: should it go first, the window
//! gives the console back and is a desktop window again that no terminal's
//! Ctrl+C or close reaches. No other parent is joined: a shell like cmd or
//! PowerShell returns to its prompt without waiting for a GUI program, and
//! its console's Ctrl+C is then for whatever it runs next. A console build
//! is on the console it was started with, whoever started it, and listens
//! there.
//!
//! - Ctrl+C and Ctrl+Break queue a close request; the gate waits a running
//!   write out, and the close saves the settings on its way.
//! - Closing the console: the system ends the process as soon as the
//!   handler returns, so the handler queues the same request and never
//!   returns — the window's own close ends the process, or the system's
//!   close timeout (`SPI_GETHUNGAPPTIMEOUT`) does.
//! - Logoff and shutdown never call the handler of a process that loads
//!   user32; Qt receives them as `WM_QUERYENDSESSION`.
//! - A launcher that turned Ctrl+C off for its child
//!   (`CREATE_NEW_PROCESS_GROUP`, `SetConsoleCtrlHandler(NULL, TRUE)`) is
//!   obeyed, as console programs obey it; its Ctrl+Break still arrives.
//! - `AttachConsole` and `FreeConsole` empty the process's handler list:
//!   the handler goes in after the attach, and the release takes it out.
//!
//! Headless: `cargo xtask shipped` (a waiting cargo's Ctrl+C closes the
//! window normally; once cargo left, or under a parent that is not cargo,
//! the console's Ctrl+C does not).
#![expect(unsafe_code, reason = "Windows console control has no std binding")]

use std::ffi::{OsString, c_void};
use std::io;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use super::{AppMsg, Feed};

/// `STD_INPUT_HANDLE`, `STD_OUTPUT_HANDLE`, `STD_ERROR_HANDLE`.
const STD_HANDLES: [u32; 3] = [-10_i32 as u32, -11_i32 as u32, -12_i32 as u32];
const CTRL_CLOSE_EVENT: u32 = 2;
const ERROR_INVALID_HANDLE: i32 = 6;
const ERROR_INVALID_PARAMETER: i32 = 87;
const TH32CS_SNAPPROCESS: u32 = 2;
const SYNCHRONIZE: u32 = 0x0010_0000;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const INFINITE: u32 = u32::MAX;
const WAIT_OBJECT_0: u32 = 0;

/// The log line of a release; `cargo xtask shipped` waits for it.
const RELEASED: &str = "cargo left; its console no longer reaches the window";

static DESTINATION: Mutex<Option<Arc<Feed<AppMsg>>>> = Mutex::new(None);

/// `PROCESSENTRY32W`.
#[repr(C)]
struct ProcessEntry {
    size: u32,
    usage: u32,
    id: u32,
    heap: usize,
    module: u32,
    threads: u32,
    parent: u32,
    priority: i32,
    flags: u32,
    image: [u16; 260],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AttachConsole(process: u32) -> i32;
    fn FreeConsole() -> i32;
    fn SetConsoleCtrlHandler(handler: Option<extern "system" fn(u32) -> i32>, add: i32) -> i32;
    fn GetStdHandle(kind: u32) -> *mut c_void;
    fn SetStdHandle(kind: u32, handle: *mut c_void) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, process: u32) -> *mut c_void;
    fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
    fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
    fn OpenProcess(access: u32, inherit: i32, process: u32) -> *mut c_void;
    fn GetCurrentProcess() -> *mut c_void;
    fn GetProcessTimes(
        process: *mut c_void,
        created: *mut u64,
        exited: *mut u64,
        kernel: *mut u64,
        user: *mut u64,
    ) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
    fn GetConsoleProcessList(ids: *mut u32, count: u32) -> u32;
    fn QueryFullProcessImageNameW(
        process: *mut c_void,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
}

fn destination() -> MutexGuard<'static, Option<Arc<Feed<AppMsg>>>> {
    DESTINATION.lock().unwrap_or_else(|e| e.into_inner())
}

pub(super) struct Listener;

/// Listens on the console a stop reaches the window through: its own, or
/// its cargo parent's, joined. `None`: there is none — no console of its
/// own, and no cargo parent with one.
pub(super) fn listen(feed: Arc<Feed<AppMsg>>) -> io::Result<Option<Listener>> {
    let mut destination = destination();
    if destination.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "console listener already installed",
        ));
    }
    let lender = if has_console() {
        None
    } else {
        let Some((pid, parent)) = cargo_parent()? else {
            return Ok(None);
        };
        // AttachConsole can point the standard handles at the console. Keep
        // inherited pipes (the IDE's output, a verification run's logs)
        // where they were.
        // SAFETY: GetStdHandle only reads this process's handle table.
        let handles = STD_HANDLES.map(|kind| unsafe { GetStdHandle(kind) });
        // SAFETY: attaches this process to its parent's console; it creates no window.
        if unsafe { AttachConsole(pid) } == 0 {
            let error = io::Error::last_os_error();
            return match error.raw_os_error() {
                // Cargo has no console, or has just left.
                Some(ERROR_INVALID_HANDLE | ERROR_INVALID_PARAMETER) => Ok(None),
                _ => Err(error),
            };
        }
        if let Err(error) = restore(handles) {
            release();
            return Err(error);
        }
        Some(parent)
    };
    *destination = Some(feed);
    // SAFETY: a static callback; its destination is held until Listener drops.
    if unsafe { SetConsoleCtrlHandler(Some(interrupted), 1) } == 0 {
        let error = io::Error::last_os_error();
        *destination = None;
        if lender.is_some() {
            release();
        }
        return Err(error);
    }
    if let Some(parent) = lender {
        let follower = std::thread::Builder::new()
            .name("console-cargo".into())
            .spawn(move || follow(&parent));
        if let Err(error) = follower {
            *destination = None;
            release();
            return Err(error);
        }
    }
    Ok(Some(Listener))
}

/// Whether this process is on a console already: a console build, on the
/// one it was started with.
fn has_console() -> bool {
    let mut id = 0_u32;
    // SAFETY: room for one id; a console with more answers their count
    // without writing, and no console answers zero.
    let count = unsafe { GetConsoleProcessList(&raw mut id, 1) };
    count != 0
}

/// This process's parent when it is `cargo`, and its id, held open so the
/// id cannot pass to another process while the window follows it. `None`:
/// the parent is gone, is not cargo, or the id already names a process
/// younger than this one.
fn cargo_parent() -> io::Result<Option<(u32, OwnedHandle)>> {
    let me = std::process::id();
    // SAFETY: a process snapshot takes no pointer.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if raw as isize == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a handle just returned, owned by nothing else.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: integers and an array of them, all valid when zeroed.
    let mut entry: ProcessEntry = unsafe { std::mem::zeroed() };
    entry.size = u32::try_from(size_of::<ProcessEntry>()).map_err(io::Error::other)?;
    let mut pid = None;
    // SAFETY: the live snapshot and a sized entry, as Toolhelp asks.
    let mut more = unsafe { Process32FirstW(snapshot.as_raw_handle(), &raw mut entry) } != 0;
    while more {
        if entry.id == me {
            pid = Some(entry.parent);
            break;
        }
        // SAFETY: as above.
        more = unsafe { Process32NextW(snapshot.as_raw_handle(), &raw mut entry) } != 0;
    }
    let Some(pid) = pid else {
        return Ok(None);
    };
    // SAFETY: opens a handle with wait and query rights only.
    let raw = unsafe { OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if raw.is_null() {
        return Ok(None);
    }
    // SAFETY: a handle just returned, owned by nothing else.
    let parent = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: the pseudo-handle of this process needs no closing.
    if created(parent.as_raw_handle())? > created(unsafe { GetCurrentProcess() })? {
        return Ok(None);
    }
    if !is_cargo(parent.as_raw_handle()) {
        return Ok(None);
    }
    Ok(Some((pid, parent)))
}

/// Whether `process` runs an executable named `cargo`. A name that cannot
/// be read is a parent already gone, or none this window joins.
fn is_cargo(process: *mut c_void) -> bool {
    let mut name = [0_u16; 1024];
    let Ok(mut size) = u32::try_from(name.len()) else {
        return false;
    };
    // SAFETY: a process handle with query rights, and a buffer of the size
    // passed.
    if unsafe { QueryFullProcessImageNameW(process, 0, name.as_mut_ptr(), &raw mut size) } == 0 {
        return false;
    }
    let read = usize::try_from(size).map_or(0, |size| size.min(name.len()));
    PathBuf::from(OsString::from_wide(&name[..read]))
        .file_stem()
        .is_some_and(|stem| stem.eq_ignore_ascii_case("cargo"))
}

fn created(process: *mut c_void) -> io::Result<u64> {
    let (mut created, mut exited, mut kernel, mut user) = (0, 0, 0, 0);
    // SAFETY: a process handle with query rights, and four writable times.
    let read = unsafe {
        GetProcessTimes(
            process,
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        )
    };
    if read == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(created)
}

fn restore(handles: [*mut c_void; 3]) -> io::Result<()> {
    for (kind, handle) in STD_HANDLES.into_iter().zip(handles) {
        if !handle.is_null() && handle as isize != -1 {
            // SAFETY: restores an existing inherited handle without closing it.
            if unsafe { SetStdHandle(kind, handle) } == 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

fn release() {
    // SAFETY: detaches this process from the console it attached to.
    unsafe { FreeConsole() };
}

/// Gives the console back once the cargo that lent it leaves.
fn follow(parent: &OwnedHandle) {
    // SAFETY: a process handle opened with SYNCHRONIZE, alive for the call.
    if unsafe { WaitForSingleObject(parent.as_raw_handle(), INFINITE) } != WAIT_OBJECT_0 {
        tracing::warn!(error = %io::Error::last_os_error(), "could not follow cargo");
        return;
    }
    release();
    tracing::info!("{RELEASED}");
}

extern "system" fn interrupted(event: u32) -> i32 {
    // Logoff and shutdown: never sent here (user32 is loaded).
    if event > CTRL_CLOSE_EVENT {
        return 0;
    }
    // Windows calls on a thread of its own. Feed only queues a Qt
    // invocation; the signal, the close gate and the settings flush run on
    // the main thread.
    if let Some(feed) = destination().as_ref() {
        feed.push_coalescing(AppMsg::Stop);
    } else {
        return 0;
    }
    if event == CTRL_CLOSE_EVENT {
        loop {
            std::thread::park();
        }
    }
    1
}

impl Drop for Listener {
    fn drop(&mut self) {
        // Waits for any callback queuing a request before removing its owner.
        // The handler stays: with no destination it passes every event on, as
        // no handler would, and removing it could wait on the system's call of
        // it that a console close parks.
        *destination() = None;
    }
}
