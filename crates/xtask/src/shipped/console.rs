//! A launcher's stop through its console (the app's
//! `app_backend/console.rs`), checked in an invisible console of its own:
//! the runs here send Ctrl+C, which reaches every process on the console it
//! is sent to, and the caller's — a gate's, a terminal's — must not get it.
//!
//! The window joins only a parent named `cargo`, so `shipped` starts itself
//! again as the host of that console from a link named so ([`run_host`]).
//! The host is the cargo that waits: the window its QML check started is
//! stopped with Ctrl+C, and must close normally ([`super::stop`]). Then a
//! cargo that leaves ([`LEAVING`], the host's child on the same console):
//! once the window says it gave the console back, the console's Ctrl+C must
//! leave it standing ([`check_left_cargo`]). Last a parent that is not
//! cargo ([`OTHER`], a second link beside the first) and stays up beside the
//! window, as a shell does: the window never joins, and the console's Ctrl+C
//! must leave it standing ([`check_other_parent`]).

use std::ffi::c_void;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use crate::app_out::lines;
use crate::wait::{Budget, LOOK_AGAIN, Wait, stood};

/// Which run of `shipped` this is: unset for the caller, else one of the
/// three below.
const ROLE: &str = "PGG_SHIPPED_CONSOLE";
const HOST: &str = "host";
/// The cargo that starts the window on the host's console and leaves.
const LEAVING: &str = "leaving";
/// The parent that is not cargo and stays up beside the window.
const OTHER: &str = "other";

/// The links [`run_host`] runs this runner from: the one parent name the
/// window joins, and one it does not.
const CARGO_LINK: &str = "cargo.exe";
const OTHER_LINK: &str = "shell.exe";

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CTRL_C_EVENT: u32 = 0;
const SYNCHRONIZE: u32 = 0x0010_0000;
const PROCESS_TERMINATE: u32 = 0x0001;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const WAIT_OBJECT_0: u32 = 0;

/// The app's line this reads, kept by hand beside the app's
/// (`app_backend/console.rs`), as [`super::LOADED`] is.
const RELEASED: &str = "cargo left; its console no longer reaches the window";

/// The app's line when it could not listen at all
/// (`app_backend/lifecycle.rs`), kept by hand as [`RELEASED`] is.
const LISTEN_FAILED: &str = "could not listen for a stop from outside";

/// The parent's line naming the window it started.
const LAUNCHED: &str = "launched the shipped build, pid ";

/// How long a line already written may take to be read: a line through a
/// pipe, or the window's word woken by its parent's exit — many times
/// over.
const ANSWER_CEILING: Duration = Duration::from_secs(30);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(handler: Option<extern "system" fn(u32) -> i32>, add: i32) -> i32;
    fn GenerateConsoleCtrlEvent(event: u32, group: u32) -> i32;
    fn GetConsoleProcessList(ids: *mut u32, count: u32) -> u32;
    fn OpenProcess(access: u32, inherit: i32, process: u32) -> *mut c_void;
    fn QueryFullProcessImageNameW(
        process: *mut c_void,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
    fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
    fn TerminateProcess(process: *mut c_void, code: u32) -> i32;
}

fn role() -> Option<String> {
    std::env::var(ROLE).ok()
}

pub(super) fn is_host() -> bool {
    role().as_deref() == Some(HOST)
}

/// The run of a parent the host started, which holds no room: the host
/// does. `None` for the caller and the host.
pub(super) fn run_as_parent() -> Option<Result<(), String>> {
    match role().as_deref() {
        Some(LEAVING) => Some(start_and_leave()),
        Some(OTHER) => Some(stand_beside()),
        _ => None,
    }
}

/// Runs `shipped` again as the host of an invisible console, from a link
/// named `cargo` in a directory of its own, and relays what it said.
pub(super) fn run_host() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = crate::verify::claim_dir(crate::temp::root(), "shipped-parents")?;
    let ran = link(&exe, &dir, CARGO_LINK)
        .and_then(|host| link(&exe, &dir, OTHER_LINK).map(|_| host))
        .and_then(|host| {
            Command::new(host)
                .args(["shipped", "--no-build"])
                .env(ROLE, HOST)
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("could not start the console host: {e}"))
        });
    // Every run makes its own; none is worth keeping.
    let _ = std::fs::remove_dir_all(&dir);
    let output = ran?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(format!("the console host failed: {}", output.status));
    }
    Ok(())
}

/// `exe` under `name` in `dir`: a hard link, which costs no copy, or a copy
/// where the two are on different volumes.
fn link(exe: &Path, dir: &Path, name: &str) -> Result<PathBuf, String> {
    let linked = dir.join(name);
    std::fs::hard_link(exe, &linked)
        .or_else(|_| std::fs::copy(exe, &linked).map(|_| ()))
        .map_err(|e| {
            format!(
                "could not put {} at {}: {e}",
                exe.display(),
                linked.display()
            )
        })?;
    Ok(linked)
}

extern "system" fn handled(event: u32) -> i32 {
    i32::from(event == CTRL_C_EVENT)
}

/// The host's Ctrl+C reaches every process on its console: the host's own
/// processes let it pass.
pub(super) fn protect() -> Result<(), String> {
    // SAFETY: a process-lifetime callback; no child inherits it.
    if unsafe { SetConsoleCtrlHandler(Some(handled), 1) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

/// Ctrl+C to every process on the host's console.
pub(super) fn send() -> Result<(), String> {
    // SAFETY: the host's console is its own (`run_host`); group zero is
    // every process on it.
    if unsafe { GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0) } == 0 {
        return Err(format!(
            "could not send Ctrl+C: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// Starts the window with this process's output, which the host reads, and
/// names it there.
fn start_window() -> Result<Child, String> {
    let root = crate::tree::workspace_root();
    let path = crate::qt::path_with_qt()?;
    let exe =
        crate::app_build::shipped_exe(&root, &path, crate::app_build::BuildEnv::default(), false)?;
    let config = crate::keepsakes::keepsake_dir("shipped")?;
    let child = super::app_command(&exe, &path, &config)
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    println!("{LAUNCHED}{}", child.id());
    Ok(child)
}

/// The cargo that leaves: starts the window on the host's console, waits
/// until the window has joined it, and goes. A cargo gone before the join is
/// a parent already gone, which the window never joins.
fn start_and_leave() -> Result<(), String> {
    let mut child = start_window()?;
    let mut wait = Wait::new(
        "the leaving cargo",
        Budget::whole(super::LOAD_CEILING),
        LOOK_AGAIN,
    );
    while !on_console(child.id())? {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!(
                "the shipped build ended before it joined the console: {status}"
            ));
        }
        wait.look_again("the window on its console")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The parent that is not cargo: starts the window and stays up beside it,
/// through the host's Ctrl+C, until the window goes.
fn stand_beside() -> Result<(), String> {
    protect()?;
    start_window()?.wait().map_err(|e| e.to_string())?;
    Ok(())
}

fn on_console(pid: u32) -> Result<bool, String> {
    let mut ids = [0_u32; 64];
    let room = u32::try_from(ids.len()).map_err(|e| e.to_string())?;
    // SAFETY: a writable buffer of the length passed.
    let count = unsafe { GetConsoleProcessList(ids.as_mut_ptr(), room) };
    if count == 0 {
        return Err(format!(
            "could not list the console's processes: {}",
            std::io::Error::last_os_error()
        ));
    }
    let listed = usize::try_from(count).map_err(|e| e.to_string())?;
    Ok(ids[..listed.min(ids.len())].contains(&pid))
}

/// What a parent's run and its window said, as it arrives.
#[derive(Default)]
struct Heard {
    window: AtomicU32,
    loaded: AtomicBool,
    released: AtomicBool,
}

fn listen(
    pipe: impl std::io::Read + Send + 'static,
    heard: &Arc<Heard>,
) -> std::thread::JoinHandle<Vec<String>> {
    let heard = Arc::clone(heard);
    std::thread::spawn(move || {
        let mut said = Vec::new();
        for line in lines(pipe) {
            if let Some(pid) = line.strip_prefix(LAUNCHED).and_then(|pid| pid.parse().ok()) {
                heard.window.store(pid, Ordering::SeqCst);
            }
            if super::said_loaded(&line) {
                heard.loaded.store(true, Ordering::SeqCst);
            }
            if line.contains(RELEASED) {
                heard.released.store(true, Ordering::SeqCst);
            }
            said.push(line);
        }
        said
    })
}

/// The window a parent started, by its id: no `Child` here holds it.
struct Window(OwnedHandle);

impl Window {
    /// Opens the window by `pid`, and only if that id is still `exe`.
    fn open(pid: u32, exe: &Path) -> Result<Self, String> {
        // SAFETY: opens a handle with wait, query and terminate rights only.
        let raw = unsafe {
            OpenProcess(
                SYNCHRONIZE | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            )
        };
        if raw.is_null() {
            return Err(format!(
                "could not open the launched window {pid}: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: a handle just returned, owned by nothing else.
        let window = Self(unsafe { OwnedHandle::from_raw_handle(raw) });
        let mut name = [0_u16; 1024];
        let mut size = u32::try_from(name.len()).map_err(|e| e.to_string())?;
        // SAFETY: a live handle with query rights, and a buffer of the size passed.
        let read = unsafe {
            QueryFullProcessImageNameW(
                window.0.as_raw_handle(),
                0,
                name.as_mut_ptr(),
                &raw mut size,
            )
        };
        let len = usize::try_from(size).map_err(|e| e.to_string())?;
        let image = String::from_utf16_lossy(&name[..len.min(name.len())]);
        let same =
            read != 0 && std::fs::canonicalize(&image).ok() == std::fs::canonicalize(exe).ok();
        if !same {
            return Err(format!("process {pid} is {image}, not the launched window"));
        }
        Ok(window)
    }

    /// Its exit code once it has ended.
    fn ended(&self) -> Option<u32> {
        // SAFETY: a live handle with wait rights.
        if unsafe { WaitForSingleObject(self.0.as_raw_handle(), 0) } != WAIT_OBJECT_0 {
            return None;
        }
        let mut code = 0;
        // SAFETY: a live handle with query rights, and a writable code.
        unsafe { GetExitCodeProcess(self.0.as_raw_handle(), &raw mut code) };
        Some(code)
    }

    fn kill(&self) {
        // SAFETY: a live handle with terminate rights, to the window this
        // run started.
        unsafe { TerminateProcess(self.0.as_raw_handle(), 1) };
        // SAFETY: a live handle with wait rights.
        unsafe { WaitForSingleObject(self.0.as_raw_handle(), u32::MAX) };
    }
}

/// A cargo that left takes its console's Ctrl+C away from the window: once
/// the window says it gave the console back, Ctrl+C on that console must
/// leave it standing.
pub(super) fn check_left_cargo(exe: &Path) -> Result<(), String> {
    let host = std::env::current_exe().map_err(|e| e.to_string())?;
    let (verdict, _) = judge_window(&host, LEAVING, exe, |cargo, window, heard| {
        let status = cargo.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("the leaving cargo failed: {status}"));
        }
        stand_after_release(window, heard)
    });
    let stood_for = verdict?;
    println!(
        "PASS: once cargo left, the console's Ctrl+C left the shipped build standing {:.1}s",
        stood_for.as_secs_f32()
    );
    Ok(())
}

/// A parent that is not cargo is not joined: once the window loaded, Ctrl+C
/// on the console it was started on must leave it standing — a window that
/// listened, and turned that parent down.
pub(super) fn check_other_parent(exe: &Path) -> Result<(), String> {
    let host = std::env::current_exe().map_err(|e| e.to_string())?;
    let (verdict, said) = judge_window(
        &host.with_file_name(OTHER_LINK),
        OTHER,
        exe,
        |_, window, heard| stand_unjoined(window, heard),
    );
    let stood_for = verdict?;
    if said.iter().any(|line| line.contains(LISTEN_FAILED)) {
        return Err(format!(
            "the shipped build said `{LISTEN_FAILED}`: its standing says nothing of the parent \
             it turned down"
        ));
    }
    println!(
        "PASS: under a parent that is not cargo, the console's Ctrl+C left the shipped build \
         standing {:.1}s",
        stood_for.as_secs_f32()
    );
    Ok(())
}

/// Starts `program` as the parent in `role` on the host's console, opens the
/// window it names, has `judge` judge it, and kills it; what both said is
/// printed and handed back.
fn judge_window(
    program: &Path,
    role: &str,
    exe: &Path,
    judge: impl FnOnce(&mut Child, &Window, &Heard) -> Result<Duration, String>,
) -> (Result<Duration, String>, Vec<String>) {
    let spawned = Command::new(program)
        .args(["shipped", "--no-build"])
        .env(ROLE, role)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut parent = match spawned {
        Ok(parent) => parent,
        Err(error) => {
            return (
                Err(format!("could not start the {role}: {error}")),
                Vec::new(),
            );
        }
    };
    let heard = Arc::new(Heard::default());
    let out = parent.stdout.take().map(|pipe| listen(pipe, &heard));
    let err = parent.stderr.take().map(|pipe| listen(pipe, &heard));
    let window = named_window(&heard).and_then(|pid| {
        crate::budget::child_started(pid, &exe.display().to_string());
        Window::open(pid, exe)
    });
    let verdict = match &window {
        Ok(window) => judge(&mut parent, window, &heard),
        Err(error) => Err(error.clone()),
    };
    // A window named past the wait still holds the pipes — this run's, and
    // the host's own its parent was given — so it is opened once more to be
    // killed.
    let named = heard.window.load(Ordering::SeqCst);
    let window = window.ok().or_else(|| {
        (named != 0)
            .then(|| Window::open(named, exe).ok())
            .flatten()
    });
    match &window {
        Some(window) => window.kill(),
        None => {
            let _ = parent.kill();
        }
    }
    let _ = parent.wait();
    // One named and not opened may still hold them, and then they never
    // close: its lines are left unread.
    if window.is_none() && named != 0 {
        return (verdict, Vec::new());
    }
    let join = |h: Option<std::thread::JoinHandle<Vec<String>>>| {
        h.and_then(|h| h.join().ok()).unwrap_or_default()
    };
    let said: Vec<String> = join(err).into_iter().chain(join(out)).collect();
    for line in &said {
        println!("  | {line}");
    }
    (verdict, said)
}

/// The window its parent named. Its line was written as the window started,
/// and may still be on its way through the pipe.
fn named_window(heard: &Heard) -> Result<u32, String> {
    let mut wait = Wait::new("the parent", Budget::whole(ANSWER_CEILING), LOOK_AGAIN);
    loop {
        let pid = heard.window.load(Ordering::SeqCst);
        if pid != 0 {
            return Ok(pid);
        }
        wait.look_again("its line naming the window")
            .map_err(|e| e.to_string())?;
    }
}

fn stand_after_release(window: &Window, heard: &Heard) -> Result<Duration, String> {
    let mut wait = Wait::new(
        "the shipped build",
        Budget::whole(ANSWER_CEILING),
        LOOK_AGAIN,
    );
    while !heard.released.load(Ordering::SeqCst) {
        if let Some(code) = window.ended() {
            return Err(format!(
                "the shipped build ended before it gave its console back: exit {code}"
            ));
        }
        wait.look_again("its word that cargo left")
            .map_err(|e| e.to_string())?;
    }
    send()?;
    stood(super::STAND, LOOK_AGAIN, || window.ended()).map_err(|code| {
        format!("the console's Ctrl+C reached the shipped build after cargo left: exit {code}")
    })
}

/// The window listens from its QML's start, which comes before its
/// [`super::LOADED`].
fn stand_unjoined(window: &Window, heard: &Heard) -> Result<Duration, String> {
    let mut wait = Wait::new(
        "the shipped build",
        Budget::whole(super::LOAD_CEILING),
        LOOK_AGAIN,
    );
    while !heard.loaded.load(Ordering::SeqCst) {
        if let Some(code) = window.ended() {
            return Err(format!(
                "the shipped build ended before its window loaded: exit {code}"
            ));
        }
        wait.look_again("its window to load")
            .map_err(|e| e.to_string())?;
    }
    send()?;
    stood(super::STAND, LOOK_AGAIN, || window.ended()).map_err(|code| {
        format!(
            "the console's Ctrl+C reached the shipped build under a parent that is not cargo: \
             exit {code}"
        )
    })
}
