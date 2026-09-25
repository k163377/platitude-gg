//! Partial-patch construction for hunk- and line-level staging.
//!
//! Works on the raw bytes of a `git diff` run: rebuilding from
//! [`crate::parse::diff`] would lose bytes `git apply` cares about (a CRLF
//! file's `\r`, trailing whitespace, the index header). Hunk and line
//! positions match what the parser yields for the same bytes, so the UI
//! selects on the parsed model while the patch is rebuilt from source.
//!
//! This file is the scan; writing a hunk out again is [`render`].

mod render;

use render::render_hunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchSide {
    /// Applied as-is, onto the patch's old side (staging).
    Forward,
    /// Applied with `git apply -R`, onto the patch's new side (unstaging).
    Reverse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkSelect {
    /// Hunk index within the whole patch, counted in output order (the
    /// same order [`crate::parse::diff::parse_patch`] yields).
    pub hunk: usize,
    /// Indices of the changed lines to include, addressing the hunk's line
    /// list as the parser produces it. `None` selects the entire hunk.
    pub lines: Option<Vec<usize>>,
}

impl HunkSelect {
    pub fn whole(hunk: usize) -> Self {
        Self { hunk, lines: None }
    }

    pub fn lines(hunk: usize, lines: impl IntoIterator<Item = usize>) -> Self {
        Self {
            hunk,
            lines: Some(lines.into_iter().collect()),
        }
    }

    fn includes(&self, line: usize) -> bool {
        match &self.lines {
            None => true,
            Some(set) => set.contains(&line),
        }
    }
}

/// Builds a patch containing only the selected hunks/lines.
///
/// Returns `None` when the selection produces nothing to apply (no matching
/// hunk, or every selected hunk collapsed to pure context).
pub fn build_partial(raw: &[u8], selects: &[HunkSelect], side: PatchSide) -> Option<Vec<u8>> {
    let files = split_files(raw);
    let mut out: Vec<u8> = Vec::with_capacity(raw.len());
    let mut hunk_index = 0usize;

    for file in &files {
        let mut file_out: Vec<u8> = Vec::new();
        // Side offset over the emitted hunks; per file, as line numbers are.
        let mut offset: i64 = 0;
        for hunk in &file.hunks {
            let index = hunk_index;
            hunk_index += 1;
            let Some(select) = selects.iter().find(|s| s.hunk == index) else {
                continue;
            };
            if let Some(rendered) = render_hunk(hunk, select, side, &mut offset) {
                file_out.extend_from_slice(&rendered);
            }
        }
        if file_out.is_empty() {
            continue;
        }
        for line in &file.header {
            push_line(&mut out, line);
        }
        out.extend_from_slice(&file_out);
    }

    (!out.is_empty()).then_some(out)
}

/// Number of hunks in a raw patch (across all files), matching the indices
/// [`build_partial`] expects.
pub fn hunk_count(raw: &[u8]) -> usize {
    split_files(raw).iter().map(|f| f.hunks.len()).sum()
}

/// Whether these bytes are the combined form git prints for a path with
/// more than one side (`diff --cc`, hunks headed `@@@`). Nothing can be
/// built from one — there is no single pre-image — so the staging path
/// refuses it here, underneath the UI withholding the pieces on a
/// conflicted file.
pub fn is_combined(raw: &[u8]) -> bool {
    lines(raw).any(|line| {
        line.starts_with(b"@@@")
            || line.starts_with(b"diff --cc ")
            || line.starts_with(b"diff --combined ")
    })
}

// --- raw splitting ------------------------------------------------------

struct RawFile<'a> {
    /// Everything from `diff --git` up to the first hunk, verbatim.
    header: Vec<&'a [u8]>,
    hunks: Vec<RawHunk<'a>>,
}

struct RawHunk<'a> {
    header: &'a [u8],
    body: Vec<&'a [u8]>,
}

/// Body lines always carry a marker byte (` `, `+`, `-`, `\`), so a line
/// starting with `@@ ` or `diff --git ` is unambiguously a header.
fn split_files(raw: &[u8]) -> Vec<RawFile<'_>> {
    let mut files: Vec<RawFile<'_>> = Vec::new();
    let mut current: Option<RawFile<'_>> = None;
    let mut hunk: Option<RawHunk<'_>> = None;

    fn flush_hunk<'a>(current: &mut Option<RawFile<'a>>, hunk: &mut Option<RawHunk<'a>>) {
        if let (Some(file), Some(h)) = (current.as_mut(), hunk.take()) {
            file.hunks.push(h);
        }
    }

    for line in lines(raw) {
        if line.starts_with(b"diff --git ") {
            flush_hunk(&mut current, &mut hunk);
            if let Some(f) = current.take() {
                files.push(f);
            }
            current = Some(RawFile {
                header: vec![line],
                hunks: Vec::new(),
            });
            continue;
        }
        if line.starts_with(b"@@ ") {
            flush_hunk(&mut current, &mut hunk);
            if current.is_none() {
                // Headerless hunk: `file_diff_raw` never produces it, but
                // the rewriting tests feed it bare.
                current = Some(RawFile {
                    header: Vec::new(),
                    hunks: Vec::new(),
                });
            }
            hunk = Some(RawHunk {
                header: line,
                body: Vec::new(),
            });
            continue;
        }
        match hunk.as_mut() {
            Some(h) => h.body.push(line),
            None => {
                if let Some(f) = current.as_mut() {
                    f.header.push(line);
                }
            }
        }
    }
    flush_hunk(&mut current, &mut hunk);
    if let Some(f) = current.take() {
        files.push(f);
    }
    files
}

/// Splits on `\n`, keeping every byte of each line (including a trailing
/// `\r`) and dropping the empty remainder after a final newline.
fn lines(raw: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut split = raw.split(|b| *b == b'\n').peekable();
    std::iter::from_fn(move || {
        let line = split.next()?;
        if line.is_empty() && split.peek().is_none() {
            return None;
        }
        Some(line)
    })
}

fn push_line(out: &mut Vec<u8>, line: &[u8]) {
    out.extend_from_slice(line);
    out.push(b'\n');
}

// --- what a body line is ------------------------------------------------

/// Classification of a body line, mirroring [`crate::parse::diff`] so line
/// indices agree between the two.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    Context,
    Addition,
    Deletion,
    NoNewline,
}

fn classify(line: &[u8]) -> Option<Body> {
    match line.first() {
        Some(b' ') => Some(Body::Context),
        Some(b'+') => Some(Body::Addition),
        Some(b'-') => Some(Body::Deletion),
        Some(b'\\') => Some(Body::NoNewline),
        // An empty line is empty context, as the parser reads it.
        None => Some(Body::Context),
        _ => None,
    }
}
