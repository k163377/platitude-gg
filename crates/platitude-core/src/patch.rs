//! Partial-patch construction for hunk- and line-level staging.
//!
//! Operates on the **raw bytes** of a `git diff` run, never on the parsed
//! model: rebuilding a patch from [`crate::parse::diff`] would lose bytes
//! `git apply` cares about (a CRLF file's `\r`, trailing whitespace, the
//! exact index header). Selections address hunks and lines positionally and
//! those positions match what the parser produces for the same bytes, so the
//! UI can select on the parsed model while the patch is rebuilt from source.
//!
//! Recounting follows git's own partial-staging rule: a forward patch keeps
//! every old-side line (an unselected deletion becomes context), so its
//! `old` range is unchanged and only the `new` range is recomputed. The
//! reverse case (unstaging, applied with `git apply -R`) is the mirror
//! image.

use std::borrow::Cow;

/// Which side a rebuilt patch will be applied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchSide {
    /// Applied as-is, onto the patch's old side (staging).
    Forward,
    /// Applied with `git apply -R`, onto the patch's new side (unstaging).
    Reverse,
}

/// One selected hunk of a patch.
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
    /// Selects a whole hunk.
    pub fn whole(hunk: usize) -> Self {
        Self { hunk, lines: None }
    }

    /// Selects individual changed lines of a hunk.
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
        // Offset between the two sides accumulated over emitted hunks;
        // resets per file because line numbers are per file.
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

/// Whether these bytes are the **combined** form git prints for a path
/// with more than one side (`diff --cc`, hunks headed `@@@`).
///
/// Nothing can be built from one: there is no single pre-image for a
/// rebuilt patch to sit on, and `git apply` refuses the whole shape. The
/// UI already withholds the pieces on a conflicted file
/// ([`crate::parse::diff`]), and this is the check underneath that — a
/// selection that reached here anyway must be refused rather than turned
/// into a patch git will reject with its own wording.
///
/// Read off the bytes rather than the parsed model because that is what
/// the staging path has in hand ([`build_partial`] never parses).
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

/// Splits a patch into files and hunks without interpreting content.
///
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
                // Patch without a `diff --git` header (diff between blobs).
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

// --- hunk rewriting -----------------------------------------------------

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
        // A wholly empty line inside a hunk is an empty context line (the
        // parser tolerates the same shape).
        None => Some(Body::Context),
        _ => None,
    }
}

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

/// Writes a line exactly as the source has it.
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
fn render_hunk(
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

/// Replaces the marker byte with a space (deletion/addition → context).
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

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_HUNKS: &str = "\
diff --git a/f.txt b/f.txt
index 1111111..2222222 100644
--- a/f.txt
+++ b/f.txt
@@ -1,3 +1,4 @@ fn head
 one
-two
+two changed
+two and a half
 three
@@ -10,3 +11,3 @@
 ten
-eleven
+eleven changed
 twelve
";

    fn text(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).expect("utf-8 patch")
    }

    #[test]
    fn counts_hunks() {
        assert_eq!(hunk_count(TWO_HUNKS.as_bytes()), 2);
        assert_eq!(hunk_count(b""), 0);
    }

    #[test]
    fn whole_hunk_selection_keeps_the_hunk_verbatim() {
        let out = build_partial(
            TWO_HUNKS.as_bytes(),
            &[HunkSelect::whole(0)],
            PatchSide::Forward,
        )
        .expect("patch");
        let out = text(out);
        assert!(out.contains("--- a/f.txt"), "file header kept");
        assert!(out.contains("@@ -1,3 +1,4 @@ fn head"), "got: {out}");
        assert!(out.contains("+two and a half"));
        assert!(!out.contains("eleven"), "second hunk dropped");
    }

    #[test]
    fn second_hunk_alone_is_renumbered_against_the_old_side() {
        let out = text(
            build_partial(
                TWO_HUNKS.as_bytes(),
                &[HunkSelect::whole(1)],
                PatchSide::Forward,
            )
            .expect("patch"),
        );
        // Applied on its own the new side starts where the old side does.
        assert!(out.contains("@@ -10,3 +10,3 @@"), "got: {out}");
    }

    #[test]
    fn forward_line_selection_demotes_unselected_deletions_to_context() {
        // Body indices: 0 " one", 1 "-two", 2 "+two changed",
        // 3 "+two and a half", 4 " three". Stage only the second addition.
        let out = text(
            build_partial(
                TWO_HUNKS.as_bytes(),
                &[HunkSelect::lines(0, [3])],
                PatchSide::Forward,
            )
            .expect("patch"),
        );
        assert!(out.contains("@@ -1,3 +1,4 @@"), "got: {out}");
        assert!(out.contains(" two\n"), "unselected deletion became context");
        assert!(!out.contains("+two changed"));
        assert!(out.contains("+two and a half"));
    }

    #[test]
    fn forward_deletion_only_selection_shrinks_the_new_side() {
        let out = text(
            build_partial(
                TWO_HUNKS.as_bytes(),
                &[HunkSelect::lines(0, [1])],
                PatchSide::Forward,
            )
            .expect("patch"),
        );
        // The old side is invariant (3); the new side loses the deletion
        // and gains neither addition.
        assert!(out.contains("@@ -1,3 +1,2 @@"), "got: {out}");
        assert!(out.contains("-two\n"));
        assert!(!out.contains("two changed"));
    }

    #[test]
    fn reverse_line_selection_demotes_unselected_additions_to_context() {
        let out = text(
            build_partial(
                TWO_HUNKS.as_bytes(),
                &[HunkSelect::lines(0, [1])],
                PatchSide::Reverse,
            )
            .expect("patch"),
        );
        // The new side is the pre-image when applying with -R, so it keeps
        // its original numbering and count (4); the old side grows because
        // the additions that stay staged become context.
        assert!(out.contains("@@ -1,5 +1,4 @@"), "got: {out}");
        assert!(out.contains("-two\n"), "selected deletion kept");
        assert!(
            out.contains(" two changed\n") && out.contains(" two and a half\n"),
            "unselected additions became context: {out}"
        );
    }

    #[test]
    fn reverse_second_hunk_alone_is_renumbered_against_the_new_side() {
        let out = text(
            build_partial(
                TWO_HUNKS.as_bytes(),
                &[HunkSelect::whole(1)],
                PatchSide::Reverse,
            )
            .expect("patch"),
        );
        assert!(out.contains("@@ -11,3 +11,3 @@"), "got: {out}");
    }

    #[test]
    fn all_context_selection_yields_nothing() {
        assert!(
            build_partial(
                TWO_HUNKS.as_bytes(),
                &[HunkSelect::lines(0, [])],
                PatchSide::Forward,
            )
            .is_none()
        );
        assert!(build_partial(TWO_HUNKS.as_bytes(), &[], PatchSide::Forward).is_none());
    }

    #[test]
    fn crlf_content_survives_byte_for_byte() {
        let raw = "--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n x\r\n-y\r\n+z\r\n";
        let out = build_partial(raw.as_bytes(), &[HunkSelect::whole(0)], PatchSide::Forward)
            .expect("patch");
        assert!(
            out.windows(4).any(|w| w == b"+z\r\n"),
            "carriage return kept: {:?}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn no_newline_marker_follows_its_line() {
        let raw = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
 keep
-old
\\ No newline at end of file
+new
\\ No newline at end of file
";
        // Selecting only the addition drops the deletion's marker with it.
        let out = text(
            build_partial(
                raw.as_bytes(),
                &[HunkSelect::lines(0, [3])],
                PatchSide::Forward,
            )
            .expect("patch"),
        );
        assert_eq!(out.matches("\\ No newline").count(), 1, "got: {out}");
        assert!(out.contains(" old\n"), "deletion demoted to context: {out}");
    }

    #[test]
    fn a_kept_last_line_keeps_its_no_newline_marker() {
        let raw = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
-one
-two
\\ No newline at end of file
+ONE
+TWO
\\ No newline at end of file
";
        // Body indices: 0 "-one", 1 "-two", 2 marker, 3 "+ONE", 4 "+TWO".
        // Staging one -> ONE leaves "two" as the last line of both sides,
        // so the file still ends where it did.
        let out = text(
            build_partial(
                raw.as_bytes(),
                &[HunkSelect::lines(0, [0, 3])],
                PatchSide::Forward,
            )
            .expect("patch"),
        );
        assert_eq!(
            out, "@@ -1,2 +1,2 @@\n-one\n+ONE\n two\n\\ No newline at end of file\n",
            "got: {out}"
        );
    }

    /// A replacement lists all of its deletions before its additions, so a
    /// line left unselected has to move past the additions that replace the
    /// lines above it.
    const REPLACED_PAIR: &str = "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
-one
-two
+ONE
+TWO
 tail
";

    #[test]
    fn forward_line_selection_keeps_an_unselected_line_in_place() {
        // Body indices: 0 "-one", 1 "-two", 2 "+ONE", 3 "+TWO".
        // Stage one -> ONE only: the result has to read ONE, two, tail.
        let out = text(
            build_partial(
                REPLACED_PAIR.as_bytes(),
                &[HunkSelect::lines(0, [0, 2])],
                PatchSide::Forward,
            )
            .expect("patch"),
        );
        assert_eq!(
            out, "@@ -1,3 +1,3 @@\n-one\n+ONE\n two\n tail\n",
            "got: {out}"
        );
    }

    #[test]
    fn reverse_line_selection_keeps_an_unselected_line_in_place() {
        // Unstaging two -> TWO: applied with -R the new side is the
        // pre-image, so ONE has to stay above the line being restored.
        let out = text(
            build_partial(
                REPLACED_PAIR.as_bytes(),
                &[HunkSelect::lines(0, [1, 3])],
                PatchSide::Reverse,
            )
            .expect("patch"),
        );
        assert_eq!(
            out, "@@ -1,3 +1,3 @@\n ONE\n-two\n+TWO\n tail\n",
            "got: {out}"
        );
    }

    #[test]
    fn multi_file_patch_keeps_only_files_with_selected_hunks() {
        let raw = format!(
            "{TWO_HUNKS}diff --git a/g.txt b/g.txt\n--- a/g.txt\n+++ b/g.txt\n@@ -1 +1 @@\n-a\n+b\n"
        );
        let out = text(
            build_partial(raw.as_bytes(), &[HunkSelect::whole(2)], PatchSide::Forward)
                .expect("patch"),
        );
        assert!(out.contains("g.txt"));
        assert!(!out.contains("f.txt"), "unselected file dropped: {out}");
    }
}
