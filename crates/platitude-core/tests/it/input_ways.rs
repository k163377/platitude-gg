//! Where a large patch's cost goes on its way into git, way by way, for a
//! restore's size (about 32MB): git reading it by path, from a file on its
//! stdin, from a pipe this process writes as the executor does, from that
//! pipe made 1 MiB wide (Linux), and from a memfd (Linux); the same pipes
//! into `cat` alone, which is the send and the pipe without git's reading;
//! and a temporary file the patch would go through by path (made, written,
//! synced, removed). Every round asks each reading way once, in an order of
//! its own ([`shuffled`]); the temporary file is timed as many times after
//! them, so its write and sync stand before no reading way's clock. One
//! line says each way's p10, median and p90. Then each
//! of git's ways is run once more for git's own peaks, read off the ended
//! process: on Windows its peak commit and working set; on Linux its
//! `ru_maxrss`, which exec raises to this process's peak at the spawn
//! where std spawns through posix_spawn, as here (`input_costs`'s doc),
//! said beside it:
//!
//! ```text
//! <it binary> --ignored --exact --nocapture input_ways::ways_in
//! ```
//!
//! The record and how it was read: ci/baseline/input-costs-linux-x64.md.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::input_costs::os;
use crate::support::TestRepo;

/// Rounds of the reading ways, and times the temporary file is timed.
const ROUNDS: usize = 40;

/// Where the rounds' orders start: the same orders on every run.
const SEED: u64 = 0x2545_f491_4f6c_dd1d;

/// `0..n` in an order of its own: Fisher–Yates over a splitmix64 stream
/// from `state`, so which way runs right before which changes from round
/// to round, and no way keeps a place.
fn shuffled(n: usize, state: &mut u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let span = u64::try_from(i + 1).unwrap();
        let j = usize::try_from(splitmix64(state) % span).unwrap();
        order.swap(i, j);
    }
    order
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The width a wide way asks of its pipe (the executor's on Linux).
#[cfg(target_os = "linux")]
const WIDE: usize = 1 << 20;

/// The environment the suite's git runs under, without its fsync knob.
fn env() -> Vec<(OsString, OsString)> {
    let nowhere = crate::support::temp::root().join(".no-config");
    vec![
        ("GIT_CONFIG_GLOBAL".into(), nowhere.join("global").into()),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("XDG_CONFIG_HOME".into(), nowhere.into()),
        crate::support::temp::ceiling(),
    ]
}

/// One way a patch can reach its reader.
#[derive(Clone, Copy, PartialEq)]
enum Way {
    GitByPath,
    GitFileOnStdin,
    GitPipe,
    #[cfg(target_os = "linux")]
    GitPipeWide,
    #[cfg(target_os = "linux")]
    GitMemfd,
    CatPipe,
    #[cfg(target_os = "linux")]
    CatPipeWide,
    TempFile,
}

impl Way {
    fn name(self) -> &'static str {
        match self {
            Way::GitByPath => "git_by_path",
            Way::GitFileOnStdin => "git_file_on_stdin",
            Way::GitPipe => "git_pipe",
            #[cfg(target_os = "linux")]
            Way::GitPipeWide => "git_pipe_1mib",
            #[cfg(target_os = "linux")]
            Way::GitMemfd => "git_memfd",
            Way::CatPipe => "cat_pipe",
            #[cfg(target_os = "linux")]
            Way::CatPipeWide => "cat_pipe_1mib",
            Way::TempFile => "temp_file",
        }
    }

    /// What it hands its reader on stdin.
    fn fed(self) -> Fed {
        match self {
            Way::GitByPath | Way::TempFile => Fed::None,
            Way::GitFileOnStdin => Fed::File,
            Way::GitPipe | Way::CatPipe => Fed::Pipe { wide: false },
            #[cfg(target_os = "linux")]
            Way::GitPipeWide | Way::CatPipeWide => Fed::Pipe { wide: true },
            #[cfg(target_os = "linux")]
            Way::GitMemfd => Fed::Memfd,
        }
    }

    fn is_git(self) -> bool {
        self.name().starts_with("git_")
    }
}

#[derive(Clone, Copy)]
enum Fed {
    /// Nothing: the patch is named by path.
    None,
    /// The patch file, open on stdin.
    File,
    /// A pipe this process writes, made 1 MiB wide where `wide`.
    Pipe { wide: bool },
    /// A memfd holding the patch, open on stdin from its start.
    #[cfg(target_os = "linux")]
    Memfd,
}

/// What every way works on.
struct Ground {
    repo: PathBuf,
    git_dir: PathBuf,
    patch_file: PathBuf,
    patch: Vec<u8>,
    /// The git the product spawns, found once: Git for Windows' launcher on
    /// the PATH stands for another process, whose peaks these would not be.
    git: PathBuf,
    cat: Option<PathBuf>,
}

impl Ground {
    /// The program and arguments of a way with a reader.
    fn reader(&self, way: Way) -> (PathBuf, Vec<String>) {
        if way.is_git() {
            let patch = match way.fed() {
                Fed::None => self.patch_file.to_string_lossy().into_owned(),
                _ => "-".into(),
            };
            let args = vec!["apply".into(), "--check".into(), patch];
            return (self.git.clone(), args);
        }
        (self.cat.clone().unwrap(), Vec::new())
    }
}

/// One timed run of `way`: from what its reader is handed (a memfd's write
/// among it) to the reap, or the temporary file's make-write-sync-remove.
async fn timed(ground: &Ground, way: Way) -> Duration {
    if way == Way::TempFile {
        // Under the git directory, as a patch handed by path would be.
        let dir = ground.git_dir.join("platitude");
        let path = dir.join(format!("probe-{}", std::process::id()));
        // waits(measured): the way's own time is the measurement and judges nothing
        let started = Instant::now();
        std::fs::create_dir_all(&dir).unwrap();
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&ground.patch).unwrap();
        file.sync_all().unwrap();
        drop(file);
        std::fs::remove_file(&path).unwrap();
        return started.elapsed();
    }
    let (program, args) = ground.reader(way);
    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(&ground.repo)
        .envs(env())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let fed = way.fed();
    // waits(measured): the way's own time is the measurement and judges nothing
    let started = Instant::now();
    match fed {
        Fed::None => command.stdin(Stdio::null()),
        Fed::File => command.stdin(std::fs::File::open(&ground.patch_file).unwrap()),
        Fed::Pipe { .. } => command.stdin(Stdio::piped()),
        #[cfg(target_os = "linux")]
        Fed::Memfd => command.stdin(linux::memfd(&ground.patch)),
    };
    let mut child = command.spawn().unwrap();
    let out = if let Fed::Pipe { wide } = fed {
        let mut pipe = child.stdin.take().unwrap();
        if wide {
            widen(&pipe);
        }
        let sent = async move {
            pipe.write_all(&ground.patch).await.unwrap();
            pipe.flush().await.unwrap();
        };
        tokio::join!(child.wait_with_output(), sent).0
    } else {
        child.wait_with_output().await
    };
    let out = out.unwrap();
    assert!(
        out.status.success(),
        "{program:?} {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    started.elapsed()
}

/// One more run of a git way, untimed, for git's own peaks.
fn peaks(ground: &Ground, way: Way) -> String {
    let (program, args) = ground.reader(way);
    let mut command = std::process::Command::new(&program);
    command
        .args(&args)
        .current_dir(&ground.repo)
        .envs(env())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match way.fed() {
        Fed::None => command.stdin(Stdio::null()),
        Fed::File => command.stdin(std::fs::File::open(&ground.patch_file).unwrap()),
        Fed::Pipe { .. } => command.stdin(Stdio::piped()),
        #[cfg(target_os = "linux")]
        Fed::Memfd => command.stdin(linux::memfd(&ground.patch)),
    };
    // This process's own peak as git is spawned: on Linux, the floor of
    // git's `ru_maxrss`.
    let own = os::peak_working_set();
    let mut child = command.spawn().unwrap();
    std::thread::scope(|scope| {
        if let (Some(mut pipe), Fed::Pipe { wide }) = (child.stdin.take(), way.fed()) {
            if wide {
                widen(&pipe);
            }
            scope.spawn(move || pipe.write_all(&ground.patch).unwrap());
        }
        ended_peaks(&mut child, own)
    })
}

#[cfg(windows)]
fn ended_peaks(child: &mut std::process::Child, _own: usize) -> String {
    use std::os::windows::io::AsRawHandle;
    assert!(child.wait().unwrap().success());
    let (commit, working_set) = os::process_peaks(child.as_raw_handle().cast());
    format!(
        "git_commit_peak_mib={:.1} git_working_set_peak_mib={:.1}",
        commit as f64 / crate::input_costs::MIB,
        working_set as f64 / crate::input_costs::MIB
    )
}

#[cfg(target_os = "linux")]
fn ended_peaks(child: &mut std::process::Child, own: usize) -> String {
    let peak = os::reap_peak(child.id());
    format!(
        "git_maxrss_mib={:.1} own_peak_at_spawn_mib={:.1}",
        peak as f64 / crate::input_costs::MIB,
        own as f64 / crate::input_costs::MIB
    )
}

#[cfg(not(any(windows, target_os = "linux")))]
fn ended_peaks(child: &mut std::process::Child, _own: usize) -> String {
    assert!(child.wait().unwrap().success());
    "git_peaks=unread".into()
}

#[cfg(target_os = "linux")]
fn widen(pipe: &impl std::os::fd::AsRawFd) {
    let wide = linux::set_pipe_size(pipe.as_raw_fd(), i32::try_from(WIDE).unwrap());
    assert!(wide >= WIDE as i32, "the pipe stayed at {wide}");
}

#[cfg(not(target_os = "linux"))]
fn widen<T>(_: &T) {
    unreachable!("a pipe is widened on Linux alone");
}

#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "a pipe's width and a memfd have no binding in std, for a measurement"
)]
mod linux {
    use std::ffi::c_char;
    use std::io::{Seek, Write};
    use std::os::fd::{FromRawFd, OwnedFd, RawFd};

    const F_SETPIPE_SZ: i32 = 1031;
    const MFD_CLOEXEC: u32 = 1;

    // SAFETY: transcribed from glibc's fcntl.h and sys/mman.h; an fd and
    // integers in, and a NUL-ended name.
    unsafe extern "C" {
        fn fcntl(fd: i32, cmd: i32, ...) -> i32;
        fn memfd_create(name: *const c_char, flags: u32) -> i32;
    }

    /// The pipe's new width, as the kernel rounded it.
    pub fn set_pipe_size(fd: RawFd, width: i32) -> i32 {
        // SAFETY: a live pipe fd this process owns, and an integer.
        let got = unsafe { fcntl(fd, F_SETPIPE_SZ, width) };
        assert!(got > 0, "{}", std::io::Error::last_os_error());
        got
    }

    /// A memfd holding `bytes`, read from its start.
    pub fn memfd(bytes: &[u8]) -> std::fs::File {
        // SAFETY: a static NUL-ended name and the close-on-exec flag.
        let fd = unsafe { memfd_create(c"pgg-input".as_ptr(), MFD_CLOEXEC) };
        assert!(fd >= 0, "{}", std::io::Error::last_os_error());
        // SAFETY: a new fd, owned by nothing else.
        let mut file = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(fd) });
        file.write_all(bytes).unwrap();
        file.rewind().unwrap();
        file
    }
}

/// `cat` beside this machine's git: Git for Windows keeps one of its own.
fn cat() -> Option<PathBuf> {
    if cfg!(windows) {
        let exec = std::process::Command::new("git")
            .arg("--exec-path")
            .output()
            .ok()?;
        let core = PathBuf::from(String::from_utf8_lossy(&exec.stdout).trim());
        let usr = core
            .ancestors()
            .nth(3)?
            .join("usr")
            .join("bin")
            .join("cat.exe");
        usr.is_file().then_some(usr)
    } else {
        Some(PathBuf::from("cat"))
    }
}

/// `count` lines of about 80 bytes, written as they are made.
fn write_lines(path: &Path, count: usize, word: &str) {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    for n in 0..count {
        writeln!(out, "{word} {n:08} {}", "x".repeat(60)).unwrap();
    }
    out.flush().unwrap();
}

#[tokio::test]
#[ignore = "a measurement for ci/baseline: run by hand"]
async fn ways_in() {
    let mut repo = TestRepo::init();
    write_lines(&repo.path.join("big.txt"), 200_000, "kept");
    repo.git(&["add", "--", "big.txt"]);
    repo.git(&["commit", "-q", "-m", "base"]);
    write_lines(&repo.path.join("big.txt"), 200_000, "thrown away");
    let patch = repo.git_raw(&["diff", "--binary", "--full-index"]);
    repo.git(&["checkout", "-q", "--", "big.txt"]);
    let held = crate::support::temp::dir();
    let patch_file = held.path().join("p.patch");
    std::fs::write(&patch_file, &patch).unwrap();
    let ground = Ground {
        git_dir: repo.path.join(".git"),
        repo: repo.path.clone(),
        patch_file,
        patch,
        git: PathBuf::from(platitude_core::process::default_program_path()),
        cat: cat(),
    };

    let mut ways = vec![Way::GitByPath, Way::GitFileOnStdin, Way::GitPipe];
    #[cfg(target_os = "linux")]
    ways.extend([Way::GitPipeWide, Way::GitMemfd]);
    if ground.cat.is_some() {
        ways.push(Way::CatPipe);
        #[cfg(target_os = "linux")]
        ways.push(Way::CatPipeWide);
    }

    let mut took: Vec<Vec<Duration>> = vec![Vec::new(); ways.len()];
    let mut state = SEED;
    for _ in 0..ROUNDS {
        for at in shuffled(ways.len(), &mut state) {
            took[at].push(timed(&ground, ways[at]).await);
        }
    }
    let mut temp = Vec::new();
    for _ in 0..ROUNDS {
        temp.push(timed(&ground, Way::TempFile).await);
    }
    ways.push(Way::TempFile);
    took.push(temp);
    let lines: Vec<String> = ways
        .iter()
        .zip(took.iter_mut())
        .map(|(way, took)| {
            took.sort();
            let at = |q: usize| took[(took.len() - 1) * q / 100].as_secs_f64() * 1e3;
            let mem = match way.is_git() {
                true => peaks(&ground, *way),
                false => String::new(),
            };
            format!(
                "input_ways way={} bytes={} rounds={} p10_ms={:.1} median_ms={:.1} p90_ms={:.1} {mem}",
                way.name(),
                ground.patch.len(),
                took.len(),
                at(10),
                at(50),
                at(90)
            )
        })
        .collect();
    report(&lines);
}

#[expect(
    clippy::print_stdout,
    reason = "the measurement's lines are its output"
)]
fn report(lines: &[String]) {
    for line in lines {
        println!("{}", line.trim_end());
    }
}
