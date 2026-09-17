//! A source file as the scanner reads it: the code with its comments and
//! string literals blanked out, the `#[cfg(test)]` regions of a Rust
//! file, and the markers that let a timed wait stand.

use std::ops::RangeInclusive;

use super::{Candidate, Exception, Finding};

/// Why a timed wait is allowed to stand, as its marker names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Purpose {
    /// A sleep that paces a retry whose completion is causal: the
    /// loop ends on an answer.
    Paced,
    /// A wall-clock ceiling outside the common budget that only names
    /// a failure.
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
/// with a malformed one is carried in the purpose's place, so the file
/// names it as a finding.
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
/// blanked too; a lifetime's `'` is kept, since it opens nothing. A QML
/// regular expression is a literal like any other ([`regex_len`]) — the
/// `'` of a `/'/` opens no string.
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
        } else if lang == Lang::Qml && c == '/' {
            regex_len(&chars, i)
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

/// The words a value may follow, which end in a word character the way
/// a name does: after any other name the `/` of [`regex_len`] divides.
const VALUE_WORDS: [&str; 8] = [
    "return", "typeof", "case", "in", "of", "new", "delete", "void",
];

/// A JavaScript regular expression literal at `at`; 0 where the `/`
/// divides instead. One stands only where a value may begin, and closes
/// with an unescaped `/` on its own line — a `[…]` class holds a `/`
/// without one. Blanked like a string, since `/'/` opens none.
fn regex_len(chars: &[char], at: usize) -> usize {
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

/// Whether a value may begin at `at`: after a name, a number or a
/// closing bracket what follows is an operator, and after anything else
/// — a `(`, a `,`, a `:`, an operator, the start of a statement — it is
/// a value. Read across a newline it says the other thing too: a line
/// that ends where no value may begin is a statement QML has closed.
pub(super) fn opens_a_value(chars: &[char], at: usize) -> bool {
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

/// The line ranges of a Rust file's `#[cfg(test)]` blocks — a `mod`, a
/// `fn` or an `impl` — from the item's first attribute to its closing
/// brace, read off its code view. A test-only item is test code whatever
/// shape it takes, and a helper `fn` beside the production code of a
/// file is the shape a suite's fixture takes there. A `#[cfg(test)] mod
/// x;` declares a file that is read on its own ([`declared_test_modules`])
/// and opens no region here.
pub(super) fn test_regions(code: &str) -> Vec<RangeInclusive<usize>> {
    let chars: Vec<char> = code.chars().collect();
    let mut regions = Vec::new();
    for item in test_items(code, &chars) {
        if chars.get(item.after) != Some(&'{') {
            continue;
        }
        let close = matching_brace(&chars, item.after).unwrap_or(chars.len() - 1);
        regions.push(line_of(&chars, item.attribute)..=line_of(&chars, close));
    }
    regions
}

/// A module a file declares as test code, and where its file stands.
pub(super) enum Declared {
    /// `mod x;`: the file is the one that name resolves to.
    Named(String),
    /// `#[path = "…"] mod x;`: the file is the one the attribute names,
    /// relative to the declaring file's own directory.
    Path(String),
}

/// The modules a file declares as test code — `#[cfg(test)] mod x;` —
/// whose files are test code from top to bottom wherever they stand and
/// whatever they are called.
///
/// `text` is the raw source the code view was taken from, character for
/// character: a `#[path]`'s string is blanked in the view, so it is read
/// there.
pub(super) fn declared_test_modules(code: &str, text: &str) -> Vec<Declared> {
    let chars: Vec<char> = code.chars().collect();
    let raw: Vec<char> = text.chars().collect();
    test_items(code, &chars)
        .into_iter()
        .filter(|item| item.kind == Kind::Mod && chars.get(item.after) == Some(&';'))
        .map(
            |item| match module_path(&raw, item.attribute, item.keyword) {
                Some(path) => Declared::Path(path),
                None => Declared::Named(item.name),
            },
        )
        .collect()
}

const CFG: &str = "#[cfg(";

/// The item shapes a `#[cfg(test)]` opens a region on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Mod,
    Fn,
    Impl,
}

/// An item that a `#[cfg(test)]` applies to.
struct TestItem {
    kind: Kind,
    /// The item's own name, empty for an `impl`.
    name: String,
    /// The index of the item's first attribute, where its lines begin.
    attribute: usize,
    /// The index of the keyword, past the attributes and the modifiers.
    keyword: usize,
    /// The index of what follows the head — `{` for a block, `;` for a
    /// declaration.
    after: usize,
}

/// Every item that a `#[cfg(…)]` naming `test` in `code` applies to.
fn test_items(code: &str, chars: &[char]) -> Vec<TestItem> {
    let mut items = Vec::new();
    let mut from = 0;
    while let Some(found) = code[from..].find(CFG) {
        let at = from + found;
        from = at + CFG.len();
        let attribute = code[..at].chars().count();
        let open = attribute + CFG.chars().count() - 1;
        let Some(close) = matching(chars, open, '(', ')') else {
            continue;
        };
        if !cfg_names_test(chars, open) || chars.get(close + 1) != Some(&']') {
            continue;
        }
        if let Some(item) = test_item(chars, close + 2, first_attribute(chars, attribute)) {
            items.push(item);
        }
    }
    items
}

/// Whether the `#[cfg(…)]` whose `(` stands at `open` holds under
/// `cfg(test)`: `test` itself, or a `test` among the terms of an `all`
/// / `any`, however deep. A `not(test)` names the opposite, and a
/// `"test"` written as a value is blanked in the code view this reads,
/// so only the bare word is ever found.
fn cfg_names_test(chars: &[char], open: usize) -> bool {
    let Some(close) = matching(chars, open, '(', ')') else {
        return false;
    };
    // The depths at which a `not(` stands: what it holds says nothing.
    let mut nots: Vec<usize> = Vec::new();
    let mut depth = 0usize;
    let mut at = open;
    while at < close {
        let c = chars[at];
        if c == '(' {
            depth += 1;
            at += 1;
        } else if c == ')' {
            nots.retain(|d| *d < depth);
            depth -= 1;
            at += 1;
        } else if c.is_alphanumeric() || c == '_' {
            let word = word_at(chars, at);
            at += word.chars().count();
            if chars.get(past_whitespace(chars, at)) == Some(&'(') {
                if word == "not" {
                    nots.push(depth + 1);
                }
            } else if word == "test" && nots.is_empty() {
                return true;
            }
        } else {
            at += 1;
        }
    }
    false
}

/// The index of the first attribute of the item whose `#[cfg(…)]` stands
/// at `at`: attributes are written in any order, so a module's `#[path]`
/// may stand above its `#[cfg(test)]`.
fn first_attribute(chars: &[char], mut at: usize) -> usize {
    loop {
        let Some(close) = chars[..at]
            .iter()
            .rposition(|c| !c.is_whitespace())
            .filter(|i| chars[*i] == ']')
        else {
            return at;
        };
        let Some(open) = matching_back(chars, close) else {
            return at;
        };
        if open == 0 || chars[open - 1] != '#' {
            return at;
        }
        at = open - 1;
    }
}

/// The `[` that the `]` at `close` closes.
fn matching_back(chars: &[char], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for i in (0..=close).rev() {
        if chars[i] == ']' {
            depth += 1;
        } else if chars[i] == '[' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// The modifiers an item wears between its visibility and its keyword.
const MODIFIERS: [&str; 5] = ["async", "const", "default", "extern", "unsafe"];

/// The item that follows a `#[cfg(test)]`'s `]` at `at`, past whatever
/// other attributes and modifiers stand between them: its kind, its
/// name, and the index of the `{` or `;` that follows its head. `pub`
/// and `pub(…)` are stepped over — a test item is no less one for being
/// reachable.
fn test_item(chars: &[char], mut at: usize, attribute: usize) -> Option<TestItem> {
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
    while MODIFIERS.contains(&word.as_str()) {
        at = past_whitespace(chars, at + word.chars().count());
        word = word_at(chars, at);
    }
    let kind = match word.as_str() {
        "mod" => Kind::Mod,
        "fn" => Kind::Fn,
        "impl" => Kind::Impl,
        _ => return None,
    };
    let keyword = at;
    at = past_whitespace(chars, at + word.chars().count());
    let name = if kind == Kind::Impl {
        String::new()
    } else {
        word_at(chars, at)
    };
    if kind != Kind::Impl && name.is_empty() {
        return None;
    }
    // A `mod`'s head ends at its name; the rest carry a signature, whose
    // own `;` — an array's length — closes nothing.
    let after = if kind == Kind::Mod {
        past_whitespace(chars, at + name.chars().count())
    } else {
        head_end(chars, at)
    };
    Some(TestItem {
        kind,
        name,
        attribute,
        keyword,
        after,
    })
}

/// Where the head that runs from `at` ends: the `{` that opens the body,
/// or the `;` that stands in place of one, past whatever brackets the
/// signature holds.
fn head_end(chars: &[char], at: usize) -> usize {
    let mut depth = 0usize;
    for (i, c) in chars.iter().enumerate().skip(at) {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '{' | ';' if depth == 0 => return i,
            _ => {}
        }
    }
    chars.len()
}

/// The file a `#[path = "…"]` among the attributes in `raw[from..to]`
/// names. Read off the raw text, which the code view stands in step
/// with character for character — in the view the string is blanked.
fn module_path(raw: &[char], from: usize, to: usize) -> Option<String> {
    let head = raw.get(from..to)?;
    for at in 0..head.len() {
        if word_before(head, at) || word_at(head, at) != "path" {
            continue;
        }
        // Inside an attribute of its own and nowhere else: a comment
        // between the attributes says `path` as freely as any prose.
        if !head[..at]
            .iter()
            .rposition(|c| !c.is_whitespace())
            .is_some_and(|i| head[i] == '[' && i > 0 && head[i - 1] == '#')
        {
            continue;
        }
        let mut i = past_whitespace(head, at + 4);
        if head.get(i) != Some(&'=') {
            continue;
        }
        i = past_whitespace(head, i + 1);
        if head.get(i) != Some(&'"') {
            continue;
        }
        let mut path = String::new();
        i += 1;
        while let Some(c) = head.get(i) {
            match c {
                '"' => return Some(path),
                '\\' => {
                    path.extend(head.get(i + 1));
                    i += 2;
                }
                _ => {
                    path.push(*c);
                    i += 1;
                }
            }
        }
        return None;
    }
    None
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

/// Whether `token` occurs in `haystack` on a word boundary at each end
/// it has one: `timeout(` is its own token apart from `no_timeout(`,
/// `bounded(` from `unbounded(`, `Instant::now` from
/// `Instant::nowhere`. An end that is not a word byte (`.outcome()`,
/// `sleep(`) guards nothing and matches anywhere — which is what lets a
/// name be looked for however the call spells it, `Instant::now()` and
/// the `Instant::now` a `get_or_init` is handed alike.
pub(super) fn has_token(haystack: &str, token: &str) -> bool {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = token.as_bytes();
    let opens = bytes.first().copied().is_some_and(word);
    let closes = bytes.last().copied().is_some_and(word);
    let mut from = 0;
    while let Some(found) = haystack[from..].find(token) {
        let at = from + found;
        let end = at + token.len();
        let before = !opens || at == 0 || !word(haystack.as_bytes()[at - 1]);
        let after = !closes || haystack.as_bytes().get(end).is_none_or(|b| !word(*b));
        if before && after {
            return true;
        }
        from = end;
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
    use super::{
        Declared, Lang, Purpose, code_view, declared_test_modules, has_token, markers, test_regions,
    };

    /// A token is guarded at each end that has a word byte on it, and
    /// nowhere else: what the far guard buys is a name found however the
    /// call spells it, without `Instant::nowhere` answering to it.
    #[test]
    fn a_token_is_bounded_at_the_ends_it_has_words_on() {
        assert!(has_token("let t = Instant::now();", "Instant::now"));
        assert!(has_token(
            "*CELL.get_or_init(Instant::now);",
            "Instant::now"
        ));
        assert!(!has_token("let t = Instant::nowhere();", "Instant::now"));
        assert!(has_token("tokio::time::timeout(BUDGET, x)", "timeout("));
        assert!(!has_token("no_timeout(x)", "timeout("));
        assert!(has_token("let a = task.outcome().await;", ".outcome()"));
    }

    /// The declared modules of `text`, each as the file it names or the
    /// name it is known by.
    fn declared(text: &str) -> Vec<String> {
        declared_test_modules(&code_view(text, Lang::Rust), text)
            .into_iter()
            .map(|module| match module {
                Declared::Named(name) => name,
                Declared::Path(path) => format!("path {path}"),
            })
            .collect()
    }

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
        assert_eq!(declared(text), vec!["testkit".to_string()]);
        assert_eq!(
            test_regions(&code_view(text, Lang::Rust)),
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

    #[test]
    fn a_cfg_holds_under_test_when_test_is_one_of_its_terms_and_never_under_a_not() {
        let text = "\
#[cfg(all(test, target_os = \"linux\"))]
fn handle() {
    let a = 1;
}
#[cfg(any(test, feature = \"kit\"))]
mod kit {
    fn t() {}
}
#[cfg(not(test))]
fn production() {
    let b = 2;
}
#[cfg(all(unix, not(test)))]
fn on_unix() {
    let c = 3;
}
#[cfg(feature = \"test\")]
fn behind_a_feature() {
    let d = 4;
}
";
        assert_eq!(
            test_regions(&code_view(text, Lang::Rust)),
            vec![1..=4, 5..=8],
            "a `test` among the terms opens a region; a negated or quoted one does not"
        );
    }

    #[test]
    fn a_test_only_fn_or_impl_is_a_region_and_a_use_is_not() {
        let text = "\
fn production() {
    let a = 1;
}
#[cfg(test)]
pub(crate) async fn helper() -> [u8; 2] {
    [1, 2]
}
#[cfg(test)]
impl<'a> Fixture<'a> {
    fn new() {}
}
#[cfg(test)]
use std::io;
";
        assert_eq!(
            test_regions(&code_view(text, Lang::Rust)),
            vec![4..=7, 8..=11],
            "the `;` of an array in the signature closes no head, and a `use` opens no block"
        );
    }

    #[test]
    fn a_modules_own_path_is_read_off_the_raw_text_in_either_order() {
        let text = "\
#[cfg(test)]
#[path = \"avatar_tests.rs\"]
mod tests;
#[path = \"kit/other.rs\"]
#[cfg(test)]
mod more;
#[cfg(test)]
// a note whose path = \"elsewhere.rs\" is prose and no attribute
mod plain;
#[cfg(test)]
mod tests_in_a_block {
    fn t() {}
}
";
        assert_eq!(
            declared(text),
            vec![
                "path avatar_tests.rs".to_string(),
                "path kit/other.rs".to_string(),
                "plain".to_string()
            ],
            "a block declares no file, and only a `#[path]` attribute names one"
        );
    }

    #[test]
    fn a_qml_regex_is_a_literal_and_a_division_is_not() {
        let text = "\
var re = /'/;
var half = width / 2; var s = 'a; b';
";
        let code = code_view(text, Lang::Qml);
        assert!(
            !code.contains('\''),
            "the regex opens no string, so the line after it is code: {code}"
        );
        assert!(
            code.contains("width / 2"),
            "a division is no literal: {code}"
        );
        assert_eq!(code.lines().count(), 2);
        assert_eq!(
            code.matches(';').count(),
            3,
            "the `;` inside the string is nobody's: {code}"
        );
    }

    #[test]
    fn a_regex_stands_where_a_value_may_begin_and_closes_on_its_own_line() {
        // A `;` inside the literal is blanked with it and stands where
        // the `/` only divides: what it counts is which reading won.
        let semicolons = |text: &str| code_view(text, Lang::Qml).matches(';').count();
        assert_eq!(semicolons("var re = /a;b/;"), 1, "a value follows a `=`");
        assert_eq!(semicolons("return /a;b/.test(s);"), 1, "and a `return`");
        assert_eq!(semicolons("f(x, /a;b/);"), 1, "and a comma");
        assert_eq!(
            semicolons("var x = a / b; var y = c / d;"),
            2,
            "after a name the `/` divides"
        );
        assert_eq!(
            semicolons("var x = f() / b; var y = c / d;"),
            2,
            "after a `)` it divides too"
        );
        assert_eq!(
            semicolons("f(x, /a;\n b/);"),
            2,
            "a literal closes on its own line or opens nothing"
        );
    }
}
