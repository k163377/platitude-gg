//! The lexical layer of a Rust or QML file — where its comments and
//! its literals stand — read here for the tools that read sources
//! without reading them for meaning (`waits::source`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lang {
    Rust,
    Qml,
}

pub(crate) fn word_before(chars: &[char], at: usize) -> bool {
    at > 0 && (chars[at - 1].is_alphanumeric() || chars[at - 1] == '_')
}

pub(crate) fn line_comment_len(chars: &[char], at: usize) -> usize {
    chars[at..].iter().take_while(|c| **c != '\n').count()
}

/// A block comment, nested where the language nests them (Rust does, JS
/// does not). An unclosed one runs to the end of the file.
pub(crate) fn block_comment_len(chars: &[char], at: usize, lang: Lang) -> usize {
    let mut depth = 1;
    let mut i = at + 2;
    while i < chars.len() && depth > 0 {
        if lang == Lang::Rust && chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            i += 2;
        } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
            depth -= 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    i - at
}

/// A literal under `quote`, escapes skipped. An unclosed one runs to the
/// end of the file.
pub(crate) fn quoted_len(chars: &[char], at: usize, quote: char) -> usize {
    let mut i = at + 1;
    while i < chars.len() && chars[i] != quote {
        i += if chars[i] == '\\' { 2 } else { 1 };
    }
    (i + 1).min(chars.len()) - at
}

/// `r"..."` or `r#"..."#` with any number of hashes; 0 where `r` opens no
/// raw string here.
pub(crate) fn raw_string_len(chars: &[char], at: usize) -> usize {
    let hashes = chars[at + 1..].iter().take_while(|c| **c == '#').count();
    if chars.get(at + 1 + hashes) != Some(&'"') {
        return 0;
    }
    let mut i = at + 2 + hashes;
    while i < chars.len() {
        if chars[i] == '"'
            && chars[i + 1..]
                .iter()
                .take(hashes)
                .filter(|c| **c == '#')
                .count()
                == hashes
        {
            return i + 1 + hashes - at;
        }
        i += 1;
    }
    chars.len() - at
}

/// `'x'`, `'\n'`, `'\''`, `'\u{..}'`; 0 for a lifetime.
pub(crate) fn char_literal_len(chars: &[char], at: usize) -> usize {
    match chars.get(at + 1) {
        // The escaped character is stepped over before the closing quote
        // is looked for: in `'\''` that character is a quote itself.
        Some('\\') => chars
            .get(at + 3..)
            .and_then(|rest| rest.iter().position(|c| *c == '\''))
            .map_or(0, |close| close + 4),
        Some(_) if chars.get(at + 2) == Some(&'\'') => 3,
        _ => 0,
    }
}

/// The words a value may follow, which end in a word character the way
/// a name does: after any other name the `/` of [`regex_len`] divides.
const VALUE_WORDS: [&str; 8] = [
    "return", "typeof", "case", "in", "of", "new", "delete", "void",
];

/// A JavaScript regular expression literal at `at`; 0 where the `/`
/// divides instead. One stands only where a value may begin, and closes
/// with an unescaped `/` outside a `[…]` class, on its own line.
pub(crate) fn regex_len(chars: &[char], at: usize) -> usize {
    if !opens_a_value(chars, at) {
        return 0;
    }
    let mut i = at + 1;
    let mut class = false;
    while let Some(c) = chars.get(i) {
        match c {
            '\n' => return 0,
            '\\' if chars.get(i + 1).is_some_and(|next| *next != '\n') => i += 1,
            '[' => class = true,
            ']' => class = false,
            '/' if !class => return i + 1 - at,
            _ => {}
        }
        i += 1;
    }
    0
}

/// Whether a value may begin at `at`: after a name (other than
/// [`VALUE_WORDS`]), a number, a closing bracket or a quote what follows
/// is an operator, and after anything else a value.
pub(crate) fn opens_a_value(chars: &[char], at: usize) -> bool {
    let Some(before) = chars[..at].iter().rposition(|c| !c.is_whitespace()) else {
        return true;
    };
    let c = chars[before];
    if matches!(c, ')' | ']' | '"' | '\'' | '`') {
        return false;
    }
    if !(c.is_alphanumeric() || c == '_') {
        return true;
    }
    let start = chars[..before]
        .iter()
        .rposition(|c| !(c.is_alphanumeric() || *c == '_'))
        .map_or(0, |i| i + 1);
    let word: String = chars[start..=before].iter().collect();
    VALUE_WORDS.contains(&word.as_str())
}

