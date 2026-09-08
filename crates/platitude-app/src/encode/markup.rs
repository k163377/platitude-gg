//! The theme's runs laid over one line as the markup
//! `Text.StyledText` reads.

use super::columns::{is_wide, step_of};
use platitude_core::highlight::Span;

/// One line as `Text.StyledText` reads it: the theme's runs where there
/// are any, and the line escaped either way.
///
/// **Every row goes through here, coloured or not** — the rows are set in
/// one format, so nothing about a row changes when the colours arrive an
/// instant later except which colour its letters are. Two things came of
/// the rows being two formats: a row was measured with its own `<font …>`
/// tags counted as text in the turn the markup landed and the format had
/// not caught up (the widths that come off the rows are read by
/// `DiffReach`), and an uncoloured row drew its tabs at Qt's own stop
/// while every other walk of the line stepped them at
/// [`super::columns::TAB_WIDTH`]. Neither can be spelled out of a single
/// format.
pub(super) fn styled(text: &str, spans: &[Span]) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut at = 0;
    let mut col = 0usize;
    for span in spans {
        let end = (at + span.len).min(text.len());
        // The runs are byte offsets into this same string; one that cuts
        // inside a character sends the rest of the line out plain.
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

/// What `Text.StyledText` would otherwise read as markup, plus the
/// whitespace it would otherwise fold away.
///
/// Rich text folds whitespace runs exactly as HTML does, so escaped
/// spaces are what keep indentation. `<pre>` would turn the folding off,
/// but Qt renders what is inside it in a substituted font: thinner
/// strokes, washed-out colours next to the window's own words
/// (measured).
///
/// `col` is the column the next character is drawn at, carried across the
/// calls one line is spelled in; [`step_of`] moves it, so a tab is spelled
/// as exactly the `&nbsp;` that reach its stop.
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

/// How many UTF-16 units one character is spelled in — the units the row's
/// own layout counts a place in (`LineRuler`). A tab is spelled as the
/// `&nbsp;` that reach its stop, so it is worth as many units as the
/// columns it takes; everything else is worth what the character itself is
/// held as, which is two for an astral glyph.
fn spelled_units(ch: char, col: usize) -> usize {
    if ch == '\t' {
        step_of(ch, col)
    } else {
        ch.len_utf16()
    }
}

/// Where byte ranges of the source line (`platitude_core::intraline`, the
/// reader's own selection) fall in the line as the row spells it:
/// `"from:len,…"` in UTF-16 units, empty where there is nothing.
///
/// **Places, not pixels, and not columns.** What turns a place into an x is
/// the row's own layout and only that — a column is not a width and no
/// arithmetic makes it one (`DiffTextMetrics`), and a combining mark is a
/// character this walk counts and a glyph the font draws nothing for. The
/// pane asks the layout where these places are drawn (`LineRuler`),
/// which is the same layout the reader's press is read against, so the
/// wash and the hit cannot disagree.
pub fn spelled_ranges(text: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return String::new();
    }
    let mut out = String::new();
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
        here += spelled_units(ch, col);
        col += step_of(ch, col);
    }
    // A range that runs to the line's end closes here.
    if let Some(from) = open {
        push_span(&mut out, from, here);
    }
    out
}

fn push_span(out: &mut String, from: usize, to: usize) {
    if to <= from {
        return;
    }
    if !out.is_empty() {
        out.push(',');
    }
    out.push_str(&from.to_string());
    out.push(':');
    out.push_str(&(to - from).to_string());
}

/// Which byte of the source line the place `at` in the spelled line stands
/// at — the inverse of [`spelled_ranges`], and the second half of reading a
/// press: the layout says which place of the row the pointer is over
/// (`LineRuler`), and this says which byte of the file that is.
///
/// The answer is a **boundary** between two characters. A place inside what
/// one character is spelled in — a tab's spaces, the two units an astral
/// glyph is held as — belongs to the nearer of its ends, ties going right,
/// the way the layout itself decides between two glyphs. Past the end of
/// the line it is the line's length.
pub fn source_byte(text: &str, at: usize) -> usize {
    let mut here = 0usize;
    let mut col = 0usize;
    for (byte, ch) in text.char_indices() {
        if at <= here {
            return byte;
        }
        let units = spelled_units(ch, col);
        if at < here + units {
            return if (at - here) * 2 >= units {
                byte + ch.len_utf8()
            } else {
                byte
            };
        }
        here += units;
        col += step_of(ch, col);
    }
    text.len()
}

/// How far along the row a walk of it stands: the display column reached,
/// and how many of the glyphs behind it were drawn wide. The two travel
/// together because it takes both to reach a pixel — a column is one
/// advance of the mono font, and a wide glyph is drawn from whatever
/// fallback carries it, which need not advance two of them.
#[derive(Clone, Copy)]
struct Stand {
    col: usize,
    wide: usize,
}

/// The display columns `ranges` (byte ranges into `text`,
/// `platitude_core::intraline`) land on, as the row is actually drawn:
/// this walks the line by [`step_of`], the same steps [`push_escaped`]
/// spells it in, so the expanded tabs and the double-width glyphs are
/// counted here exactly where they are drawn.
///
/// `"col:wides:width:wides,…"`, empty where there is nothing — where the
/// run starts and how far it runs, each said twice: in columns, and in the
/// wide glyphs standing in them. The pane needs the second number to place
/// the first: where the mono family is Latin-only, a wide glyph comes from
/// a fallback that advances one em rather than two mono columns
/// (`CommandsPane.wideDelta`), and a wash placed on columns alone slid
/// right of the characters it names by that difference a glyph.
///
/// **The command log's, and only its.** Columns are what this arrives at,
/// and columns are not widths — the fallback carrying a wide glyph is not
/// monospaced at all, so what comes out of here stands beside the
/// characters it names rather than on them wherever the two differ. The
/// diff asks its rows' own layout instead ([`spelled_ranges`]).
pub fn display_ranges(text: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut iter = ranges.iter().copied();
    let mut current = iter.next();
    let mut open: Option<Stand> = None;
    let mut here = Stand { col: 0, wide: 0 };
    for (at, ch) in text.char_indices() {
        if let Some((start, len)) = current {
            if open.is_none() && at == start {
                open = Some(here);
            }
            if at == start + len {
                if let Some(from) = open.take() {
                    push_run(&mut out, from, here);
                }
                current = iter.next();
                if let Some((next_start, _)) = current
                    && at == next_start
                {
                    open = Some(here);
                }
            }
        }
        here.col += step_of(ch, here.col);
        here.wide += usize::from(is_wide(ch));
    }
    // A range that runs to the line's end closes here.
    if let Some(from) = open {
        push_run(&mut out, from, here);
    }
    out
}

fn push_run(out: &mut String, from: Stand, to: Stand) {
    if to.col <= from.col {
        return;
    }
    if !out.is_empty() {
        out.push(',');
    }
    let mut first = true;
    for number in [from.col, from.wide, to.col - from.col, to.wide - from.wide] {
        if !first {
            out.push(':');
        }
        first = false;
        out.push_str(&number.to_string());
    }
}

/// Which byte of `text` a press `x` pixels along the row lands on, worked
/// out from the columns the row stands in.
///
/// The inverse of [`display_ranges`], and it has to be walked by the same
/// rule: a hit worked out any other way would put the selection's edge
/// beside the character the reader pressed on rather than on it. So this
/// steps by [`step_of`] and counts the wide
/// glyphs the way [`Stand`] does — one column is `char_w`, and each wide
/// glyph is worth `wide_delta` more than the two columns it is counted as
/// (`CommandsPane.wideDelta`, measured).
///
/// The answer is a **boundary**, not a character: past the middle of a
/// glyph the hit belongs to the gap after it, which is what makes a drag
/// select the character it was dragged across. Past the end of the line it
/// is the line's length.
///
/// **The command log's, and only its** — for the reason
/// [`display_ranges`] says, and with the same consequence: a press on a
/// line whose glyphs the mono family does not carry lands beside the
/// character under it. The diff reads its press off the row's own layout
/// ([`source_byte`]).
pub fn hit_byte(text: &str, x: f64, char_w: f64, wide_delta: f64) -> usize {
    // Before the row's own font has been measured there is no mapping to
    // make; the head of the line is the only honest answer.
    if x <= 0.0 || char_w <= 0.0 {
        return 0;
    }
    let mut here = Stand { col: 0, wide: 0 };
    let pixels = |at: Stand| at.col as f64 * char_w + at.wide as f64 * wide_delta;
    for (at, ch) in text.char_indices() {
        let next = Stand {
            col: here.col + step_of(ch, here.col),
            wide: here.wide + usize::from(is_wide(ch)),
        };
        let (left, right) = (pixels(here), pixels(next));
        if x < left + (right - left) / 2.0 {
            return at;
        }
        if x < right {
            return at + ch.len_utf8();
        }
        here = next;
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
        // Nothing to colour is still a row to draw: escaped, in the one
        // format every row is read in (see the note above).
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
        // 日 is drawn two columns wide, so two `&nbsp;` reach the stop at
        // 4 and `x` stands on it. Three would carry `x` past it.
        let out = styled("日\tx", &[run(5, 0, 0, 0)]);
        assert_eq!(out, "<font color=\"#000000\">日&nbsp;&nbsp;x</font>");
    }

    #[test]
    fn the_wash_falls_on_the_columns_the_row_is_drawn_at() {
        // "日\tab" is drawn 日 on 0..2, the tab's two `&nbsp;` on 2..4,
        // then ab on 4..6 — byte ranges 0..3, 3..4 and 4..6 of the source.
        // Each run says where it starts and how far it runs, in columns
        // and in the wide glyphs standing in them.
        let text = "日\tab";
        assert_eq!(display_ranges(text, &[(0, 3)]), "0:0:2:1");
        assert_eq!(display_ranges(text, &[(3, 1)]), "2:1:2:0");
        assert_eq!(display_ranges(text, &[(4, 2)]), "4:1:2:0");
        assert_eq!(display_ranges(text, &[(0, 6)]), "0:0:6:1");
        assert_eq!(display_ranges(text, &[(0, 3), (4, 1)]), "0:0:2:1,4:1:1:0");
    }

    #[test]
    fn a_run_says_how_many_of_its_columns_wide_glyphs_took() {
        // Columns alone do not reach pixels: 日本語 is six columns but
        // three fallback advances, so a run that stands behind it starts
        // three of those differences to the left of where six columns of
        // the mono font would put it (`DiffPane.wideDelta`).
        let text = "日本語value";
        assert_eq!(display_ranges(text, &[(0, 9)]), "0:0:6:3");
        assert_eq!(display_ranges(text, &[(9, 5)]), "6:3:5:0");
    }

    #[test]
    fn a_line_nothing_changed_in_carries_no_wash() {
        assert!(display_ranges("日\tab", &[]).is_empty());
    }

    #[test]
    fn a_run_is_where_the_row_spells_it() {
        // "日\tab": the tab reaches the stop at 4, so the row spells it as
        // two `&nbsp;` — 日 on 0..1, the tab on 1..3, ab on 3..5 of the
        // line as spelled. Byte ranges 0..3, 3..4 and 4..6 of the source.
        let text = "日\tab";
        assert_eq!(spelled_ranges(text, &[(0, 3)]), "0:1");
        assert_eq!(spelled_ranges(text, &[(3, 1)]), "1:2");
        assert_eq!(spelled_ranges(text, &[(4, 2)]), "3:2");
        assert_eq!(spelled_ranges(text, &[(0, 6)]), "0:5");
        assert_eq!(spelled_ranges(text, &[(0, 3), (4, 1)]), "0:1,3:1");
        assert!(spelled_ranges(text, &[]).is_empty());
    }

    #[test]
    fn an_astral_glyph_is_two_of_the_units_a_place_is_counted_in() {
        // What QML holds a string in, and so what the row's layout counts
        // its places in: one character, two units.
        assert_eq!(spelled_ranges("a\u{1f600}b", &[(1, 4)]), "1:2");
        assert_eq!(spelled_ranges("a\u{1f600}b", &[(5, 1)]), "3:1");
    }

    #[test]
    fn a_combining_mark_is_a_place_of_its_own() {
        // The font draws nothing extra for it, but the string holds it and
        // the layout counts it — which is exactly why a walk of columns
        // could never place it (`encode::columns`).
        assert_eq!(spelled_ranges("e\u{301}x", &[(0, 3)]), "0:2");
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
        // Past the end of the line is the end of the line.
        assert_eq!(source_byte(text, 400), 6);
        assert_eq!(source_byte("", 400), 0);
    }

    #[test]
    fn a_place_inside_a_tab_belongs_to_the_nearer_end_of_it() {
        // The tab is spelled as four spaces, so places 0..4 stand in it and
        // the two halves go to its two ends — the same rule the layout
        // itself decides between two glyphs by.
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
    fn a_press_lands_on_the_nearer_edge_of_the_glyph_it_is_over() {
        // Ten pixels a column, nothing wide: `abc` is drawn 0..10,
        // 10..20, 20..30.
        assert_eq!(hit_byte("abc", 0.0, 10.0, 0.0), 0);
        assert_eq!(hit_byte("abc", 4.0, 10.0, 0.0), 0);
        assert_eq!(hit_byte("abc", 6.0, 10.0, 0.0), 1);
        assert_eq!(hit_byte("abc", 14.0, 10.0, 0.0), 1);
        assert_eq!(hit_byte("abc", 16.0, 10.0, 0.0), 2);
    }

    #[test]
    fn a_press_past_the_end_of_the_line_is_the_end_of_the_line() {
        assert_eq!(hit_byte("abc", 400.0, 10.0, 0.0), 3);
        // And before the font is measured there is no mapping to make.
        assert_eq!(hit_byte("abc", 400.0, 0.0, 0.0), 0);
    }

    #[test]
    fn a_tab_is_hit_across_all_the_columns_it_stands_for() {
        // The tab reaches the stop at 4, so it is drawn 0..40 and `a`
        // stands on 40..50.
        assert_eq!(hit_byte("\tab", 10.0, 10.0, 0.0), 0);
        assert_eq!(hit_byte("\tab", 30.0, 10.0, 0.0), 1);
        assert_eq!(hit_byte("\tab", 46.0, 10.0, 0.0), 2);
    }

    #[test]
    fn a_wide_glyph_is_hit_where_it_is_drawn_rather_than_where_columns_put_it() {
        // 日 is counted as two columns but drawn one fallback advance
        // wide: 2 * 10 - 3 = 17px, so its middle is at 8.5 and not at 10.
        assert_eq!(hit_byte("日x", 8.0, 10.0, -3.0), 0);
        assert_eq!(hit_byte("日x", 9.0, 10.0, -3.0), 3);
        assert_eq!(hit_byte("日x", 20.0, 10.0, -3.0), 3);
        assert_eq!(hit_byte("日x", 25.0, 10.0, -3.0), 4);
    }
}
