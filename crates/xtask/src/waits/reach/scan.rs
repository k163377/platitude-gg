//! What a stretch of code does, read off its tokens before any name in it
//! is resolved: the names it declares, and the paths it reads, writes and
//! calls, each with the token it stands at — so that a stretch carved out
//! of it later (an argument read in a context of its own) takes its own
//! facts with it ([`Facts::within`]).

use std::ops::Range;

use super::parse::{expression_end, is_name, params};
use crate::lex::{Token, matching_close};

/// Where a path starts: a name, or the value of an expression before it
/// (a call's answer, an index, a literal).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Head {
    Name(String),
    Value,
}

#[derive(Clone, Debug)]
pub(super) struct Path {
    pub head: Head,
    /// The names after the head, in order.
    pub members: Vec<String>,
    /// The token the path starts at.
    pub at: usize,
}

impl Path {
    /// The path as written: `a.b.c`, or `….c` from a value.
    pub fn spelled(&self) -> String {
        let head = match &self.head {
            Head::Name(name) => name.clone(),
            Head::Value => "…".to_string(),
        };
        std::iter::once(head)
            .chain(self.members.iter().cloned())
            .collect::<Vec<_>>()
            .join(".")
    }
    /// The path without its last member: a method's receiver.
    pub fn receiver(&self) -> Option<Path> {
        if self.members.is_empty() {
            return None;
        }
        Some(Path {
            head: self.head.clone(),
            members: self.members[..self.members.len() - 1].to_vec(),
            at: self.at,
        })
    }
    pub fn last(&self) -> Option<&str> {
        match (&self.head, self.members.last()) {
            (_, Some(last)) => Some(last),
            (Head::Name(name), None) => Some(name),
            (Head::Value, None) => None,
        }
    }
}

pub(super) struct Call {
    pub path: Path,
    /// Each argument's tokens.
    pub args: Vec<Range<usize>>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WriteKind {
    /// Given a value: its tokens.
    Value(Range<usize>),
    /// Given a literal.
    Constant,
    /// Stepped: `++`, `--`, `+=`, `x = x + …`.
    Step,
    /// Changed in place: an element set, a `push`, a `delete`.
    Mutate,
}

pub(super) struct Write {
    pub path: Path,
    pub kind: WriteKind,
    pub line: usize,
}

pub(super) struct Local {
    pub name: String,
    pub at: usize,
    /// Its initial value's tokens, where it is given one.
    pub init: Option<Range<usize>>,
}

#[derive(Default)]
pub(super) struct Facts {
    pub locals: Vec<Local>,
    pub calls: Vec<Call>,
    pub writes: Vec<Write>,
    /// Every path read, called or written: what a carved stretch reaches
    /// out of itself for.
    pub paths: Vec<(Path, usize)>,
    /// Paths held against a number other than zero, with their lines.
    pub compares: Vec<(Path, usize)>,
}

impl Facts {
    /// The facts whose tokens fall in `range`.
    pub fn within(&self, range: &Range<usize>) -> Facts {
        let inside = |at: usize| range.contains(&at);
        Facts {
            locals: self
                .locals
                .iter()
                .filter(|l| inside(l.at))
                .map(|l| Local {
                    name: l.name.clone(),
                    at: l.at,
                    init: l.init.clone(),
                })
                .collect(),
            calls: self
                .calls
                .iter()
                .filter(|c| inside(c.path.at))
                .map(|c| Call {
                    path: c.path.clone(),
                    args: c.args.clone(),
                    line: c.line,
                })
                .collect(),
            writes: self
                .writes
                .iter()
                .filter(|w| inside(w.path.at))
                .map(|w| Write {
                    path: w.path.clone(),
                    kind: w.kind.clone(),
                    line: w.line,
                })
                .collect(),
            paths: self
                .paths
                .iter()
                .filter(|(p, _)| inside(p.at))
                .cloned()
                .collect(),
            compares: self
                .compares
                .iter()
                .filter(|(p, _)| inside(p.at))
                .cloned()
                .collect(),
        }
    }
}

const KEYWORDS: [&str; 30] = [
    "if",
    "else",
    "for",
    "while",
    "do",
    "return",
    "const",
    "let",
    "var",
    "function",
    "new",
    "typeof",
    "instanceof",
    "in",
    "of",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "throw",
    "try",
    "catch",
    "finally",
    "this",
    "void",
    "delete",
    "null",
    "undefined",
    "Infinity",
];

/// The methods that change the value they are called on.
const MUTATORS: [&str; 10] = [
    "push", "pop", "shift", "unshift", "splice", "sort", "reverse", "fill", "clear", "delete",
];

const COMPARISONS: [&str; 8] = ["===", "!==", "==", "!=", "<=", ">=", "<", ">"];

fn is_count(text: &str) -> bool {
    text.chars().all(|c| c.is_ascii_digit()) && text.chars().any(|c| c != '0')
}

fn constant(text: &str) -> bool {
    matches!(text, "true" | "false" | "null" | "undefined")
        || text.starts_with('"')
        || text.starts_with('\'')
        || (text.starts_with('`') && !text.ends_with("${"))
        || text.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// The facts of `range` in `tokens`.
pub(super) fn facts(tokens: &[Token], range: Range<usize>) -> Facts {
    let mut out = Facts::default();
    let end = range.end;
    let mut k = range.start;
    while k < end {
        if let Some(next) = declares(tokens, k, end, &mut out) {
            k = next;
            continue;
        }
        let Some((path, last)) = path_at(tokens, k, end, &mut out) else {
            k += 1;
            continue;
        };
        k = after_path(tokens, path, k, last, end, &mut out);
    }
    out
}

/// The names declared at `k` — a `let` / `const` / `var`, a `catch`'s,
/// a function's name and parameters, an arrow's parameters — and an
/// object literal's key, which names nothing: where reading goes on, or
/// `None` where nothing is declared at `k`.
fn declares(tokens: &[Token], k: usize, end: usize, out: &mut Facts) -> Option<usize> {
    let text = tokens[k].text.as_str();
    let before = k.checked_sub(1).map(|b| tokens[b].text.as_str());
    let local = |name: String, at: usize| Local {
        name,
        at,
        init: None,
    };
    if matches!(text, "let" | "const" | "var") {
        return Some(declaration(tokens, k + 1, end, out));
    }
    if text == "catch" && tokens.get(k + 1).is_some_and(|t| t.text == "(") {
        if let Some(name) = tokens.get(k + 2) {
            out.locals.push(local(name.text.clone(), k + 2));
        }
        return Some(k + 3);
    }
    if text == "function" {
        // A function's name and its parameters are the stretch's own names.
        let mut j = k + 1;
        if tokens.get(j).is_some_and(|t| is_name(&t.text)) {
            out.locals.push(local(tokens[j].text.clone(), j));
            j += 1;
        }
        if tokens.get(j).is_some_and(|t| t.text == "(")
            && let Some(close) = matching_close(tokens, j, "(", ")")
        {
            for name in params(&tokens[j + 1..close]) {
                out.locals.push(local(name, j));
            }
            return Some(close + 1);
        }
        return Some(j);
    }
    // An arrow's parameters.
    if text == "("
        && let Some(close) = matching_close(tokens, k, "(", ")")
        && close < end
        && tokens.get(close + 1).is_some_and(|t| t.text == "=>")
        && before != Some(".")
        && !before.is_some_and(|b| is_name(b) && !KEYWORDS.contains(&b))
    {
        for name in params(&tokens[k + 1..close]) {
            out.locals.push(local(name, k));
        }
        return Some(close + 2);
    }
    if is_name(text) && tokens.get(k + 1).is_some_and(|t| t.text == "=>") && before != Some(".") {
        out.locals.push(local(text.to_string(), k));
        return Some(k + 2);
    }
    // An object literal's key.
    if is_name(text)
        && tokens.get(k + 1).is_some_and(|t| t.text == ":")
        && matches!(before, Some("{" | ","))
    {
        return Some(k + 2);
    }
    None
}

/// The path that starts at `k` — a name's, or one read off a value
/// before it (`f().g`, `x[i].y`, `"s".length`) — and the index of its
/// last token. `delete a.b` is a write to `a`, said here; then the path
/// is `None` and reading goes on past it.
fn path_at(tokens: &[Token], k: usize, end: usize, out: &mut Facts) -> Option<(Path, usize)> {
    let text = tokens[k].text.as_str();
    let before = k.checked_sub(1).map(|b| tokens[b].text.as_str());
    if is_name(text) && before != Some(".") && !KEYWORDS.contains(&text) {
        return Some(read_path(tokens, k, Head::Name(text.to_string()), end));
    }
    if text == "."
        && before.is_some_and(|b| {
            matches!(b, ")" | "]")
                || constant(b)
                || ((b.starts_with('}') || b.starts_with('/')) && b.len() > 1 && b != "/=")
        })
        && tokens.get(k + 1).is_some_and(|t| is_name(&t.text))
    {
        let (mut path, last) = read_path(tokens, k + 1, Head::Value, end);
        path.at = k;
        return Some((path, last));
    }
    if text == "delete" && tokens.get(k + 1).is_some_and(|t| is_name(&t.text)) {
        let (path, _) = read_path(tokens, k + 1, Head::Name(tokens[k + 1].text.clone()), end);
        if let Some(receiver) = path.receiver() {
            out.writes.push(Write {
                path: receiver,
                kind: WriteKind::Mutate,
                line: tokens[k].line,
            });
        }
    }
    None
}

/// What the path at `k` (ending at `last`) is used for: a call, a write,
/// a read — said into `out`. Answers where reading goes on: into a call's
/// arguments and a write's value, which are code of this stretch too.
fn after_path(
    tokens: &[Token],
    path: Path,
    k: usize,
    last: usize,
    end: usize,
    out: &mut Facts,
) -> usize {
    let line = tokens[k].line;
    let before = k.checked_sub(1).map(|b| tokens[b].text.as_str());
    let next = tokens.get(last + 1).map(|t| t.text.as_str());
    out.paths.push((path.clone(), line));
    // Held against a count, on either side.
    let left_count =
        before.is_some_and(|b| COMPARISONS.contains(&b)) && k >= 2 && is_count(&tokens[k - 2].text);
    let right_count = next.is_some_and(|n| COMPARISONS.contains(&n))
        && tokens.get(last + 2).is_some_and(|t| is_count(&t.text));
    if left_count || right_count {
        out.compares.push((path.clone(), line));
    }
    let written = |kind: WriteKind| Write {
        path: path.clone(),
        kind,
        line,
    };
    match next {
        Some("(") => {
            let close = matching_close(tokens, last + 1, "(", ")").unwrap_or(end);
            let args = arguments(tokens, last + 1, close);
            if let Some(method) = path.members.last()
                && MUTATORS.contains(&method.as_str())
                && let Some(receiver) = path.receiver()
            {
                out.writes.push(Write {
                    path: receiver,
                    kind: WriteKind::Mutate,
                    line,
                });
            }
            out.calls.push(Call { path, args, line });
            last + 2
        }
        Some("=") => {
            let stop = expression_end(tokens, last + 2, end);
            let stepped = tokens
                .get(last + 2)
                .is_some_and(|t| t.text == tokens[k].text)
                && tokens
                    .get(last + 2 + 2 * path.members.len() + 1)
                    .is_some_and(|t| matches!(t.text.as_str(), "+" | "-"));
            let alone = stop == last + 3 || (stop == last + 4 && tokens[last + 2].text == "-");
            let value_at = if tokens[last + 2].text == "-" {
                last + 3
            } else {
                last + 2
            };
            let kind = if stepped {
                WriteKind::Step
            } else if alone && tokens.get(value_at).is_some_and(|t| constant(&t.text)) {
                WriteKind::Constant
            } else {
                WriteKind::Value(last + 2..stop)
            };
            out.writes.push(written(kind));
            last + 2
        }
        Some("+=" | "-=" | "*=" | "/=") => {
            out.writes.push(written(WriteKind::Step));
            last + 2
        }
        Some("++" | "--") if tokens[last + 1].line == line => {
            out.writes.push(written(WriteKind::Step));
            last + 2
        }
        Some("[") => {
            // `a.b[i] = …` changes `a.b`.
            if let Some(close) = matching_close(tokens, last + 1, "[", "]")
                && tokens
                    .get(close + 1)
                    .is_some_and(|t| matches!(t.text.as_str(), "=" | "+=" | "-=" | "++" | "--"))
            {
                out.writes.push(written(WriteKind::Mutate));
            }
            last + 1
        }
        _ => {
            if matches!(before, Some("++" | "--")) && tokens[k - 1].line == line {
                out.writes.push(written(WriteKind::Step));
            }
            last + 1
        }
    }
}

/// The path from `at` (`a.b.c`, `a?.b`): itself, and the index of its
/// last token.
fn read_path(tokens: &[Token], at: usize, head: Head, end: usize) -> (Path, usize) {
    let mut members = Vec::new();
    let mut last = at;
    if head == Head::Value {
        members.push(tokens[at].text.clone());
    }
    loop {
        let dot = if tokens.get(last + 1).is_some_and(|t| t.text == ".") {
            last + 1
        } else if tokens.get(last + 1).is_some_and(|t| t.text == "?")
            && tokens.get(last + 2).is_some_and(|t| t.text == ".")
        {
            last + 2
        } else {
            break;
        };
        if dot + 1 >= end || !tokens.get(dot + 1).is_some_and(|t| is_name(&t.text)) {
            break;
        }
        members.push(tokens[dot + 1].text.clone());
        last = dot + 1;
    }
    (Path { head, members, at }, last)
}

/// The arguments between the `(` at `open` and its `)` at `close`.
fn arguments(tokens: &[Token], open: usize, close: usize) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut from = open + 1;
    for (k, token) in tokens.iter().enumerate().take(close).skip(open + 1) {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            "," if depth == 0 => {
                out.push(from..k);
                from = k + 1;
            }
            _ => {}
        }
    }
    if from < close {
        out.push(from..close);
    }
    out
}

/// The names of a `let` / `const` / `var` from `at`, with their values;
/// answers where reading goes on (the first value, read as code).
fn declaration(tokens: &[Token], at: usize, end: usize, out: &mut Facts) -> usize {
    let mut k = at;
    loop {
        let Some(token) = tokens.get(k) else {
            return end;
        };
        let names: Vec<(String, usize)> = match token.text.as_str() {
            "[" | "{" => {
                let close_text = if token.text == "[" { "]" } else { "}" };
                let Some(close) = matching_close(tokens, k, &token.text, close_text) else {
                    return k + 1;
                };
                let names = (k + 1..close)
                    .filter(|j| {
                        is_name(&tokens[*j].text)
                            && !tokens.get(j + 1).is_some_and(|t| t.text == ":")
                    })
                    .map(|j| (tokens[j].text.clone(), j))
                    .collect();
                k = close + 1;
                names
            }
            text if is_name(text) => {
                k += 1;
                vec![(text.to_string(), k - 1)]
            }
            _ => return k,
        };
        let init = if tokens.get(k).is_some_and(|t| t.text == "=") {
            let stop = expression_end(tokens, k + 1, end);
            Some(k + 1..stop)
        } else {
            None
        };
        for (name, at) in names {
            out.locals.push(Local {
                name,
                at,
                init: init.clone(),
            });
        }
        match init {
            // The value is read as code from here; a `,` after it declares the next.
            Some(range) => {
                let after = range.end;
                if tokens.get(after).is_some_and(|t| t.text == ",") && after < end {
                    // Read the value first, then the next name.
                    let inner = facts(tokens, range.clone());
                    out.locals.extend(inner.locals);
                    out.calls.extend(inner.calls);
                    out.writes.extend(inner.writes);
                    out.paths.extend(inner.paths);
                    out.compares.extend(inner.compares);
                    k = after + 1;
                    continue;
                }
                return range.start;
            }
            None => {
                if tokens.get(k).is_some_and(|t| t.text == ",") {
                    k += 1;
                    continue;
                }
                return k;
            }
        }
    }
}
