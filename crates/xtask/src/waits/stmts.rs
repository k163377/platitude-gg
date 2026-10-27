//! A handler's body as a tree of statements ([`parse_handler`]): what
//! the beats proof walks for every way to a step.

use crate::lex::matching_close;

/// A statement of a handler as the proof reads it: the tokens it spans
/// (`from..to`, indices into the handler's body) and its shape.
#[derive(Debug)]
pub(super) enum Stmt {
    /// `{ … }`.
    Block {
        from: usize,
        to: usize,
        body: Vec<Stmt>,
    },
    /// `if (…) … [else …]`; `cond` spans the tokens inside the parentheses.
    If {
        from: usize,
        to: usize,
        cond: (usize, usize),
        then: Box<Stmt>,
        els: Option<Box<Stmt>>,
    },
    /// `for (…) …` / `while (…) …`: the body runs any number of times.
    Loop {
        from: usize,
        to: usize,
        body: Box<Stmt>,
    },
    /// `return …` / `throw …`: the handler is left here.
    Exit { from: usize, to: usize },
    /// Anything else that runs and goes on: a declaration, an expression,
    /// `break`, `continue`.
    Plain { from: usize, to: usize },
}

impl Stmt {
    pub(super) fn span(&self) -> (usize, usize) {
        match self {
            Stmt::Block { from, to, .. }
            | Stmt::If { from, to, .. }
            | Stmt::Loop { from, to, .. }
            | Stmt::Exit { from, to }
            | Stmt::Plain { from, to } => (*from, *to),
        }
    }

    pub(super) fn holds(&self, at: usize) -> bool {
        let (from, to) = self.span();
        from <= at && at < to
    }

    /// Whether every way through leaves the handler.
    pub(super) fn always_exits(&self) -> bool {
        match self {
            Stmt::Exit { .. } => true,
            Stmt::Block { body, .. } => body.iter().any(Stmt::always_exits),
            Stmt::If {
                then,
                els: Some(els),
                ..
            } => then.always_exits() && els.always_exits(),
            _ => false,
        }
    }
}

/// The tokens an expression goes on past a line break from, and the ones
/// it goes on into — the one reading of a missing `;` (JavaScript's own:
/// a line that ends on an operator or an opener, or begins on one, is the
/// same expression).
const CONTINUES_AFTER: [&str; 30] = [
    ",", "(", "[", ".", "=", "+", "-", "*", "/", "%", "&&", "||", "??", "?", ":", "=>", "!", "===",
    "!==", "==", "!=", "<", ">", "<=", ">=", "+=", "-=", "*=", "/=", "{",
];
const CONTINUES_BEFORE: [&str; 28] = [
    ".", ")", "]", ",", "+", "-", "*", "/", "%", "&&", "||", "??", "?", ":", "===", "!==", "==",
    "!=", "<", ">", "<=", ">=", "=", "+=", "-=", "*=", "/=", "(",
];

/// The handler's statements, or `None` for a body the proof does not
/// read — a `switch`, `try`, `do`, `function`, `class` or `with`, an
/// `else` with no `if`, a bracket never closed. Such a handler is `unread`.
pub(super) fn parse_handler(body: &[crate::lex::Token]) -> Option<Vec<Stmt>> {
    if body.first().is_some_and(|t| t.text == "{") {
        let (stmts, close) = parse_stmts(body, 1, body.len())?;
        (close + 1 == body.len() && body[close].text == "}").then_some(stmts)
    } else {
        // A bare expression runs to the end of its line, which may hold the `}` of the component it is in.
        let (stmts, next) = parse_stmts(body, 0, body.len())?;
        (next == body.len() || body[next].text == "}").then_some(stmts)
    }
}

/// The statements from `i` to the `}` that closes this level (whose index
/// comes back), or to `end`.
fn parse_stmts(body: &[crate::lex::Token], mut i: usize, end: usize) -> Option<(Vec<Stmt>, usize)> {
    let mut out = Vec::new();
    while i < end {
        match body[i].text.as_str() {
            "}" => return Some((out, i)),
            ";" => {
                i += 1;
                continue;
            }
            _ => {}
        }
        let (stmt, next) = parse_stmt(body, i, end)?;
        out.push(stmt);
        i = next;
    }
    Some((out, i))
}

/// One statement beginning at `i`, with the index past it.
fn parse_stmt(body: &[crate::lex::Token], i: usize, end: usize) -> Option<(Stmt, usize)> {
    match body[i].text.as_str() {
        "{" => {
            let (stmts, close) = parse_stmts(body, i + 1, end)?;
            if close >= end || body[close].text != "}" {
                return None;
            }
            Some((
                Stmt::Block {
                    from: i,
                    to: close + 1,
                    body: stmts,
                },
                close + 1,
            ))
        }
        "if" => {
            let close = parenthesised(body, i, end)?;
            let (then, next) = parse_stmt(body, close + 1, end)?;
            let (els, next) = if body.get(next).is_some_and(|t| t.text == "else") && next + 1 < end
            {
                let (els, next) = parse_stmt(body, next + 1, end)?;
                (Some(Box::new(els)), next)
            } else {
                (None, next)
            };
            Some((
                Stmt::If {
                    from: i,
                    to: next,
                    cond: (i + 2, close),
                    then: Box::new(then),
                    els,
                },
                next,
            ))
        }
        "for" | "while" => {
            let close = parenthesised(body, i, end)?;
            let (inner, next) = parse_stmt(body, close + 1, end)?;
            Some((
                Stmt::Loop {
                    from: i,
                    to: next,
                    body: Box::new(inner),
                },
                next,
            ))
        }
        "return" | "throw" => {
            let next = expression_end(body, i + 1, end, true)?;
            Some((Stmt::Exit { from: i, to: next }, next))
        }
        "switch" | "try" | "do" | "function" | "class" | "with" | "else" | "case" | "default"
        | ")" | "]" => None,
        _ => {
            let next = expression_end(body, i, end, false)?;
            (next > i).then_some((Stmt::Plain { from: i, to: next }, next))
        }
    }
}

/// The `)` closing the `(` right after the keyword at `i`.
fn parenthesised(body: &[crate::lex::Token], i: usize, end: usize) -> Option<usize> {
    if body.get(i + 1).is_none_or(|t| t.text != "(") {
        return None;
    }
    let close = matching_close(body, i + 1, "(", ")")?;
    (close + 1 < end).then_some(close)
}

/// The index past the expression beginning at `i`: a `;` (taken), the `}`
/// closing this level (left), or a line break the expression does not go
/// on over (`CONTINUES_AFTER` / `CONTINUES_BEFORE`). Brackets of every
/// kind are crossed whole, so a function or an object inside is one
/// token's worth of expression. `restricted` is a `return`'s: nothing on
/// a later line is its value.
fn expression_end(
    body: &[crate::lex::Token],
    i: usize,
    end: usize,
    restricted: bool,
) -> Option<usize> {
    if restricted && i < end && i > 0 && body[i].line > body[i - 1].line {
        return Some(i);
    }
    let mut depth = 0i32;
    let mut k = i;
    while k < end {
        if k > i
            && depth == 0
            && body[k].line > body[k - 1].line
            && !CONTINUES_AFTER.contains(&body[k - 1].text.as_str())
            && !CONTINUES_BEFORE.contains(&body[k].text.as_str())
        {
            return Some(k);
        }
        match body[k].text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                if depth == 0 {
                    return (body[k].text == "}").then_some(k);
                }
                depth -= 1;
            }
            ";" if depth == 0 => return Some(k + 1),
            _ => {}
        }
        k += 1;
    }
    (depth == 0).then_some(k)
}
