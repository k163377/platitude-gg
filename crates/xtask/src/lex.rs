//! The lexical layer of a Rust or QML file — where its comments, its
//! literals and its code stand — read here, once, for every tool that
//! reads sources without reading them for meaning: `cargo xtask waits`
//! blanks the comments and the literals (`waits::source`), the gate keys a
//! file's code on what stands when the comments and the spacing go with
//! the literals kept to the character ([`normalized`], `gate::inputs`), and
//! the harness rules read the code as tokens ([`tokens`]).
//!
//! What is read is what the language's own lexer reads, and no more:
//! Rust's `//` and nested `/* */`, its `"…"`, `r#"…"#` (under `b` or `c`
//! too) and `'x'` (a lifetime's tick opens nothing); QML's `//` and
//! `/* */`, its `"…"` and `'…'`, a template literal with each `${…}` read
//! as the code it is (a template inside it included), and a regular
//! expression where a value may begin ([`opens_a_value`]).
//!
//! What this reading cannot settle it says ([`Unread`]) rather than
//! guesses: a literal, a comment or a template's expression left open at
//! the end of the file, and a `/` after a `}` that reads as a division or
//! as a regular expression. The two readers lean opposite ways on that —
//! the gate keys such a file on its bytes, so no two inputs share a key;
//! the waits check names the place, so no code hides behind it — which is
//! why the reading is handed over as it is, not as one lenient view.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lang {
    Rust,
    Qml,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `//` to the end of its line, or `/* … */`.
    Comment,
    /// A string, a Rust char, a QML regular expression, or a piece of a
    /// template literal's text (with the `` ` ``, `${` and `}` that
    /// bound it).
    Literal,
    Code,
}

/// One stretch of a file, as character indices (`end` exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Lexeme {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

/// Where the reading stopped, and why. The lexemes before it stand; from
/// `at` on, nothing is said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Unread {
    /// The character index of what could not be read.
    pub at: usize,
    pub what: &'static str,
}

/// `chars` cut into comments, literals and the code between them, in
/// order and without a gap, up to where the reading stops ([`Unread`]).
pub(crate) fn lexemes(chars: &[char], lang: Lang) -> (Vec<Lexeme>, Option<Unread>) {
    let mut out = Vec::new();
    match lex_until(chars, 0, lang, Stop::End, &mut out) {
        Ok(_) => (out, None),
        Err(unread) => (out, Some(unread)),
    }
}

/// Where one reading ends: the file, or the `}` closing a template's
/// `${…}`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    End,
    Brace,
}

/// Reads from `from` to `stop`, pushing lexemes, and answers with the
/// index the reading ended at: the file's end, or the closing `}`.
fn lex_until(
    chars: &[char],
    from: usize,
    lang: Lang,
    stop: Stop,
    out: &mut Vec<Lexeme>,
) -> Result<usize, Unread> {
    let mut code_from = from;
    let mut depth = 0;
    let mut i = from;
    let close_code = |out: &mut Vec<Lexeme>, code_from: usize, at: usize| {
        if code_from < at {
            out.push(Lexeme {
                kind: Kind::Code,
                start: code_from,
                end: at,
            });
        }
    };
    while i < chars.len() {
        let c = chars[i];
        if stop == Stop::Brace {
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                if depth == 0 {
                    close_code(out, code_from, i);
                    return Ok(i);
                }
                depth -= 1;
            }
        }
        if lang == Lang::Qml && c == '`' {
            close_code(out, code_from, i);
            i = template(chars, i, lang, out)?;
            code_from = i;
            continue;
        }
        let (kind, len) = piece(chars, i, lang)?;
        if len == 0 {
            i += 1;
            continue;
        }
        close_code(out, code_from, i);
        out.push(Lexeme {
            kind,
            start: i,
            end: i + len,
        });
        i += len;
        code_from = i;
    }
    match stop {
        Stop::End => {
            close_code(out, code_from, chars.len());
            Ok(chars.len())
        }
        Stop::Brace => Err(Unread {
            at: from,
            what: "a template's `${…}` left open",
        }),
    }
}

/// A template literal from its opening backtick: its text in pieces, each
/// `${…}` read as code between them, to one past the closing backtick.
fn template(chars: &[char], at: usize, lang: Lang, out: &mut Vec<Lexeme>) -> Result<usize, Unread> {
    let mut piece_from = at;
    let mut i = at + 1;
    loop {
        match chars.get(i) {
            None => {
                return Err(Unread {
                    at,
                    what: "a template literal left open",
                });
            }
            Some('\\') => i += 2,
            Some('`') => {
                out.push(Lexeme {
                    kind: Kind::Literal,
                    start: piece_from,
                    end: i + 1,
                });
                return Ok(i + 1);
            }
            Some('$') if chars.get(i + 1) == Some(&'{') => {
                out.push(Lexeme {
                    kind: Kind::Literal,
                    start: piece_from,
                    end: i + 2,
                });
                let close = lex_until(chars, i + 2, lang, Stop::Brace, out)?;
                piece_from = close;
                i = close + 1;
            }
            Some(_) => i += 1,
        }
    }
}

/// The comment or literal opening at `at`, as its kind and its length; a
/// length of 0 where code stands.
fn piece(chars: &[char], at: usize, lang: Lang) -> Result<(Kind, usize), Unread> {
    let c = chars[at];
    let next = chars.get(at + 1).copied();
    if c == '/' && next == Some('/') {
        return Ok((Kind::Comment, line_comment_len(chars, at)));
    }
    if c == '/' && next == Some('*') {
        return block_comment_len(chars, at, lang).map(|len| (Kind::Comment, len));
    }
    let len = match lang {
        Lang::Qml if c == '/' => regex_len(chars, at)?,
        Lang::Qml if matches!(c, '"' | '\'') => quoted_len(chars, at, c)?,
        Lang::Rust if c == '"' => quoted_len(chars, at, c)?,
        Lang::Rust if c == 'r' && !word_before(chars, at) => raw_string_len(chars, at)?,
        Lang::Rust if matches!(c, 'b' | 'c') && next == Some('r') && !word_before(chars, at) => {
            // `br"…"` / `cr"…"`: the raw string under its byte or C prefix.
            match raw_string_len(chars, at + 1)? {
                0 => 0,
                raw => raw + 1,
            }
        }
        Lang::Rust if c == '\'' => char_literal_len(chars, at),
        _ => 0,
    };
    Ok((Kind::Literal, len))
}

/// Whether a word character stands right before `at`.
pub(crate) fn word_before(chars: &[char], at: usize) -> bool {
    at > 0 && (chars[at - 1].is_alphanumeric() || chars[at - 1] == '_')
}

fn line_comment_len(chars: &[char], at: usize) -> usize {
    chars[at..].iter().take_while(|c| **c != '\n').count()
}

/// A block comment, nested where the language nests them (Rust does, JS
/// does not). One left open is unread.
fn block_comment_len(chars: &[char], at: usize, lang: Lang) -> Result<usize, Unread> {
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
    if depth > 0 {
        return Err(Unread {
            at,
            what: "a block comment left open",
        });
    }
    Ok(i - at)
}

/// A literal under `quote`, escapes skipped. One left open is unread.
fn quoted_len(chars: &[char], at: usize, quote: char) -> Result<usize, Unread> {
    let mut i = at + 1;
    while i < chars.len() && chars[i] != quote {
        i += if chars[i] == '\\' { 2 } else { 1 };
    }
    if i >= chars.len() {
        return Err(Unread {
            at,
            what: "a string left open",
        });
    }
    Ok(i + 1 - at)
}

/// `r"..."` or `r#"..."#` with any number of hashes; 0 where `r` opens no
/// raw string here. One left open is unread.
fn raw_string_len(chars: &[char], at: usize) -> Result<usize, Unread> {
    let hashes = chars[at + 1..].iter().take_while(|c| **c == '#').count();
    if chars.get(at + 1 + hashes) != Some(&'"') {
        return Ok(0);
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
            return Ok(i + 1 + hashes - at);
        }
        i += 1;
    }
    Err(Unread {
        at,
        what: "a raw string left open",
    })
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
const VALUE_WORDS: [&str; 14] = [
    "return",
    "typeof",
    "case",
    "in",
    "of",
    "new",
    "delete",
    "void",
    "else",
    "do",
    "throw",
    "await",
    "yield",
    "instanceof",
];

/// The words whose parenthesis a statement follows: after its `)` a value
/// may begin, where after any other `)` what follows is an operator.
const CONDITION_WORDS: [&str; 4] = ["if", "while", "for", "with"];

/// A JavaScript regular expression literal at `at`; 0 where the `/`
/// divides instead. One stands only where a value may begin, and closes
/// with an unescaped `/` outside a `[…]` class, on its own line. After a
/// `}` either reading is possible — a block ended, or an object literal
/// divided — so a `/` there that could close as a regular expression is
/// unread.
fn regex_len(chars: &[char], at: usize) -> Result<usize, Unread> {
    if !opens_a_value(chars, at) {
        return Ok(0);
    }
    let mut i = at + 1;
    let mut class = false;
    while let Some(c) = chars.get(i) {
        match c {
            '\n' => return Ok(0),
            '\\' if chars.get(i + 1).is_some_and(|next| *next != '\n') => i += 1,
            '[' => class = true,
            ']' => class = false,
            '/' if !class => {
                if after_a_brace(chars, at) {
                    return Err(Unread {
                        at,
                        what: "a `/` after a `}`: a regular expression, or a division",
                    });
                }
                return Ok(i + 1 - at);
            }
            _ => {}
        }
        i += 1;
    }
    Ok(0)
}

/// Whether the last character before `at` that is not spacing is a `}`.
fn after_a_brace(chars: &[char], at: usize) -> bool {
    chars[..at]
        .iter()
        .rposition(|c| !c.is_whitespace())
        .is_some_and(|before| chars[before] == '}')
}

/// Whether a value may begin at `at`: after a name (other than
/// [`VALUE_WORDS`]), a number, a closing bracket or a quote what follows
/// is an operator — except after the `)` that closes a condition
/// ([`CONDITION_WORDS`]), where a statement begins — and after anything
/// else a value.
pub(crate) fn opens_a_value(chars: &[char], at: usize) -> bool {
    let Some(before) = chars[..at].iter().rposition(|c| !c.is_whitespace()) else {
        return true;
    };
    let c = chars[before];
    if c == ')' {
        return closes_a_condition(chars, before);
    }
    if matches!(c, ']' | '"' | '\'' | '`') {
        return false;
    }
    if !(c.is_alphanumeric() || c == '_') {
        return true;
    }
    VALUE_WORDS.contains(&word_ending_at(chars, before).as_str())
}

/// Whether a statement begins at `at`: after a `;`, a brace, the start
/// of the file, the `)` closing a condition ([`CONDITION_WORDS`]), an
/// `else` or a `do`, or a line that ended on a value (QML supplies the
/// `;`). Not the same question as [`opens_a_value`]: after `else` or a
/// condition's `)` both a value and a statement may begin, and after a
/// line that ended on an operator neither does.
pub(crate) fn opens_a_statement(chars: &[char], at: usize) -> bool {
    let Some(before) = chars[..at].iter().rposition(|c| !c.is_whitespace()) else {
        return true;
    };
    let c = chars[before];
    if matches!(c, ';' | '{' | '}') {
        return true;
    }
    if c == ')' && closes_a_condition(chars, before) {
        return true;
    }
    if (c.is_alphanumeric() || c == '_')
        && matches!(word_ending_at(chars, before).as_str(), "else" | "do")
    {
        return true;
    }
    chars[before + 1..at].contains(&'\n') && !opens_a_value(chars, at)
}

/// Whether the `)` at `close` closes the parenthesis of a
/// [`CONDITION_WORDS`] word.
fn closes_a_condition(chars: &[char], close: usize) -> bool {
    let mut depth = 0;
    let mut i = close;
    loop {
        match chars[i] {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        if i == 0 {
            return false;
        }
        i -= 1;
    }
    let Some(before) = chars[..i].iter().rposition(|c| !c.is_whitespace()) else {
        return false;
    };
    (chars[before].is_alphanumeric() || chars[before] == '_')
        && CONDITION_WORDS.contains(&word_ending_at(chars, before).as_str())
}

/// The word whose last character is at `last`.
fn word_ending_at(chars: &[char], last: usize) -> String {
    let start = chars[..last]
        .iter()
        .rposition(|c| !(c.is_alphanumeric() || *c == '_'))
        .map_or(0, |i| i + 1);
    chars[start..=last].iter().collect()
}

/// `text` as the gate keys it: the comments gone, every literal kept to
/// the character, and the code between them with each run of spacing
/// made one newline where it held one and one space where it did not —
/// a token never joins its neighbour, and a line end that may end a
/// statement (QML supplies the `;`) stays one. A comment spanning lines
/// is such a line end too. Spacing beside a literal, which its own
/// delimiter keeps apart from the code, goes with the rest of the spacing
/// (a `${…}` written tight or loose is one expression), and nothing of the
/// leading or trailing spacing stays. `None` where the reading stopped
/// short ([`Unread`]): the gate then keys the file on its bytes.
pub(crate) fn normalized(text: &str, lang: Lang) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let (lexemes, unread) = lexemes(&chars, lang);
    if unread.is_some() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut gap: Option<char> = None;
    let mut after_literal = false;
    for lexeme in lexemes {
        let span = &chars[lexeme.start..lexeme.end];
        match lexeme.kind {
            Kind::Comment => widen(&mut gap, span.contains(&'\n')),
            Kind::Literal => {
                close(&mut out, &mut gap, true);
                out.extend(span);
                after_literal = true;
            }
            Kind::Code => {
                for c in span {
                    if c.is_whitespace() {
                        widen(&mut gap, *c == '\n');
                    } else {
                        close(&mut out, &mut gap, after_literal);
                        out.push(*c);
                        after_literal = false;
                    }
                }
            }
        }
    }
    Some(out)
}

/// A gap that held a newline stays one whatever comes after.
fn widen(gap: &mut Option<char>, newline: bool) {
    if newline || gap.is_none() {
        *gap = Some(if newline { '\n' } else { ' ' });
    }
}

/// Writes the gap out before the next piece: a newline always (a line end
/// may end a statement), a space only between two pieces of code — a
/// literal's delimiter keeps it apart on its own (`beside_literal`).
fn close(out: &mut String, gap: &mut Option<char>, beside_literal: bool) {
    if let Some(c) = gap.take()
        && !out.is_empty()
        && (c == '\n' || !beside_literal)
    {
        out.push(c);
    }
}

/// `text` with every comment blanked to spaces, line for line, and every
/// literal too unless `keep_literals` — the code inside a template's
/// `${…}` is code and stays. Where the reading stopped short the view is
/// blank from there, and the [`Unread`] says so: nothing there was read.
pub(crate) fn blanked(text: &str, lang: Lang, keep_literals: bool) -> (String, Option<Unread>) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let (lexemes, unread) = lexemes(&chars, lang);
    let blank = |out: &mut String, span: &[char]| {
        out.extend(span.iter().map(|c| if *c == '\n' { '\n' } else { ' ' }));
    };
    let mut read_to = 0;
    for lexeme in lexemes {
        let span = &chars[lexeme.start..lexeme.end];
        match lexeme.kind {
            Kind::Code => out.extend(span),
            Kind::Literal if keep_literals => out.extend(span),
            _ => blank(&mut out, span),
        }
        read_to = lexeme.end;
    }
    blank(&mut out, &chars[read_to..]);
    (out, unread)
}

/// One token of a code view, with the line it stands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub text: String,
    pub line: usize,
}

/// The longest operators first: a shorter one is read only where no
/// longer one stands.
const OPERATORS: [&str; 19] = [
    "===", "!==", "...", "=>", "==", "!=", "<=", ">=", "++", "--", "+=", "-=", "*=", "/=", "&&",
    "||", "??", "::", "->",
];

/// The tokens of `text` in `lang`: names (a `.`-joined path is several),
/// numbers, operators or punctuation one at a time, and each literal as
/// one token of its own text — a string, a regular expression, a piece of
/// a template with the `` ` ``, `${` and `}` that bound it — so that none
/// of them reads as a name, a keyword or a bracket, and a line closing on
/// one closes as a line closing on a name does. Comments and spacing are
/// no token. Where the lexical reading stopped ([`lexemes`]) the tokens
/// stop with it: what stands past that is not read.
pub(crate) fn tokens(text: &str, lang: Lang) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let (lexemes, _) = lexemes(&chars, lang);
    let mut out = Vec::new();
    let mut line = 1;
    for lexeme in lexemes {
        let span = &chars[lexeme.start..lexeme.end];
        match lexeme.kind {
            Kind::Code => cut_code(span, &mut line, &mut out),
            Kind::Literal => {
                out.push(Token {
                    text: span.iter().collect(),
                    line,
                });
                line += span.iter().filter(|c| **c == '\n').count();
            }
            Kind::Comment => line += span.iter().filter(|c| **c == '\n').count(),
        }
    }
    out
}

/// The tokens of a stretch of code, `line` carried across it.
fn cut_code(chars: &[char], line: &mut usize, out: &mut Vec<Token>) {
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            *line += 1;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
        let len = if word(c) {
            chars[i..].iter().take_while(|c| word(**c)).count()
        } else {
            OPERATORS
                .iter()
                .find(|op| {
                    chars[i..]
                        .iter()
                        .zip(op.chars())
                        .filter(|(a, b)| **a == *b)
                        .count()
                        == op.len()
                })
                .map_or(1, |op| op.len())
        };
        out.push(Token {
            text: chars[i..i + len].iter().collect(),
            line: *line,
        });
        i += len;
    }
}

/// The index of the `close` matching the `open` at `at`.
pub(crate) fn matching_close(
    tokens: &[crate::lex::Token],
    at: usize,
    open: &str,
    close: &str,
) -> Option<usize> {
    let mut depth = 0;
    for (k, token) in tokens.iter().enumerate().skip(at) {
        if token.text == open {
            depth += 1;
        } else if token.text == close {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        Kind, Lang, Unread, lexemes, normalized, opens_a_statement, opens_a_value, tokens,
    };

    fn kinds(text: &str, lang: Lang) -> (Vec<(Kind, String)>, Option<Unread>) {
        let chars: Vec<char> = text.chars().collect();
        let (lexemes, unread) = lexemes(&chars, lang);
        (
            lexemes
                .into_iter()
                .map(|l| (l.kind, chars[l.start..l.end].iter().collect()))
                .collect(),
            unread,
        )
    }

    fn of_kind(cut: &[(Kind, String)], kind: Kind) -> Vec<&str> {
        cut.iter()
            .filter(|(k, _)| *k == kind)
            .map(|(_, s)| s.as_str())
            .collect()
    }

    #[test]
    fn rust_is_cut_at_its_comments_strings_chars_and_lifetimes() {
        let text = "let s = r#\"a // b\"#; let c = '/'; fn f<'a>(x: &'a str) {} // gone\n/* x /* y */ z */ b\"q\"";
        let (cut, unread) = kinds(text, Lang::Rust);
        assert_eq!(unread, None);
        assert_eq!(
            of_kind(&cut, Kind::Literal),
            ["r#\"a // b\"#", "'/'", "\"q\""]
        );
        assert_eq!(
            of_kind(&cut, Kind::Comment),
            ["// gone", "/* x /* y */ z */"]
        );
        let code: String = of_kind(&cut, Kind::Code).concat();
        assert!(code.contains("<'a>(x: &'a str)"), "{code}");
    }

    #[test]
    fn qml_is_cut_at_its_quotes_and_at_a_regex_but_not_at_a_division() {
        let text = "x: 'a // b' + \"e\"\nvar r = /\\/* not a comment/; var y = a / b / c\n";
        let (cut, unread) = kinds(text, Lang::Qml);
        assert_eq!(unread, None);
        assert_eq!(
            of_kind(&cut, Kind::Literal),
            ["'a // b'", "\"e\"", "/\\/* not a comment/"]
        );
        assert!(cut.iter().all(|(k, _)| *k != Kind::Comment), "{cut:?}");
    }

    /// A template's text is literal to the character, and each `${…}`
    /// is the code it is — a comment in it a comment, a string in it a
    /// string (its `}` closing nothing), a template in it a template.
    #[test]
    fn a_template_literal_is_text_in_pieces_around_the_code_it_holds() {
        let text = "t: `a ${ x /* c */ + \"}\" }  b ${ `in ${ y } ner` } c`";
        let (cut, unread) = kinds(text, Lang::Qml);
        assert_eq!(unread, None);
        assert_eq!(
            of_kind(&cut, Kind::Literal),
            ["`a ${", "\"}\"", "}  b ${", "`in ${", "} ner`", "} c`"]
        );
        assert_eq!(of_kind(&cut, Kind::Comment), ["/* c */"]);
        let code: String = of_kind(&cut, Kind::Code).concat();
        assert!(
            code.contains(" x ") && code.contains(" + ") && code.contains(" y "),
            "{code}"
        );
        assert!(!code.contains("ner"), "the text is no code: {code}");
    }

    /// What cannot be read is said, with where, and nothing is guessed
    /// past it.
    #[test]
    fn what_is_left_open_is_unread_at_its_place() {
        for (text, lang, what) in [
            ("x: `a ${ y } b", Lang::Qml, "a template literal left open"),
            ("x: `a ${ y ", Lang::Qml, "a template's `${…}` left open"),
            ("x: \"abc\n", Lang::Qml, "a string left open"),
            (
                "let a = 1; /* open",
                Lang::Rust,
                "a block comment left open",
            ),
            ("let s = r#\"open\";", Lang::Rust, "a raw string left open"),
        ] {
            let (_, unread) = kinds(text, lang);
            assert_eq!(unread.map(|u| u.what), Some(what), "{text}");
            assert_eq!(normalized(text, lang), None, "{text}");
        }
        let (cut, unread) = kinds("a = 1\n} /x/", Lang::Qml);
        assert!(
            unread.is_some_and(|u| u.what.starts_with("a `/` after a `}`")),
            "{cut:?}"
        );
        assert_eq!(
            kinds("a = {} / 2", Lang::Qml).1,
            None,
            "no `/` closes it: a division"
        );
    }

    #[test]
    fn a_value_begins_after_a_conditions_close_and_an_operator_after_a_calls() {
        let begins = |text: &str| {
            let chars: Vec<char> = text.chars().collect();
            opens_a_value(&chars, chars.len() - 1)
        };
        assert!(begins("if (ok) /"), "a statement follows a condition");
        assert!(begins("while (ok) /"));
        assert!(!begins("f(x) /"), "an operator follows a call");
        assert!(!begins("(a + b) /"));
        assert!(begins("return /"));
        assert!(!begins("x /"));
    }

    #[test]
    fn the_normalized_text_lets_comments_and_spacing_go_and_keeps_every_literal() {
        let code = "fn a() {\n    let x = 1;\n}\n";
        let commented =
            "// a doc\nfn a() {\n\n    let x = 1; // one\n    /* block\n       comment */\n}   \n";
        assert_eq!(
            normalized(code, Lang::Rust),
            normalized(commented, Lang::Rust)
        );
        assert_eq!(
            normalized(code, Lang::Rust).as_deref(),
            Some("fn a() {\nlet x = 1;\n}")
        );
        assert_eq!(
            normalized("a/*c*/b", Lang::Rust).as_deref(),
            Some("a b"),
            "a comment keeps the tokens on either side apart"
        );
        // A blank line, or a trailing space, inside a literal is the literal's — a template's text too, while
        // the spacing of the code inside its `${…}` is not.
        let one = "let s = r\"a\n\nb\";";
        let other = "let s = r\"a\nb\";";
        assert_ne!(normalized(one, Lang::Rust), normalized(other, Lang::Rust));
        let spaced = "x: `a \n b ${ y }`";
        let tight = "x: `a\n b ${ y }`";
        assert_ne!(normalized(spaced, Lang::Qml), normalized(tight, Lang::Qml));
        assert_eq!(
            normalized("x: `a ${  y  }`", Lang::Qml),
            normalized("x: `a ${y}`", Lang::Qml)
        );
        assert_eq!(
            normalized("let s = \"a\" + b;", Lang::Rust),
            normalized("let s =\"a\"+ b;", Lang::Rust),
            "a literal's delimiter keeps it apart from the code"
        );
        assert_ne!(
            normalized("x = \"a\"\ny", Lang::Qml),
            normalized("x = \"a\" y", Lang::Qml),
            "but a line end after it may end the statement"
        );
        // A line end may end a statement in QML, so it stays one.
        assert_ne!(
            normalized("return\nx", Lang::Qml),
            normalized("return x", Lang::Qml)
        );
        assert_eq!(
            normalized("return\n\n\nx", Lang::Qml).as_deref(),
            Some("return\nx")
        );
        // A `//` inside a literal is the literal's, and what follows a regex is code.
        let regex = "var r = /\\/* a/; var y = 1 // two\n";
        assert_eq!(
            normalized(regex, Lang::Qml).as_deref(),
            Some("var r =/\\/* a/; var y = 1")
        );
        assert_eq!(
            normalized("let u = \"https://x\"; // gone", Lang::Rust).as_deref(),
            Some("let u =\"https://x\";")
        );
    }

    #[test]
    fn the_blanked_view_keeps_lines_and_the_code_inside_a_template_and_blanks_the_rest() {
        let text = "a: 'x; {' // c\nb: `${ y } }`\nc: \"d";
        let (view, unread) = super::blanked(text, Lang::Qml, false);
        assert_eq!(view.lines().count(), 3);
        assert!(!view.contains('{') && !view.contains("c"), "{view:?}");
        assert!(view.contains(" y "), "the expression is code: {view:?}");
        assert!(unread.is_some_and(|u| u.what == "a string left open"));
        assert_eq!(
            view.lines().last(),
            Some("     "),
            "nothing past the stop is read"
        );
        let (kept, _) = super::blanked("a: 'x' // c", Lang::Qml, true);
        assert_eq!(kept, "a: 'x'     ");
    }

    /// A literal is one token of its own text, a template's pieces one each with the code between them cut as
    /// code; a comment is none; the lines are the file's.
    /// A statement's head is read where the ignored rule needs it: under a bare condition or an `else`, on the
    /// condition's own line, after a line that ended on a value — and not where a line goes on.
    #[test]
    fn a_statement_begins_after_a_condition_an_else_or_a_line_that_ended_on_a_value() {
        let heads = |text: &str| {
            let chars: Vec<char> = text.chars().collect();
            opens_a_statement(&chars, text.find("go").expect("go"))
        };
        assert!(heads("if (ok)\n    go()"));
        assert!(heads("if (ok) go()"));
        assert!(heads("while (f(x))\n    go()"));
        assert!(heads("else\n    go()"));
        assert!(heads("do go()"));
        assert!(heads("x = 1\ngo()"));
        assert!(heads("{ go()"));
        assert!(heads("go()"));
        assert!(!heads("f(a)\n    .go()"));
        assert!(!heads("a +\n    go()"));
        assert!(!heads("f(a)\n    + go()"));
        assert!(!heads("f(a) go()"));
    }

    #[test]
    fn tokens_are_names_numbers_operators_and_literals_with_their_lines() {
        let text =
            "acts.beats++; // a comment\nif (x <= 10) a = b + 1\nc = 'y' + `t ${z}\n` + \"w\"";
        let read: Vec<(String, usize)> = tokens(text, Lang::Qml)
            .into_iter()
            .map(|t| (t.text, t.line))
            .collect();
        let texts: Vec<&str> = read.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(
            texts,
            [
                "acts", ".", "beats", "++", ";", "if", "(", "x", "<=", "10", ")", "a", "=", "b",
                "+", "1", "c", "=", "'y'", "+", "`t ${", "z", "}\n`", "+", "\"w\""
            ]
        );
        assert_eq!(read[3].1, 1);
        assert_eq!(read[5].1, 2);
        assert_eq!(read[16].1, 3, "the line after a comment");
        assert_eq!(
            read[22].1, 3,
            "a template's last piece stands where it begins"
        );
        assert_eq!(read[24].1, 4, "the line after a template that spans two");
        let short: Vec<String> = tokens("a = 1 // c\nb = \"open", Lang::Qml)
            .into_iter()
            .map(|t| t.text)
            .collect();
        assert_eq!(
            short,
            ["a", "=", "1"],
            "the stretch the stop is in is no token"
        );
    }
}
