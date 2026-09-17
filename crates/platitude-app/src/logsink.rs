//! Where a log line goes, and where it goes when the stream it was meant
//! for will not take it.
//!
//! **Every write succeeds.** What it is standing in front of is
//! `tracing_subscriber`'s answer to a failed write, which is `eprintln!`
//! — and that panics when stderr is the thing that just failed. Anything
//! that makes stderr unwritable then takes the process with it at the
//! next log line: `exit 101` on the way up, and `0xC0000409` once QML is
//! driving, because the panic unwinds out of a slot Qt called and abort
//! is what crossing that boundary means (measured 2026-09-03 by closing
//! the read end at both times under `verify-ui`).
//!
//! Unwritable stderr is ordinary for a window. A GUI-subsystem process
//! started from the shell or the taskbar has no standard error at all —
//! `GetStdHandle` answers null and `std` turns that into
//! `ERROR_INVALID_HANDLE` on every write — and a run whose reader walked
//! away leaves a pipe nobody is holding open.
//!
//! So the line is offered somewhere else. On Windows that is
//! `OutputDebugStringW`, which is where Qt's own logging goes in
//! the shipped build for exactly the same reason (no console to write
//! to): one debugger window shows both halves of the app's log. Nowhere
//! else has a second sink that costs nothing, so nowhere else has one.

use std::io::{self, Write};

/// The app's log stream.
pub(crate) struct LogSink;

/// What `tracing_subscriber` is handed for `with_writer` — the shape
/// `std::io::stderr` has, so it goes in the same seat.
pub(crate) fn sink() -> LogSink {
    LogSink
}

impl Write for LogSink {
    /// **Answers `Ok` whatever happened.** A subscriber that is told the
    /// write failed reaches for stderr to say so, which is the road to
    /// the panic above; there is nothing further up that can do anything
    /// with the error either.
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

/// The second place to offer a line to, for a window whose stderr will
/// not take it.
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "OutputDebugStringW has no safe binding, and one signature does \
              not earn a dependency (winframe::win32 says the same)"
)]
fn elsewhere(line: &[u8]) {
    // Every line came from `format!`, so the lossy conversion is only
    // here because the signature cannot say so.
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
