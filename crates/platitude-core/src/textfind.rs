//! What typed lines match in the text of a diff (デザイン規約 §diff の中を探す).
//!
//! A diff is two files read through one patch: the new side is its
//! context and added lines in order, the old side its context and removed
//! lines. A query is asked of each side as one text — so a query of two
//! lines matches where a line's end runs into the next line's start on
//! that side, over whatever the other side put between them — and a hit
//! that lies on context lines alone is reported once, from the new side.
//!
//! Every option is one regular expression: the typed lines escaped or
//! taken as a pattern, case folded or not, and a whole word judged at the
//! hit's two ends (a hit that starts or ends on a word character must not
//! continue one). Nothing here knows rows or columns: a hit is spans of
//! bytes on lines named by the caller's ids.

use std::ops::Range;

use crate::parse::diff::{DiffLineKind, FilePatch};

/// How a query reads what was typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    pub match_case: bool,
    pub whole_word: bool,
    pub regex: bool,
}

/// The query could not be read as a pattern: regex's own reason, its last
/// line, for the bar's tooltip — as a sentence, so its first letter is a
/// capital (regex writes `unclosed group`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct QueryError(String);

impl From<regex::Error> for QueryError {
    fn from(error: regex::Error) -> Self {
        let text = error.to_string();
        let last = text.lines().last().unwrap_or("").trim();
        let reason = last.strip_prefix("error: ").unwrap_or(last);
        let mut chars = reason.chars();
        Self(match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        })
    }
}

/// Typed lines, read once for every line they are asked of.
#[derive(Debug, Clone)]
pub struct Query {
    pattern: regex::Regex,
    whole_word: bool,
}

impl Query {
    /// Reads what was typed. `Ok(None)` when there is nothing to search
    /// for: empty, or whitespace only — taken literally, whitespace would
    /// mark nearly every line (as the graph's find, `find::Query::new`).
    pub fn new(text: &str, options: Options) -> Result<Option<Self>, QueryError> {
        if text.chars().all(char::is_whitespace) {
            return Ok(None);
        }
        let body = if options.regex {
            text.to_string()
        } else {
            regex::escape(text)
        };
        // `m`: `^` and `$` are a line's ends, the lines being what is
        // searched; `.` keeps to a line. Case is folded by the engine, so
        // a hit's bytes are the text's own.
        let flags = if options.match_case { "(?m)" } else { "(?mi)" };
        let pattern = regex::RegexBuilder::new(&format!("{flags}{body}")).build()?;
        Ok(Some(Self {
            pattern,
            whole_word: options.whole_word,
        }))
    }
}

/// One line of a side: the caller's id for it, and its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideLine<'a> {
    pub id: usize,
    pub text: &'a str,
}

/// What a hit covers of one line, in bytes of that line's text — never
/// empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub id: usize,
    pub bytes: Range<usize>,
}

/// One hit: its spans, one per line it runs over, in line order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub spans: Vec<Span>,
}

impl Hit {
    /// Where the hit starts: the first line's id and the byte on it.
    pub fn at(&self) -> (usize, usize) {
        self.spans
            .first()
            .map_or((0, 0), |span| (span.id, span.bytes.start))
    }
}

/// The hits of `query` over `lines`, read as one text with a line break
/// between the lines, in text order. A match of nothing, and one of the
/// break alone, are no hits.
pub fn search(query: &Query, lines: &[SideLine<'_>]) -> Vec<Hit> {
    let mut text = String::new();
    let mut starts = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            text.push('\n');
        }
        starts.push(text.len());
        text.push_str(line.text);
    }
    let mut hits = Vec::new();
    let mut from = 0;
    // `find_at`, not `find_iter`: a match refused as a word may hold the
    // word that stands (`a-a` in `ba-a-a`), so the search goes on from
    // its second character. The context before `from` still counts.
    while from <= text.len() {
        let Some(found) = query.pattern.find_at(&text, from) else {
            break;
        };
        let bytes = found.range();
        if bytes.is_empty() || query.whole_word && !word_bounded(&text, bytes.clone()) {
            from = bytes.start + text[bytes.start..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        from = bytes.end;
        let hit = spans_of(lines, &starts, bytes);
        if !hit.spans.is_empty() {
            hits.push(hit);
        }
    }
    hits
}

/// Whether a hit on `text[bytes]` stands as a word: where it begins on a
/// word character the one before is not one, where it ends on one the one
/// after is not one. A hit beginning or ending on punctuation asks nothing
/// of that side (`clone(` is found before `clone(x)`).
fn word_bounded(text: &str, bytes: Range<usize>) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let before = text[..bytes.start].chars().next_back();
    let after = text[bytes.end..].chars().next();
    let hit = &text[bytes];
    let starts_word = hit.chars().next().is_some_and(is_word);
    let ends_word = hit.chars().next_back().is_some_and(is_word);
    !(starts_word && before.is_some_and(is_word)) && !(ends_word && after.is_some_and(is_word))
}

/// The spans of one match, `bytes` of the joined text, over the lines it
/// runs through — none for a match of the break alone.
fn spans_of(lines: &[SideLine<'_>], starts: &[usize], bytes: Range<usize>) -> Hit {
    let first = starts
        .partition_point(|&s| s <= bytes.start)
        .saturating_sub(1);
    let last = starts.partition_point(|&s| s < bytes.end).saturating_sub(1);
    let mut spans = Vec::with_capacity(last - first + 1);
    for (i, line) in lines.iter().enumerate().take(last + 1).skip(first) {
        let line_start = starts[i];
        let line_end = line_start + line.text.len();
        let on_line =
            bytes.start.max(line_start) - line_start..bytes.end.min(line_end) - line_start;
        // The break between two lines is nobody's: a match that begins on
        // it marks nothing on the line it closes.
        if !on_line.is_empty() {
            spans.push(Span {
                id: line.id,
                bytes: on_line,
            });
        }
    }
    Hit { spans }
}

/// The hits of `query` over the patches, both sides, in reading order.
/// Ids are the lines' places counted through every hunk of every patch
/// (`\ No newline` notes counted, never searched). A hit on context lines
/// alone is one hit, the new side's.
pub fn hits_in(query: &Query, patches: &[FilePatch]) -> Vec<Hit> {
    let mut new_side = Vec::new();
    let mut old_side = Vec::new();
    let mut context = Vec::new();
    let lines = patches.iter().flat_map(|p| &p.hunks).flat_map(|h| &h.lines);
    for (id, line) in lines.enumerate() {
        let side = SideLine {
            id,
            text: &line.text,
        };
        match line.kind {
            DiffLineKind::Context => {
                new_side.push(side);
                old_side.push(side);
                context.push(id);
            }
            DiffLineKind::Addition => new_side.push(side),
            DiffLineKind::Deletion => old_side.push(side),
            DiffLineKind::NoNewline => {}
        }
    }
    let mut hits = search(query, &new_side);
    hits.extend(search(query, &old_side).into_iter().filter(|hit| {
        !hit.spans
            .iter()
            .all(|s| context.binary_search(&s.id).is_ok())
    }));
    hits.sort_by_key(Hit::at);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::diff::{DiffHunk, DiffLine};

    fn lines<'a>(texts: &[&'a str]) -> Vec<SideLine<'a>> {
        texts
            .iter()
            .enumerate()
            .map(|(id, text)| SideLine { id, text })
            .collect()
    }

    fn query(text: &str, options: Options) -> Query {
        Query::new(text, options).unwrap().unwrap()
    }

    fn plain(text: &str) -> Query {
        query(text, Options::default())
    }

    fn spans(hits: &[Hit]) -> Vec<Vec<(usize, Range<usize>)>> {
        hits.iter()
            .map(|h| h.spans.iter().map(|s| (s.id, s.bytes.clone())).collect())
            .collect()
    }

    #[test]
    fn nothing_typed_is_nothing_to_search() {
        assert!(Query::new("", Options::default()).unwrap().is_none());
        assert!(Query::new("  \n ", Options::default()).unwrap().is_none());
    }

    #[test]
    fn a_line_is_found_wherever_it_stands_and_in_either_case() {
        let hits = search(&plain("note"), &lines(&["a Note of notes", "none"]));
        assert_eq!(spans(&hits), [[(0, 2..6)], [(0, 10..14)]]);
    }

    #[test]
    fn matching_case_keeps_to_the_typed_case() {
        let hits = search(
            &query(
                "Note",
                Options {
                    match_case: true,
                    ..Options::default()
                },
            ),
            &lines(&["a Note of notes"]),
        );
        assert_eq!(spans(&hits), [[(0, 2..6)]]);
    }

    #[test]
    fn a_whole_word_does_not_continue_into_another() {
        let whole = Options {
            whole_word: true,
            ..Options::default()
        };
        let hits = search(&query("note", whole), &lines(&["notes note _note (note)"]));
        assert_eq!(spans(&hits), [[(0, 6..10)], [(0, 18..22)]]);
        // A hit ending on punctuation asks nothing of what follows.
        let hits = search(&query("clone(", whole), &lines(&["x.clone(y)"]));
        assert_eq!(spans(&hits), [[(0, 2..8)]]);
        // A word hidden inside a refused hit (3..6 inside 1..4) is found:
        // the search goes on from the refused hit's second character, not
        // its end.
        let hits = search(&query("a-a", whole), &lines(&["ba-a-a"]));
        assert_eq!(spans(&hits), [[(0, 3..6)]]);
    }

    #[test]
    fn a_hit_on_the_break_alone_is_no_hit() {
        let re = Options {
            regex: true,
            ..Options::default()
        };
        assert!(search(&query(r"\n", re), &lines(&["a", "b"])).is_empty());
        // One that ends on the break marks the line it closes.
        let hits = search(&query(r"a\n", re), &lines(&["a", "b"]));
        assert_eq!(spans(&hits), [[(0, 0..1)]]);
    }

    #[test]
    fn a_pattern_is_read_as_one_and_refused_with_its_reason() {
        let re = Options {
            regex: true,
            ..Options::default()
        };
        let hits = search(&query(r"no\w+", re), &lines(&["a note, none"]));
        assert_eq!(spans(&hits), [[(0, 2..6)], [(0, 8..12)]]);
        let hits = search(&query("^a", re), &lines(&["ab", "ba", "a"]));
        assert_eq!(spans(&hits), [[(0, 0..1)], [(2, 0..1)]]);
        let error = Query::new("(", re).unwrap_err();
        assert_eq!(error.to_string(), "Unclosed group");
        // An empty match is no hit.
        assert!(search(&query("x*", re), &lines(&["abc"])).is_empty());
    }

    #[test]
    fn two_typed_lines_run_over_a_line_end() {
        let hits = search(
            &plain("the file\n0"),
            &lines(&["down the file", "09: a line", "down the file", "1: x"]),
        );
        assert_eq!(spans(&hits), [[(0, 5..13), (1, 0..1)]]);
        // Beginning on the break itself marks nothing on the line it closes.
        let hits = search(&plain("\n09"), &lines(&["down the file", "09: a line"]));
        assert_eq!(spans(&hits), [[(1, 0..2)]]);
    }

    /// Spans are bytes of each line's own text: a tab is one, 日 three, an
    /// astral glyph four, and the second line counts from its own start.
    #[test]
    fn a_span_is_the_bytes_of_its_own_line() {
        let hits = search(
            &plain("本\u{1f600}b\n語"),
            &lines(&["a\t日本\u{1f600}b", "語x"]),
        );
        assert_eq!(spans(&hits), [[(0, 5..13), (1, 0..3)]]);
    }

    fn patch(lines: &[(DiffLineKind, &str)]) -> FilePatch {
        FilePatch {
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                extra_old: Vec::new(),
                heading: String::new(),
                lines: lines
                    .iter()
                    .map(|(kind, text)| DiffLine {
                        kind: *kind,
                        old_no: None,
                        new_no: None,
                        text: (*text).to_string(),
                        markers: String::new(),
                    })
                    .collect(),
            }],
            ..FilePatch::default()
        }
    }

    #[test]
    fn a_patch_is_two_sides_read_through_each_other() {
        use DiffLineKind::{Addition, Context, Deletion, NoNewline};
        let patches = [patch(&[
            (Context, "keep"),
            (Deletion, "old end"),
            (Addition, "new end"),
            (Context, "keep"),
            (NoNewline, ""),
        ])];
        // Context lines are reported once.
        assert_eq!(
            spans(&hits_in(&plain("keep"), &patches)),
            [[(0, 0..4)], [(3, 0..4)]]
        );
        // Each side's changed line runs into the shared context line, past
        // the other side's line between them.
        assert_eq!(
            spans(&hits_in(&plain("end\nkeep"), &patches)),
            [[(1, 4..7), (3, 0..4)], [(2, 4..7), (3, 0..4)]]
        );
    }
}
