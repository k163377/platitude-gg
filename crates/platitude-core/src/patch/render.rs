//! Writing one hunk out again: which of its lines go, what each of them
//! costs the two sides, and the header that has to add up afterwards.
//!
//! Recounting follows git's own partial-staging rule: a forward patch keeps
//! every old-side line (an unselected deletion becomes context), so its
//! `old` range is unchanged and only the `new` range is recomputed. The
//! reverse case (unstaging, applied with `git apply -R`) is the mirror
//! image.

use std::borrow::Cow;

use super::{Body, HunkSelect, PatchSide, RawHunk, classify, push_line};

/// A body line of the source hunk with the `\ No newline` marker that
/// followed it. The marker describes the line above it, so it travels with
/// that line rather than with a position in the output.
struct SourceLine<'a> {
    bytes: &'a [u8],
    kind: Body,
    selected: bool,
    marker: Option<&'a [u8]>,
}

/// A line to write out, with what it costs each side.
struct Emitted<'a> {
    bytes: Cow<'a, [u8]>,
    old: bool,
    new: bool,
    changed: bool,
    /// Written with its original marker byte; a demoted line is not.
    verbatim: bool,
    marker: Option<&'a [u8]>,
}

/// Reads a hunk body into indexed lines, folding each `\ No newline` marker
/// into the line it describes. Marker lines still consume an index so that
/// selections stay aligned with [`crate::parse::diff`].
fn source_lines<'a>(hunk: &RawHunk<'a>, select: &HunkSelect) -> Vec<SourceLine<'a>> {
    let mut out: Vec<SourceLine<'a>> = Vec::new();
    let mut index = 0usize;
    for &line in &hunk.body {
        let Some(kind) = classify(line) else {
            // Unclassifiable lines are skipped by the parser too, so the
            // index stays aligned by not counting them.
            continue;
        };
        let this = index;
        index += 1;
        if kind == Body::NoNewline {
            if let Some(previous) = out.last_mut() {
                previous.marker = Some(line);
            }
            continue;
        }
        out.push(SourceLine {
            bytes: line,
            kind,
            selected: select.includes(this),
            marker: None,
        });
    }
    out
}

fn keep_line<'a>(src: &SourceLine<'a>) -> Emitted<'a> {
    Emitted {
        bytes: Cow::Borrowed(src.bytes),
        old: matches!(src.kind, Body::Deletion | Body::Context),
        new: matches!(src.kind, Body::Addition | Body::Context),
        changed: src.kind != Body::Context,
        verbatim: true,
        marker: src.marker,
    }
}

/// Writes a line the patch leaves alone: it belongs to both sides now.
fn demote_line<'a>(src: &SourceLine<'a>) -> Emitted<'a> {
    Emitted {
        bytes: Cow::Owned(to_context(src.bytes)),
        old: true,
        new: true,
        changed: false,
        verbatim: false,
        marker: src.marker,
    }
}

/// Emits one run of changed lines.
///
/// The side being applied to owns the lines that survive unselected: a
/// forward patch keeps every old-side line (an unselected deletion becomes
/// context), a reverse patch keeps every new-side line. A kept line now
/// belongs to both sides, so where it sits decides the order of the result
/// — left in the source's own place it would sort ahead of the lines that
/// replace the lines above it. Pairing each selected kept line with a
/// selected counterpart puts it back where the reader expects it.
fn render_block<'a>(block: &[SourceLine<'a>], side: PatchSide, out: &mut Vec<Emitted<'a>>) {
    let (kept, counterpart) = match side {
        PatchSide::Forward => (Body::Deletion, Body::Addition),
        PatchSide::Reverse => (Body::Addition, Body::Deletion),
    };

    // With nothing demoted, every line keeps its marker byte and the source
    // order already holds, so the run goes out byte for byte.
    if !block.iter().any(|l| l.kind == kept && !l.selected) {
        out.extend(block.iter().filter(|l| l.selected).map(keep_line));
        return;
    }

    let keeps: Vec<&SourceLine<'a>> = block.iter().filter(|l| l.kind == kept).collect();
    let mates: Vec<&SourceLine<'a>> = block
        .iter()
        .filter(|l| l.kind == counterpart && l.selected)
        .collect();
    let last_selected = keeps.iter().rposition(|l| l.selected);

    let mut body: Vec<Emitted<'a>> = Vec::new();
    let mut taken = 0usize;
    for (i, line) in keeps.iter().copied().enumerate() {
        if !line.selected {
            body.push(demote_line(line));
            continue;
        }
        // One counterpart per selected line, and whatever is left over on
        // the last of them.
        let take = if Some(i) == last_selected {
            mates.len() - taken
        } else {
            (mates.len() - taken).min(1)
        };
        let pair = mates[taken..taken + take].iter().copied().map(keep_line);
        taken += take;
        // A hunk reads its old side before its new one, so the counterpart
        // trails a forward patch's deletion and leads a reverse patch's
        // addition.
        match side {
            PatchSide::Forward => {
                body.push(keep_line(line));
                body.extend(pair);
            }
            PatchSide::Reverse => {
                body.extend(pair);
                body.push(keep_line(line));
            }
        }
    }

    if last_selected.is_none() {
        // Nothing to pair with: the counterparts keep the source's own
        // order against the run.
        let rest = mates.iter().copied().map(keep_line);
        match side {
            PatchSide::Forward => {
                out.append(&mut body);
                out.extend(rest);
            }
            PatchSide::Reverse => {
                out.extend(rest);
                out.append(&mut body);
            }
        }
        return;
    }
    out.append(&mut body);
}

/// Rewrites one hunk for the selection, updating the running side offset.
/// Returns `None` when nothing changed remains.
pub(super) fn render_hunk(
    hunk: &RawHunk<'_>,
    select: &HunkSelect,
    side: PatchSide,
    offset: &mut i64,
) -> Option<Vec<u8>> {
    let (old_start, new_start, heading) = parse_header(hunk.header)?;

    let source = source_lines(hunk, select);
    let mut emitted: Vec<Emitted<'_>> = Vec::new();
    let mut at = 0usize;
    while at < source.len() {
        if source[at].kind == Body::Context {
            emitted.push(keep_line(&source[at]));
            at += 1;
            continue;
        }
        let end = source[at..]
            .iter()
            .position(|l| l.kind == Body::Context)
            .map_or(source.len(), |p| at + p);
        render_block(&source[at..end], side, &mut emitted);
        at = end;
    }

    let mut body: Vec<u8> = Vec::new();
    let mut old_count: u32 = 0;
    let mut new_count: u32 = 0;
    let mut changes = 0usize;
    let last = emitted.len().saturating_sub(1);
    for (i, line) in emitted.iter().enumerate() {
        push_line(&mut body, &line.bytes);
        old_count += u32::from(line.old);
        new_count += u32::from(line.new);
        changes += usize::from(line.changed);
        // The marker describes the line above it. It holds on a line
        // written with its original marker byte, and on a demoted line only
        // while nothing follows: a line after it would contradict the claim
        // that the content ends here.
        if let Some(marker) = line.marker
            && (line.verbatim || i == last)
        {
            push_line(&mut body, marker);
        }
    }

    if changes == 0 {
        return None;
    }

    // The side the patch is applied to keeps its original numbering; the
    // other side is renumbered against the hunks actually emitted.
    let (old_start, new_start) = match side {
        PatchSide::Forward => (old_start, (old_start as i64 + *offset).max(0) as u32),
        PatchSide::Reverse => ((new_start as i64 + *offset).max(0) as u32, new_start),
    };
    *offset += match side {
        PatchSide::Forward => new_count as i64 - old_count as i64,
        PatchSide::Reverse => old_count as i64 - new_count as i64,
    };

    let mut out = Vec::with_capacity(body.len() + 64);
    out.extend_from_slice(
        format!("@@ -{old_start},{old_count} +{new_start},{new_count} @@").as_bytes(),
    );
    out.extend_from_slice(heading);
    out.push(b'\n');
    out.extend_from_slice(&body);
    Some(out)
}

fn to_context(line: &[u8]) -> Vec<u8> {
    let mut v = line.to_vec();
    if let Some(first) = v.first_mut() {
        *first = b' ';
    }
    v
}

/// Extracts `(old_start, new_start, heading)` from `@@ -a,b +c,d @@ head`.
/// The heading keeps its leading bytes verbatim (usually a space).
fn parse_header(line: &[u8]) -> Option<(u32, u32, &[u8])> {
    let text = std::str::from_utf8(line).ok()?;
    let rest = text.strip_prefix("@@ ")?;
    let close = rest.find("@@")?;
    let (ranges, heading_at) = (&rest[..close], close + 2);
    let heading = &line[3 + heading_at..];
    let mut parts = ranges.split_whitespace();
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let start = |s: &str| -> Option<u32> {
        match s.split_once(',') {
            Some((a, _)) => a.parse().ok(),
            None => s.parse().ok(),
        }
    };
    Some((start(old)?, start(new)?, heading))
}
