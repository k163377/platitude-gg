//! What this process was started with stays its own: none of it reaches a
//! process it starts.
//!
//! std starts every child with handle inheritance on and no list of what
//! to hand over (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST` reaches std only
//! through the unstable `raw_attribute`), so a child is handed every
//! inheritable handle this process holds, besides the stdio std makes for
//! it. Among them is what this process was handed at its start — its
//! launcher's pipes (an IDE's run console, through `cargo run`), and
//! whatever had leaked to the launcher in turn. A git holding the run
//! console keeps the IDE's run going until that git ends: one left behind
//! by a crash keeps it going for good, and the IDE's Stop reaches nothing,
//! the process it started being gone.
//!
//! So before anything is started, every handle this process holds is made
//! non-inheritable. The standard handles stay usable, and a child meant to
//! share them still gets them: std hands `Stdio::inherit` over as a
//! duplicate made inheritable for that one spawn (`start_again`).
//!
//! What this process opens later stays as its opener made it. Measured on
//! Windows 11: `AttachConsole` to cargo's console adds two handles, neither
//! inheritable; under the offscreen platform Qt's FreeType holds every font
//! file it reads open through the C runtime's `fopen`, which is
//! inheritable, so a git started there holds those fonts while it runs (a
//! real window draws with DirectWrite instead).

/// Makes every handle this process holds non-inheritable. Called before
/// anything starts a process or a thread that could, so no handle comes or
/// goes between the listing and the change.
#[cfg(windows)]
pub(crate) fn keep_from_children() -> std::io::Result<()> {
    win32::keep_from_children()
}

/// Off Windows nothing is done: a child is handed the standard streams its
/// spawn sets and what is left open across exec, and std and Qt open
/// everything close-on-exec.
#[cfg(not(windows))]
pub(crate) fn keep_from_children() -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "a process's own handle table has no safe binding, and four signatures do not earn \
              a dependency (winframe::win32 says the same)"
)]
mod win32 {
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::io::RawHandle;

    /// `ProcessHandleTable`: the process's handle values, one `ULONG` each.
    const PROCESS_HANDLE_TABLE: i32 = 58;
    const HANDLE_FLAG_INHERIT: u32 = 1;

    // SAFETY: transcribed from the native API (ntpsapi.h, which ships no
    // documented equivalent for this class): a process handle, a class, a
    // buffer with its length, and the length written out.
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryInformationProcess(
            process: RawHandle,
            class: i32,
            info: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
    }

    // SAFETY: the signatures are transcribed from the Win32 headers
    // (processthreadsapi.h, handleapi.h); the arguments are handles,
    // integers and one integer out.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> RawHandle;
        fn GetHandleInformation(handle: RawHandle, flags: *mut u32) -> i32;
        fn SetHandleInformation(handle: RawHandle, mask: u32, flags: u32) -> i32;
    }

    pub(super) fn keep_from_children() -> io::Result<()> {
        let mut kept = 0usize;
        let mut refused = None;
        for handle in handles()? {
            let mut flags = 0;
            // SAFETY: a value from this process's own table; one closed since
            // is refused, and an answer goes to a local.
            let inheritable = unsafe { GetHandleInformation(handle, &mut flags) } != 0
                && flags & HANDLE_FLAG_INHERIT != 0;
            if !inheritable {
                continue;
            }
            // SAFETY: as above; only the inherit flag is touched.
            if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) } == 0 {
                refused = Some(io::Error::last_os_error());
            } else {
                kept += 1;
            }
        }
        tracing::debug!(
            kept,
            "handles this process was started with kept from its children"
        );
        match refused {
            None => Ok(()),
            Some(error) => Err(io::Error::other(format!(
                "a handle stayed inheritable: {error}"
            ))),
        }
    }

    /// This process's handle values. A buffer too short is filled to its end
    /// and answered as a success (measured on Windows 11), so the listing is
    /// whole only once it leaves room over.
    fn handles() -> io::Result<Vec<RawHandle>> {
        let mut capacity = 256usize;
        loop {
            let mut values = vec![0u32; capacity];
            let length = u32::try_from(capacity * 4).map_err(io::Error::other)?;
            let mut returned = 0u32;
            // SAFETY: the pseudo-handle names this process; the buffer is as
            // long as `length` says, and the answer goes to a local.
            let status = unsafe {
                NtQueryInformationProcess(
                    GetCurrentProcess(),
                    PROCESS_HANDLE_TABLE,
                    values.as_mut_ptr().cast(),
                    length,
                    &mut returned,
                )
            };
            if status < 0 {
                return Err(io::Error::other(format!(
                    "NtQueryInformationProcess: {status:#010x}"
                )));
            }
            if returned < length {
                values.truncate(returned as usize / 4);
                return Ok(values
                    .into_iter()
                    .map(|value| std::ptr::without_provenance_mut(value as usize))
                    .collect());
            }
            capacity *= 2;
        }
    }
}

#[cfg(test)]
mod tests;
