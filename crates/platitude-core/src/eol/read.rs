//! Turning patch bytes into one reading per file.

use super::Reading;
use super::scan::{Scan, text_after};

/// One file of a patch and what its lines said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sighting {
    /// The path as the patch header spelled it, prefix stripped.
    ///
    /// Left C-quoted where git quoted it, as `parse::diff` does: a caller
    /// matching against known paths misses such a file — a lost mark,
    /// never a wrong one.
    pub path: String,
    pub reading: Reading,
}

/// Reads every file in a patch — one file's or a whole-tree `git diff`.
/// Empty when there is nothing to say.
pub fn read(raw: &[u8]) -> Vec<Sighting> {
    let mut out = Vec::new();
    let mut scan: Option<Scan> = None;

    let mut rest = raw;
    while !rest.is_empty() {
        let (line, tail, terminated) = match rest.iter().position(|b| *b == b'\n') {
            Some(i) => (&rest[..i], &rest[i + 1..], true),
            None => (rest, &rest[rest.len()..], false),
        };
        rest = tail;

        // Inside a hunk every line is content until the counts run out, so
        // a context line starting with `diff --git` is not a header.
        if let Some(s) = scan.as_mut()
            && s.in_hunk()
        {
            s.content(line, terminated);
            continue;
        }

        if line.starts_with(b"diff --git ") {
            flush(&mut out, scan.take());
            scan = Some(Scan::default());
            continue;
        }
        if line.starts_with(b"diff --cc ") || line.starts_with(b"diff --combined ") {
            flush(&mut out, scan.take());
            let mut s = Scan {
                path: text_after(line, b"diff --cc ")
                    .or_else(|| text_after(line, b"diff --combined ")),
                ..Scan::default()
            };
            // More than one old side: no single "before" to compare against.
            s.quiet = true;
            scan = Some(s);
            continue;
        }
        if let Some(path) = text_after(line, b"* Unmerged path ") {
            flush(&mut out, scan.take());
            scan = Some(Scan {
                path: Some(path),
                quiet: true,
                ..Scan::default()
            });
            continue;
        }

        let Some(s) = scan.as_mut() else {
            continue;
        };
        s.header(line);
    }
    flush(&mut out, scan);
    out
}

/// Reads a patch that covers one file. [`Reading::Quiet`] when it covers
/// none or more than one.
pub fn read_one(raw: &[u8]) -> Reading {
    match read(raw).as_slice() {
        [only] => only.reading,
        _ => Reading::Quiet,
    }
}

fn flush(out: &mut Vec<Sighting>, scan: Option<Scan>) {
    let Some(s) = scan else { return };
    let reading = s.settle();
    if reading == Reading::Quiet {
        return;
    }
    out.push(Sighting {
        path: s.path.unwrap_or_default(),
        reading,
    });
}
