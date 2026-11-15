//! What a word is in a line of text: the one definition the find bar's
//! whole word (`textfind`) and the double-click on the diff and the command
//! log (the app's `DiffTextSelect` / `CommandsTextSelect`) share, so a word
//! the bar finds as a whole is the word a double-click on it takes.
//!
//! A word is a run of letters, digits and `_` — an identifier, as an editor
//! reads code. Whitespace runs together too; any other character is a run
//! of its own, so a double-click on `(` takes the `(` alone. Characters, not
//! grapheme clusters, and no dictionary: a line of kanji with no space or
//! punctuation in it is one word, and a combining mark is a character of
//! its own that is no letter, so a word written with one decomposed stops
//! before it.

use std::ops::Range;

/// Whether `c` belongs to a word.
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether two neighbouring characters stand in one run.
fn joins(a: char, b: char) -> bool {
    (is_word_char(a) && is_word_char(b)) || (a.is_whitespace() && b.is_whitespace())
}

/// The bytes of the run the character starting at byte `at` stands in.
/// None where no character starts there: at or past the line's end, or
/// inside a character.
pub fn run_at(text: &str, at: usize) -> Option<Range<usize>> {
    let first = text.get(at..)?.chars().next()?;
    let mut start = at;
    let mut leftmost = first;
    for c in text[..at].chars().rev() {
        if !joins(c, leftmost) {
            break;
        }
        start -= c.len_utf8();
        leftmost = c;
    }
    let mut end = at + first.len_utf8();
    let mut rightmost = first;
    for c in text[end..].chars() {
        if !joins(rightmost, c) {
            break;
        }
        end += c.len_utf8();
        rightmost = c;
    }
    Some(start..end)
}

/// The run the boundary at byte `at` cuts in two, or None where it stands
/// between two runs (or at either end of the line) — what a drag that
/// selects word by word snaps out to.
pub fn run_across(text: &str, at: usize) -> Option<Range<usize>> {
    let before = text.get(..at)?.chars().next_back()?;
    let after = text[at..].chars().next()?;
    if joins(before, after) {
        run_at(text, at)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text a run holds, for reading the cases.
    fn ran(text: &str, at: usize) -> Option<&str> {
        run_at(text, at).map(|bytes| &text[bytes])
    }

    #[test]
    fn a_word_is_letters_digits_and_underscores() {
        let line = "let snake_case2 = foo(bar);";
        assert_eq!(ran(line, 4), Some("snake_case2"), "from its head");
        assert_eq!(ran(line, 9), Some("snake_case2"), "from the underscore");
        assert_eq!(ran(line, 14), Some("snake_case2"), "from its tail");
        assert_eq!(ran(line, 18), Some("foo"));
        assert_eq!(ran(line, 0), Some("let"), "at the line's head");
    }

    #[test]
    fn whitespace_runs_together_and_anything_else_stands_alone() {
        let line = "a  \t b ===(c)";
        assert_eq!(ran(line, 1), Some("  \t "), "spaces and a tab");
        assert_eq!(ran(line, 7), Some("="), "one of three");
        assert_eq!(ran(line, 10), Some("("));
        assert_eq!(ran(line, 12), Some(")"), "the last character");
    }

    #[test]
    fn letters_of_any_script_are_word_characters() {
        let line = "// 日本語のコメント。next";
        let head = line.find('日').unwrap();
        let comment = &line[head..line.find('。').unwrap()];
        assert_eq!(ran(line, head), Some(comment));
        assert_eq!(ran(line, line.find('。').unwrap()), Some("。"));
    }

    #[test]
    fn no_run_starts_past_the_end_or_inside_a_character() {
        let line = "ab日";
        assert_eq!(run_at(line, line.len()), None, "the line's end");
        assert_eq!(run_at(line, 99), None, "past it");
        assert_eq!(run_at(line, 3), None, "inside 日");
        assert_eq!(run_at("", 0), None, "an empty line");
    }

    #[test]
    fn a_boundary_inside_a_run_is_across_it_and_one_between_runs_is_not() {
        let line = "foo.bar  baz";
        assert_eq!(run_across(line, 1), Some(0..3), "inside foo");
        assert_eq!(run_across(line, 3), None, "between foo and .");
        assert_eq!(run_across(line, 4), None, "between . and bar");
        assert_eq!(run_across(line, 8), Some(7..9), "inside the spaces");
        assert_eq!(run_across(line, 0), None, "the line's head");
        assert_eq!(run_across(line, line.len()), None, "the line's end");
        assert_eq!(run_across(line, 99), None, "past it");
    }
}
