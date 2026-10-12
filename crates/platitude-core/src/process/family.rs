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
//! credential prompt) is stopped at git alone: what it starts can be the
//! person's own — the browser a credential helper opened is theirs from
//! then on. So is every run off Windows.
//!
//! A whole run has a job object of its own. git is created suspended and
//! let go once it is in the job, so every process it starts is born in it.

use tokio::process::{Child, Command};

use super::live::Running;

/// One run's processes, and the count ([`super::Live`]) it holds until
/// they are known gone.
pub(super) struct Family {
    /// The job a whole run is ended through.
    #[cfg(windows)]
    job: Option<std::sync::Arc<std::os::windows::io::OwnedHandle>>,
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
        if whole {
            return win32::spawn(command, running);
        }
        command.creation_flags(win32::CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = whole;
    let child = command.spawn()?;
    Ok((
        child,
        Family {
            #[cfg(windows)]
            job: None,
            running,
            settled: false,
        },
    ))
}

impl Family {
    /// git ended by itself: nothing of the run is ended.
    pub(super) fn settle(&mut self) {
        self.settled = true;
    }

    /// Ends the run — whole where it has a job, git alone otherwise — and
    /// answers once that is gone. A whole run's end that cannot be told is
    /// an error; git alone's failed reap is logged and the stop stands as the
    /// run's end. Either way the count goes saying the end was not told
    /// ([`super::Live::idle`]).
    pub(super) async fn end(&mut self, child: &mut Child) -> std::io::Result<()> {
        #[cfg(windows)]
        if let Some(job) = &self.job {
            let ended = win32::end(job, child).await;
            if ended.is_err() {
                self.untold();
            }
            self.settled = true;
            return ended;
        }
        if let Err(e) = child.start_kill() {
            tracing::debug!(error = %e, "kill failed (process already exited?)");
        }
        if let Err(e) = child.wait().await {
            tracing::debug!(error = %e, "failed to reap killed process");
            self.untold();
        }
        self.settled = true;
        Ok(())
    }

    /// The count goes with the end not told.
    fn untold(&mut self) {
        if let Some(running) = self.running.as_mut() {
            running.untold();
        }
    }
}

impl Drop for Family {
    /// Dropped mid-run (a runtime going down with the command in it): a
    /// whole run is ended whole, and counted until it is gone. git alone is
    /// ended by `kill_on_drop`, which waits for nothing, so its count goes
    /// untold.
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        #[cfg(windows)]
        if let Some(job) = &self.job {
            win32::end_detached(std::sync::Arc::clone(job), self.running.take());
            return;
        }
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
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
    use std::sync::Arc;

    use tokio::process::{Child, Command};

    use super::{Family, Running};

    // The id list is read one word per id.
    const _: () = assert!(usize::BITS == 64);

    pub(super) const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_SUSPENDED: u32 = 0x0000_0004;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const INFINITE: u32 = 0xFFFF_FFFF;
    const WAIT_OBJECT_0: u32 = 0;
    const ERROR_MORE_DATA: i32 = 234;
    /// `JobObjectBasicProcessIdList`.
    const BASIC_PROCESS_ID_LIST: i32 = 3;

    // SAFETY: the signatures are transcribed from the Win32 headers
    // (jobapi2.h, processthreadsapi.h, synchapi.h, winbase.h); the
    // arguments are handles, integers, and a buffer with its length.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> RawHandle;
        fn AssignProcessToJobObject(job: RawHandle, process: RawHandle) -> i32;
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

    // SAFETY: transcribed from the native API (ntpsapi.h, which ships no
    // documented equivalent): one process handle in, a status out.
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtResumeProcess(process: RawHandle) -> i32;
    }

    /// git, created suspended, put in a job of its own and let go: nothing
    /// it starts is born outside the job.
    pub(super) fn spawn(
        command: &mut Command,
        running: Option<Running>,
    ) -> io::Result<(Child, Family)> {
        // SAFETY: no attributes and no name — a job nobody else can open.
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a handle just created, owned by nothing else.
        let job = Arc::new(unsafe { OwnedHandle::from_raw_handle(raw) });
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
        let child = command.spawn()?;
        let mut family = Family {
            job: Some(Arc::clone(&job)),
            running,
            settled: false,
        };
        let Some(process) = child.raw_handle() else {
            // Not reaped yet, so never here; `kill_on_drop` ends git, and
            // waits for nothing.
            family.settled = true;
            family.untold();
            return Err(io::Error::other("the spawned git has no handle"));
        };
        // SAFETY: both handles are live — the job is held above, and the
        // child is not reaped until it has run.
        let put = unsafe { AssignProcessToJobObject(job.as_raw_handle(), process) } != 0;
        let refused = if put {
            // SAFETY: as above.
            let status = unsafe { NtResumeProcess(process) };
            (status < 0).then(|| io::Error::other(format!("NtResumeProcess: {status:#010x}")))
        } else {
            Some(io::Error::last_os_error())
        };
        if let Some(error) = refused {
            // Never ran a single instruction: ended where it stands, and gone
            // before the error is told.
            // SAFETY: as above.
            let gone = unsafe {
                TerminateProcess(process, 1) != 0
                    && WaitForSingleObject(process, INFINITE) == WAIT_OBJECT_0
            };
            family.settled = true;
            if !gone {
                family.untold();
            }
            return Err(error);
        }
        Ok((child, family))
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
    /// still be opened and is still in it — a pid is reused once its process
    /// is gone, and the reuser is not this run's.
    fn members(job: &OwnedHandle) -> io::Result<(u32, Vec<OwnedHandle>)> {
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
            let mut held = Vec::new();
            for word in &buf[1..=listed as usize] {
                let Ok(pid) = u32::try_from(*word) else {
                    continue;
                };
                // SAFETY: integer arguments; a null answer is checked.
                let raw =
                    unsafe { OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
                if raw.is_null() {
                    continue;
                }
                // SAFETY: a handle just opened, owned by nothing else.
                let process = unsafe { OwnedHandle::from_raw_handle(raw) };
                let mut inside = 0;
                // SAFETY: both handles are live; the answer goes to a local.
                let asked = unsafe {
                    IsProcessInJob(process.as_raw_handle(), job.as_raw_handle(), &mut inside)
                };
                if asked != 0 && inside != 0 {
                    held.push(process);
                }
            }
            return Ok((assigned, held));
        }
    }
}
