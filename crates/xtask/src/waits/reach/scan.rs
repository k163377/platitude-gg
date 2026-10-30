//! What a stretch of code does, read off its tokens before any name in it
//! is resolved: the names it declares, and the paths it reads, writes and
//! calls, each with the token it stands at — so that a stretch carved out
//! of it later (an argument read in a context of its own) takes its own
//! facts with it ([`Facts::within`]).

use std::ops::Range;

use super::parse::{Bound, expression_end, is_name, pattern_names};
use super::scope::{lexical_scope, parameter_scope};
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
    /// Changed in place: a `push`, a `delete`, an element stepped.
    Mutate,
    /// Changed in place by a value put into it (`a[i] = v`): the value's tokens.
    Put(Range<usize>),
}

pub(super) struct Write {
    pub path: Path,
    pub kind: WriteKind,
    pub line: usize,
}

#[derive(Clone)]
pub(super) struct Local {
    pub name: String,
    pub at: usize,
    /// Where this declaration binds the name, including its defaults.
    pub scope: Range<usize>,
    /// Its initial value's tokens, where it is given one.
    pub init: Option<Range<usize>>,
    /// A parameter — of a function or arrow spelled in the stretch, or a
    /// `catch`'s: given whatever it is handed.
    pub param: bool,
    /// What of its value it is bound to (`parse::Bound::depth`): 0 the
    /// value whole, 1 an element or field of it (`const [a] = x`), and so
    /// on; `None` where nothing here follows what it is bound to — a
    /// parameter list's own rest, a `for … of` / `in`'s name.
    pub depth: Option<usize>,
    /// The defaults a pattern gives it, each with how deep in it it stands.
    pub defaults: Vec<(Range<usize>, usize)>,
}

#[derive(Default)]
pub(super) struct Facts {
    /// The stretch they were read from.
    pub whole: Range<usize>,
    /// Stretches read as part of them though outside `whole` — what a
    /// unit's parameter list evaluates —, kept by `within` the whole.
    pub beside: Vec<Range<usize>>,
    pub locals: Vec<Local>,
    pub calls: Vec<Call>,
    pub writes: Vec<Write>,
    /// Every path read, called or written: what a carved stretch reaches
    /// out of itself for.
    pub paths: Vec<(Path, usize)>,
    /// Paths held against a number other than zero, with their lines.
    pub compares: Vec<(Path, usize)>,
    /// Each `return`'s value: its tokens, and the token of its `return`.
    pub returns: Vec<(Range<usize>, usize)>,
}

impl Facts {
    /// The innermost declaration visible at this use, not any declaration
    /// elsewhere in the enclosing function with the same spelling.
    pub fn local(&self, name: &str, at: usize) -> Option<&Local> {
        self.locals
            .iter()
            .filter(|l| l.name == name && l.scope.contains(&at))
            .min_by_key(|l| l.scope.len())
    }
    /// Takes in what another stretch read, as these facts' own.
    pub fn absorb(&mut self, other: Facts) {
        self.locals.extend(other.locals);
        self.calls.extend(other.calls);
        self.writes.extend(other.writes);
        self.paths.extend(other.paths);
        self.compares.extend(other.compares);
        self.returns.extend(other.returns);
    }

    /// Takes in what `range`, outside the stretch these were read from, read
    /// as part of them (`beside`).
    pub fn absorb_beside(&mut self, other: Facts, range: Range<usize>) {
        self.absorb(other);
        self.beside.push(range);
    }

    /// The facts whose tokens fall in `range` — and, for the whole stretch
    /// they were read from, in what was read beside it.
    pub fn within(&self, range: &Range<usize>) -> Facts {
        let beside: &[Range<usize>] = if *range == self.whole {
            &self.beside
        } else {
            &[]
        };
        let inside = |at: usize| range.contains(&at) || beside.iter().any(|b| b.contains(&at));
        Facts {
            whole: range.clone(),
            beside: beside.to_vec(),
            locals: self
                .locals
                .iter()
                .filter(|l| inside(l.at))
                .map(|l| Local {
                    name: l.name.clone(),
                    at: l.at,
                    scope: l.scope.clone(),
                    init: l.init.clone(),
                    param: l.param,
                    depth: l.depth,
                    defaults: l.defaults.clone(),
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
            returns: self
                .returns
                .iter()
                .filter(|(_, at)| inside(*at))
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

fn assignment(text: &str) -> bool {
    text == "=" || (text.ends_with('=') && !COMPARISONS.contains(&text))
}

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
    let mut out = Facts {
        whole: range.clone(),
        ..Facts::default()
    };
    let end = range.end;
    let mut k = range.start;
    while k < end {
        if tokens[k].text == "return" {
            let stop = expression_end(tokens, k + 1, end);
            if stop > k + 1 {
                out.returns.push((k + 1..stop, k));
            }
        }
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
        scope: lexical_scope(tokens, at, false),
        init: None,
        param: true,
        depth: Some(0),
        defaults: Vec::new(),
    };
    let param = |bound: Bound| Local {
        name: bound.name,
        at: bound.at,
        scope: parameter_scope(tokens, k, end),
        init: None,
        param: true,
        depth: bound.depth,
        defaults: bound.defaults,
    };
    if matches!(text, "let" | "const" | "var") {
        return Some(declaration(tokens, k + 1, end, out));
    }
    if text == "catch"
        && tokens.get(k + 1).is_some_and(|t| t.text == "(")
        && let Some(close) = matching_close(tokens, k + 1, "(", ")")
    {
        let pattern = pattern_names(&tokens[k + 2..close], k + 2);
        for bound in pattern.bound {
            out.locals.push(param(bound));
        }
        evaluate(tokens, pattern.evaluated, out);
        return Some(close + 1);
    }
    if text == "function" {
        // A function's name and its parameters are the stretch's own names; the name holds the function.
        let named = tokens.get(k + 1).is_some_and(|t| is_name(&t.text));
        let j = if named { k + 2 } else { k + 1 };
        let close = tokens
            .get(j)
            .filter(|t| t.text == "(")
            .and_then(|_| matching_close(tokens, j, "(", ")"));
        if named {
            let whole = close
                .filter(|close| tokens.get(close + 1).is_some_and(|t| t.text == "{"))
                .and_then(|close| matching_close(tokens, close + 1, "{", "}"))
                .map(|end| k..end + 1);
            out.locals.push(Local {
                param: false,
                init: whole,
                ..local(tokens[k + 1].text.clone(), k + 1)
            });
        }
        let Some(close) = close else {
            return Some(j);
        };
        let pattern = pattern_names(&tokens[j + 1..close], j + 1);
        for bound in pattern.bound {
            out.locals.push(param(bound));
        }
        evaluate(tokens, pattern.evaluated, out);
        return Some(close + 1);
    }
    // An arrow's parameters.
    if text == "("
        && let Some(close) = matching_close(tokens, k, "(", ")")
        && close < end
        && tokens.get(close + 1).is_some_and(|t| t.text == "=>")
        && before != Some(".")
        && !before.is_some_and(|b| is_name(b) && !KEYWORDS.contains(&b))
    {
        let pattern = pattern_names(&tokens[k + 1..close], k + 1);
        for bound in pattern.bound {
            out.locals.push(param(bound));
        }
        evaluate(tokens, pattern.evaluated, out);
        return Some(close + 2);
    }
    if is_name(text) && tokens.get(k + 1).is_some_and(|t| t.text == "=>") && before != Some(".") {
        out.locals.push(Local {
            scope: parameter_scope(tokens, k, end),
            ..local(text.to_string(), k)
        });
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
    // The indexed receiver is the value immediately before this bracket.
    if text == "["
        && matches!(before, Some("]" | ")"))
        && let Some(close) = matching_close(tokens, k, "[", "]")
        && tokens
            .get(close + 1)
            .is_some_and(|t| assignment(&t.text) || matches!(t.text.as_str(), "++" | "--"))
    {
        out.writes.push(Write {
            path: Path {
                head: Head::Value,
                members: Vec::new(),
                at: k,
            },
            kind: if tokens[close + 1].text == "=" {
                WriteKind::Put(close + 2..expression_end(tokens, close + 2, end))
            } else {
                WriteKind::Mutate
            },
            line: tokens[k].line,
        });
    }
    // `this` heads a path as a name does: what it stands for is read where it is spelled (`Program::this_origin`).
    if is_name(text) && before != Some(".") && (!KEYWORDS.contains(&text) || text == "this") {
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
        Some(
            "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "**=" | "<<=" | ">>=" | ">>>=",
        ) => {
            out.writes.push(written(WriteKind::Step));
            last + 2
        }
        Some("&&=" | "||=" | "??=") => {
            out.writes.push(written(WriteKind::Value(
                last + 2..expression_end(tokens, last + 2, end),
            )));
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
                    .is_some_and(|t| assignment(&t.text) || matches!(t.text.as_str(), "++" | "--"))
            {
                let kind = if tokens[close + 1].text == "=" {
                    WriteKind::Put(close + 2..expression_end(tokens, close + 2, end))
                } else {
                    WriteKind::Mutate
                };
                out.writes.push(written(kind));
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

/// Reads what binding by a pattern evaluates — its defaults, its computed
/// keys — as code where the pattern stands.
fn evaluate(tokens: &[Token], evaluated: Vec<Range<usize>>, out: &mut Facts) {
    for range in evaluated {
        out.absorb(facts(tokens, range));
    }
}

/// The names of a `let` / `const` / `var` from `at`, with their values;
/// answers where reading goes on (the first value, read as code).
fn declaration(tokens: &[Token], at: usize, end: usize, out: &mut Facts) -> usize {
    let mut k = at;
    loop {
        let Some(token) = tokens.get(k) else {
            return end;
        };
        let names: Vec<Bound> = match token.text.as_str() {
            "[" | "{" => {
                let close_text = if token.text == "[" { "]" } else { "}" };
                let Some(close) = matching_close(tokens, k, &token.text, close_text) else {
                    return k + 1;
                };
                let pattern = pattern_names(&tokens[k..=close], k);
                evaluate(tokens, pattern.evaluated, out);
                k = close + 1;
                pattern.bound
            }
            text if is_name(text) => {
                k += 1;
                vec![Bound {
                    name: text.to_string(),
                    at: k - 1,
                    depth: Some(0),
                    defaults: Vec::new(),
                }]
            }
            _ => return k,
        };
        // A loop's name is given what the loop walks, which nothing here follows.
        let walked = tokens
            .get(k)
            .is_some_and(|t| t.text == "of" || t.text == "in");
        let init = if tokens.get(k).is_some_and(|t| t.text == "=") {
            let stop = expression_end(tokens, k + 1, end);
            Some(k + 1..stop)
        } else {
            None
        };
        for bound in names {
            out.locals.push(Local {
                name: bound.name,
                at: bound.at,
                scope: lexical_scope(tokens, at - 1, tokens[at - 1].text == "var"),
                init: init.clone(),
                param: false,
                depth: bound.depth.filter(|_| !walked),
                defaults: bound.defaults,
            });
        }
        match init {
            // The value is read as code from here; a `,` after it declares the next.
            Some(range) => {
                let after = range.end;
                if tokens.get(after).is_some_and(|t| t.text == ",") && after < end {
                    // Read the value first, then the next name.
                    out.absorb(facts(tokens, range.clone()));
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
