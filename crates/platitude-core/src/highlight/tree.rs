//! One text highlighted whole by its grammar, cut into the per-row runs
//! the diff pane draws — the fast path of `highlight`. One parse per
//! side, so no walk to budget, no checkpoint and no quick pass.

use tree_sitter_highlight::{HighlightEvent, Highlighter};

use crate::parse::diff::{DiffLineKind, FilePatch};

use super::grammar::{self, Lang};
use super::{LineColors, PatchColors, Rgb, Span};

/// Colours for one ordinary unified patch of a language a grammar
/// knows. The side the diff's line numbers count in is highlighted
/// whole (when `source` is there and still agrees with the hunks); the
/// other side's rows, and every row when the file cannot be used, read
/// from a fragment stitched out of the hunk's own lines — the parser's
/// error recovery reads a fragment the way a reader does.
pub(super) fn patch_colors(lang: &Lang, patch: &FilePatch, source: Option<&str>) -> PatchColors {
    let new_side = patch.new_path.is_some();
    // One highlighter for the whole patch: it wraps a parser.
    let mut highlighter = Highlighter::new();
    let file: Option<Vec<Vec<Span>>> = source.and_then(|s| line_spans(lang, &mut highlighter, s));
    let source_lines: Vec<&str> = source.map(|s| s.lines().collect()).unwrap_or_default();
    patch
        .hunks
        .iter()
        .map(|hunk| {
            hunk_colors(
                lang,
                &mut highlighter,
                hunk,
                new_side,
                file.as_deref(),
                &source_lines,
            )
        })
        .collect()
}

fn hunk_colors(
    lang: &Lang,
    highlighter: &mut Highlighter,
    hunk: &crate::parse::diff::DiffHunk,
    new_side: bool,
    file: Option<&[Vec<Span>]>,
    source_lines: &[&str],
) -> Vec<LineColors> {
    // Each side's rows stitched into a fragment, built on first use and
    // indexed by that side's running row count: the other side always
    // reads from its fragment, this side only where the file is missing
    // or has drifted.
    let mut this_fragment: Option<Vec<Vec<Span>>> = None;
    let mut other_fragment: Option<Vec<Vec<Span>>> = None;
    let mut this_at = 0usize;
    let mut other_at = 0usize;
    let mut out = Vec::with_capacity(hunk.lines.len());
    for line in &hunk.lines {
        if line.kind == DiffLineKind::NoNewline {
            out.push(LineColors::nothing());
            continue;
        }
        let spans = if line.kind.on_side(new_side) {
            let at = this_at;
            this_at += 1;
            // A context line holds a place in the other side's fragment too.
            if line.kind == DiffLineKind::Context {
                other_at += 1;
            }
            let number = if new_side { line.new_no } else { line.old_no };
            let from_file = file.and_then(|lines| {
                let file_at = number?.checked_sub(1)? as usize;
                // A file that no longer says what the diff says cannot
                // colour it: drifted rows read from the fragment.
                (source_lines.get(file_at) == Some(&line.text.as_str()))
                    .then(|| lines.get(file_at))
                    .flatten()
            });
            match from_file {
                Some(spans) => spans.clone(),
                None => fragment_line(
                    lang,
                    highlighter,
                    hunk,
                    new_side,
                    at,
                    &mut this_fragment,
                    Side::This,
                ),
            }
        } else {
            let at = other_at;
            other_at += 1;
            fragment_line(
                lang,
                highlighter,
                hunk,
                new_side,
                at,
                &mut other_fragment,
                Side::Other,
            )
        };
        out.push(LineColors {
            spans,
            fence: false,
        });
    }
    out
}

/// Which of the hunk's two sides a fragment stitches together.
#[derive(Clone, Copy, PartialEq)]
enum Side {
    This,
    Other,
}

/// The colours of the `at`-th row of one side's fragment, building the
/// fragment on first use. `at` counts that side's own rows, the same
/// order the fragment stitched them in.
fn fragment_line(
    lang: &Lang,
    highlighter: &mut Highlighter,
    hunk: &crate::parse::diff::DiffHunk,
    new_side: bool,
    at: usize,
    cache: &mut Option<Vec<Vec<Span>>>,
    which: Side,
) -> Vec<Span> {
    let fragment = cache.get_or_insert_with(|| {
        let mut text = String::new();
        for line in &hunk.lines {
            if line.kind == DiffLineKind::NoNewline {
                continue;
            }
            let this = line.kind.on_side(new_side);
            let wanted = match which {
                Side::This => this,
                // The other side's rows are its changes and the shared
                // context — everything but this side's own changes.
                Side::Other => !this || line.kind == DiffLineKind::Context,
            };
            if wanted {
                text.push_str(&line.text);
                text.push('\n');
            }
        }
        line_spans(lang, highlighter, &text).unwrap_or_default()
    });
    fragment.get(at).cloned().unwrap_or_default()
}

/// Whether the grammar reads this text as its own language: a parse
/// with no error node in it. A grammar can predate syntax in the file
/// (Kotlin's `when` guards), and past the break its recovery can leave
/// every row plain; the caller then sends the file to the line-by-line
/// regex lexer.
pub(super) fn reads(lang: &Lang, text: &str) -> bool {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&lang.config.language).is_err() {
        return false;
    }
    parser
        .parse(text.as_bytes(), None)
        .is_some_and(|tree| !tree.root_node().has_error())
}

/// Every line of `text` as the runs the theme paints, or `None` when
/// the grammar refuses the text outright.
fn line_spans(lang: &Lang, highlighter: &mut Highlighter, text: &str) -> Option<Vec<Vec<Span>>> {
    let events = highlighter
        .highlight(&lang.config, text.as_bytes(), None, None, |name| {
            grammar::injection(name)
        })
        .ok()?;
    let mut out: Vec<Vec<Span>> = Vec::new();
    let mut line: Vec<Span> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for event in events {
        match event.ok()? {
            HighlightEvent::HighlightStart(h) => stack.push(h.0),
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                let color = stack
                    .last()
                    .and_then(|&i| grammar::style(i))
                    .unwrap_or(grammar::PLAIN);
                split_source(&mut out, &mut line, &text[start..end], color);
            }
        }
    }
    out.push(std::mem::take(&mut line));
    Some(out)
}

/// Appends one event's bytes to the lines, cutting at every `\n` and
/// leaving the terminators (and a `\r` before them) out of the runs —
/// the rows the spans are laid over never carry them.
fn split_source(out: &mut Vec<Vec<Span>>, line: &mut Vec<Span>, piece: &str, color: Rgb) {
    let mut rest = piece;
    while let Some(nl) = rest.find('\n') {
        let head = &rest[..nl];
        let head = head.strip_suffix('\r').unwrap_or(head);
        push_run(line, head.len(), color);
        out.push(std::mem::take(line));
        rest = &rest[nl + 1..];
    }
    push_run(line, rest.len(), color);
}

/// Runs the theme paints alike are one run (see `Walk::paint`).
fn push_run(line: &mut Vec<Span>, len: usize, color: Rgb) {
    if len == 0 {
        return;
    }
    match line.last_mut() {
        Some(last) if last.color == color => last.len += len,
        _ => line.push(Span { len, color }),
    }
}
