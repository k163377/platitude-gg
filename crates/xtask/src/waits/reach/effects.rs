//! What a stretch of code does ([`Effect`]), read off its facts: each call
//! followed to what it calls (`super::calls`), each argument a call runs in
//! a context of its own carved out as a code of its own, and each write
//! judged by what it changes — a name's own, or the value whose origin says
//! whose it is (`super::origin`). What the product's own code does that
//! cannot be followed is an input.

use std::ops::Range;

use super::Context;
use super::builtins;
use super::calls::function_body;
use super::origin::{Origin, join_into};
use super::program::{ArgContext, Effect, Program, Root, Value};
use super::scan::{Head, Path, WriteKind};

impl Program {
    /// Fills in what `code` does: its own facts, those of the arguments
    /// carved out of it left to their own codes.
    pub(super) fn read(&mut self, code: usize) {
        let (d, u, range) = {
            let c = &self.codes[code];
            (c.doc, c.unit, c.range.clone())
        };
        let facts = self.facts_of(d, u).within(&range);
        let mut carved: Vec<Range<usize>> = Vec::new();
        let mut effects = Vec::new();
        // Calls first: they say which arguments are stretches of their own — an arrow a local
        // holds among them, which is carved where it is spelled, so its calls, met before, are
        // the carved code's.
        let mut called = Vec::new();
        for call in &facts.calls {
            if carved.iter().any(|r| r.contains(&call.path.at)) {
                continue;
            }
            let (effect, contexts) = self.call(code, &call.path, call.args.len());
            if let Some(Effect::Calls(callees)) = &effect {
                for callee in callees.clone() {
                    self.hand_on(code, callee, &call.args);
                }
            }
            self.puts(code, &call.path, &call.args);
            if let Some(called_back) = self.callback(code, &call.path, &call.args) {
                called.push((called_back, call.line, call.path.at));
            }
            // A global function that changes the value it is handed first (`Object.assign`)
            // changes it in place.
            if let (Some(object), Some(method)) =
                (self.global_of(code, &call.path), call.path.last())
                && builtins::global_puts(&object, method).is_some()
                && let Some(first) = call.args.first().cloned()
            {
                let origin = self.origin_of(code, first);
                if let Some(changed) = self.change(code, origin, None, &call.path, false, &[]) {
                    called.push((changed, call.line, call.path.at));
                }
            }
            for (arg, context) in contexts {
                let Some(arg_range) = call.args.get(arg).cloned() else {
                    continue;
                };
                carved.extend(self.carve(
                    code,
                    arg_range.clone(),
                    context,
                    &mut effects,
                    call.line,
                ));
                carved.push(arg_range);
            }
            if let Some(effect) = effect {
                called.push((effect, call.line, call.path.at));
            }
        }
        let inside = |at: usize| carved.iter().any(|r| r.contains(&at));
        effects.extend(
            called
                .into_iter()
                .filter(|(_, _, at)| !inside(*at))
                .map(|(effect, line, _)| (effect, line)),
        );
        // Starts, for the writes that configure an operation before it.
        let starts: Vec<(Value, usize)> = facts
            .calls
            .iter()
            .filter(|c| !inside(c.path.at))
            .filter(|c| matches!(c.path.last(), Some("start" | "restart")))
            .filter_map(|c| c.path.receiver().map(|r| (r, c.path.at)))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(r, at)| (self.resolve(code, &r), at))
            .collect();
        for write in &facts.writes {
            if inside(write.path.at) {
                continue;
            }
            if let Some(effect) = self.write(code, &write.path, &write.kind, &starts) {
                effects.push((effect, write.line));
            }
        }
        if self.codes[code].parent.is_some() {
            for (path, line) in &facts.paths {
                if inside(path.at) {
                    continue;
                }
                if let Head::Name(name) = &path.head
                    && let Value::Captured(_, None) = self.head(code, name, path.at)
                {
                    effects.push((Effect::Captured(name.clone()), *line));
                }
            }
        }
        let mut compares = Vec::new();
        for (path, line) in &facts.compares {
            if inside(path.at) {
                continue;
            }
            let value = self.resolve(code, path);
            compares.push((value, path.spelled(), *line));
        }
        if d >= self.harness {
            as_the_products(&mut effects);
        }
        self.codes[code].effects = effects;
        self.codes[code].compares = compares;
    }

    /// The code of a function an argument names instead of spelling it: a
    /// local of `code` given an arrow — read as carved from `code` — or a
    /// harness function, read on its own.
    fn named_function(
        &mut self,
        code: usize,
        d: usize,
        u: usize,
        range: Range<usize>,
    ) -> Option<(usize, Option<Range<usize>>)> {
        let path = super::scan::one_path(&self.docs[d].tokens, range)?;
        if let (Head::Name(name), true) = (&path.head, path.members.is_empty()) {
            let within = self.codes[code].range.clone();
            let local = self
                .facts_of(d, u)
                .within(&within)
                .locals
                .into_iter()
                .find(|l| &l.name == name);
            if let Some(local) = local {
                let init = local.init?;
                function_body(&self.docs[d].tokens[init.clone()])?;
                let child = self.new_code(d, u, init.clone(), Some(code));
                self.read(child);
                return Some((child, Some(init)));
            }
        }
        match self.resolve(code, &path) {
            Value::Function(fd, fu) if fd < self.harness => Some((self.code_of(fd, fu), None)),
            _ => None,
        }
    }

    /// Records what a call in `code` that calls back the function whose
    /// code is `callee` hands each of its parameters: `handed`, the callee's
    /// to build where the caller made it.
    pub(super) fn hand_over(&mut self, code: usize, callee: usize, handed: &Origin) {
        let (cd, cu) = (self.codes[callee].doc, self.codes[callee].unit);
        let handed = handed.clone().handed_from(code, callee);
        for i in 0..self.docs[cd].units[cu].params.len() {
            self.bind(cd, cu, i, None);
            join_into(&mut self.origin_collected, (cd, cu, i), handed.clone());
        }
    }

    /// Records what a call in `code` hands each parameter of the function
    /// whose code is `callee`: the object it names, and where its value came
    /// from.
    fn hand_on(&mut self, code: usize, callee: usize, args: &[Range<usize>]) {
        let d = self.codes[code].doc;
        let (cd, cu) = (self.codes[callee].doc, self.codes[callee].unit);
        for (i, arg) in args.iter().enumerate() {
            let value = self.arg_value(code, d, arg.clone());
            self.bind(cd, cu, i, value);
            // A value the caller made, handed down, is the callee's to build as the caller's own.
            let origin = self.origin_of(code, arg.clone()).handed_from(code, callee);
            join_into(&mut self.origin_collected, (cd, cu, i), origin);
        }
    }

    /// Carves an argument of a call in `code` out as a code of its own,
    /// read in `context`; answers the stretches it carved beside the
    /// argument (the arrow a local the argument names was given).
    fn carve(
        &mut self,
        code: usize,
        range: Range<usize>,
        context: ArgContext,
        effects: &mut Vec<(Effect, usize)>,
        line: usize,
    ) -> Vec<Range<usize>> {
        let (d, u) = (self.codes[code].doc, self.codes[code].unit);
        match context {
            ArgContext::Of(context) => {
                if function_body(&self.docs[d].tokens[range.clone()]).is_some() {
                    let child = self.new_code(d, u, range, Some(code));
                    self.read(child);
                    self.roots.push(Root {
                        code: child,
                        context,
                    });
                } else if let Some((root, spelled)) = self.named_function(code, d, u, range) {
                    self.roots.push(Root {
                        code: root,
                        context,
                    });
                    return spelled.into_iter().collect();
                } else {
                    effects.push((
                        Effect::Unread(format!(
                            "an argument read as {} that is no function",
                            context.name()
                        )),
                        line,
                    ));
                }
            }
            ArgContext::Spec => {
                let spec = fields(&self.docs[d].tokens, range);
                for wanted in ["when", "report"] {
                    if !spec.iter().any(|(field, _)| field == wanted) {
                        effects.push((
                            Effect::Unread(format!("a completion with no `{wanted}` of its own")),
                            line,
                        ));
                    }
                }
                for (field, value) in spec {
                    if !matches!(field.as_str(), "when" | "report") {
                        continue;
                    }
                    if function_body(&self.docs[d].tokens[value.clone()]).is_some() {
                        let child = self.new_code(d, u, value, Some(code));
                        self.read(child);
                        self.roots.push(Root {
                            code: child,
                            context: Context::Read,
                        });
                    } else {
                        effects.push((
                            Effect::Unread(format!("a completion's `{field}` that is no function")),
                            line,
                        ));
                    }
                }
            }
        }
        Vec::new()
    }

    /// What a write does, judged by what it changes: a name given a new
    /// value changes the name — a local's own, or the harness property the
    /// name is — and anything else changes the value the path names, whose
    /// origin says whose it is (`super::origin`).
    fn write(
        &mut self,
        code: usize,
        path: &Path,
        kind: &WriteKind,
        starts: &[(Value, usize)],
    ) -> Option<Effect> {
        let step = *kind == WriteKind::Step;
        let in_place = matches!(kind, WriteKind::Mutate | WriteKind::Put(_));
        if path.members.is_empty() && !in_place {
            let Head::Name(name) = &path.head else {
                return Some(Effect::Unread(path.spelled()));
            };
            if self.is_local(code, name, path.at) {
                return None;
            }
            return match self.head(code, name, path.at) {
                Value::Captured(..) => Some(Effect::Smuggle(name.clone())),
                // Of the product's: an input.
                Value::Prop(d, ..) | Value::Object(d, _) if d >= self.harness => {
                    Some(Effect::Input(path.spelled()))
                }
                Value::Prop(d, o, p) => Some(Effect::Write {
                    target: (d, o, p),
                    step,
                    config: false,
                }),
                Value::Object(d, o) => Some(Effect::Write {
                    target: (d, o, name.clone()),
                    step,
                    config: false,
                }),
                Value::Product(_) => Some(Effect::Input(path.spelled())),
                _ => Some(Effect::Unread(path.spelled())),
            };
        }
        // What is changed: the value itself, changed in place, or the one whose member is given
        // a value.
        let (changed, member) = if in_place {
            (path.clone(), None)
        } else {
            (path.receiver()?, path.members.last().cloned())
        };
        let origin = self.path_origin(code, &changed);
        if let Some(name) = &member {
            self.give(&origin, name);
        }
        self.written_into(code, &origin, kind, in_place);
        self.change(code, origin, member, path, step, starts)
    }

    /// What changing a value of `origin` does — in place, or its `member`
    /// given a value.
    fn change(
        &mut self,
        code: usize,
        origin: Origin,
        member: Option<String>,
        path: &Path,
        step: bool,
        starts: &[(Value, usize)],
    ) -> Option<Effect> {
        match origin {
            Origin::Plain => None,
            // Made by this code: its own to build.
            Origin::Fresh { maker, .. } if maker == code => None,
            // Made by the code this one was carved from: a value carried out of it.
            Origin::Fresh { .. } => Some(Effect::Smuggle(path.spelled())),
            Origin::Shared(d, o, p) => Some(Effect::Write {
                target: (d, o, p),
                step,
                config: false,
            }),
            Origin::Object(Value::Object(d, o)) if d < self.harness => {
                let Some(name) = member else {
                    return Some(Effect::Unread(path.spelled()));
                };
                let own = self.member(d, o, &name);
                if own.is_none() && name == "running" && self.is_operation(d, o) {
                    return Some(Effect::Start);
                }
                let config = self.is_operation(d, o)
                    && starts
                        .iter()
                        .any(|(started, at)| *started == Value::Object(d, o) && *at > path.at);
                let (td, to) = match own {
                    Some(Value::Prop(pd, po, _)) => (pd, po),
                    _ => (d, o),
                };
                Some(Effect::Write {
                    target: (td, to, name),
                    step,
                    config,
                })
            }
            Origin::Object(_) | Origin::Product => Some(Effect::Input(path.spelled())),
            Origin::Unset | Origin::Unknown => Some(Effect::Unread(path.spelled())),
        }
    }
}

/// The effects of the product's own code, which is not read on trust: what
/// it does that cannot be followed is an input.
fn as_the_products(effects: &mut [(Effect, usize)]) {
    for (effect, _) in effects {
        if let Effect::Unread(what) = effect {
            *effect = Effect::Input(std::mem::take(what));
        }
    }
}

/// The fields of an object literal spanning `range`: each key and its
/// value's tokens.
fn fields(tokens: &[crate::lex::Token], range: Range<usize>) -> Vec<(String, Range<usize>)> {
    let mut out = Vec::new();
    if tokens.get(range.start).is_none_or(|t| t.text != "{") {
        return out;
    }
    let Some(close) = crate::lex::matching_close(tokens, range.start, "{", "}") else {
        return out;
    };
    let mut k = range.start + 1;
    while k < close {
        let key = tokens[k].text.trim_matches(['"', '\'']).to_string();
        if tokens.get(k + 1).is_none_or(|t| t.text != ":") {
            k += 1;
            continue;
        }
        let from = k + 2;
        let mut depth = 0;
        let mut end = from;
        while end < close {
            match tokens[end].text.as_str() {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                "," if depth == 0 => break,
                _ => {}
            }
            end += 1;
        }
        out.push((key, from..end));
        k = end + 1;
    }
    out
}
