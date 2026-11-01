//! What a message or a patch costs on its way into git, measured through
//! the product's own functions, by the input's size and by how many times
//! git reads it: a commit (a message, one read), a partial stage of a few
//! lines and of a whole rewritten file (one read), a partial discard of the
//! same (one read), and a restore of a few changed lines and of a whole
//! rewritten file (its `--check` and its apply: two reads). Each test is
//! one operation, run in a process of its own so its peaks hold that
//! operation and nothing else run after it — its setup, the rounds and
//! what is redone between them among it:
//!
//! ```text
//! <it binary> --ignored --exact --nocapture input_costs::<op>
//! ```
//!
//! One line per test: the input's size and reads, the operation's times
//! (each round's alone, what is redone between rounds out of it), this
//! process's peak working set, and its I/O counters over the timed calls —
//! on Windows they take a pipe and a file alike; WSL's kernel keeps none.
//! Then the gits: on Windows the peak commit of a job this process puts
//! itself in once its setup is done (it and every git after, together),
//! the largest one alone, and this process's own over its whole life — a
//! one alone above that is a git's; on Linux the largest `ru_maxrss` among
//! the reaped children — the setup's gits too, and never below this
//! process's own peak when a child was spawned: std spawns through
//! posix_spawn, whose child shares this process's memory until it execs,
//! and exec carries the peak of the memory it leaves into the child
//! (`exec_mmap` in the kernel's fs/exec.c; a forked child would carry its
//! resident set at the fork instead). The record and how it was read:
//! ci/baseline/input-costs-windows-x64.md.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use platitude_core::commit::{self, CommitOptions};
use platitude_core::details::{self, DiffTarget};
use platitude_core::discards::{self, Restore, Restored};
use platitude_core::patch::HunkSelect;
use platitude_core::process::GitExecutor;
use platitude_core::{Oid, stage};
use tokio_util::sync::CancellationToken;

use crate::support::{TestRepo, info};

/// Lines of the large file (about 80 bytes each): every one rewritten, a
/// single hunk and a patch of about 32MB.
const LARGE: usize = 200_000;
/// Lines of the small file: every 20th rewritten, 50 hunks of a few lines.
const SMALL: usize = 1_000;
const ROUNDS: usize = 30;
const LARGE_ROUNDS: usize = 10;

/// The product's git, kept off this machine's configuration as the suite's
/// is — but with git's own fsync left as it ships (the suite's turns it
/// off), so a round costs what it costs a reader.
fn product_like() -> GitExecutor {
    let nowhere = crate::support::temp::root().join(".no-config");
    let env: [(OsString, OsString); 7] = [
        ("GIT_CONFIG_GLOBAL".into(), nowhere.join("global").into()),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("XDG_CONFIG_HOME".into(), nowhere.into()),
        crate::support::temp::ceiling(),
        ("GIT_CONFIG_COUNT".into(), "1".into()),
        ("GIT_CONFIG_KEY_0".into(), "gc.auto".into()),
        ("GIT_CONFIG_VALUE_0".into(), "0".into()),
    ];
    GitExecutor::new().with_env(env).without_stock_timeouts()
}

#[tokio::test]
#[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
async fn commit() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "0\n", "root");
    let repo_info = info(&repo).await;
    let (exec, cancel) = (product_like(), CancellationToken::new());
    let message = format!("Change the counter\n\n{}\n", "Why it changes. ".repeat(20));
    let mut meter = Meter::new(message.len(), 1);
    for round in 1..=ROUNDS {
        repo.write_file("a.txt", &format!("{round}\n"));
        repo.git(&["add", "--", "a.txt"]);
        meter
            .time(commit::commit(
                &exec,
                &repo_info,
                &message,
                CommitOptions::default(),
                &cancel,
            ))
            .await
            .expect("commit");
    }
    meter.report("commit");
}

#[tokio::test]
#[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
async fn stage_small() {
    stage(SMALL, ROUNDS, "stage_small").await;
}

#[tokio::test]
#[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
async fn stage_large() {
    stage(LARGE, LARGE_ROUNDS, "stage_large").await;
}

/// The first hunk of `f.txt` staged, round after round.
async fn stage(lines: usize, rounds: usize, op: &str) {
    let mut repo = TestRepo::init();
    let path = repo.path.join("f.txt");
    write_lines(&path, lines, |_| "kept");
    repo.git(&["add", "--", "f.txt"]);
    repo.git(&["commit", "-q", "-m", "base"]);
    write_lines(&path, lines, changed(lines));
    let repo_info = info(&repo).await;
    let (exec, cancel) = (product_like(), CancellationToken::new());
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    let mut meter = Meter::new(first_hunk_patch(&mut repo), 1);
    for _ in 0..rounds {
        let (_, seen) = details::file_diff_with_fingerprint(&exec, &repo.path, &target, &cancel)
            .await
            .expect("the diff the selection is made on");
        meter
            .time(stage::apply_partial(
                &exec,
                &repo_info,
                &target,
                &[HunkSelect::whole(0)],
                seen,
                &cancel,
            ))
            .await
            .expect("stage one hunk");
        repo.git(&["reset", "-q", "--", "f.txt"]);
    }
    meter.report(op);
}

#[tokio::test]
#[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
async fn discard_large() {
    let mut repo = TestRepo::init();
    let path = repo.path.join("f.txt");
    write_lines(&path, LARGE, |_| "kept");
    repo.git(&["add", "--", "f.txt"]);
    repo.git(&["commit", "-q", "-m", "base"]);
    write_lines(&path, LARGE, changed(LARGE));
    let repo_info = info(&repo).await;
    let (exec, cancel) = (product_like(), CancellationToken::new());
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    let mut meter = Meter::new(first_hunk_patch(&mut repo), 1);
    for _ in 0..LARGE_ROUNDS {
        let (_, seen) = details::file_diff_with_fingerprint(&exec, &repo.path, &target, &cancel)
            .await
            .expect("the diff the selection is made on");
        meter
            .time(stage::discard_partial(
                &exec,
                &repo_info,
                &target,
                &[HunkSelect::whole(0)],
                seen,
                &cancel,
            ))
            .await
            .expect("discard the hunk");
        write_lines(&path, LARGE, changed(LARGE));
    }
    meter.report("discard_large");
}

#[tokio::test]
#[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
async fn restore_small() {
    restore(SMALL, ROUNDS, "restore_small").await;
}

#[tokio::test]
#[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
async fn restore_large() {
    restore(LARGE, LARGE_ROUNDS, "restore_large").await;
}

/// A copy of work thrown away from `f.txt` put back, round after round.
async fn restore(lines: usize, rounds: usize, op: &str) {
    let mut repo = TestRepo::init();
    let path = repo.path.join("f.txt");
    write_lines(&path, lines, |_| "kept");
    repo.git(&["add", "--", "f.txt"]);
    repo.git(&["commit", "-q", "-m", "base"]);
    let base = repo.git(&["rev-parse", "HEAD"]);
    write_lines(&path, lines, changed(lines));
    repo.git(&["add", "--", "f.txt"]);
    let work_tree = repo.git(&["write-tree"]);
    repo.git(&["reset", "-q", "--hard", &base]);
    // A copy as a discard leaves one (`discards::copy`): the working tree's
    // tree over the base, and an index that held nothing staged.
    let base_tree = repo.git(&["rev-parse", "HEAD^{tree}"]);
    let index = repo.git(&["commit-tree", "-p", &base, "-m", "index", &base_tree]);
    let copy = repo.git(&[
        "commit-tree",
        "-p",
        &base,
        "-p",
        &index,
        "-m",
        "copy",
        &work_tree,
    ]);
    let patch = repo
        .git_raw(&["diff-tree", "-p", "--binary", "--full-index", &base, &copy])
        .len();
    let repo_info = info(&repo).await;
    let part = Restore::Changes {
        copy: Oid::from_hex_str(&copy).expect("an id"),
        path: repo_info.workdir.to_string_lossy().into_owned(),
        git_dir: repo_info.git_dir.to_string_lossy().into_owned(),
    };
    let (exec, cancel) = (product_like(), CancellationToken::new());
    let mut meter = Meter::new(patch, 2);
    for _ in 0..rounds {
        let restored = meter
            .time(discards::restore(&exec, &repo.path, &part, &cancel))
            .await
            .expect("restore");
        assert_eq!(restored, Restored::Whole);
        repo.git(&["reset", "-q", "--hard", &base]);
    }
    meter.report(op);
}

/// The words of a file of `lines` once changed: all of a large one, every
/// 20th line of a small one.
fn changed(lines: usize) -> impl Fn(usize) -> &'static str {
    move |n| match lines >= LARGE || n % 20 == 0 {
        true => "thrown away",
        false => "kept",
    }
}

/// The size of the unstaged diff's first hunk as a patch: about what the
/// partial operations hand git.
fn first_hunk_patch(repo: &mut TestRepo) -> usize {
    let diff = repo.git_raw(&["diff", "--", "f.txt"]);
    let starts: Vec<usize> = diff
        .windows(3)
        .enumerate()
        .filter(|(at, w)| *w == b"@@ " && (*at == 0 || diff[at - 1] == b'\n'))
        .map(|(at, _)| at)
        .collect();
    match starts.as_slice() {
        [_, second, ..] => *second,
        [_] => diff.len(),
        [] => 0,
    }
}

/// `count` lines of about 80 bytes, line `n` led by `word(n)`, written as
/// they are made.
fn write_lines(path: &Path, count: usize, word: impl Fn(usize) -> &'static str) {
    let file = std::fs::File::create(path).expect("create");
    let mut out = std::io::BufWriter::new(file);
    for n in 0..count {
        writeln!(out, "{} {n:08} {}", word(n), "x".repeat(60)).expect("write");
    }
    out.flush().expect("flush");
}

/// Each round's time and the I/O counters over the timed calls alone, and
/// what this process and its gits hold from the meter's start.
struct Meter {
    input: usize,
    reads: usize,
    rounds: Vec<Duration>,
    io: os::Io,
    children: os::Children,
}

impl Meter {
    /// Started once the setup is done: `input` bytes for git, read `reads`
    /// times in an operation.
    fn new(input: usize, reads: usize) -> Self {
        Self {
            input,
            reads,
            rounds: Vec::new(),
            io: os::Io::default(),
            children: os::Children::start(),
        }
    }

    async fn time<T>(&mut self, call: impl std::future::Future<Output = T>) -> T {
        let before = os::io();
        // waits(measured): the round's own time is the measurement and judges nothing
        let started = Instant::now();
        let answer = call.await;
        self.rounds.push(started.elapsed());
        self.io.add(&os::io().since(&before));
        answer
    }

    #[expect(
        clippy::print_stdout,
        reason = "the measurement's one line is its output"
    )]
    fn report(mut self, op: &str) {
        self.rounds.sort();
        let at = |q: usize| self.rounds[(self.rounds.len() - 1) * q / 100].as_secs_f64() * 1e3;
        println!(
            "input_costs op={op} input_bytes={} reads={} rounds={} p10_ms={:.1} median_ms={:.1} \
             p90_ms={:.1} peak_ws_mib={:.1} {} {}",
            self.input,
            self.reads,
            self.rounds.len(),
            at(10),
            at(50),
            at(90),
            os::peak_working_set() as f64 / MIB,
            self.children.line(),
            self.io.line(),
        );
    }
}

pub(crate) const MIB: f64 = 1024.0 * 1024.0;

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "process and job counters with no safe binding in std, for a measurement"
)]
pub(crate) mod os {
    use std::ffi::c_void;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

    /// `JOBOBJECT_BASIC_LIMIT_INFORMATION`.
    #[repr(C)]
    #[derive(Default)]
    struct BasicLimit {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    /// `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`: the peaks are commit charge.
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimit {
        basic: BasicLimit,
        io: Io,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    /// `JobObjectExtendedLimitInformation`.
    const EXTENDED_LIMIT_INFORMATION: i32 = 9;

    /// A job this process puts itself in: it and every process it starts
    /// from then on — git, and what git starts — are counted together.
    pub struct Children(OwnedHandle);

    impl Children {
        pub fn start() -> Self {
            // SAFETY: no attributes and no name — a job nobody else can open.
            let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            assert!(!raw.is_null(), "{}", std::io::Error::last_os_error());
            // SAFETY: a handle just created, owned by nothing else.
            let job = unsafe { OwnedHandle::from_raw_handle(raw) };
            // SAFETY: a live job handle and this process's pseudo handle.
            let joined =
                unsafe { AssignProcessToJobObject(job.as_raw_handle(), GetCurrentProcess()) };
            assert_ne!(joined, 0, "{}", std::io::Error::last_os_error());
            Self(job)
        }

        /// The most the job's processes held at once, the most any one of
        /// them held, and the most this process held — all commit.
        pub fn line(&self) -> String {
            let mut info = ExtendedLimit::default();
            // SAFETY: a live job handle and a buffer of the struct's size.
            let read = unsafe {
                QueryInformationJobObject(
                    self.0.as_raw_handle(),
                    EXTENDED_LIMIT_INFORMATION,
                    (&raw mut info).cast(),
                    std::mem::size_of::<ExtendedLimit>() as u32,
                    std::ptr::null_mut(),
                )
            };
            assert_ne!(read, 0, "{}", std::io::Error::last_os_error());
            // SAFETY: this process's pseudo handle, open for the call.
            let (own, _) = process_peaks(unsafe { GetCurrentProcess() });
            format!(
                "together_peak_mib={:.1} one_process_peak_mib={:.1} own_commit_peak_mib={:.1}",
                info.peak_job_memory_used as f64 / super::MIB,
                info.peak_process_memory_used as f64 / super::MIB,
                own as f64 / super::MIB,
            )
        }
    }

    #[repr(C)]
    #[derive(Default)]
    struct MemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    /// `IO_COUNTERS`: every I/O the process asked for — files, pipes and
    /// devices alike.
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    pub struct Io {
        read_ops: u64,
        write_ops: u64,
        other_ops: u64,
        read_bytes: u64,
        write_bytes: u64,
        other_bytes: u64,
    }

    // SAFETY: transcribed from the Win32 headers (processthreadsapi.h,
    // psapi.h, jobapi2.h, winbase.h, winnt.h); handles and buffers with
    // their sizes.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> *mut c_void;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
        fn QueryInformationJobObject(
            job: *mut c_void,
            class: i32,
            info: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut MemoryCounters,
            cb: u32,
        ) -> i32;
        fn GetProcessIoCounters(process: *mut c_void, counters: *mut Io) -> i32;
    }

    pub fn peak_working_set() -> usize {
        // SAFETY: this process's pseudo handle, open for the call.
        process_peaks(unsafe { GetCurrentProcess() }).1
    }

    /// The peak commit and the peak working set of the process behind
    /// `process` — running or ended, while the handle is open.
    pub(crate) fn process_peaks(process: *mut c_void) -> (usize, usize) {
        let mut counters = MemoryCounters {
            cb: std::mem::size_of::<MemoryCounters>() as u32,
            ..MemoryCounters::default()
        };
        // SAFETY: a process handle its owner keeps open across the call, and
        // a buffer of the size named.
        let read = unsafe { K32GetProcessMemoryInfo(process, &mut counters, counters.cb) };
        assert_ne!(read, 0, "{}", std::io::Error::last_os_error());
        (counters.peak_pagefile_usage, counters.peak_working_set_size)
    }

    pub fn io() -> Io {
        let mut counters = Io::default();
        // SAFETY: this process's pseudo handle and a buffer of the struct's size.
        let read = unsafe { GetProcessIoCounters(GetCurrentProcess(), &mut counters) };
        assert_ne!(read, 0, "{}", std::io::Error::last_os_error());
        counters
    }

    impl Io {
        pub fn since(&self, before: &Io) -> Io {
            Io {
                read_ops: self.read_ops - before.read_ops,
                write_ops: self.write_ops - before.write_ops,
                other_ops: self.other_ops - before.other_ops,
                read_bytes: self.read_bytes - before.read_bytes,
                write_bytes: self.write_bytes - before.write_bytes,
                other_bytes: self.other_bytes - before.other_bytes,
            }
        }

        pub fn add(&mut self, more: &Io) {
            self.read_ops += more.read_ops;
            self.write_ops += more.write_ops;
            self.other_ops += more.other_ops;
            self.read_bytes += more.read_bytes;
            self.write_bytes += more.write_bytes;
            self.other_bytes += more.other_bytes;
        }

        pub fn line(&self) -> String {
            format!(
                "write_ops={} write_mib={:.1} other_ops={} read_ops={} read_mib={:.1}",
                self.write_ops,
                self.write_bytes as f64 / (1024.0 * 1024.0),
                self.other_ops,
                self.read_ops,
                self.read_bytes as f64 / (1024.0 * 1024.0),
            )
        }
    }
}

/// `/proc/self`: `VmHWM`, and `io`'s `wchar` (every write asked for) and
/// `write_bytes` / `cancelled_write_bytes` (what went to storage, and what
/// was dropped before it got there) — where the kernel keeps `io` at all
/// (task I/O accounting; WSL's kernel does not). The children's peak is
/// `getrusage`'s.
#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "the children's peak has no binding in std, for a measurement"
)]
pub(crate) mod os {
    /// `struct rusage` on 64-bit Linux: two `timeval`s, then fourteen
    /// `long`s, `ru_maxrss` (KiB) the first.
    #[repr(C)]
    struct Rusage {
        times: [i64; 4],
        maxrss: i64,
        rest: [i64; 13],
    }

    const RUSAGE_CHILDREN: i32 = -1;

    // SAFETY: transcribed from glibc's sys/resource.h and sys/wait.h; a pid,
    // integers, and buffers of the structs' sizes.
    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut Rusage) -> i32;
        fn wait4(pid: i32, status: *mut i32, options: i32, usage: *mut Rusage) -> i32;
    }

    /// Reaps the child `pid` and answers its peak resident set, in bytes —
    /// for a child std spawned through posix_spawn, never below this
    /// process's own peak when it was spawned (the module's doc). The
    /// caller's `Child` must not be waited on after.
    pub(crate) fn reap_peak(pid: u32) -> usize {
        let mut usage = Rusage {
            times: [0; 4],
            maxrss: 0,
            rest: [0; 13],
        };
        let mut status = 0;
        let pid = i32::try_from(pid).unwrap();
        // SAFETY: a child of this process nobody else reaps, and buffers of
        // the right sizes.
        let reaped = unsafe { wait4(pid, &mut status, 0, &mut usage) };
        assert_eq!(reaped, pid, "{}", std::io::Error::last_os_error());
        assert_eq!(status, 0, "the child's wait status");
        usage.maxrss as usize * 1024
    }

    /// The largest peak of the children reaped so far — the setup's gits
    /// among them; one at a time, not together.
    pub struct Children;

    impl Children {
        pub fn start() -> Self {
            Self
        }

        pub fn line(&self) -> String {
            let mut usage = Rusage {
                times: [0; 4],
                maxrss: 0,
                rest: [0; 13],
            };
            // SAFETY: a buffer of the struct's size.
            let read = unsafe { getrusage(RUSAGE_CHILDREN, &mut usage) };
            assert_eq!(read, 0, "{}", std::io::Error::last_os_error());
            format!(
                "one_child_peak_mib={:.1}",
                usage.maxrss as f64 * 1024.0 / super::MIB
            )
        }
    }

    #[derive(Default, Clone, Copy)]
    pub struct Io {
        kept: bool,
        wchar: u64,
        syscw: u64,
        write_bytes: u64,
        cancelled_write_bytes: u64,
    }

    fn field(text: &str, name: &str) -> u64 {
        text.lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix(':'))
            .and_then(|value| value.split_whitespace().next()?.parse().ok())
            .unwrap_or_else(|| panic!("{name} in /proc/self: {text}"))
    }

    pub fn peak_working_set() -> usize {
        let status = std::fs::read_to_string("/proc/self/status").expect("status");
        field(&status, "VmHWM") as usize * 1024
    }

    pub fn io() -> Io {
        let Ok(io) = std::fs::read_to_string("/proc/self/io") else {
            return Io::default();
        };
        Io {
            kept: true,
            wchar: field(&io, "wchar"),
            syscw: field(&io, "syscw"),
            write_bytes: field(&io, "write_bytes"),
            cancelled_write_bytes: field(&io, "cancelled_write_bytes"),
        }
    }

    impl Io {
        pub fn since(&self, before: &Io) -> Io {
            Io {
                kept: self.kept && before.kept,
                wchar: self.wchar - before.wchar,
                syscw: self.syscw - before.syscw,
                write_bytes: self.write_bytes - before.write_bytes,
                cancelled_write_bytes: self.cancelled_write_bytes - before.cancelled_write_bytes,
            }
        }

        pub fn add(&mut self, more: &Io) {
            self.kept = more.kept;
            self.wchar += more.wchar;
            self.syscw += more.syscw;
            self.write_bytes += more.write_bytes;
            self.cancelled_write_bytes += more.cancelled_write_bytes;
        }

        pub fn line(&self) -> String {
            if !self.kept {
                return "io=unread".into();
            }
            let mib = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
            format!(
                "write_calls={} written_mib={:.1} storage_mib={:.1} cancelled_mib={:.1}",
                self.syscw,
                mib(self.wchar),
                mib(self.write_bytes),
                mib(self.cancelled_write_bytes),
            )
        }
    }
}

/// Elsewhere (macOS) the counters are not read.
#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) mod os {
    pub struct Children;

    impl Children {
        pub fn start() -> Self {
            Self
        }

        pub fn line(&self) -> String {
            "children=unread".into()
        }
    }

    /// No counters, in the shape the others' are started in (`default()`).
    #[derive(Default, Clone, Copy)]
    pub struct Io {}

    pub fn peak_working_set() -> usize {
        0
    }

    pub fn io() -> Io {
        Io {}
    }

    impl Io {
        pub fn since(&self, _: &Io) -> Io {
            Io {}
        }

        pub fn add(&mut self, _: &Io) {}

        pub fn line(&self) -> String {
            "io=unread".into()
        }
    }
}
