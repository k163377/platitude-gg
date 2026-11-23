//! What one git run starts, held from git's first instruction to the run's
//! end.
//!
//! A run of this machine's own (`Pace::Here`) that is stopped — cancelled,
//! out of its budget, or its pipes broken — is ended whole on Windows: git
//! and every process git started for it that is still there (a hook, a
//! filter, `git-lfs`), and the end is answered once each of them is gone
//! ([`Family::end`]). A run that ends by itself ends nothing: what git
//! leaves running on purpose (the fsmonitor daemon) runs on.
//!
//! A run paced by something outside (`Pace::Elsewhere`: a remote, a
//! credential prompt) is stopped at its git alone (user decision — ask
//! before changing): what it starts can be the person's own — the browser a
//! credential helper opened is theirs from then on. Its git on Windows is
//! the program spawned and, where that is a launcher (Git for Windows'
//! `cmd\git.exe`), the git it started in its place — its child given the
//! same arguments — which a stop of the launcher alone would leave running
//! ([`Family::end`]). What either of them started runs on, a git started
//! for another command (`pull`'s merge, auto-maintenance) too. Off Windows
//! every run is stopped at the program spawned.
//!
//! Only a native process holds a folder against a rename: MSYS's `sh`
//! reopens its working directory shared, and the fsmonitor daemon moves to
//! `$HOME` and watches with sharing, so neither is in its way. The rename's
//! witness in the tests is a native git held on its hook
//! (`exec::HeldOnItsHook`).
//!
//! Every run on Windows has a job object of its own — ended whole for a
//! whole run; for one stopped at its git, where a launcher's git is found,
//! and one a process may break away from. git is created suspended and let
//! go once it is in the job, so every process it starts is born in it.
//! Until it is let go, the job ends git when this process ends: a git whose
//! parent went (a crash, a kill) before letting it go would sit suspended
//! for good, holding its folder and every handle it was handed. Once let
//! go, git runs on by itself when this process ends, as it does off
//! Windows — a write the window went down in the middle of still finishes.
//! The span from git's creation to its joining the job (std's and tokio's
//! spawn, then the assignment) stays open: closing it takes creating git
//! inside its job (`PROC_THREAD_ATTRIBUTE_JOB_LIST`), which std passes only
//! through the unstable `raw_attribute` — or a spawn of this crate's own in
//! place of std's.

use tokio::process::{Child, Command};

use super::live::Running;

/// One run's processes, and the count ([`super::Live`]) it holds until
/// they are known gone.
pub(super) struct Family {
    /// The job the run is in: ended whole, or read for a launcher's git.
    #[cfg(windows)]
    job: std::sync::Arc<std::os::windows::io::OwnedHandle>,
    /// What a stop of the run ends.
    #[cfg(windows)]
    stop: win32::Stop,
    running: Option<Running>,
    /// Ended by itself, or ended: nothing of it is ended on drop.
    settled: bool,
}

/// Spawns `command` as a run of its own — `whole` where a stop is to end
/// what git starts too — counted by `running` until the run is gone, at
/// once where nothing ran.
pub(super) fn spawn(
    command: &mut Command,
    whole: bool,
    running: Option<Running>,
) -> std::io::Result<(Child, Family)> {
    #[cfg(windows)]
    {
        win32::spawn(command, whole, running)
    }
    #[cfg(not(windows))]
    {
        let _ = whole;
        let child = command.spawn()?;
        Ok((
            child,
            Family {
                running,
                settled: false,
            },
        ))
    }
}

impl Family {
    /// git ended by itself: nothing of the run is ended.
    pub(super) fn settle(&mut self) {
        self.settled = true;
    }

    /// Ends the run — whole, or its git alone — and answers once that is
    /// gone. A whole run's end that cannot be told is an error; git alone's
    /// that cannot is logged and the stop stands as the run's end. Either
    /// way the count goes saying the end was not told
    /// ([`super::Live::idle`]).
    pub(super) async fn end(&mut self, child: &mut Child) -> std::io::Result<()> {
        #[cfg(windows)]
        let ended =
            match &self.stop {
                win32::Stop::Whole => win32::end(&self.job, child).await,
                win32::Stop::Git(spawned) => win32::end_git(&self.job, spawned, child)
                    .await
                    .or_else(|e| {
                        tracing::debug!(error = %e, "a stopped git could not be told gone");
                        self.untold();
                        Ok(())
                    }),
            };
        #[cfg(not(windows))]
        let ended = {
            if let Err(e) = child.start_kill() {
                tracing::debug!(error = %e, "kill failed (process already exited?)");
            }
            if let Err(e) = child.wait().await {
                tracing::debug!(error = %e, "failed to reap killed process");
                self.untold();
            }
            Ok(())
        };
        if ended.is_err() {
            self.untold();
        }
        self.settled = true;
        ended
    }

    /// The count goes with the end not told.
    fn untold(&mut self) {
        if let Some(running) = self.running.as_mut() {
            running.untold();
        }
    }
}

impl Drop for Family {
    /// Dropped mid-run (a runtime going down with the command in it): on
    /// Windows the run is ended whole or at its git, and counted until that
    /// is gone. Off Windows git alone is ended by `kill_on_drop`, which waits
    /// for nothing, so its count goes untold.
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        #[cfg(windows)]
        {
            let job = std::sync::Arc::clone(&self.job);
            match &self.stop {
                win32::Stop::Whole => win32::end_detached(job, self.running.take()),
                win32::Stop::Git(spawned) => {
                    win32::end_git_detached(
                        job,
                        std::sync::Arc::clone(spawned),
                        self.running.take(),
                    );
                }
            }
        }
        #[cfg(not(windows))]
        self.untold();
    }
}

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "job objects have no safe binding, and a handful of signatures does not earn a \
              dependency (the app's winframe::win32 says the same)"
)]
mod win32 {
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::io::{
        AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle, RawHandle,
    };
    use std::sync::Arc;

    use tokio::process::{Child, Command};

    use super::{Family, Running};

    // The id list, a process's basic information, and a command line's
    // header are read one word per id, field, or address.
    const _: () = assert!(usize::BITS == 64);

    /// Every git runs with a console of its own that shows no window. Not
    /// `DETACHED_PROCESS` (no console at all): git then starts its children
    /// with none either, and a child that is an MSYS shell (the `git
    /// mergetool` script, a hook, a textconv or fsmonitor run through sh)
    /// makes a console of its own, which on a machine whose default
    /// terminal is Windows Terminal opens a visible window that stays. The
    /// price is one `conhost.exe` per git; a shape that avoids paying it is
    /// an open question in internal-docs/ハング調査.md.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_SUSPENDED: u32 = 0x0000_0004;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_TERMINATE: u32 = 0x0001;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const INFINITE: u32 = 0xFFFF_FFFF;
    const WAIT_OBJECT_0: u32 = 0;
    const ERROR_MORE_DATA: i32 = 234;
    /// `JobObjectBasicProcessIdList`.
    const BASIC_PROCESS_ID_LIST: i32 = 3;
    /// `JobObjectExtendedLimitInformation`: the only class that takes
    /// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    const EXTENDED_LIMIT_INFORMATION: i32 = 9;
    const KILL_ON_JOB_CLOSE: u32 = 0x2000;
    /// `JOB_OBJECT_LIMIT_BREAKAWAY_OK`: a process of the job may start one
    /// outside it (`CREATE_BREAKAWAY_FROM_JOB`).
    const BREAKAWAY_OK: u32 = 0x0800;
    /// `ProcessBasicInformation`: six words, the parent's id the last.
    const BASIC_INFORMATION: i32 = 0;
    /// `ProcessCommandLineInformation`: a `UNICODE_STRING` — its length in
    /// bytes in the first half-word, its characters' address in the second
    /// word — and the characters after it.
    const COMMAND_LINE_INFORMATION: i32 = 60;

    /// `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`. Only `flags` is set: a limit
    /// whose flag is clear is not read.
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimits {
        per_process_user_time: i64,
        per_job_user_time: i64,
        flags: u32,
        minimum_working_set: usize,
        maximum_working_set: usize,
        active_processes: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
        io_counters: [u64; 6],
        process_memory: usize,
        job_memory: usize,
        peak_process_memory: usize,
        peak_job_memory: usize,
    }

    const _: () = assert!(size_of::<ExtendedLimits>() == 144);

    // SAFETY: the signatures are transcribed from the Win32 headers
    // (jobapi2.h, processthreadsapi.h, synchapi.h, winbase.h); the
    // arguments are handles, integers, and a buffer with its length.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> RawHandle;
        fn AssignProcessToJobObject(job: RawHandle, process: RawHandle) -> i32;
        fn SetInformationJobObject(
            job: RawHandle,
            class: i32,
            info: *const c_void,
            length: u32,
        ) -> i32;
        fn TerminateJobObject(job: RawHandle, code: u32) -> i32;
        fn QueryInformationJobObject(
            job: RawHandle,
            class: i32,
            info: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
        fn IsProcessInJob(process: RawHandle, job: RawHandle, result: *mut i32) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> RawHandle;
        fn TerminateProcess(process: RawHandle, code: u32) -> i32;
        fn WaitForSingleObject(handle: RawHandle, millis: u32) -> u32;
    }

    // SAFETY: transcribed from the native API (ntpsapi.h, winternl.h; the
    // resume ships no documented equivalent): a process handle in, a status
    // out, and for a query a buffer with its length.
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtResumeProcess(process: RawHandle) -> i32;
        fn NtQueryInformationProcess(
            process: RawHandle,
            class: i32,
            info: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
    }

    /// What a stop of a run ends.
    pub(super) enum Stop {
        /// Everything the job holds.
        Whole,
        /// The run's git: the program spawned, and the process it started in
        /// its place where it is a launcher ([`stand_ins`]).
        Git(Arc<Spawned>),
    }

    /// The program a run stopped at its git spawned — held, so its id names
    /// it until the stop is done — and the arguments it was given.
    pub(super) struct Spawned {
        process: OwnedHandle,
        id: u32,
        /// Unread, a stop ends the program spawned alone.
        arguments: Option<Vec<u16>>,
    }

    impl Spawned {
        /// `process`, held on a handle of its own. Its arguments are read
        /// while it has run nothing yet.
        fn hold(process: RawHandle, id: u32) -> io::Result<Self> {
            // SAFETY: the caller's handle is live: the child is not reaped
            // until it has run.
            let process = unsafe { BorrowedHandle::borrow_raw(process) }.try_clone_to_owned()?;
            let arguments = match command_line(&process) {
                Ok(line) => Some(arguments(&line).to_vec()),
                Err(e) => {
                    tracing::debug!(error = %e, "a git's arguments could not be read; its stop ends it alone");
                    None
                }
            };
            Ok(Self {
                process,
                id,
                arguments,
            })
        }
    }

    /// git, created suspended, put in a job of its own and let go: nothing
    /// it starts is born outside the job but by asking to break away, which
    /// a run stopped at its git lets it. `whole` is what a stop ends.
    pub(super) fn spawn(
        command: &mut Command,
        whole: bool,
        mut running: Option<Running>,
    ) -> io::Result<(Child, Family)> {
        // SAFETY: no attributes and no name — a job nobody else can open.
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a handle just created, owned by nothing else.
        let job = Arc::new(unsafe { OwnedHandle::from_raw_handle(raw) });
        // What the job keeps once git runs. What a run stopped at its git
        // starts is left to run on, as it would in no job: free to leave it.
        let kept = if whole { 0 } else { BREAKAWAY_OK };
        // Every handle to the job is this process's, so the job closes when
        // it ends, however it ends.
        limit(&job, KILL_ON_JOB_CLOSE | kept)?;
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
        let child = command.spawn()?;
        let Some((process, id)) = child.raw_handle().zip(child.id()) else {
            // Not reaped yet, so never here; `kill_on_drop` ends git, and
            // waits for nothing.
            if let Some(running) = running.as_mut() {
                running.untold();
            }
            return Err(io::Error::other("the spawned git has no handle"));
        };
        match joined(&job, process, id, whole, kept) {
            Ok(stop) => Ok((
                child,
                Family {
                    job,
                    stop,
                    running,
                    settled: false,
                },
            )),
            Err(error) => {
                // Ended where it stands — before its first instruction, or at
                // its start when the job would not let go of it — and gone
                // before the error is told. Whatever it could have started is
                // the job's, which still ends what it holds as it closes here.
                // SAFETY: the child's handle is live, as above.
                let gone = unsafe {
                    TerminateProcess(process, 1) != 0
                        && WaitForSingleObject(process, INFINITE) == WAIT_OBJECT_0
                };
                if !gone && let Some(running) = running.as_mut() {
                    running.untold();
                }
                Err(error)
            }
        }
    }

    /// The suspended git `process` put in `job` and let go, and what a stop
    /// of its run ends.
    fn joined(
        job: &OwnedHandle,
        process: RawHandle,
        id: u32,
        whole: bool,
        kept: u32,
    ) -> io::Result<Stop> {
        // SAFETY: both handles are live — the job is the caller's, and the
        // child is not reaped until it has run.
        if unsafe { AssignProcessToJobObject(job.as_raw_handle(), process) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let stop = if whole {
            Stop::Whole
        } else {
            Stop::Git(Arc::new(Spawned::hold(process, id)?))
        };
        let_go(job, process, kept)?;
        Ok(stop)
    }

    /// Lets git run, and from then on leaves it running should this process
    /// end first: of the job's limits, `kept` stays. A test leaves git where
    /// it stands instead, to end its process on a git never let go (mry).
    #[mry::mry(skip_args(OwnedHandle, RawHandle))]
    fn let_go(job: &OwnedHandle, process: RawHandle, kept: u32) -> io::Result<()> {
        // SAFETY: the caller's process handle is live: the child is not
        // reaped until it has run.
        let status = unsafe { NtResumeProcess(process) };
        if status < 0 {
            return Err(io::Error::other(format!("NtResumeProcess: {status:#010x}")));
        }
        limit(job, kept)
    }

    /// Sets the job's limits to `flags`: the job has no limit with a value,
    /// so the flags are the whole of what is set.
    fn limit(job: &OwnedHandle, flags: u32) -> io::Result<()> {
        let limits = ExtendedLimits {
            flags,
            ..ExtendedLimits::default()
        };
        // SAFETY: the job handle is live, borrowed from the caller; the
        // structure is as long as the length says, and only read.
        let set = unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                EXTENDED_LIMIT_INFORMATION,
                std::ptr::from_ref(&limits).cast(),
                size_of::<ExtendedLimits>() as u32,
            )
        } != 0;
        if set {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Stops every process in the job and answers once each is gone.
    pub(super) async fn end(job: &Arc<OwnedHandle>, child: &mut Child) -> io::Result<()> {
        // Opened before the stop, so each is waited on its own handle — the
        // end git's own reap waits on — rather than read off the count.
        // Unread, the stop still goes out, and the count after it decides.
        let held = members(job).map(|(_, held)| held).unwrap_or_default();
        // SAFETY: the job handle is live, held by `job`.
        let stopped = unsafe { TerminateJobObject(job.as_raw_handle(), 1) } != 0;
        let refused = (!stopped).then(io::Error::last_os_error);
        if refused.is_some() {
            // The job refused: git is still stopped, and reaped below.
            let _ = child.start_kill();
        }
        child.wait().await?;
        if let Some(error) = refused {
            return Err(error);
        }
        let job = Arc::clone(job);
        tokio::task::spawn_blocking(move || gone(&job, held))
            .await
            .map_err(io::Error::other)?
    }

    /// [`end`] for a run dropped mid-way: a drop cannot wait, so a thread
    /// does, holding the count until the run is gone. Untold until that
    /// thread has seen the end: a stop the job refused, or a thread that
    /// never starts, lets the count go at once, saying so.
    pub(super) fn end_detached(job: Arc<OwnedHandle>, mut running: Option<Running>) {
        if let Some(running) = running.as_mut() {
            running.untold();
        }
        let held = members(&job).map(|(_, held)| held).unwrap_or_default();
        // SAFETY: the job handle is live, held by `job`.
        if unsafe { TerminateJobObject(job.as_raw_handle(), 1) } == 0 {
            tracing::debug!(
                error = %io::Error::last_os_error(),
                "a dropped git run could not be stopped"
            );
            return;
        }
        let waiting = std::thread::Builder::new()
            .name("git-run-end".into())
            .spawn(move || match gone(&job, held) {
                Ok(()) => {
                    if let Some(running) = running.as_mut() {
                        running.told();
                    }
                }
                Err(e) => tracing::debug!(error = %e, "a dropped git run could not be told gone"),
            });
        if let Err(e) = waiting {
            tracing::debug!(error = %e, "no thread to wait out a dropped git run");
        }
    }

    /// Stops the run's git ([`git_gone`]) and answers once it is gone; the
    /// rest of the job runs on. git is reaped whatever the stop answered.
    pub(super) async fn end_git(
        job: &Arc<OwnedHandle>,
        spawned: &Arc<Spawned>,
        child: &mut Child,
    ) -> io::Result<()> {
        let job = Arc::clone(job);
        let spawned = Arc::clone(spawned);
        let gone = tokio::task::spawn_blocking(move || git_gone(&job, &spawned))
            .await
            .unwrap_or_else(|e| Err(io::Error::other(e)));
        if gone.is_err()
            && let Err(e) = child.start_kill()
        {
            tracing::debug!(error = %e, "kill failed (process already exited?)");
        }
        child.wait().await?;
        gone
    }

    /// [`end_git`] for a run dropped mid-way: a thread ends the git and
    /// holds the count until it is gone — untold until then. The program
    /// spawned is ended by `kill_on_drop` too, whichever comes first.
    pub(super) fn end_git_detached(
        job: Arc<OwnedHandle>,
        spawned: Arc<Spawned>,
        mut running: Option<Running>,
    ) {
        if let Some(running) = running.as_mut() {
            running.untold();
        }
        let waiting = std::thread::Builder::new()
            .name("git-run-end".into())
            .spawn(move || match git_gone(&job, &spawned) {
                Ok(()) => {
                    if let Some(running) = running.as_mut() {
                        running.told();
                    }
                }
                Err(e) => tracing::debug!(error = %e, "a dropped git run could not be told gone"),
            });
        if let Err(e) = waiting {
            tracing::debug!(error = %e, "no thread to wait out a dropped git run");
        }
    }

    /// Ends the program spawned and then, gone — and so starting nothing
    /// more — what it started in its place ([`stand_ins`]); answers once
    /// each is gone.
    fn git_gone(job: &OwnedHandle, spawned: &Spawned) -> io::Result<()> {
        ended(&spawned.process)?;
        let Some(given) = spawned.arguments.as_deref() else {
            return Ok(());
        };
        for stand_in in stand_ins(job, spawned.id, given)? {
            ended(&stand_in)?;
        }
        Ok(())
    }

    /// Each process in the job that `parent` started in its place, opened to
    /// be ended and waited on: its child given the same arguments, `given`,
    /// as a launcher passes them on (Git for Windows' `cmd\git.exe` copies
    /// them as they are). A child given others — a helper, a git for
    /// another command — is not one.
    fn stand_ins(job: &OwnedHandle, parent: u32, given: &[u16]) -> io::Result<Vec<OwnedHandle>> {
        let (_, ids) = ids(job)?;
        let access = SYNCHRONIZE | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION;
        Ok(ids
            .into_iter()
            .filter_map(|pid| opened(job, pid, access))
            .filter(|process| parent_of(process).is_ok_and(|id| id == parent))
            .filter(|process| command_line(process).is_ok_and(|line| arguments(&line) == given))
            .collect())
    }

    /// Ends `process` and waits until it is gone. One already ending refuses
    /// the stop, which is the end wanted.
    fn ended(process: &OwnedHandle) -> io::Result<()> {
        // SAFETY: a live handle that can end the process and be waited on.
        let waited = unsafe {
            TerminateProcess(process.as_raw_handle(), 1);
            WaitForSingleObject(process.as_raw_handle(), INFINITE)
        };
        if waited == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// The id of the process that started `process`.
    fn parent_of(process: &OwnedHandle) -> io::Result<u32> {
        let mut words = [0usize; 6];
        // SAFETY: a live handle opened with PROCESS_QUERY_LIMITED_INFORMATION;
        // the buffer is as long as the length says.
        let status = unsafe {
            NtQueryInformationProcess(
                process.as_raw_handle(),
                BASIC_INFORMATION,
                words.as_mut_ptr().cast(),
                size_of_val(&words) as u32,
                std::ptr::null_mut(),
            )
        };
        if status < 0 {
            return Err(io::Error::other(format!(
                "NtQueryInformationProcess: {status:#010x}"
            )));
        }
        u32::try_from(words[5]).map_err(io::Error::other)
    }

    /// The command line `process` was started with.
    fn command_line(process: &OwnedHandle) -> io::Result<Vec<u16>> {
        let mut wanted = 0u32;
        // SAFETY: a live handle opened with PROCESS_QUERY_LIMITED_INFORMATION;
        // an empty buffer asks for the length wanted, and is not written.
        unsafe {
            NtQueryInformationProcess(
                process.as_raw_handle(),
                COMMAND_LINE_INFORMATION,
                std::ptr::null_mut(),
                0,
                &mut wanted,
            )
        };
        let mut buf = vec![0u64; (wanted as usize).div_ceil(8).max(2)];
        let bytes = u32::try_from(buf.len() * 8).map_err(io::Error::other)?;
        // SAFETY: as above; the buffer is as long as `bytes` says, and
        // aligned for the string's header.
        let status = unsafe {
            NtQueryInformationProcess(
                process.as_raw_handle(),
                COMMAND_LINE_INFORMATION,
                buf.as_mut_ptr().cast(),
                bytes,
                &mut wanted,
            )
        };
        if status < 0 {
            return Err(io::Error::other(format!(
                "NtQueryInformationProcess: {status:#010x}"
            )));
        }
        let length = usize::from(buf[0] as u16) / 2;
        // SAFETY: the words are read as the half-words they hold.
        let halves =
            unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u16>(), buf.len() * 4) };
        // The characters follow the header in the buffer itself.
        (buf[1] as usize)
            .checked_sub(buf.as_ptr() as usize)
            .map(|at| at / 2)
            .and_then(|at| halves.get(at..at + length))
            .map(<[u16]>::to_vec)
            .ok_or_else(|| io::Error::other("a command line outside its buffer"))
    }

    /// What follows the program's name on a command `line`, blanks before
    /// it skipped: the name read as Windows reads it — to its closing quote
    /// where it opens with one, else to the first blank.
    fn arguments(line: &[u16]) -> &[u16] {
        let quote = u16::from(b'"');
        let blank = |c: &u16| *c == u16::from(b' ') || *c == u16::from(b'\t');
        let name = match line.split_first() {
            Some((first, rest)) if *first == quote => rest
                .iter()
                .position(|c| *c == quote)
                .map_or(line.len(), |at| at + 2),
            _ => line.iter().position(blank).unwrap_or(line.len()),
        };
        let rest = &line[name..];
        &rest[rest.iter().position(|c| !blank(c)).unwrap_or(rest.len())..]
    }

    /// Waits until each of `held` and every process still in the job is
    /// gone. One born between the stop and its listing is stopped too: the
    /// stop is asked again for every listing that finds the job not empty.
    /// One that cannot be opened to be waited on, twice running, is an
    /// error — once can be one that left between its count and its opening.
    fn gone(job: &OwnedHandle, mut held: Vec<OwnedHandle>) -> io::Result<()> {
        let mut unopened = false;
        loop {
            for process in held.drain(..) {
                // SAFETY: a live handle opened with SYNCHRONIZE.
                let waited = unsafe { WaitForSingleObject(process.as_raw_handle(), INFINITE) };
                if waited != WAIT_OBJECT_0 {
                    return Err(io::Error::last_os_error());
                }
            }
            let (assigned, still) = members(job)?;
            if assigned == 0 {
                return Ok(());
            }
            if still.is_empty() {
                if unopened {
                    return Err(io::Error::other(format!(
                        "{assigned} process(es) of the run could not be opened to be waited on"
                    )));
                }
                unopened = true;
                continue;
            }
            unopened = false;
            // SAFETY: the job handle is live, borrowed from the caller.
            if unsafe { TerminateJobObject(job.as_raw_handle(), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            held = still;
        }
    }

    /// How many processes the job holds, and a handle to each one that can
    /// still be opened and is still in it.
    fn members(job: &OwnedHandle) -> io::Result<(u32, Vec<OwnedHandle>)> {
        let (assigned, ids) = ids(job)?;
        let access = SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION;
        let held = ids
            .into_iter()
            .filter_map(|pid| opened(job, pid, access))
            .collect();
        Ok((assigned, held))
    }

    /// `pid` opened with `access`, where it still can be and is still in
    /// the job — a pid is reused once its process is gone, and the reuser
    /// is not this run's.
    fn opened(job: &OwnedHandle, pid: u32, access: u32) -> Option<OwnedHandle> {
        // SAFETY: integer arguments; a null answer is checked.
        let raw = unsafe { OpenProcess(access, 0, pid) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: a handle just opened, owned by nothing else.
        let process = unsafe { OwnedHandle::from_raw_handle(raw) };
        let mut inside = 0;
        // SAFETY: both handles are live; the answer goes to a local.
        let asked =
            unsafe { IsProcessInJob(process.as_raw_handle(), job.as_raw_handle(), &mut inside) };
        (asked != 0 && inside != 0).then_some(process)
    }

    /// How many processes the job holds, and the id of each.
    fn ids(job: &OwnedHandle) -> io::Result<(u32, Vec<u32>)> {
        let mut capacity: u32 = 16;
        loop {
            // `JOBOBJECT_BASIC_PROCESS_ID_LIST`: the assigned and listed
            // counts in the first word (in that order, low half first), then
            // one id per word.
            let mut buf = vec![0u64; 1 + capacity as usize];
            let bytes = u32::try_from(buf.len() * 8).map_err(io::Error::other)?;
            // SAFETY: the buffer is as long as `bytes` says, and aligned for
            // the ids.
            let ok = unsafe {
                QueryInformationJobObject(
                    job.as_raw_handle(),
                    BASIC_PROCESS_ID_LIST,
                    buf.as_mut_ptr().cast(),
                    bytes,
                    std::ptr::null_mut(),
                )
            } != 0;
            if !ok {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(ERROR_MORE_DATA) {
                    return Err(error);
                }
            }
            let assigned = (buf[0] & 0xFFFF_FFFF) as u32;
            let listed = (buf[0] >> 32) as u32;
            if !ok || listed < assigned {
                capacity = assigned + 8;
                continue;
            }
            let ids = buf[1..=listed as usize]
                .iter()
                .filter_map(|word| u32::try_from(*word).ok())
                .collect();
            return Ok((assigned, ids));
        }
    }

    #[cfg(test)]
    mod tests;
}
