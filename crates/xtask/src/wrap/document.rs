//! A whole document with its paragraphs filled, every other line as it
//! stood, and a line end at its end.

use super::blocks::paragraphs;
use super::fill::fill;

pub(super) struct Rewrapped {
    pub(super) text: String,
    /// The first line (1-based) of each paragraph the fill changed.
    pub(super) changed: Vec<usize>,
    /// The first line of each paragraph kept as it stands (`fill`).
    pub(super) kept: Vec<usize>,
    /// Whether the document lacked the line end at its end, which the text
    /// now has.
    pub(super) ended: bool,
}

/// Whether a document lacks the line end at its end. An empty one has no
/// last line to end.
pub(super) fn unended(text: &str) -> bool {
    !text.is_empty() && !text.ends_with('\n')
}

pub(super) fn rewrap(text: &str, width: usize) -> Rewrapped {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut changed = Vec::new();
    let mut kept = Vec::new();
    let mut at = 0;
    for paragraph in paragraphs(&lines) {
        out.extend(
            lines[at..paragraph.start]
                .iter()
                .map(|line| (*line).to_string()),
        );
        let own = &lines[paragraph.start..paragraph.end];
        match fill(&lines, &paragraph, width) {
            Some(filled) if filled.iter().map(String::as_str).ne(own.iter().copied()) => {
                changed.push(paragraph.start + 1);
                out.extend(filled);
            }
            Some(_) => out.extend(own.iter().map(|line| (*line).to_string())),
            None => {
                kept.push(paragraph.start + 1);
                out.extend(own.iter().map(|line| (*line).to_string()));
            }
        }
        at = paragraph.end;
    }
    out.extend(lines[at..].iter().map(|line| (*line).to_string()));
    let mut text_out = out.join("\n");
    if !text.is_empty() {
        text_out.push('\n');
    }
    Rewrapped {
        text: text_out,
        changed,
        kept,
        ended: unended(text),
    }
}

#[cfg(test)]
mod tests {
    use super::rewrap;

    #[test]
    fn only_the_paragraphs_move() {
        let text = "\
# 見出しはどれだけ長くても一行のまま

段落の一行目
二行目

```
コードの行はどれだけ長くても触らない
```

- 箇条 の 続き
";
        let done = rewrap(text, 16);
        assert_eq!(
            done.text,
            "\
# 見出しはどれだけ長くても一行のまま

段落の一行目二行
目

```
コードの行はどれだけ長くても触らない
```

- 箇条 の 続き
"
        );
        assert_eq!(done.changed, vec![3]);
    }

    #[test]
    fn a_wrapped_document_wraps_to_itself() {
        let text = "本文の一行目で、長いので折り返す。次の文も続く。\n\n- 箇条の本文も同じく長くて折り返す\n";
        let once = rewrap(text, 20).text;
        let twice = rewrap(&once, 20);
        assert_eq!(twice.text, once);
        assert!(twice.changed.is_empty());
    }

    /// Every document ends on a line end; the blank lines before it, and an
    /// empty document, stay as they are.
    #[test]
    fn a_document_always_ends_on_a_line_end() {
        let done = rewrap("a\nb", 80);
        assert_eq!(done.text, "a b\n");
        assert!(done.ended);
        let done = rewrap("a\nb\n", 80);
        assert_eq!(done.text, "a b\n");
        assert!(!done.ended);
        assert_eq!(rewrap("a\n\n\n", 80).text, "a\n\n\n");
        assert_eq!(rewrap("```\ncode\n```", 80).text, "```\ncode\n```\n");
        assert_eq!(rewrap("", 80).text, "");
        assert!(!rewrap("", 80).ended);
    }
}
