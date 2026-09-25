//! What the app said, read off a pipe of its own.
//!
//! **Read as bytes and decode each line lossily, to the stream's end —
//! never `BufRead::lines()`.** Qt writes stderr in the ANSI codepage
//! (CP932 on a Japanese Windows), so a non-ASCII line is not UTF-8. A
//! strict reader stops at it, the pipe's read end closes while the app
//! still runs, and the app dies on its next write with `0xC0000409` —
//! before the lines that carry the run's verdict. Linux writes UTF-8 and
//! never trips.

use std::io::{BufRead, BufReader, Read};

/// The app's output, one line at a time, however it spelled them.
pub(crate) struct Lines<R: Read> {
    reader: BufReader<R>,
}

/// Whole lines of `reader`, lossily decoded, to the end of the stream.
pub(crate) fn lines<R: Read>(reader: R) -> Lines<R> {
    Lines {
        reader: BufReader::new(reader),
    }
}

impl<R: Read> Iterator for Lines<R> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let mut raw = Vec::new();
        // A read error ends the stream like end-of-file: the verdict is
        // decided on what did arrive.
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

/// What one of the app's streams said, and when each line arrived. The
/// default is a stream the run never had: a child spawned without the
/// pipe, or a reader that could not be joined.
#[derive(Default)]
pub(crate) struct Said {
    pub(crate) lines: Vec<String>,
    /// How far into the read each line arrived, index for index with
    /// [`Self::lines`]. A run at a ceiling is judged by these: the silence
    /// up to a named line tells a process going round from one that
    /// stopped (`verify::wedge`).
    pub(crate) at: Vec<std::time::Duration>,
}

impl Said {
    /// When the stream last carried anything; `None` if it never did.
    pub(crate) fn last(&self) -> Option<std::time::Duration> {
        self.at.last().copied()
    }
}

/// Drains a pipe on its own thread, so a chatty child never blocks on a
/// full pipe while the parent waits for it to exit. The clock of
/// [`Said::at`] starts here, within microseconds of the spawn.
pub(crate) fn collect<R: Read + Send + 'static>(reader: R) -> std::thread::JoinHandle<Said> {
    collect_marking(reader, std::sync::Arc::default(), |_| false)
}

/// The same, raising `mark` the moment a line `when` recognises arrives,
/// so the parent's wait can end on a verdict already decided: QML that
/// would not load leaves a windowless app in its event loop until the
/// ceiling (`verify::child`).
pub(crate) fn collect_marking<R: Read + Send + 'static>(
    reader: R,
    mark: std::sync::Arc<std::sync::atomic::AtomicBool>,
    when: fn(&str) -> bool,
) -> std::thread::JoinHandle<Said> {
    std::thread::spawn(move || {
        // waits(measured): the stream's own clock, which the time of every line is
        // read off (`Said::at`) — worded into a verdict only
        let started = std::time::Instant::now();
        let mut said = Said {
            lines: Vec::new(),
            at: Vec::new(),
        };
        for line in lines(reader) {
            if when(&line) {
                mark.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            said.lines.push(line);
            said.at.push(started.elapsed());
        }
        said
    })
}

#[cfg(test)]
mod tests {
    /// A ceiling reads the silence up to one named line off the pair
    /// (`verify::wedge`): a stream that timed only some lines would answer
    /// for the wrong one.
    #[test]
    fn every_line_is_timed_at_its_own_index() {
        let said = super::collect(std::io::Cursor::new(b"one\ntwo\nthree".to_vec()))
            .join()
            .expect("the reader to finish");

        assert_eq!(said.lines.len(), 3, "{:?}", said.lines);
        assert_eq!(said.at.len(), said.lines.len());
        assert_eq!(said.last(), said.at.last().copied());
    }

    /// `qml: here…` as Qt writes it on a CP932 Windows, then the report
    /// the run is judged on: the line a strict reader stopped at, and the
    /// one it never reached.
    #[test]
    fn a_line_that_is_not_utf8_does_not_end_the_stream() {
        let mut bytes = b"first\nqml: here\x81\x63\n".to_vec();
        bytes.extend_from_slice(b"screenshot saved=true\n");

        let read: Vec<String> = super::lines(std::io::Cursor::new(bytes)).collect();

        assert_eq!(read.len(), 3, "{read:?}");
        assert_eq!(read[0], "first");
        // The CP932 bytes are past undoing; the ASCII around them is not.
        assert!(read[1].starts_with("qml: here"), "{read:?}");
        assert_eq!(read[2], "screenshot saved=true");
    }

    #[test]
    fn a_watched_line_raises_its_mark_and_the_rest_still_arrives() {
        let mark = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let said = super::collect_marking(
            std::io::Cursor::new(b"one\nthe needle\ntwo".to_vec()),
            std::sync::Arc::clone(&mark),
            |line| line.contains("needle"),
        )
        .join()
        .expect("the reader to finish");

        assert!(mark.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(said.lines.len(), 3, "{:?}", said.lines);
    }

    #[test]
    fn a_stream_with_nothing_to_watch_for_raises_nothing() {
        let mark = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        super::collect_marking(
            std::io::Cursor::new(b"one\ntwo\n".to_vec()),
            std::sync::Arc::clone(&mark),
            |line| line.contains("needle"),
        )
        .join()
        .expect("the reader to finish");

        assert!(!mark.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn the_last_line_arrives_without_a_newline_after_it() {
        let read: Vec<String> =
            super::lines(std::io::Cursor::new(b"one\r\ntwo".to_vec())).collect();
        assert_eq!(read, vec!["one".to_string(), "two".to_string()]);
    }
}
