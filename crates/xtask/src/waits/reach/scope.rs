//! Lexical visibility of a declaration. A binding's identity is its token,
//! not its spelling; blocks and function parameters limit where it is seen.

use std::ops::Range;

use super::parse::expression_end;
use crate::lex::{Token, matching_close};

/// A function, arrow or catch clause, including its parameter defaults.
pub(super) fn parameter_scope(tokens: &[Token], start: usize, end: usize) -> Range<usize> {
    let mut body = start + 1;
    if tokens[start].text == "(" || matches!(tokens[start].text.as_str(), "function" | "catch") {
        let open = (start..end).find(|k| tokens[*k].text == "(");
        body = open
            .and_then(|k| matching_close(tokens, k, "(", ")"))
            .map_or(end, |k| k + 1);
    }
    if tokens.get(body).is_some_and(|t| t.text == "=>") {
        body += 1;
    }
    let stop = if tokens.get(body).is_some_and(|t| t.text == "{") {
        matching_close(tokens, body, "{", "}").map_or(end, |k| k + 1)
    } else {
        expression_end(tokens, body, end)
    };
    start..stop
}

/// A lexical declaration belongs to its enclosing block or for statement;
/// `var` belongs to its enclosing function instead.
pub(super) fn lexical_scope(tokens: &[Token], at: usize, function: bool) -> Range<usize> {
    let mut scope = 0..tokens.len();
    for k in 0..at {
        let text = tokens[k].text.as_str();
        let candidate = if function {
            if text == "function"
                || tokens.get(k + 1).is_some_and(|t| t.text == "=>")
                || (text == "("
                    && matching_close(tokens, k, "(", ")")
                        .is_some_and(|c| tokens.get(c + 1).is_some_and(|t| t.text == "=>")))
            {
                Some(parameter_scope(tokens, k, tokens.len()))
            } else {
                None
            }
        } else if text == "{" {
            matching_close(tokens, k, "{", "}").map(|close| k..close + 1)
        } else if text == "(" && k > 0 && tokens[k - 1].text == "for" {
            matching_close(tokens, k, "(", ")").map(|close| {
                let body = close + 1;
                let stop = if tokens.get(body).is_some_and(|t| t.text == "{") {
                    matching_close(tokens, body, "{", "}").map_or(tokens.len(), |c| c + 1)
                } else {
                    expression_end(tokens, body, tokens.len())
                };
                k..stop
            })
        } else {
            None
        };
        if let Some(candidate) = candidate
            && candidate.contains(&at)
            && candidate.len() < scope.len()
        {
            scope = candidate;
        }
    }
    scope
}
