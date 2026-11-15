//! One paragraph joined into a single run of text and broken again at the
//! width, at the places `inline` offers.
//!
//! Nor does a line begin with what would open a block of its own there (a
//! bullet, a number and a stop, `#`, `>`, `<`, `|`, a fence, a rule): such
//! a break is not offered, and the text stays on the line above.

use super::Paragraph;
use super::inline::{Piece, pieces};
use super::width::text_columns;

/// The paragraph's lines filled to `width`, or `None` where it is kept as
/// it stands: a line that could end on a hard break (the last one too —
/// `blocks` may end a paragraph early on a line CommonMark reads as its
/// continuation), a tab, which this counts no columns for, or a span that
/// runs from one line onto the next where joining it would change it.
pub(super) fn fill(lines: &[&str], paragraph: &Paragraph, width: usize) -> Option<Vec<String>> {
    let own = &lines[paragraph.start..paragraph.end];
    if own
        .iter()
        .any(|line| hard_break(line) || line.contains('\t'))
    {
        return None;
    }
    let mut joined = String::new();
    for (index, line) in own.iter().enumerate() {
        if index == 0 {
            joined.push_str(&line[paragraph.lead..]);
        } else {
            // CommonMark takes a continuation line's leading spaces off; a
            // line's trailing ones are text inside a code span.
            joined.push('\n');
            joined.push_str(line.trim_start_matches(' '));
        }
    }
    let pieces = glued(pieces(joined.trim_end_matches(' '))?);
    // A line the source began, standing at the paragraph's indent now: a
    // continuation that stood four columns in, or lazily at none, may open
    // a block there.
    let reopens = pieces.windows(2).enumerate().any(|(index, pair)| {
        matches!(pair, [Piece::Keep, Piece::Text(text)]
            if opens_a_block(text, matches!(pieces.get(index + 2), None | Some(Piece::Space(_)))))
    });
    if reopens {
        return None;
    }
    Some(laid(
        &own[0][..paragraph.lead],
        paragraph.lead,
        &pieces,
        width,
    ))
}

/// Two spaces or a backslash at a line's end: CommonMark's hard break.
fn hard_break(line: &str) -> bool {
    line.ends_with("  ") || (line.len() - line.trim_end_matches('\\').len()) % 2 == 1
}

/// The pieces with every break taken out that would open a line on a
/// block's marker: the space before such a piece binds it to the text
/// before.
fn glued(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::with_capacity(pieces.len());
    for (index, piece) in pieces.iter().enumerate() {
        if let Piece::Text(text) = piece {
            let spaced_after = matches!(pieces.get(index + 1), None | Some(Piece::Space(_)));
            if opens_a_block(text, spaced_after)
                && let Some(Piece::Space(gap)) = out.last().cloned()
            {
                out.pop();
                if let Some(Piece::Text(before)) = out.last_mut() {
                    before.push_str(&gap);
                    before.push_str(text);
                    continue;
                }
                out.push(Piece::Space(gap));
            }
        }
        out.push(piece.clone());
    }
    out
}

/// Whether a line that began with `text` would open a block of its own.
/// Read widely, wider than `blocks` reads a paragraph's lines: a piece of
/// nothing but rule, underline and delimiter-row signs (`---`, `===`,
/// `:---|---:`) counts whatever follows it.
fn opens_a_block(text: &str, spaced_after: bool) -> bool {
    if text.starts_with(['#', '>', '<', '|'])
        || text.starts_with("```")
        || text.starts_with("~~~")
        || text.starts_with("$$")
    {
        return true;
    }
    if text
        .chars()
        .all(|c| matches!(c, '-' | '+' | '*' | '=' | '_' | ':' | '|'))
    {
        return true;
    }
    let digits = text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    (1..=9).contains(&digits) && matches!(&text[digits..], "." | ")") && spaced_after
}

/// The pieces laid into lines: each takes pieces while they fit, and the
/// next line begins at the piece that did not, or where the source kept
/// its line end.
///
/// A gap counts one column, however many spaces it holds: a line ended at
/// it reads back as one space, and the next fill must decide as this one
/// did.
fn laid(prefix: &str, indent: usize, pieces: &[Piece], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = prefix.to_string();
    let mut used = text_columns(prefix);
    let mut fresh = true;
    let mut gap = "";
    for piece in pieces {
        match piece {
            Piece::Space(spaces) => gap = spaces,
            Piece::Tight => gap = "",
            Piece::Keep => {
                lines.push(std::mem::replace(&mut line, " ".repeat(indent)));
                used = indent;
                fresh = true;
                gap = "";
            }
            Piece::Text(text) => {
                let columns = text_columns(text);
                let room = usize::from(!gap.is_empty());
                if fresh || used + room + columns <= width {
                    line.push_str(gap);
                    line.push_str(text);
                    used += room + columns;
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = format!("{}{text}", " ".repeat(indent));
                    used = indent + columns;
                }
                fresh = false;
                gap = "";
            }
        }
    }
    lines.push(line);
    lines
}

#[cfg(test)]
mod tests {
    use super::super::Paragraph;
    use super::fill;

    fn filled(text: &str, lead: usize, width: usize) -> Vec<String> {
        let lines: Vec<&str> = text.lines().collect();
        let paragraph = Paragraph {
            start: 0,
            end: lines.len(),
            lead,
        };
        fill(&lines, &paragraph, width).expect("filled")
    }

    fn kept(lines: &[&str]) -> bool {
        let paragraph = Paragraph {
            start: 0,
            end: lines.len(),
            lead: 0,
        };
        fill(lines, &paragraph, 80).is_none()
    }

    #[test]
    fn spaces_become_breaks_and_breaks_spaces() {
        assert_eq!(
            filled("one two\nthree four five", 0, 13),
            vec!["one two three", "four five"]
        );
    }

    /// A line end between two Japanese characters was never drawn, so the
    /// join takes nothing and a break may go between any two of them.
    #[test]
    fn japanese_joins_tight_and_breaks_between_characters() {
        assert_eq!(filled("日本\n語の文", 0, 6), vec!["日本語", "の文"]);
        assert_eq!(filled("日本語の\ntext", 0, 40), vec!["日本語の text"]);
    }

    /// `§無効 の灰` broken at its space would draw `§無効の灰`.
    #[test]
    fn a_space_between_two_japanese_characters_is_never_a_break() {
        assert_eq!(filled("§無効 の灰", 0, 7), vec!["§無", "効 の灰"]);
        assert_eq!(filled("の `x` で", 0, 3), vec!["の", "`x`", "で"]);
    }

    #[test]
    fn a_list_item_goes_on_at_its_text_column() {
        assert_eq!(
            filled("- one two three four", 2, 10),
            vec!["- one two", "  three", "  four"]
        );
    }

    #[test]
    fn no_line_opens_on_a_stop_or_closes_on_a_bracket() {
        assert_eq!(filled("あいう、えお", 0, 6), vec!["あい", "う、え", "お"]);
        assert_eq!(filled("あい「うえ」", 0, 6), vec!["あい", "「う", "え」"]);
    }

    #[test]
    fn a_code_span_is_never_split_and_its_line_end_is_a_space() {
        assert_eq!(
            filled("see `a b c d` here", 0, 8),
            vec!["see", "`a b c d`", "here"]
        );
        assert_eq!(filled("`日本\n語`", 0, 40), vec!["`日本 語`"]);
    }

    /// CommonMark keeps a code span's trailing space before a line end.
    #[test]
    fn a_code_span_keeps_the_space_before_its_line_end() {
        assert_eq!(
            filled("use `foo   bar \nbaz` here", 0, 80),
            vec!["use `foo   bar  baz` here"]
        );
    }

    /// The mark and its span stay on one line, and the line does not open
    /// on the mark: a `<!--` opening a line is an HTML block.
    #[test]
    fn a_command_span_stays_on_one_line_with_its_mark() {
        assert_eq!(
            filled("走らせる <!--cmd:a.b-->`cargo xtask a b` を", 0, 20),
            vec!["走らせ", "る <!--cmd:a.b-->`cargo xtask a b`", "を"]
        );
    }

    #[test]
    fn a_line_never_opens_on_a_blocks_marker() {
        assert_eq!(filled("aaa - bbb", 0, 4), vec!["aaa -", "bbb"]);
        assert_eq!(filled("aaa 1. bbb", 0, 4), vec!["aaa 1.", "bbb"]);
        assert_eq!(filled("aaa 2026) bbb", 0, 4), vec!["aaa 2026)", "bbb"]);
        assert_eq!(filled("aaa # bbb", 0, 4), vec!["aaa #", "bbb"]);
        assert_eq!(filled("aaa | bbb", 0, 4), vec!["aaa |", "bbb"]);
        assert_eq!(filled("aaa --- bbb", 0, 4), vec!["aaa ---", "bbb"]);
        assert_eq!(filled("aaa > bbb", 0, 4), vec!["aaa >", "bbb"]);
        assert_eq!(filled("aaa ```x bbb", 0, 4), vec!["aaa ```x", "bbb"]);
        assert_eq!(filled("aaa 1.5 bbb", 0, 4), vec!["aaa", "1.5", "bbb"]);
        assert_eq!(filled("aaa ---|--- bbb", 0, 4), vec!["aaa ---|---", "bbb"]);
        assert_eq!(filled("aaa $$ bbb", 0, 4), vec!["aaa $$", "bbb"]);
        assert_eq!(filled("aaa :--- bbb", 0, 4), vec!["aaa :---", "bbb"]);
    }

    /// The source's own line end, kept, would stand the next line at the
    /// paragraph's indent — where `    - x` under a paragraph is a bullet.
    #[test]
    fn a_kept_line_end_before_a_blocks_marker_keeps_the_paragraph() {
        assert!(kept(&["本文の行 <b>強</b>", "    - 箇条ではない"]));
        assert!(kept(&["本文 [注]", "    # 見出しではない"]));
        assert!(kept(&["本文 [注]", "    ---"]));
    }

    #[test]
    fn a_link_is_taken_whole() {
        assert_eq!(
            filled("文 [a b](x/y%20z.md \"t u\") c", 0, 3),
            vec!["文", "[a b](x/y%20z.md \"t u\")", "c"]
        );
        assert_eq!(
            filled("の[機械化.md](機械化.md)を読む", 0, 4),
            vec!["の[機械化.md](機械化.md)を", "読む"]
        );
        assert_eq!(filled("[a `]` b](x) c", 0, 3), vec!["[a `]` b](x)", "c"]);
        assert_eq!(filled("[機\n械](x)", 0, 40), vec!["[機械](x)"]);
        assert_eq!(filled("[a\nb](x)", 0, 40), vec!["[a b](x)"]);
        // Its text measured from the link's own `[`, wherever it stands.
        assert_eq!(
            filled("前置きの文 [長い\n名前](x)", 0, 80),
            vec!["前置きの文 [長い名前](x)"]
        );
        // A destination running onto the next line is kept as it stands.
        assert!(kept(&["前 [a", "b](x", "y)"]));
    }

    /// A line end beside a delimiter, between Japanese, stays where it is:
    /// the source drew `重要です`, and a space would draw `重要 です`.
    #[test]
    fn a_line_end_that_cannot_be_told_stays_where_it_is() {
        assert_eq!(
            filled("これは**重要**\nです。長い続きの文が来る", 0, 80),
            vec!["これは**重要**", "です。長い続きの文が来る"]
        );
    }

    #[test]
    fn a_hard_break_keeps_the_paragraph_as_it_stands() {
        assert!(kept(&["one  ", "two"]));
        assert!(kept(&["one\\", "two"]));
        // The last line as well: what follows may be its continuation.
        assert!(kept(&["one two  "]));
        assert!(!kept(&["one two \\\\"]));
    }

    #[test]
    fn a_tab_keeps_the_paragraph_as_it_stands() {
        assert!(kept(&["one\ttwo"]));
    }

    #[test]
    fn a_backslash_before_a_space_is_never_left_at_a_lines_end() {
        assert_eq!(filled("aa\\ bb", 0, 3), vec!["aa\\ bb"]);
        assert_eq!(
            filled("path C:\\ \nnext words", 0, 80),
            vec!["path C:\\ next words"]
        );
    }

    #[test]
    fn a_word_longer_than_the_width_stands_alone() {
        assert_eq!(
            filled("a https://example.com/long b", 0, 6),
            vec!["a", "https://example.com/long", "b"]
        );
    }

    /// A gap of two spaces counts one column, as it reads back once a line
    /// ends at it.
    #[test]
    fn a_run_of_spaces_fills_the_same_twice() {
        let text = format!("{}  bb cc", "a".repeat(117));
        let once = filled(&text, 0, 120);
        let again = filled(&once.join("\n"), 0, 120);
        assert_eq!(once, again);
    }
}
