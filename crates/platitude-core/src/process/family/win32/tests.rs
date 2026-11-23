use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use super::{
    INFINITE, OpenProcess, PROCESS_TERMINATE, SYNCHRONIZE, TerminateProcess, WAIT_OBJECT_0,
    WaitForSingleObject, arguments,
};

/// What starts [`never_let_go_side`]: run by itself, as `--ignored` runs
/// it, the side does nothing.
const SIDE: &str = "_PGG_NEVER_LET_GO_SIDE";

/// A process the test did not spawn, held by its id: ended when the test
/// lets go of it, so one the test finds still there — a run left suspended,
/// a part that runs on — does not outlive the test.
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

    /// Whether the process has ended by now, asked without waiting.
    fn gone(&self) -> bool {
        // SAFETY: a live handle opened with SYNCHRONIZE.
        unsafe { WaitForSingleObject(self.0.as_raw_handle(), 0) == WAIT_OBJECT_0 }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // SAFETY: a live handle opened with PROCESS_TERMINATE; one already
        // ended is refused, which is the end wanted.
        unsafe { TerminateProcess(self.0.as_raw_handle(), 1) };
    }
}

/// Which part of a run stopped at its git a process plays
/// ([`launched_side`]).
const ROLE: &str = "_PGG_LAUNCHED_ROLE";
/// The file the launcher's part starts as its git, and the git's part as a
/// git for another command.
const BEHIND: &str = "_PGG_LAUNCHED_BEHIND";
/// The file the git's part starts as the person's own.
const HELPER: &str = "_PGG_LAUNCHED_HELPER";
/// How a part runs [`launched_side`] alone.
const LAUNCHED_SIDE: [&str; 4] = [
    "process::family::win32::tests::launched_side",
    "--exact",
    "--ignored",
    "--nocapture",
];

/// The parts of a run stopped at its git, by [`ROLE`]; run by itself, as
/// `--ignored` runs it, it does nothing.
/// - `launcher`: Git for Windows' launcher — starts the git that does the
///   work as a process of its own, given the launcher's own arguments, and
///   waits on it.
/// - `git`: starts a helper of the person's own and a git for another
///   command, each given other arguments than its own, says the three ids,
///   and holds until its stdin closes.
/// - `helper`, `other`: hold until their stdin closes.
#[test]
#[ignore = "the parts a run stopped at its git spawns"]
fn launched_side() {
    let Some(role) = std::env::var_os(ROLE) else {
        return;
    };
    let start = |file: &str, role: &str| {
        let mut part = std::process::Command::new(std::env::var_os(file).unwrap());
        part.args(LAUNCHED_SIDE).env(ROLE, role);
        part
    };
    match role.to_str() {
        Some("launcher") => {
            let ended = start(BEHIND, "git").status().unwrap();
            std::process::exit(ended.code().unwrap_or(1));
        }
        Some("git") => {
            let runs_on = |file: &str, role: &str| {
                #[expect(
                    clippy::zombie_processes,
                    reason = "the part runs on: the git ends and leaves it running"
                )]
                let part = start(file, role)
                    .arg("--quiet")
                    .stdout(Stdio::null())
                    .spawn()
                    .unwrap();
                part.id()
            };
            let helper = runs_on(HELPER, "helper");
            let other = runs_on(BEHIND, "other");
            let mut said = std::io::stdout();
            writeln!(
                said,
                "git {} helper {helper} other {other}",
                std::process::id()
            )
            .unwrap();
            said.flush().unwrap();
        }
        _ => {}
    }
    let mut rest = String::new();
    std::io::stdin().read_line(&mut rest).unwrap();
    std::process::exit(0);
}

/// A run of [`launched_side`]'s parts from hard links of this binary,
/// stopped at its git.
struct Stopped {
    git: Held,
    helper: Held,
    other: Held,
    /// The parts' stdin: closed, the parts that run on end.
    _stdin: Option<tokio::process::ChildStdin>,
    _dir: tempfile::TempDir,
}

impl Stopped {
    /// The run spawned as `role` from `folder`'s `git.exe` — `launcher` from
    /// `cmd`, `git` from `bin` — stopped once its git said its parts' ids.
    async fn run(folder: &str, role: &str) -> Self {
        use tokio::io::AsyncBufReadExt;
        let dir = crate::temp::dir();
        let this = std::env::current_exe().unwrap();
        let place = |folder: &str, file: &str| -> PathBuf {
            let at = dir.path().join(folder);
            std::fs::create_dir_all(&at).unwrap();
            let path = at.join(file);
            if !path.exists() {
                std::fs::hard_link(&this, &path).unwrap();
            }
            path
        };
        let behind = place("bin", "git.exe");
        let helper = place("helper", "helper.exe");
        let spawned = place(folder, "git.exe");
        let mut command = tokio::process::Command::new(&spawned);
        command
            .args(LAUNCHED_SIDE)
            .env(ROLE, role)
            .env(BEHIND, behind)
            .env(HELPER, helper)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let (mut child, mut family) = super::super::spawn(&mut command, false, None).unwrap();
        let stdin = child.stdin.take();
        let mut lines = tokio::io::BufReader::new(child.stdout.take().unwrap()).lines();
        let [git, helper, other] = crate::wait::bounded("the git's word", async {
            while let Some(line) = lines.next_line().await.unwrap() {
                let ids: Vec<u32> = line
                    .strip_prefix("git ")
                    .map(|rest| {
                        rest.split(' ')
                            .filter_map(|word| word.parse().ok())
                            .collect()
                    })
                    .unwrap_or_default();
                if let [git, helper, other] = ids[..] {
                    return [git, helper, other].map(Held::open);
                }
            }
            panic!("the git said its id and its parts'");
        })
        .await;
        crate::wait::bounded("the stop", family.end(&mut child))
            .await
            .unwrap();
        Self {
            git,
            helper,
            other,
            _stdin: stdin,
            _dir: dir,
        }
    }

    /// What the git started runs on: a helper, which can be the person's
    /// own, and a git for another command, given other arguments.
    fn left_what_the_git_started(&self) {
        assert!(!self.helper.gone(), "a helper the git started runs on");
        assert!(
            !self.other.gone(),
            "a git the git started for another command runs on"
        );
    }
}

/// A run stopped at its git ends the git a launcher in front of it started
/// — a stop of the launcher alone leaves the git that does the work running
/// — and leaves what that git started.
#[tokio::test]
async fn a_run_stopped_at_its_git_ends_the_git_behind_a_launcher() {
    let stopped = Stopped::run("cmd", "launcher").await;
    assert!(
        stopped.git.gone(),
        "the git behind the launcher is gone once the stop is told"
    );
    stopped.left_what_the_git_started();
}

/// A run stopped at its git with no launcher in front of it ends that git
/// alone: its own children, given other arguments, are not stand-ins.
#[tokio::test]
async fn a_run_stopped_at_its_git_ends_no_child_of_the_git() {
    let stopped = Stopped::run("bin", "git").await;
    assert!(stopped.git.gone(), "the git is gone once the stop is told");
    stopped.left_what_the_git_started();
}

/// A launcher's git is given the arguments the launcher was: Git for
/// Windows' `cmd\git.exe` names its git bare and copies what follows its
/// own name as it is (lines read off a stock install's pair).
#[test]
fn a_launchers_git_is_given_the_launchers_arguments() {
    let wide = |line: &str| line.encode_utf16().collect::<Vec<u16>>();
    let launcher = wide(
        r#""C:\Program Files\Git\cmd\git.exe" -c "alias.nap=!sleep 20; :" -c "user.name=A B" nap "two words" "#,
    );
    let git = wide(r#"git.exe -c "alias.nap=!sleep 20; :" -c "user.name=A B" nap "two words" "#);
    let expected = wide(r#"-c "alias.nap=!sleep 20; :" -c "user.name=A B" nap "two words" "#);
    assert_eq!(arguments(&launcher), &expected[..]);
    assert_eq!(arguments(&git), &expected[..]);
}

/// A name with nothing after it, a quoted one too, has no arguments; blanks
/// after the name, tabs too, are skipped.
#[test]
fn a_name_alone_has_no_arguments() {
    let wide = |line: &str| line.encode_utf16().collect::<Vec<u16>>();
    assert!(arguments(&wide("git.exe")).is_empty());
    assert!(arguments(&wide(r#""C:\a b\git.exe""#)).is_empty());
    assert_eq!(arguments(&wide("git.exe \t status")), &wide("status")[..]);
}

/// The spawning side of [`a_git_never_let_go_ends_with_its_spawner`]: a run
/// spawned and left where it stands, its id said on stdout, then this
/// process ended once the parent has the run in hand — no destructor runs,
/// as when a crash takes the window down.
#[tokio::test]
#[ignore = "the spawning process of a_git_never_let_go_ends_with_its_spawner"]
#[mry::lock(crate::process::family::win32::let_go)]
async fn never_let_go_side() {
    super::mock_let_go(mry::Any).returns_with(|_| Ok(()));
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
