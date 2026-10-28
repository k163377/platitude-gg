//! What a function spelled in the code is handed — its parameters and its
//! `this` — by the calls that call it back: handed as spelled, or by the
//! name of a local given it, to calls whose contracts say what they hand
//! it (`builtins::calls_back`). A function that goes anywhere else too is
//! handed what nothing here says.

use std::ops::Range;

use super::builtins::{CallsBack, Given};
use super::calls::function_body;
use super::origin::Origin;
use super::program::Program;
use super::scan::{Call, Head};
use crate::lex::{Token, matching_close};

impl Program {
    /// The calls in `code` that call back the function spelled at
    /// `function` — handed it as spelled, or by the name of a local given it
    /// (`local_function`) — each with how it calls back. `None` where it is
    /// handed nowhere such, or goes anywhere else too (called, kept, handed
    /// to what no contract answers for): there it is given what nothing
    /// here says.
    fn calls_handed(
        &mut self,
        code: usize,
        function: &Range<usize>,
    ) -> Option<Vec<(Call, CallsBack)>> {
        let (d, u, range) = {
            let c = &self.codes[code];
            (c.doc, c.unit, c.range.clone())
        };
        let facts = self.facts_of(d, u).within(&range);
        let mut names = Vec::new();
        for local in &facts.locals {
            if local.init.as_ref() == Some(function) && self.local_function(code, &local.name) {
                names.push(local.name.clone());
            }
        }
        let tokens = &self.docs[d].tokens;
        let is_handed = |arg: &Range<usize>| {
            arg == function || arg.len() == 1 && names.contains(&tokens[arg.start].text)
        };
        let handing: Vec<(Call, usize, usize)> = facts
            .calls
            .into_iter()
            .filter_map(|call| {
                let (index, arg) = call
                    .args
                    .iter()
                    .enumerate()
                    .find(|(_, arg)| is_handed(arg))?;
                let at = arg.start;
                Some((call, index, at))
            })
            .collect();
        let mut handed = Vec::new();
        let mut handed_at = Vec::new();
        for (call, index, at) in handing {
            let how = self.callback_of(code, &call.path)?;
            if how.at != index {
                return None;
            }
            handed_at.push(at);
            handed.push((call, how));
        }
        let elsewhere = facts.paths.iter().any(|(path, _)| {
            matches!(&path.head, Head::Name(name) if names.contains(name))
                && !handed_at.contains(&path.at)
        });
        (!handed.is_empty() && !elsewhere).then_some(handed)
    }

    /// What the parameter declared at `at` in `code` is handed: a parameter
    /// of a function every call of which calls it back (`calls_handed`) is
    /// given what those calls' contracts say (`builtins::calls_back`) — the
    /// elements each walks; any other, what nothing here says.
    pub(super) fn param_origin(&mut self, code: usize, at: usize) -> Origin {
        let (d, u, range) = {
            let c = &self.codes[code];
            (c.doc, c.unit, c.range.clone())
        };
        let facts = self.facts_of(d, u).within(&range);
        let spelled = facts
            .calls
            .iter()
            .flat_map(|call| call.args.iter())
            .chain(facts.locals.iter().filter_map(|local| local.init.as_ref()));
        // Its own parameters stand before its body; one inside the body is a nested function's.
        let tokens = &self.docs[d].tokens;
        let Some(function) = spelled
            .filter(|spelled| {
                function_body(&tokens[(*spelled).clone()])
                    .is_some_and(|body| (spelled.start..spelled.start + body.start).contains(&at))
            })
            .min_by_key(|spelled| spelled.len())
            .cloned()
        else {
            return Origin::Unknown;
        };
        let Some(handed) = self.calls_handed(code, &function) else {
            return Origin::Unknown;
        };
        let mut origin = Origin::Unset;
        for (call, how) in handed {
            origin = origin.join(match how.given {
                Given::Elements => {
                    let walked = self.walked_origin(code, &call.path, &call.args);
                    self.inside_of(&walked)
                }
                Given::Unknown => Origin::Unknown,
            });
        }
        origin
    }

    /// What `this` stands for at `at` in `code`, by the innermost `function`
    /// spelled around it — an arrow's `this` is its maker's: in a `function`
    /// every call of which calls it back (`calls_handed`), the arguments
    /// those calls bind it to (`builtins::CallsBack::this_at`); in a QML
    /// function or handler, or a `function` no call binds, nothing known.
    pub(super) fn this_origin(&mut self, code: usize, at: usize) -> Origin {
        let (d, range) = {
            let c = &self.codes[code];
            (c.doc, c.range.clone())
        };
        let Some(function) = innermost_function(&self.docs[d].tokens, range, at) else {
            return Origin::Unknown;
        };
        let Some(handed) = self.calls_handed(code, &function) else {
            return Origin::Unknown;
        };
        let mut origin = Origin::Unset;
        for (call, how) in handed {
            origin = origin.join(
                match how.this_at.and_then(|this_at| call.args.get(this_at)) {
                    Some(bound) => self.origin_of(code, bound.clone()),
                    None => Origin::Unknown,
                },
            );
        }
        origin
    }
}

/// The innermost `function` spelled in `range` of `tokens` whose body holds
/// `at`, from its `function` to its closing brace.
fn innermost_function(tokens: &[Token], range: Range<usize>, at: usize) -> Option<Range<usize>> {
    let mut found = None;
    for k in range.clone() {
        if tokens[k].text != "function" || k >= at {
            continue;
        }
        let Some(open) = (k + 1..range.end).find(|&j| tokens[j].text == "(") else {
            continue;
        };
        let Some(close) = matching_close(tokens, open, "(", ")") else {
            continue;
        };
        let body = close + 1;
        if tokens.get(body).is_none_or(|t| t.text != "{") {
            continue;
        }
        let Some(end) = matching_close(tokens, body, "{", "}") else {
            continue;
        };
        if (body..end).contains(&at) {
            found = Some(k..end + 1);
        }
    }
    found
}
