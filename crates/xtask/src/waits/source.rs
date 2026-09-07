//! A source file as the scanner reads it: the code with its comments and
//! string literals blanked out, the `#[cfg(test)]` regions of a Rust
//! file, and the markers that let a timed wait stand.

use std::ops::RangeInclusive;

use super::{Candidate, Exception, Finding};

/// Why a timed wait is allowed to stand, as its marker names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Purpose {
    /// A sleep that paces a retry whose completion is causal: the loop
    /// ends on an answer, never on the clock.
    Paced,
    /// A wall-clock ceiling outside the common budget that only names a
    /// failure, never decides one.
    Ceiling,
    /// A clock read that is printed or handed to the code under test,
    /// and judged by nothing.
    Measured,
    /// A real-time property of the product under test, judged as a bound
    /// no load can break.
    Timed,
}

impl Purpose {
    pub(super) const ALL: [Purpose; 4] = [
        Purpose::Paced,
        Purpose::Ceiling,
        Purpose::Measured,
        Purpose::Timed,
    ];

    pub(super) fn name(self) -> &'static str {
        match self {
            Purpose::Paced => "paced",
            Purpose::Ceiling => "ceiling",
            Purpose::Measured => "measured",
            Purpose::Timed => "timed",
        }
    }

    fn named(word: &str) -> Option<Purpose> {
        Purpose::ALL.into_iter().find(|p| p.name() == word)
    }
}

/// A `// waits(<purpose>): <reason>` marker on one line. What is wrong
/// with a malformed one is carried instead of the purpose, so the file
/// names it as a finding rather than reading it as nothing.
pub(super) struct Marker {
    pub line: usize,
    pub purpose: Result<Purpose, String>,
}

const MARKER: &str = "// waits(";

/// The markers of `text`, in line order.
pub(super) fn markers(text: &str) -> Vec<Marker> {
    text.lines()
        .enumerate()
        .filter_map(|(at, line)| {
            let start = line.find(MARKER)?;
            let rest = &line[start + MARKER.len()..];
            Some(Marker {
                line: at + 1,
                purpose: parse_marker(rest),
            })
        })
        .collect()
}

fn parse_marker(rest: &str) -> Result<Purpose, String> {
    let Some(close) = rest.find(')') else {
        return Err("the purpose is not closed with `)`".into());
    };
    let word = rest[..close].trim();
    let purpose = Purpose::named(word).ok_or_else(|| {
        let known = Purpose::ALL.map(Purpose::name).join(" / ");
        format!("`{word}` is not a purpose ({known})")
    })?;
    let after = rest[close + 1..].trim_start();
    let reason = after
        .strip_prefix(':')
        .ok_or_else(|| "a `:` and the reason must follow the purpose".to_string())?;
    if reason.trim().is_empty() {
        return Err("the reason after the `:` is empty".into());
    }
    Ok(purpose)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lang {
    Rust,
    Qml,
}

/// `text` with every comment and string literal replaced by spaces, line
/// for line: what is left is the code, and a `;` or a brace inside a
/// string no longer reads as one. Rust char literals and raw strings are
/// blanked too; a lifetime's `'` is kept, since it opens nothing.
pub(super) fn code_view(text: &str, lang: Lang) -> String {
    view(text, lang, false)
}

/// `text` with its comments blanked and its strings kept, character for
/// character beside [`code_view`]: the code with what it says — where a
/// script this runner writes stands, and where a comment names nothing.
pub(super) fn strings_view(text: &str, lang: Lang) -> String {
    view(text, lang, true)
}

fn view(text: &str, lang: Lang, keep_strings: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let comment = c == '/' && matches!(next, Some('/' | '*'));
        let skipped = if c == '/' && next == Some('/') {
            line_comment_len(&chars, i)
        } else if c == '/' && next == Some('*') {
            block_comment_len(&chars, i, lang)
        } else if c == '"' || (lang == Lang::Qml && matches!(c, '\'' | '`')) {
            quoted_len(&chars, i, c)
        } else if lang == Lang::Rust && c == 'r' && !word_before(&chars, i) {
            raw_string_len(&chars, i)
        } else if lang == Lang::Rust
            && matches!(c, 'b' | 'c')
            && next == Some('r')
            && !word_before(&chars, i)
        {
            // `br"…"` / `cr"…"`: the raw string under its byte or C prefix.
            match raw_string_len(&chars, i + 1) {
                0 => 0,
                raw => raw + 1,
            }
        } else if lang == Lang::Rust && c == '\'' {
            char_literal_len(&chars, i)
        } else {
            0
        };
        if skipped == 0 {
            out.push(c);
            i += 1;
            continue;
        }
        if comment || !keep_strings {
            for blanked in &chars[i..i + skipped] {
                out.push(if *blanked == '\n' { '\n' } else { ' ' });
            }
        } else {
            out.extend(&chars[i..i + skipped]);
        }
        i += skipped;
    }
    out
}

fn word_before(chars: &[char], at: usize) -> bool {
    at > 0 && (chars[at - 1].is_alphanumeric() || chars[at - 1] == '_')
}

fn line_comment_len(chars: &[char], at: usize) -> usize {
    chars[at..].iter().take_while(|c| **c != '\n').count()
}

/// A block comment, nested where the language nests them (Rust does, JS
/// does not). An unclosed one runs to the end of the file.
fn block_comment_len(chars: &[char], at: usize, lang: Lang) -> usize {
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
fn quoted_len(chars: &[char], at: usize, quote: char) -> usize {
    let mut i = at + 1;
    while i < chars.len() && chars[i] != quote {
        i += if chars[i] == '\\' { 2 } else { 1 };
    }
    (i + 1).min(chars.len()) - at
}

/// `r"..."` or `r#"..."#` with any number of hashes; 0 where `r` opens no
/// raw string here.
fn raw_string_len(chars: &[char], at: usize) -> usize {
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
fn char_literal_len(chars: &[char], at: usize) -> usize {
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

/// The line ranges of a Rust file's `#[cfg(test)] mod … { … }` blocks,
/// from the attribute to the closing brace, read off its code view. A
/// `#[cfg(test)] mod x;` declares a file that is read on its own
/// ([`declared_test_modules`]) and opens no region here.
pub(super) fn test_regions(code: &str) -> Vec<RangeInclusive<usize>> {
    let chars: Vec<char> = code.chars().collect();
    let mut regions = Vec::new();
    for head in test_mod_heads(code, &chars) {
        if chars.get(head.after) != Some(&'{') {
            continue;
        }
        let close = matching_brace(&chars, head.after).unwrap_or(chars.len() - 1);
        regions.push(line_of(&chars, head.attribute)..=line_of(&chars, close));
    }
    regions
}

/// The names of the modules a file declares as test code — `#[cfg(test)]
/// mod x;` — whose files are test code from top to bottom wherever they
/// stand and whatever they are called. A `#[path]` is not followed: the
/// files this tree declares that way are named `tests.rs` / `*_tests.rs`
/// and are read as whole test files by that name.
pub(super) fn declared_test_modules(code: &str) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    test_mod_heads(code, &chars)
        .into_iter()
        .filter(|head| chars.get(head.after) == Some(&';'))
        .map(|head| head.name)
        .collect()
}

const CFG_TEST: &str = "#[cfg(test)]";

/// A `mod` that a `#[cfg(test)]` applies to.
struct TestModHead {
    name: String,
    /// The index of the attribute, where the module's lines begin.
    attribute: usize,
    /// The index of what follows the name — `{` for a block, `;` for a
    /// declaration.
    after: usize,
}

/// Every `mod` that a `#[cfg(test)]` in `code` applies to.
fn test_mod_heads(code: &str, chars: &[char]) -> Vec<TestModHead> {
    let mut heads = Vec::new();
    let mut from = 0;
    while let Some(found) = code[from..].find(CFG_TEST) {
        let at = from + found;
        from = at + CFG_TEST.len();
        if let Some((name, after)) = test_mod_head(chars, code[..from].chars().count()) {
            heads.push(TestModHead {
                name,
                attribute: code[..at].chars().count(),
                after,
            });
        }
    }
    heads
}

/// The `mod` head that follows a `#[cfg(test)]` at `at`, past whatever
/// other attributes stand between them: its name, and the index just past
/// the name and the whitespace after it. `pub` and `pub(…)` are stepped
/// over — a test module is no less one for being reachable.
fn test_mod_head(chars: &[char], mut at: usize) -> Option<(String, usize)> {
    loop {
        at = past_whitespace(chars, at);
        if chars.get(at) == Some(&'#') && chars.get(at + 1) == Some(&'[') {
            at = matching(chars, at + 1, '[', ']')? + 1;
            continue;
        }
        break;
    }
    let mut word = word_at(chars, at);
    if word == "pub" {
        at = past_whitespace(chars, at + 3);
        if chars.get(at) == Some(&'(') {
            at = past_whitespace(chars, matching(chars, at, '(', ')')? + 1);
        }
        word = word_at(chars, at);
    }
    if word != "mod" {
        return None;
    }
    at = past_whitespace(chars, at + 3);
    let name = word_at(chars, at);
    if name.is_empty() {
        return None;
    }
    at = past_whitespace(chars, at + name.chars().count());
    Some((name, at))
}

fn past_whitespace(chars: &[char], mut at: usize) -> usize {
    while chars.get(at).is_some_and(|c| c.is_whitespace()) {
        at += 1;
    }
    at
}

fn word_at(chars: &[char], at: usize) -> String {
    chars
        .get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|c| c.is_alphanumeric() || **c == '_')
        .collect()
}

fn matching_brace(chars: &[char], open: usize) -> Option<usize> {
    matching(chars, open, '{', '}')
}

fn matching(chars: &[char], open: usize, opener: char, closer: char) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in chars.iter().enumerate().skip(open) {
        if *c == opener {
            depth += 1;
        } else if *c == closer {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// 1-based line of the character at `at`.
pub(super) fn line_of(chars: &[char], at: usize) -> usize {
    chars[..at].iter().filter(|c| **c == '\n').count() + 1
}

/// Whether `token` occurs in `haystack` on a word boundary: `timeout(`
/// must not be found inside `no_timeout(`, nor `bounded(` inside
/// `unbounded(`. A token that opens with a non-word byte (`.outcome()`)
/// matches anywhere.
pub(super) fn has_token(haystack: &str, token: &str) -> bool {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let guarded = token.as_bytes().first().copied().is_some_and(word);
    let mut from = 0;
    while let Some(found) = haystack[from..].find(token) {
        let at = from + found;
        if !guarded || at == 0 || !word(haystack.as_bytes()[at - 1]) {
            return true;
        }
        from = at + token.len();
    }
    false
}

/// Holds the candidates of one file against its markers. A candidate a
/// marker covers — on one of its own lines, or in the comment block
/// standing directly above it — is an exception under that marker's
/// purpose; the rest are findings. A marker that covers nothing, or says
/// nothing readable, is a finding of its own, so a rewritten wait sheds
/// its marker the way a fixed lint sheds its `#[expect]`.
pub(super) fn judged(
    file: &str,
    text: &str,
    code: &str,
    candidates: Vec<Candidate>,
) -> (Vec<Finding>, Vec<Exception>) {
    let raw: Vec<&str> = text.lines().collect();
    let code_lines: Vec<&str> = code.lines().collect();
    let comment_only = |line: usize| {
        raw.get(line - 1).is_some_and(|r| !r.trim().is_empty())
            && code_lines.get(line - 1).is_none_or(|c| c.trim().is_empty())
    };
    let markers = markers(text);
    let mut used = vec![false; markers.len()];
    let mut findings = Vec::new();
    let mut exceptions = Vec::new();
    for candidate in candidates {
        let mut above = candidate.first;
        while above > 1 && comment_only(above - 1) {
            above -= 1;
        }
        let covered = above..=candidate.last;
        match markers.iter().position(|m| covered.contains(&m.line)) {
            Some(i) => {
                used[i] = true;
                if let Ok(purpose) = markers[i].purpose {
                    exceptions.push(Exception {
                        file: file.to_string(),
                        line: markers[i].line,
                        purpose,
                    });
                }
            }
            None => findings.push(Finding {
                file: file.to_string(),
                line: candidate.shown,
                rule: candidate.rule,
                excerpt: raw
                    .get(candidate.shown.saturating_sub(1))
                    .map(|l| l.trim().to_string())
                    .unwrap_or_default(),
            }),
        }
    }
    for (marker, used) in markers.iter().zip(used) {
        let excerpt = match (&marker.purpose, used) {
            (Err(what), _) => what.clone(),
            (Ok(_), false) => "covers no wait — drop it".to_string(),
            (Ok(_), true) => continue,
        };
        findings.push(Finding {
            file: file.to_string(),
            line: marker.line,
            rule: "marker",
            excerpt,
        });
    }
    findings.sort_by_key(|f| f.line);
    (findings, exceptions)
}

#[cfg(test)]
mod tests {
    use super::{Lang, Purpose, code_view, declared_test_modules, markers, test_regions};

    #[test]
    fn strings_and_comments_are_blanked_line_for_line() {
        let text = "let a = \"x; {\"; // a comment; with punctuation\nlet b = 1;";
        let code = code_view(text, Lang::Rust);
        assert_eq!(code.lines().count(), 2);
        assert!(!code.contains("comment"), "{code}");
        assert_eq!(
            code.matches(';').count(),
            2,
            "only the code's own `;` survive: {code}"
        );
        assert_eq!(
            code.matches('{').count(),
            0,
            "a brace in a string is no brace: {code}"
        );
    }

    #[test]
    fn the_strings_view_keeps_what_the_strings_say_and_blanks_the_comments() {
        let text = "let s = \"Start-Sleep 1; {\"; // the script's Start-Sleep\nlet b = 1;";
        let said = super::strings_view(text, Lang::Rust);
        assert_eq!(said.lines().count(), 2);
        assert_eq!(said.matches("Start-Sleep").count(), 1, "{said}");
        assert!(said.contains("\"Start-Sleep 1; {\""), "{said}");
        assert!(!said.contains("script"), "{said}");
        assert_eq!(
            said.len(),
            code_view(text, Lang::Rust).len(),
            "the two views stand character for character"
        );
    }

    #[test]
    fn raw_strings_chars_and_lifetimes_are_told_apart() {
        let text = "let s = r#\"a \" b\"#; let c = '{'; let q = '\\''; fn f<'a>(x: &'a str) {}";
        let code = code_view(text, Lang::Rust);
        assert_eq!(code.matches('{').count(), 1, "{code}");
        assert_eq!(code.matches('}').count(), 1, "{code}");
        assert!(code.contains("<'a>"), "a lifetime keeps its quote: {code}");
    }

    #[test]
    fn a_nested_block_comment_closes_where_rust_closes_it() {
        let text = "/* outer /* inner */ still */ let a = 1;";
        let code = code_view(text, Lang::Rust);
        assert!(!code.contains("still"), "{code}");
        assert!(code.contains("let a = 1;"), "{code}");
    }

    #[test]
    fn qml_strings_of_all_three_quotes_are_blanked() {
        let text = "verify(x, 'a, b'); compare(y, `c, d`); tryVerify(z, \"e, f\")";
        let code = code_view(text, Lang::Qml);
        assert_eq!(code.matches(',').count(), 3, "{code}");
    }

    #[test]
    fn a_marker_names_its_purpose_and_a_malformed_one_says_what_is_wrong() {
        let text = "\
// waits(paced): the retry is paced, the answer ends it
x;
// waits(quick): nothing
y;
// waits(timed) no colon
z;
// waits(measured):
";
        let found = markers(text);
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].purpose, Ok(Purpose::Paced));
        assert!(
            found[1]
                .purpose
                .as_ref()
                .is_err_and(|e| e.contains("`quick`"))
        );
        assert!(found[2].purpose.as_ref().is_err_and(|e| e.contains("`:`")));
        assert!(
            found[3]
                .purpose
                .as_ref()
                .is_err_and(|e| e.contains("empty"))
        );
    }

    #[test]
    fn a_raw_byte_string_and_the_escaped_quote_char_hide_nothing_behind_them() {
        let text = "let b = br\"\\\"; let q = '\\''; std::thread::sleep(x);";
        let code = code_view(text, Lang::Rust);
        assert!(code.contains("std::thread::sleep(x);"), "{code}");
        assert!(!code.contains('\\'), "{code}");
        assert!(
            !code.contains('\''),
            "the char literal is blanked whole: {code}"
        );
    }

    #[test]
    fn a_declared_test_module_is_named_and_a_reachable_test_block_is_a_region() {
        let text = "\
#[cfg(test)]
mod testkit;
#[cfg(test)]
pub(crate) mod tests {
    fn t() {}
}
";
        let code = code_view(text, Lang::Rust);
        assert_eq!(declared_test_modules(&code), vec!["testkit".to_string()]);
        assert_eq!(
            test_regions(&code),
            vec![3..=6],
            "the region opens at the attribute"
        );
    }

    #[test]
    fn the_test_module_is_a_region_from_its_attribute_and_a_declaration_is_not() {
        let text = "\
fn production() {}
#[cfg(test)]
mod other_tests;
#[cfg(test)]
#[allow(dead_code)]
mod tests {
    fn t() { let b = \"}\"; }
}
fn more() {}
";
        let regions = test_regions(&code_view(text, Lang::Rust));
        assert_eq!(regions, vec![4..=8]);
    }
}
