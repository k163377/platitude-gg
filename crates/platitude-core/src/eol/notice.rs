//! What a reading and a baseline settle into, once there is
//! something to say about a file.

use super::{Baseline, Eol, Reading};

/// A settled statement about one file, ready to be worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// (a) `Line endings change · CRLF → LF`
    Flipped { from: Eol, to: Eol },
    /// (b) `Mixed line endings · 3 added lines use CRLF, this file uses LF`
    Mixed { lines: u32, added: Eol, file: Eol },
    /// (c) `New file uses CRLF · other .kt files here look like LF`
    NewFile { eol: Eol, baseline: Baseline },
    /// (d) `First line ending in this file · CRLF · …`
    FirstEnding { eol: Eol, baseline: Baseline },
}

/// Pairs a reading with a baseline. `None` means nothing is shown — also
/// for every unknown: no baseline, or a file that agrees with its
/// neighbours after all.
pub fn settle(reading: Reading, baseline: Option<&Baseline>) -> Option<Notice> {
    match reading {
        Reading::Quiet => None,
        Reading::Flipped { from, to } => Some(Notice::Flipped { from, to }),
        Reading::Mixed { lines, added, file } => Some(Notice::Mixed { lines, added, file }),
        Reading::NewFile { eol } => match baseline {
            Some(b) if b.eol != eol => Some(Notice::NewFile {
                eol,
                baseline: b.clone(),
            }),
            _ => None,
        },
        Reading::FirstEnding { eol } => match baseline {
            Some(b) if b.eol != eol => Some(Notice::FirstEnding {
                eol,
                baseline: b.clone(),
            }),
            _ => None,
        },
    }
}
