//! Which lines of a document are paragraphs, read the way CommonMark
//! (with GFM's tables) reads blocks — so that what is wrapped is prose
//! and nothing else.
//!
//! Where a paragraph goes on is CommonMark's answer: a line continues it
//! unless it opens a block that may interrupt a paragraph there. Where
//! this cannot tell, it ends the paragraph early, and a block it does not
//! read inside is passed over to its end. Both leave the lines around the
//! boundary as they stand — `fill` keeps whole a paragraph whose last line
//! could be a hard break — so being unsure costs only wrapping less.
//! Taking in a line a renderer reads as a block of its own would change
//! the document, so nothing is taken in on a guess.
//!
//! Not prose: YAML front matter, fenced and indented code, GitHub's display
//! math, tables with or without their outer pipes, headings, thematic
//! breaks, HTML blocks, block quotes, link reference definitions, and a
//! paragraph that a setext underline or a table's delimiter row makes
//! something else.
//!
//! Only a space or a tab is white space here, as in CommonMark: a line of
//! U+3000 is text.

use super::Paragraph;

/// The block a line is read into, besides the list items around it.
enum Leaf {
    None,
    Paragraph(Paragraph),
    /// A fence of `count` of `marker`, opened at `depth` list items.
    Fence {
        marker: char,
        count: usize,
        depth: usize,
    },
    /// A block not read inside, passed over to the line holding `end`, or
    /// with no `end` to a blank line. A block quote or a reference
    /// definition (`quote`) also ends where a line without `>` opens a
    /// block of its own, which an HTML block takes in.
    Opaque {
        end: Option<&'static str>,
        depth: usize,
        quote: bool,
    },
    /// A table's rows, read to a blank line or a block of another kind.
    Table {
        depth: usize,
    },
}

/// Nothing but spaces and tabs.
fn blank(text: &str) -> bool {
    text.bytes().all(|b| b == b' ' || b == b'\t')
}

fn trimmed(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

/// The paragraphs of a document, in order, as line ranges.
pub(super) fn paragraphs(lines: &[&str]) -> Vec<Paragraph> {
    let mut found = Vec::new();
    // The content column of each open list item, outermost first.
    let mut items: Vec<usize> = Vec::new();
    // The innermost item opened on a marker with nothing after it, and
    // nothing in it since: a blank line closes it (an item begins with at
    // most one blank line, and that one is its first).
    let mut empty_item = false;
    let mut leaf = Leaf::None;
    for (at, line) in lines.iter().enumerate().skip(front_matter_end(lines)) {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let blank = blank(line);
        if blank && std::mem::take(&mut empty_item) {
            items.pop();
        }
        let matched = items.iter().take_while(|&&column| indent >= column).count();
        let held = matched == items.len();
        if !blank {
            empty_item = false;
        }
        // How far in the line stands from the innermost item that holds it.
        let rel = indent - matched.checked_sub(1).map_or(0, |inner| items[inner]);
        let bare = &line[indent..];
        match &mut leaf {
            Leaf::Fence {
                marker,
                count,
                depth,
            } => {
                if blank || matched >= *depth {
                    let base = depth.checked_sub(1).map_or(0, |inner| items[inner]);
                    if !blank && indent - base <= 3 && closes(bare, *marker, *count) {
                        leaf = Leaf::None;
                    }
                    continue;
                }
                leaf = Leaf::None;
            }
            Leaf::Opaque { end, depth, quote } => {
                let ends_quote = *quote && !bare.starts_with('>') && starts_block(bare, rel);
                if (blank || matched >= *depth) && !ends_quote {
                    match end {
                        Some(end) if line.contains(*end) => leaf = Leaf::None,
                        None if blank => leaf = Leaf::None,
                        _ => {}
                    }
                    continue;
                }
                leaf = Leaf::None;
            }
            Leaf::Table { depth } => {
                if blank {
                    leaf = Leaf::None;
                    continue;
                }
                if matched >= *depth && !starts_block(bare, rel) {
                    continue;
                }
                leaf = Leaf::None;
            }
            Leaf::Paragraph(paragraph) => {
                if blank {
                    found.push(std::mem::replace(paragraph, Paragraph::EMPTY));
                    leaf = Leaf::None;
                    continue;
                }
                if held && rel <= 3 && setext_underline(bare) {
                    // The paragraph is a heading: none of it is prose.
                    leaf = Leaf::None;
                    continue;
                }
                if held && rel <= 3 && delimiter_row(bare) {
                    // Its last line is a table's header row.
                    leaf = Leaf::Table { depth: items.len() };
                    continue;
                }
                let goes_on = if held {
                    rel >= 4 || !interrupts(bare)
                } else {
                    !starts_block(bare, rel)
                };
                if goes_on {
                    paragraph.end = at + 1;
                    continue;
                }
                found.push(std::mem::replace(paragraph, Paragraph::EMPTY));
                leaf = Leaf::None;
            }
            Leaf::None => {}
        }
        if blank {
            continue;
        }
        items.truncate(matched);
        leaf = open(line, at, indent, &mut items, &mut empty_item);
    }
    if let Leaf::Paragraph(paragraph) = leaf {
        found.push(paragraph);
    }
    found
}

/// The block a line opens with no paragraph open above it, pushing each
/// list item it opens on the way; `empty_item` says the last of them has
/// nothing after its marker.
fn open(
    line: &str,
    at: usize,
    indent: usize,
    items: &mut Vec<usize>,
    empty_item: &mut bool,
) -> Leaf {
    let mut column = indent;
    loop {
        let base = items.last().copied().unwrap_or(0);
        column += line[column..].len() - line[column..].trim_start_matches(' ').len();
        let rest = &line[column..];
        let depth = items.len();
        if column - base >= 4 {
            // Indented code: what follows is passed over until a line says
            // otherwise.
            return Leaf::None;
        }
        if thematic_break(rest) || heading(rest) {
            return Leaf::None;
        }
        if let Some((marker_end, content, _)) = list_marker(rest) {
            let empty = blank(&rest[marker_end..]);
            items.push(column + if empty { marker_end + 1 } else { content });
            if empty {
                *empty_item = true;
                return Leaf::None;
            }
            column += content;
            continue;
        }
        if let Some((marker, count)) = fence(rest).or_else(|| math(rest)) {
            return Leaf::Fence {
                marker,
                count,
                depth,
            };
        }
        if rest.starts_with('|') {
            return Leaf::Table { depth };
        }
        if rest.starts_with('>') || reference_definition(rest) {
            return Leaf::Opaque {
                end: None,
                depth,
                quote: true,
            };
        }
        if let Some((end, _)) = html(rest) {
            return match end {
                // Opened and closed on this line.
                Some(end) if rest.contains(end) => Leaf::None,
                _ => Leaf::Opaque {
                    end,
                    depth,
                    quote: false,
                },
            };
        }
        return Leaf::Paragraph(Paragraph {
            start: at,
            end: at + 1,
            lead: column,
        });
    }
}

/// Whether a line every list item holds, standing at most three columns
/// in, interrupts the paragraph above it (CommonMark: only a bullet with
/// text after it, or `1.` / `1)` with text, of the list markers; neither
/// indented code nor HTML of the seventh kind).
fn interrupts(bare: &str) -> bool {
    let list = list_marker(bare).is_some_and(|(marker_end, _, number)| {
        !blank(&bare[marker_end..]) && number.is_none_or(|n| n == 1)
    });
    list || thematic_break(bare)
        || fence(bare).or_else(|| math(bare)).is_some()
        || heading(bare)
        || bare.starts_with('>')
        || html(bare).is_some_and(|(_, interrupting)| interrupting)
}

/// Whether a line opens a block where it stands, read for a line no
/// paragraph holds — lazily under one, or under a table. Any list marker
/// opens an item there, and a `|` is taken as a table, which is cautious.
fn starts_block(bare: &str, rel: usize) -> bool {
    rel < 4
        && (thematic_break(bare)
            || list_marker(bare).is_some()
            || fence(bare).or_else(|| math(bare)).is_some()
            || heading(bare)
            || bare.starts_with('>')
            || bare.starts_with('|')
            || html(bare).is_some_and(|(_, interrupting)| interrupting))
}

/// The end of the YAML front matter, or 0 when there is none.
fn front_matter_end(lines: &[&str]) -> usize {
    if lines.first().map(|line| line.trim_end_matches([' ', '\t'])) != Some("---") {
        return 0;
    }
    lines[1..]
        .iter()
        .position(|line| line.trim_end_matches([' ', '\t']) == "---")
        .map_or(0, |close| close + 2)
}

/// A list marker at the start of `rest`: where the marker ends and where
/// the item's text begins, both as offsets into `rest`, and an ordered
/// marker's number.
fn list_marker(rest: &str) -> Option<(usize, usize, Option<u64>)> {
    let (marker_end, number) = if rest.starts_with(['-', '+', '*']) {
        (1, None)
    } else {
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if !(1..=9).contains(&digits) || !rest[digits..].starts_with(['.', ')']) {
            return None;
        }
        (digits + 1, rest[..digits].parse().ok())
    };
    let after = &rest[marker_end..];
    if !(after.is_empty() || after.starts_with(' ')) {
        return None;
    }
    let spaces = after.len() - after.trim_start_matches(' ').len();
    // Five spaces or more open the item with indented code: its text
    // begins one column after the marker.
    let content = if spaces == 0 || spaces > 4 || blank(after) {
        marker_end + 1
    } else {
        marker_end + spaces
    };
    Some((marker_end, content, number))
}

fn thematic_break(bare: &str) -> bool {
    let line = trimmed(bare);
    let Some(first) = line.chars().next() else {
        return false;
    };
    matches!(first, '-' | '*' | '_')
        && line.chars().all(|c| c == first || c == ' ')
        && line.chars().filter(|&c| c == first).count() >= 3
}

fn setext_underline(bare: &str) -> bool {
    let line = trimmed(bare);
    let Some(first) = line.chars().next() else {
        return false;
    };
    matches!(first, '=' | '-') && line.chars().all(|c| c == first)
}

/// GFM's delimiter row: cells of hyphens, each with an optional colon at
/// either end, split by `|`, the outer ones optional. A row of one cell
/// and no `|` needs its colon, as a bare `---` is a setext underline.
fn delimiter_row(bare: &str) -> bool {
    let line = trimmed(bare);
    let inner = line.strip_prefix('|').unwrap_or(line);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    (line.contains('|') || line.contains(':'))
        && inner.split('|').all(|cell| {
            let cell = trimmed(cell);
            let cell = cell.strip_prefix(':').unwrap_or(cell);
            let cell = cell.strip_suffix(':').unwrap_or(cell);
            !cell.is_empty() && cell.chars().all(|c| c == '-')
        })
}

/// `[label]:` opening a line, a link reference definition or GFM's
/// footnote one.
fn reference_definition(bare: &str) -> bool {
    let Some(label) = bare.strip_prefix('[') else {
        return false;
    };
    label
        .find("]:")
        .is_some_and(|close| close > 0 && !label[..close].contains(']'))
}

fn fence(bare: &str) -> Option<(char, usize)> {
    let marker = bare.chars().next().filter(|&c| c == '`' || c == '~')?;
    let count = bare.len() - bare.trim_start_matches(marker).len();
    if count < 3 || (marker == '`' && bare[count..].contains('`')) {
        return None;
    }
    Some((marker, count))
}

/// GitHub's display math: `$$` alone on a line, read to the next one as a
/// fence is.
fn math(bare: &str) -> Option<(char, usize)> {
    (trimmed(bare) == "$$").then_some(('$', 2))
}

fn closes(bare: &str, marker: char, count: usize) -> bool {
    let run = bare.len() - bare.trim_start_matches(marker).len();
    run >= count && blank(&bare[run..])
}

/// An ATX heading: one to six `#`, then a space, a tab or the line's end.
fn heading(bare: &str) -> bool {
    let rest = bare.trim_start_matches('#');
    let hashes = bare.len() - rest.len();
    (1..=6).contains(&hashes) && (rest.is_empty() || rest.starts_with([' ', '\t']))
}

/// The tag names of CommonMark's sixth kind of HTML block.
const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

/// HTML that could open a block: the end it is kept to (`None` for a
/// blank line), and whether it may interrupt a paragraph — every kind but
/// CommonMark's seventh. Read widely: any tag opening a line counts as the
/// seventh kind, which only ever keeps more of the document as it stands.
fn html(bare: &str) -> Option<(Option<&'static str>, bool)> {
    let rest = bare.strip_prefix('<')?;
    let lower = rest.to_ascii_lowercase();
    let ends_name = |name: &str, after: &[char]| {
        lower.starts_with(name)
            && lower[name.len()..]
                .chars()
                .next()
                .is_none_or(|c| c == ' ' || after.contains(&c))
    };
    for (tag, end) in [
        ("script", "</script>"),
        ("pre", "</pre>"),
        ("style", "</style>"),
        ("textarea", "</textarea>"),
    ] {
        if ends_name(tag, &['>', '\t']) {
            return Some((Some(end), true));
        }
    }
    if rest.starts_with("!--") {
        return Some((Some("-->"), true));
    }
    if rest.starts_with('?') {
        return Some((Some("?>"), true));
    }
    if rest.starts_with("![CDATA[") {
        return Some((Some("]]>"), true));
    }
    if rest.starts_with('!') && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Some((Some(">"), true));
    }
    let closing = lower.strip_prefix('/');
    let name_at = closing.unwrap_or(&lower);
    if !name_at.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let block = BLOCK_TAGS.iter().any(|tag| {
        name_at.starts_with(tag)
            && name_at[tag.len()..]
                .chars()
                .next()
                .is_none_or(|c| matches!(c, ' ' | '\t' | '>' | '/'))
    });
    Some((None, block))
}

#[cfg(test)]
mod tests {
    use super::paragraphs;

    /// Each paragraph as (first line, last line, lead), 1-based.
    fn read(text: &str) -> Vec<(usize, usize, usize)> {
        let lines: Vec<&str> = text.lines().collect();
        paragraphs(&lines)
            .into_iter()
            .map(|p| (p.start + 1, p.end, p.lead))
            .collect()
    }

    #[test]
    fn a_paragraph_runs_to_the_blank_line() {
        assert_eq!(
            read("一行目\n二行目\n\n三行目\n"),
            vec![(1, 2, 0), (4, 4, 0)]
        );
    }

    #[test]
    fn a_list_item_wraps_at_its_text_and_a_nested_one_at_its_own() {
        let text = "\
- 外の箇条
  続き
  - 内の箇条
    続き
- 次
1. 番号
10. 二桁
";
        assert_eq!(
            read(text),
            vec![(1, 2, 2), (3, 4, 4), (5, 5, 2), (6, 6, 3), (7, 7, 4)]
        );
    }

    /// A line under an item at less than the item's indent, with no blank
    /// above, still belongs to its paragraph.
    #[test]
    fn a_lazy_line_stays_in_the_paragraph() {
        assert_eq!(read("- 箇条\n続き\n"), vec![(1, 2, 2)]);
    }

    #[test]
    fn a_later_paragraph_of_an_item_keeps_its_own_indent() {
        assert_eq!(
            read("- 箇条\n\n  二段落目\n  続き\n"),
            vec![(1, 1, 2), (3, 4, 2)]
        );
    }

    #[test]
    fn fences_tables_headings_and_front_matter_are_not_prose() {
        let text = "\
---
description: 長い説明
---

# 見出し

```
- コード
本文ではない
```

| 表 | 値 |
|---|---|
| a | b |
表の行の続き

本文
";
        assert_eq!(read(text), vec![(17, 17, 0)]);
    }

    /// GFM's outer pipes are optional: the delimiter row says it is a
    /// table, and the line above it is the header.
    #[test]
    fn a_table_without_its_outer_pipes_is_not_prose() {
        assert_eq!(
            read("abc | def\n--- | ---\nbar | baz\n\n本文\n"),
            vec![(5, 5, 0)]
        );
        assert_eq!(read("前の文\nabc | def\n:-- | --:\n"), vec![]);
    }

    #[test]
    fn a_fence_inside_an_item_ends_at_its_closing_line() {
        let text = "- 箇条\n\n  ```\n  code\n  ```\n\n  後の段落\n";
        assert_eq!(read(text), vec![(1, 1, 2), (7, 7, 2)]);
    }

    /// CommonMark reads a comment that opens a line as an HTML block, which
    /// interrupts the paragraph above it.
    #[test]
    fn a_comment_that_opens_a_line_is_a_block_of_its_own() {
        let text = "前の段落\n<!--cmd:a.b-->`cargo xtask a` は\n後の段落\n";
        assert_eq!(read(text), vec![(1, 1, 0), (3, 3, 0)]);
    }

    #[test]
    fn an_html_block_is_kept_to_the_blank_line() {
        let text = "<details>\n<summary>要約</summary>\n本文\n\n後\n";
        assert_eq!(read(text), vec![(5, 5, 0)]);
    }

    /// The seventh kind cannot interrupt a paragraph, so a tag opening a
    /// line under one goes on it — as far as a setext underline under both.
    #[test]
    fn a_tag_of_the_seventh_kind_goes_on_the_paragraph() {
        assert_eq!(read("前\n<span>x</span>\n後\n"), vec![(1, 3, 0)]);
        assert_eq!(read("前\n<span>x</span>\n===\n"), vec![]);
        assert_eq!(read("前\n<div>\n後\n"), vec![(1, 1, 0)]);
    }

    #[test]
    fn a_block_quote_is_kept_to_the_blank_line() {
        assert_eq!(read("> # T\ntext\n\n本文\n"), vec![(4, 4, 0)]);
        assert_eq!(read("> ```\n> code\ntext\n"), vec![]);
        assert_eq!(read("前の文\n> 引用\n"), vec![(1, 1, 0)]);
    }

    #[test]
    fn a_link_reference_definition_is_not_prose() {
        let text = "[a]: https://example.com/a\n[b]: https://example.com/b\n\n本文\n";
        assert_eq!(read(text), vec![(4, 4, 0)]);
        assert_eq!(read("[^注]: 脚注の本文\n"), vec![]);
    }

    #[test]
    fn a_setext_heading_is_not_prose() {
        assert_eq!(read("見出し\n===\n\n本文\n"), vec![(4, 4, 0)]);
    }

    #[test]
    fn indented_code_is_not_prose_but_an_indented_continuation_is() {
        assert_eq!(read("本文\n\n    code\n"), vec![(1, 1, 0)]);
        assert_eq!(read("本文\n    続き\n"), vec![(1, 2, 0)]);
        assert_eq!(read("- 箇条\n      - 続き\n"), vec![(1, 2, 2)]);
    }

    #[test]
    fn a_line_that_opens_a_block_ends_the_paragraph() {
        assert_eq!(read("本文\n- 箇条\n"), vec![(1, 1, 0), (2, 2, 2)]);
        assert_eq!(read("本文\n1. 箇条\n"), vec![(1, 1, 0), (2, 2, 3)]);
        assert_eq!(read("本文\n# 見出し\n"), vec![(1, 1, 0)]);
        assert_eq!(read("本文\n---\n"), vec![]);
    }

    /// CommonMark lets only a bullet with text, or `1.` with text, of the
    /// list markers interrupt a paragraph — and a line that does not open an
    /// item opens none here either, so no item stands over what follows.
    #[test]
    fn a_marker_that_cannot_interrupt_goes_on_the_paragraph() {
        assert_eq!(
            read("本文の行\n2. は箇条ではない\n\n    code\n"),
            vec![(1, 2, 0)]
        );
        assert_eq!(read("text\n*\n\n     code\n"), vec![(1, 2, 0)]);
        assert_eq!(read("本文\n#5 は番号\n"), vec![(1, 2, 0)]);
        assert_eq!(read("本文\n| 表ではない |\n"), vec![(1, 2, 0)]);
    }

    #[test]
    fn a_marker_with_nothing_after_it_opens_an_empty_item() {
        assert_eq!(read("-\n  本文\n"), vec![(2, 2, 2)]);
    }

    /// An item begins with at most one blank line, so a blank line closes
    /// an empty one, and what is four columns in under it is code.
    #[test]
    fn a_blank_line_closes_an_empty_item() {
        assert_eq!(read("-\n\n    code\n"), vec![]);
        assert_eq!(read("-\n\n  本文\n"), vec![(3, 3, 2)]);
    }

    /// A fence a quote or a definition stands over opens on its own line,
    /// and a blank line inside it is the fence's.
    #[test]
    fn a_fence_ends_a_quote_or_a_definition_above_it() {
        assert_eq!(read("> 引用\n```\ncode\n\nmore code\n```\n"), vec![]);
        assert_eq!(read("[a]: x\n```\ncode\n\nmore code\n```\n"), vec![]);
        assert_eq!(read("> 引用\n続き\n\n本文\n"), vec![(4, 4, 0)]);
    }

    #[test]
    fn only_spaces_and_tabs_are_white_space() {
        assert_eq!(read("本文\n\u{3000}\n続き\n"), vec![(1, 3, 0)]);
        assert_eq!(read("```\ncode\n\u{3000}```\n\n本文\n```\n"), vec![]);
    }

    #[test]
    fn a_preformatted_tag_with_a_tab_is_html_of_the_first_kind() {
        let text = "text\n<pre\tclass=\"a\">\n\npre line\n</pre>\n";
        assert_eq!(read(text), vec![(1, 1, 0)]);
    }

    /// GitHub's display math is a block, read to its closing `$$`.
    #[test]
    fn display_math_is_not_prose() {
        assert_eq!(
            read("前の文\n$$\na = b\n\nc = d\n$$\n\n後の文\n"),
            vec![(1, 1, 0), (8, 8, 0)]
        );
    }

    #[test]
    fn a_table_of_one_column_is_found_by_its_colon() {
        assert_eq!(read("header text\n:---\n"), vec![]);
    }
}
