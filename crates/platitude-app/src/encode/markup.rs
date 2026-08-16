//! The theme's runs laid over one line as the markup
//! `Text.StyledText` reads.

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
fn push_escaped(out: &mut String, text: &str, col: &mut usize) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            ' ' => out.push_str("&nbsp;"),
            '\t' => {
                let stop = TAB_WIDTH - (*col % TAB_WIDTH);
                for _ in 0..stop {
                    out.push_str("&nbsp;");
                }
                *col += stop;
                continue;
            }
            _ => out.push(ch),
        }
        *col += 1;
    }
}

fn push_hex(out: &mut String, byte: u8) {
    for digit in [byte >> 4, byte & 0xf] {
        out.push(char::from_digit(u32::from(digit), 16).unwrap_or('0'));
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
}
