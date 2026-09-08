//! What the app said, read off a pipe of its own.
//!
//! **Never `BufRead::lines()` on the app's output.** Qt's default message
//! handler converts every line it writes to stderr into the process's
//! ANSI codepage first, and on a Japanese Windows that is CP932 — so a
//! QML warning naming a branch, a path or a menu row with a character
//! outside ASCII arrives as bytes that are not UTF-8. `lines()` answers
//! `Err(InvalidData)` for one of those, and a reader built on
//! `map_while(Result::ok)` stops there: the thread returns, the
//! `ChildStderr` it owned drops, and the read end of the pipe closes
//! **while the app is still running**. The app then dies on its next
//! write with `0xC0000409`, having said nothing about why, and the run
//! reports a screenshot that was never taken (2026-09-03, measured with
//! `console.log("…")` under `verify-ui`).
//!
//! So: decode per line and lossily, and never stop early. The bytes are
//! the app's own choice of encoding and nothing here can undo it — a
//! CP932 line reads as replacement characters — but a line nobody can
//! spell is still a line that must be carried to the end of the stream,
//! because what follows it is the run's verdict.
//!
//! Linux is not exempt by luck: there the same conversion is UTF-8, so
//! the strict reader happened never to trip.

use std::io::{BufRead, BufReader, Read};

/// The app's output, one line at a time, however it spelled them.
pub(crate) struct Lines<R: Read> {
    reader: BufReader<R>,
}

/// Reads `reader` as the app's output: whole lines, lossily decoded, to
/// the end of the stream.
pub(crate) fn lines<R: Read>(reader: R) -> Lines<R> {
    Lines {
        reader: BufReader::new(reader),
    }
}

impl<R: Read> Iterator for Lines<R> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let mut raw = Vec::new();
        // A read error ends the stream the way end-of-file does: there is
        // nothing further to carry, and the caller's verdict is decided
        // on what it did get.
        match self.reader.read_until(b'\n', &mut raw) {
            Ok(0) | Err(_) => return None,
            Ok(_) => {}
        }
        while matches!(raw.last(), Some(b'\n' | b'\r')) {
            raw.pop();
        }
        Some(String::from_utf8_lossy(&raw).into_owned())
    }
}

/// What one of the app's streams said, and when each of it arrived. The
/// default is the stream a run never had at all — a child spawned
/// without the pipe, or one whose reader could not be joined.
#[derive(Default)]
pub(crate) struct Said {
    pub(crate) lines: Vec<String>,
    /// How far into the read each line arrived, at the index of the line
    /// it belongs to — so the same length as [`Self::lines`], and empty
    /// where the app never wrote to this stream at all. A run that
    /// reached a ceiling is read off these: how long it had been silent
    /// is the difference between a process going round and one that
    /// stopped, and **which** line the silence is counted to is the whole
    /// question for a run that ended itself, whose account is a line here
    /// like any other (`verify::wedge`).
    pub(crate) at: Vec<std::time::Duration>,
}

impl Said {
    /// When the stream last carried anything, and `None` where it never
    /// carried anything at all.
    pub(crate) fn last(&self) -> Option<std::time::Duration> {
        self.at.last().copied()
    }
}

/// Drains a pipe on its own thread, so a chatty child never blocks on a
/// full pipe while the parent waits for it to exit — and so the read end
/// stays open for as long as the app has anything to say.
///
/// The clock starts here rather than being passed in: this is within
/// microseconds of the spawn, and a stream's own start is what the times
/// off it are wanted against.
pub(crate) fn collect<R: Read + Send + 'static>(reader: R) -> std::thread::JoinHandle<Said> {
    std::thread::spawn(move || {
        // waits(measured): the stream's own clock, which the time of every line is
        // read off (`Said::at`) — worded into a verdict, never deciding one
        let started = std::time::Instant::now();
        let mut said = Said {
            lines: Vec::new(),
            at: Vec::new(),
        };
        for line in lines(reader) {
            said.lines.push(line);
            said.at.push(started.elapsed());
        }
        said
    })
}

#[cfg(test)]
mod tests {
    /// A moment for every line, at that line's own index. A ceiling reads
    /// the silence that ran up to one named line off the pair
    /// (`verify::wedge`), so a stream that timed only some of what it
    /// carried would answer for the wrong one.
    #[test]
    fn every_line_is_timed_at_its_own_index() {
        let said = super::collect(std::io::Cursor::new(b"one\ntwo\nthree".to_vec()))
            .join()
            .expect("the reader to finish");

        assert_eq!(said.lines.len(), 3, "{:?}", said.lines);
        assert_eq!(said.at.len(), said.lines.len());
        assert_eq!(said.last(), said.at.last().copied());
    }

    /// The line the strict reader stopped at, and the one it never
    /// reached: `qml: logprobe Create branch here…` as Qt writes it on a
    /// CP932 Windows, followed by the report the run is judged on.
    #[test]
    fn a_line_that_is_not_utf8_does_not_end_the_stream() {
        let mut bytes = b"first\nqml: here\x81\x63\n".to_vec();
        bytes.extend_from_slice(b"screenshot saved=true\n");

        let read: Vec<String> = super::lines(std::io::Cursor::new(bytes)).collect();

        assert_eq!(read.len(), 3, "{read:?}");
        assert_eq!(read[0], "first");
        // Delivered as a line, and as much of it as can be spelled: the
        // bytes Qt chose are past undoing, the ASCII around them is not.
        assert!(read[1].starts_with("qml: here"), "{read:?}");
        assert_eq!(read[2], "screenshot saved=true");
    }

    #[test]
    fn the_last_line_arrives_without_a_newline_after_it() {
        let read: Vec<String> =
            super::lines(std::io::Cursor::new(b"one\r\ntwo".to_vec())).collect();
        assert_eq!(read, vec!["one".to_string(), "two".to_string()]);
    }
}
