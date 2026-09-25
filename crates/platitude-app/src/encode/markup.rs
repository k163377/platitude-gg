//! The theme's runs laid over one line as the markup `Text.StyledText`
//! reads, and where a byte of a line stands in the row as spelled, and
//! back.
//!
//! **Places, never pixels**: the panes ask the row's own layout where a
//! place is drawn (`LineRuler`). Two rules, one character apart — a diff
//! row spells a tab as the `&nbsp;` that reach its stop ([`spelled_ranges`]
//! / [`source_byte`]); a command log row is the characters themselves, a
//! tab one place ([`plain_ranges`] / [`plain_byte`]).

use platitude_core::highlight::Span;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::columns::step_of;
use super::wire::{Fields, Listed, Record, field};

/// One line as `Text.StyledText` reads it: the theme's runs where there
/// are any, and the line escaped either way.
///
/// **Every row goes through here, coloured or not**: one format, so only
/// the colours change when they arrive — with two, a row is measured with
/// its `<font>` tags as text in the turn the markup lands (`DiffReach`).
pub(super) fn styled(text: &str, spans: &[Span]) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut at = 0;
    let mut col = 0usize;
    for span in spans {
        let end = (at + span.len).min(text.len());
        // A run that cuts inside a character sends the rest out plain.
        let Some(piece) = text.get(at..end) else {
            break;
        };
        out.push_str("<font color=\"#");
        push_hex(&mut out, span.color.r);
        push_hex(&mut out, span.color.g);
        push_hex(&mut out, span.color.b);
        out.push_str("\">");
        push_escaped(&mut out, piece, &mut col);
        out.push_str("</font>");
        at = end;
    }
    // Whatever the runs did not reach — a lexer that stopped short still
    // leaves a whole line on screen.
    if let Some(rest) = text.get(at..) {
        push_escaped(&mut out, rest, &mut col);
    }
    out
}

/// Escapes what `Text.StyledText` would read as markup, and the
/// whitespace it would fold as HTML does. (`<pre>` stops the folding but
/// renders in a substituted font.)
///
/// `col` is the column the next character is drawn at, carried across the
/// calls one line is spelled in; [`step_of`] moves it.
fn push_escaped(out: &mut String, text: &str, col: &mut usize) {
    for ch in text.chars() {
        let step = step_of(ch, *col);
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            ' ' => out.push_str("&nbsp;"),
            '\t' => {
                for _ in 0..step {
                    out.push_str("&nbsp;");
                }
            }
            _ => out.push(ch),
        }
        *col += step;
    }
}

fn push_hex(out: &mut String, byte: u8) {
    for digit in [byte >> 4, byte & 0xf] {
        out.push(char::from_digit(u32::from(digit), 16).unwrap_or('0'));
    }
}

/// How many UTF-16 units — the layout's places — one character is spelled
/// in: a tab, the `&nbsp;` that reach its stop; anything else, the
/// character itself.
fn spelled_units(ch: char, col: usize) -> usize {
    if ch == '\t' {
        step_of(ch, col)
    } else {
        ch.len_utf16()
    }
}

/// The same count for a line drawn as itself (the command log's plain
/// `Label`s, `CommandRowDelegate`): **a tab is one place** — the layout
/// draws it at its own stop.
fn plain_units(ch: char, _col: usize) -> usize {
    ch.len_utf16()
}

/// One run of places along a line as the row spells it, in UTF-16 units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub from: i32,
    pub len: i32,
}

/// The runs a wash is laid over, in line order; empty for nothing.
pub type Runs = Listed<Run>;

impl Record for Run {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("from", &self.from)
            .put("len", &self.len)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            from: field(map, "from")?,
            len: field(map, "len")?,
        })
    }
}

impl platitude_core::mem::Footprint for Run {
    fn heap_bytes(&self) -> usize {
        0
    }
}

/// Where byte ranges of the source line (`platitude_core::intraline`, the
/// reader's own selection) fall in the line as the row spells it. Places
/// only (see the module): the wash and the press are read against the
/// same layout, so they cannot disagree.
pub fn spelled_ranges(text: &str, ranges: &[(usize, usize)]) -> Runs {
    places_of(text, ranges, spelled_units)
}

/// [`spelled_ranges`] for a row drawn as the characters themselves
/// (`CommandsModel::spell`); the tab is the difference ([`plain_units`]).
pub fn plain_ranges(text: &str, ranges: &[(usize, usize)]) -> Runs {
    places_of(text, ranges, plain_units)
}

fn places_of(text: &str, ranges: &[(usize, usize)], units: fn(char, usize) -> usize) -> Runs {
    if ranges.is_empty() {
        return Runs::default();
    }
    let mut out: Vec<Run> = Vec::new();
    let mut iter = ranges.iter().copied();
    let mut current = iter.next();
    let mut open: Option<usize> = None;
    let mut here = 0usize;
    let mut col = 0usize;
    for (at, ch) in text.char_indices() {
        if let Some((start, len)) = current {
            if open.is_none() && at == start {
                open = Some(here);
            }
            if at == start + len {
                if let Some(from) = open.take() {
                    push_span(&mut out, from, here);
                }
                current = iter.next();
                if let Some((next_start, _)) = current
                    && at == next_start
                {
                    open = Some(here);
                }
            }
        }
        here += units(ch, col);
        col += step_of(ch, col);
    }
    // A range that runs to the line's end closes here.
    if let Some(from) = open {
        push_span(&mut out, from, here);
    }
    Runs::new(out)
}

fn push_span(out: &mut Vec<Run>, from: usize, to: usize) {
    if to <= from {
        return;
    }
    out.push(Run {
        from: i32::try_from(from).unwrap_or(i32::MAX),
        len: i32::try_from(to - from).unwrap_or(i32::MAX),
    });
}

/// Which byte of the source line the place `at` in the spelled line stands
/// at — the inverse of [`spelled_ranges`].
///
/// The answer is a **boundary**: a place inside one character's spelling (a
/// tab's spaces, an astral glyph's two units) goes to the nearer end, ties
/// right, as the layout decides between glyphs. Past the end of the line
/// it is the line's length.
pub fn source_byte(text: &str, at: usize) -> usize {
    byte_of(text, at, spelled_units)
}

/// [`source_byte`] for a row drawn as the characters themselves
/// (`CommandsModel::hit`); the tab is the difference ([`plain_units`]).
pub fn plain_byte(text: &str, at: usize) -> usize {
    byte_of(text, at, plain_units)
}

fn byte_of(text: &str, at: usize, units: fn(char, usize) -> usize) -> usize {
    let mut here = 0usize;
    let mut col = 0usize;
    for (byte, ch) in text.char_indices() {
        if at <= here {
            return byte;
        }
        let held = units(ch, col);
        if at < here + held {
            return if (at - here) * 2 >= held {
                byte + ch.len_utf8()
            } else {
                byte
            };
        }
        here += held;
        col += step_of(ch, col);
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::highlight::Rgb;

    fn run(len: usize, r: u8, g: u8, b: u8) -> Span {
        Span {
            len,
            color: Rgb { r, g, b },
        }
    }

    #[test]
    fn a_line_without_runs_is_still_escaped() {
        assert_eq!(
            styled("a plain <line>", &[]),
            "a&nbsp;plain&nbsp;&lt;line&gt;"
        );
    }

    #[test]
    fn runs_become_markup_and_source_characters_are_escaped() {
        let out = styled(
            "fn a<&b>",
            &[run(3, 0x11, 0x22, 0x33), run(5, 0xaa, 0xbb, 0xcc)],
        );
        assert_eq!(
            out,
            "<font color=\"#112233\">fn&nbsp;</font>\
             <font color=\"#aabbcc\">a&lt;&amp;b&gt;</font>"
        );
    }

    #[test]
    fn a_tab_becomes_the_columns_it_stands_for() {
        let out = styled("\tab\tc", &[run(5, 0, 0, 0)]);
        let spaces = out.matches("&nbsp;").count();
        assert_eq!(spaces, 6, "4 to the first stop, 2 to the second: {out}");
    }

    #[test]
    fn a_line_the_runs_fall_short_of_is_still_whole() {
        let out = styled("ab cd", &[run(2, 0, 0, 0)]);
        assert!(out.ends_with("&nbsp;cd"), "{out}");
    }

    #[test]
    fn a_tab_behind_a_wide_glyph_spells_only_what_is_left_of_its_stop() {
        // 日 takes two columns, so two `&nbsp;` reach the stop at 4.
        let out = styled("日\tx", &[run(5, 0, 0, 0)]);
        assert_eq!(out, "<font color=\"#000000\">日&nbsp;&nbsp;x</font>");
    }

    fn places(runs: &Runs) -> Vec<(i32, i32)> {
        runs.iter().map(|r| (r.from, r.len)).collect()
    }

    #[test]
    fn a_run_is_where_the_row_spells_it() {
        // The tab is two `&nbsp;` to the stop at 4: 日 on places 0..1, the
        // tab 1..3, ab 3..5; bytes 0..3, 3..4 and 4..6 of the source.
        let text = "日\tab";
        assert_eq!(places(&spelled_ranges(text, &[(0, 3)])), [(0, 1)]);
        assert_eq!(places(&spelled_ranges(text, &[(3, 1)])), [(1, 2)]);
        assert_eq!(places(&spelled_ranges(text, &[(4, 2)])), [(3, 2)]);
        assert_eq!(places(&spelled_ranges(text, &[(0, 6)])), [(0, 5)]);
        assert_eq!(
            places(&spelled_ranges(text, &[(0, 3), (4, 1)])),
            [(0, 1), (3, 1)]
        );
        assert!(spelled_ranges(text, &[]).is_empty());
        let runs = spelled_ranges(text, &[(0, 3), (4, 1)]);
        assert_eq!(
            Runs::try_from(&qtbridge::qtbridge_type_lib::QVariant::from(&runs)),
            Ok(runs)
        );
    }

    #[test]
    fn an_astral_glyph_is_two_of_the_units_a_place_is_counted_in() {
        assert_eq!(places(&spelled_ranges("a\u{1f600}b", &[(1, 4)])), [(1, 2)]);
        assert_eq!(places(&spelled_ranges("a\u{1f600}b", &[(5, 1)])), [(3, 1)]);
    }

    #[test]
    fn a_combining_mark_is_a_place_of_its_own() {
        // The font draws nothing for it, but the layout counts it.
        assert_eq!(places(&spelled_ranges("e\u{301}x", &[(0, 3)])), [(0, 2)]);
        assert_eq!(source_byte("e\u{301}x", 1), 1);
        assert_eq!(source_byte("e\u{301}x", 2), 3);
    }

    #[test]
    fn a_place_comes_back_as_the_byte_it_stands_on() {
        let text = "日\tab";
        assert_eq!(source_byte(text, 0), 0);
        assert_eq!(source_byte(text, 1), 3);
        assert_eq!(source_byte(text, 3), 4);
        assert_eq!(source_byte(text, 5), 6);
        assert_eq!(source_byte(text, 400), 6);
        assert_eq!(source_byte("", 400), 0);
    }

    #[test]
    fn a_place_inside_a_tab_belongs_to_the_nearer_end_of_it() {
        // Places 0..4 stand in the tab's four spaces; each half goes to its
        // nearer end.
        let text = "\tab";
        assert_eq!(source_byte(text, 0), 0);
        assert_eq!(source_byte(text, 1), 0);
        assert_eq!(source_byte(text, 2), 1);
        assert_eq!(source_byte(text, 3), 1);
        assert_eq!(source_byte(text, 4), 1);
    }

    #[test]
    fn every_place_of_a_line_comes_back_to_a_boundary_of_it() {
        // The two walks are one mapping, and a byte the wash was laid at
        // has to be a byte the copy can cut at.
        let text = "\ta 日\u{301}\u{1f600}b\t\u{ff21}";
        let spelled: usize = text
            .char_indices()
            .scan(0usize, |col, (_, ch)| {
                let units = spelled_units(ch, *col);
                *col += step_of(ch, *col);
                Some(units)
            })
            .sum();
        for at in 0..=spelled {
            let byte = source_byte(text, at);
            assert!(
                text.is_char_boundary(byte),
                "place {at} came back as byte {byte}, which is inside a character"
            );
        }
    }

    #[test]
    fn a_row_drawn_as_itself_counts_a_tab_once() {
        let text = "a\tb";
        assert_eq!(places(&plain_ranges(text, &[(0, 3)])), [(0, 3)]);
        assert_eq!(places(&plain_ranges(text, &[(1, 1)])), [(1, 1)]);
        assert_eq!(plain_byte(text, 1), 1);
        assert_eq!(plain_byte(text, 2), 2);
        // A diff row spells the same tab as three `&nbsp;` to the stop at 4.
        assert_eq!(places(&spelled_ranges(text, &[(0, 3)])), [(0, 5)]);
        assert_eq!(source_byte(text, 4), 2);
    }

    #[test]
    fn a_wide_glyph_is_one_place_of_a_row_drawn_as_itself() {
        // 日本語 is three places, one per character.
        let text = "日本語.txt";
        assert_eq!(places(&plain_ranges(text, &[(0, 9)])), [(0, 3)]);
        assert_eq!(places(&plain_ranges(text, &[(9, 4)])), [(3, 4)]);
        assert_eq!(plain_byte(text, 1), 3);
        assert_eq!(plain_byte(text, 3), 9);
        // An astral glyph is still two units.
        assert_eq!(places(&plain_ranges("a\u{1f600}b", &[(1, 4)])), [(1, 2)]);
        assert_eq!(plain_byte("a\u{1f600}b", 3), 5);
    }

    #[test]
    fn a_place_past_the_end_of_a_plain_row_is_its_end() {
        assert_eq!(plain_byte("add --all", 400), 9);
        assert_eq!(plain_byte("", 400), 0);
        assert!(plain_ranges("add --all", &[]).is_empty());
    }
}
