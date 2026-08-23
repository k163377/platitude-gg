//! The theme's runs laid over one line as the markup
//! `Text.StyledText` reads.

use super::columns::{is_wide, step_of};
use platitude_core::highlight::Span;

/// Lays the theme's runs over one line and writes what `Text.StyledText`
/// reads. Empty when there are no runs — the row then draws its own text
/// in the colour its kind gives it.
pub(super) fn styled(text: &str, spans: &[Span]) -> String {
    if spans.is_empty() {
        return String::new();
    }
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

/// Columns a tab stands for (デザイン規約 §シンタックスハイライト).
const TAB_WIDTH: usize = 4;

/// What `Text.StyledText` would otherwise read as markup, plus the
/// whitespace it would otherwise fold away.
///
/// Rich text folds whitespace runs exactly as HTML does, so escaped
/// spaces are what keep indentation. `<pre>` would turn the folding off,
/// but Qt renders what is inside it in a substituted font: thinner
/// strokes, washed-out colours next to the window's own words
/// (measured 2026-08-13).
///
/// `col` is the column the next character is drawn at, carried across the
/// calls one line is spelled in; [`step_of`] moves it, so a tab is spelled
/// as exactly the `&nbsp;` that reach its stop.
fn push_escaped(out: &mut String, text: &str, col: &mut usize) {
    for ch in text.chars() {
        let step = step_of(ch, *col, TAB_WIDTH);
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
/// (`DiffPane.wideDelta`), and a wash placed on columns alone slid right of
/// the characters it names by that difference a glyph. What `DiffRow::emph`
/// carries and the pane turns into the stronger wash.
pub(super) fn display_ranges(text: &str, ranges: &[(usize, usize)]) -> String {
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
        here.col += step_of(ch, here.col, TAB_WIDTH);
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
    for number in [
        from.col,
        from.wide,
        to.col - from.col,
        to.wide - from.wide,
    ] {
        if !first {
            out.push(':');
        }
        first = false;
        out.push_str(&number.to_string());
    }
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
    fn a_line_without_runs_stays_plain() {
        assert!(styled("plain", &[]).is_empty());
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
}
