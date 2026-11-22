use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::Stdio;
use std::sync::Arc;

use super::{
    INFINITE, OpenProcess, SYNCHRONIZE, TerminateProcess, WAIT_OBJECT_0, WaitForSingleObject,
};

const PROCESS_TERMINATE: u32 = 0x0001;

/// What starts [`never_let_go_side`]: run by itself, as `--ignored` runs
/// it, the side does nothing.
const SIDE: &str = "_PGG_NEVER_LET_GO_SIDE";

/// A process of somebody else's, held from before its spawner ends: ended
/// when the test lets go of it, so a run the test finds still there is not
/// left suspended behind it.
struct Held(Arc<OwnedHandle>);

impl Held {
    fn open(pid: u32) -> Self {
        // SAFETY: integer arguments; a null answer is checked.
        let raw = unsafe { OpenProcess(SYNCHRONIZE | PROCESS_TERMINATE, 0, pid) };
        assert!(
            !raw.is_null(),
            "the held run {pid} is there to open: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: a handle just opened, owned by nothing else.
        Self(Arc::new(unsafe { OwnedHandle::from_raw_handle(raw) }))
    }

    /// Whether the process ended, once it has.
    fn ended(&self) -> impl FnOnce() -> bool + Send + use<> {
        let process = Arc::clone(&self.0);
        // SAFETY: a live handle opened with SYNCHRONIZE.
        move || unsafe { WaitForSingleObject(process.as_raw_handle(), INFINITE) == WAIT_OBJECT_0 }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // SAFETY: a live handle opened with PROCESS_TERMINATE; one already
        // ended is refused, which is the end wanted.
        unsafe { TerminateProcess(self.0.as_raw_handle(), 1) };
    }
}

/// The spawning side of [`a_git_never_let_go_ends_with_its_spawner`]: a run
/// spawned and left where it stands, its id said on stdout, then this
/// process ended once the parent has the run in hand — no destructor runs,
/// as when a crash takes the window down.
#[tokio::test]
#[ignore = "the spawning process of a_git_never_let_go_ends_with_its_spawner"]
#[mry::lock(crate::process::family::win32::let_go)]
async fn never_let_go_side() {
    super::mock_let_go().returns_with(|| Ok(()));
    if std::env::var_os(SIDE).is_none() {
        return;
    }
    // Never let go, so never run; should it run after all, it only lists
    // its tests.
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .arg("--list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let (child, _family) = super::super::spawn(&mut command, true, None).unwrap();
    let mut said = std::io::stdout();
    writeln!(said, "held {}", child.id().unwrap()).unwrap();
    said.flush().unwrap();
    // Until the parent closes stdin, which it does once it holds the run.
    let mut rest = String::new();
    std::io::stdin().read_line(&mut rest).unwrap();
    std::process::exit(0);
}

/// A process that spawns a run and ends before letting git go takes git
/// with it. Left behind, git would stay suspended for good — never running,
/// never ending — and hold its folder and everything it inherited: an IDE
/// keeps its run going while anything holds the run's console.
#[tokio::test]
async fn a_git_never_let_go_ends_with_its_spawner() {
    let mut side = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "process::family::win32::tests::never_let_go_side",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(SIDE, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = side.stdout.take().unwrap();
    let pid = crate::wait::bounded(
        "the spawning side's word",
        tokio::task::spawn_blocking(move || {
            BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
                .find_map(|line| {
                    line.strip_prefix("held ")
                        .and_then(|pid| pid.trim().parse::<u32>().ok())
                })
        }),
    )
    .await
    .unwrap()
    .expect("the spawning side said which run it holds");
    let held = Held::open(pid);
    drop(side.stdin.take());
    let side = crate::wait::bounded(
        "the spawning side's end",
        tokio::task::spawn_blocking(move || side.wait()),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(side.success(), "the spawning side ended as told: {side}");
    let ended = crate::wait::bounded(
        "the held run's end",
        tokio::task::spawn_blocking(held.ended()),
    )
    .await
    .unwrap();
    assert!(ended, "the held run is waited on to its end");
}
