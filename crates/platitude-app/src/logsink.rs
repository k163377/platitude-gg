//! Where a log line goes, and where it goes when stderr will not take it.
//!
//! **Every write succeeds.** `tracing_subscriber` answers a failed write
//! with `eprintln!`, which panics when stderr is what failed — and a panic
//! out of a slot Qt called aborts the process. Unwritable stderr is
//! ordinary for a window: a GUI-subsystem process started from the shell
//! or the taskbar has none (`ERROR_INVALID_HANDLE` on every write), and a
//! reader that walked away leaves a pipe nobody holds open.
//!
//! The fallback is `OutputDebugStringW` on Windows, where Qt's own logging
//! goes in the shipped build, so one debugger window shows both. Nowhere
//! else has a second sink that costs nothing.

use std::io::{self, Write};

pub(crate) struct LogSink;

/// For `with_writer`: the shape `std::io::stderr` has.
pub(crate) fn sink() -> LogSink {
    LogSink
}

impl Write for LogSink {
    /// Answers `Ok` whatever happened: a subscriber told of a failure
    /// reaches for stderr, the road to the panic above.
    fn write(&mut self, line: &[u8]) -> io::Result<usize> {
        if io::stderr().write_all(line).is_err() {
            elsewhere(line);
        }
        Ok(line.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Where a line stderr would not take goes instead.
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "OutputDebugStringW has no safe binding, and one signature does \
              not earn a dependency (winframe::win32 says the same)"
)]
fn elsewhere(line: &[u8]) {
    // Lines come from `format!`; lossy only because the signature cannot
    // say so.
    let mut wide: Vec<u16> = String::from_utf8_lossy(line).encode_utf16().collect();
    wide.push(0);
    // SAFETY: the pointer is to a NUL-terminated UTF-16 buffer that
    // outlives the call, and the callee only reads it.
    unsafe { OutputDebugStringW(wide.as_ptr()) };
}

// SAFETY: the signature is transcribed from the Win32 headers
// (debugapi.h); the argument is a pointer the callee only reads.
#[cfg(windows)]
#[expect(unsafe_code, reason = "as above")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn OutputDebugStringW(text: *const u16);
}

#[cfg(not(windows))]
fn elsewhere(_line: &[u8]) {}
