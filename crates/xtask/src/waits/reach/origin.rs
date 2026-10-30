//! Where a value a stretch of code holds came from — made by the code
//! itself, or borrowed from state it does not own — so that a write
//! through it is judged by what it changes, not by the name it is written
//! through: a local that names a harness property's array changes that
//! array when it is pushed to, a local the code made changes nothing
//! outside the code.
//!
//! A container is told apart from what it holds. A value made by the code
//! names the places that made it (its sites: an array or object literal, a
//! `new`, a call that answers a new container); what each site's container
//! holds is one table for the whole program (`contents`), filled by every
//! put — a literal's elements, `x[i] = v`, `x.f = v`, `push`, `Object.assign`
//! — wherever it is spelled. A local, an argument and a returned value name
//! the same sites, so what a callee puts inside a container its caller made
//! is inside it for the caller too, under any name. The language's own
//! functions answer and put by their contracts (`super::builtins`); a call
//! no contract or code answers for, and the inside of a container nothing
//! was put in, are of no known origin, and a change through them is not
//! passed as safe.

use std::collections::BTreeSet;
use std::ops::Range;

use super::builtins::{self, Answers, CONSTRUCTED, GlobalAnswers, Made};
use super::calls::function_body;
use super::members::{ANY, Assigned};
use super::parse::is_name;
use super::program::{Program, Value};
use super::scan::{Head, Path, WriteKind};
use crate::lex::{Token, matching_close};

/// Where a container was made: its file and the token that makes it.
pub(super) type Site = (usize, usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Origin {
    /// Nothing put in yet: the bottom of a join.
    Unset,
    /// A value that holds nothing — a number, a string, a function — and so
    /// has nothing to change.
    Plain,
    /// A container made in code `maker` — a literal, a constructed value, a
    /// container a function made and handed over — at `sites`: the
    /// containers it may be.
    Fresh { maker: usize, sites: BTreeSet<Site> },
    /// A harness or product object itself: a write to its members is the
    /// object's owner's.
    Object(Value),
    /// A harness property's value, or anything inside it.
    Shared(usize, usize, String),
    /// A value of the product's.
    Product,
    /// Nothing says where it came from.
    Unknown,
}

/// The maker a container has as what another container holds: none of its
/// own — it is whoever holds the outer container's (`Program::inside_of`).
const HELD: usize = usize::MAX;

impl Origin {
    /// Transfer only what the giving context owns. A borrowed container
    /// keeps its owner through a wrapper or an identity function.
    fn handed_from(self, owner: usize, holder: usize) -> Origin {
        match self {
            made @ Origin::Fresh { maker, .. } if maker == owner => made.handed_to(holder),
            other => other,
        }
    }
    /// The container made at `site` in `maker`.
    fn made(maker: usize, site: Site) -> Origin {
        Origin::Fresh {
            maker,
            sites: BTreeSet::from([site]),
        }
    }

    /// What either of two values may be: a value that may be borrowed is
    /// borrowed, and two different borrowings are unknown.
    pub fn join(self, other: Origin) -> Origin {
        match (self, other) {
            (Origin::Unset, o) | (o, Origin::Unset) => o,
            (a, b) if a == b => a,
            (Origin::Plain, o) | (o, Origin::Plain) => o,
            (
                Origin::Fresh {
                    maker: m1,
                    sites: mut s1,
                },
                Origin::Fresh {
                    maker: m2,
                    sites: s2,
                },
            ) if m1 == m2 => {
                s1.extend(s2);
                Origin::Fresh {
                    maker: m1,
                    sites: s1,
                }
            }
            (Origin::Fresh { .. }, Origin::Fresh { .. }) => Origin::Unknown,
            (Origin::Fresh { .. }, Origin::Product | Origin::Object(Value::Product(_)))
            | (Origin::Product | Origin::Object(Value::Product(_)), Origin::Fresh { .. }) => {
                Origin::Unknown
            }
            (Origin::Fresh { .. }, o) | (o, Origin::Fresh { .. }) => o,
            // Either of two things of the product's is the product's.
            (
                Origin::Product | Origin::Object(Value::Product(_)),
                Origin::Product | Origin::Object(Value::Product(_)),
            ) => Origin::Product,
            _ => Origin::Unknown,
        }
    }

    /// The value as the code `holder` holds it — a value a callee made,
    /// handed back, or what a container of `holder`'s holds: `holder`'s own.
    pub(super) fn handed_to(self, holder: usize) -> Origin {
        match self {
            Origin::Fresh { sites, .. } => Origin::Fresh {
                maker: holder,
                sites,
            },
            other => other,
        }
    }
}

impl Program {
    /// Where the value of the expression `range` of `code` came from.
    pub(super) fn origin_of(&mut self, code: usize, range: Range<usize>) -> Origin {
        let d = self.codes[code].doc;
        let tokens: Vec<Token> = self.docs[d].tokens[range.clone()].to_vec();
        self.origin_in(code, &tokens, range.start)
    }

    /// Where an element of a list — an array's, an argument list's — came
    /// from: a spread (`...x`) is what `x` holds.
    pub(super) fn element_origin(&mut self, code: usize, range: Range<usize>) -> Origin {
        let d = self.codes[code].doc;
        if range.len() > 1 && self.docs[d].tokens[range.start].text == "..." {
            let spread = self.origin_of(code, range.start + 1..range.end);
            return self.inside_of(&spread);
        }
        self.origin_of(code, range)
    }

    /// Puts `value` inside every container `container` may be, for the
    /// next reading to find (`contents`). Nothing is put inside what is not
    /// the code's: the change itself is judged where it is spelled.
    pub(super) fn store(&mut self, container: &Origin, value: Origin) {
        let Origin::Fresh { maker, sites } = container else {
            return;
        };
        for site in sites {
            self.store_at(*maker, *site, value.clone());
        }
    }

    fn store_at(&mut self, maker: usize, site: Site, value: Origin) {
        let joined = self
            .contents_collected
            .remove(&site)
            .unwrap_or(Origin::Unset)
            .join(value.handed_from(maker, HELD));
        self.contents_collected.insert(site, joined);
    }

    /// What a call in `code` puts inside the containers it is handed, by
    /// its contract (`super::builtins`): a method's arguments inside its
    /// receiver (`push`), what a global function's arguments hold inside its
    /// first (`Object.assign`).
    pub(super) fn puts(&mut self, code: usize, path: &Path, args: &[Range<usize>]) {
        let Some(method) = path.last() else {
            return;
        };
        if let Some(object) = self.global_of(code, path) {
            let (Some(from), Some(first)) = (builtins::global_puts(&object, method), args.first())
            else {
                return;
            };
            let container = self.origin_of(code, first.clone());
            self.give(&container, ANY);
            for arg in args.iter().skip(from) {
                let given = self.element_origin(code, arg.clone());
                let held = self.inside_of(&given);
                self.store(&container, held);
            }
            return;
        }
        let (Some(from), Some(receiver)) = (builtins::puts(method), path.receiver()) else {
            return;
        };
        let container = self.path_origin(code, &receiver);
        if !self.language_method(&container, method) {
            return;
        }
        for arg in args.iter().skip(from) {
            let given = self.element_origin(code, arg.clone());
            self.store(&container, given);
        }
    }

    /// What a write in `code` puts inside the value it changes (`origin`):
    /// the value an element or a field is given.
    pub(super) fn written_into(
        &mut self,
        code: usize,
        origin: &Origin,
        kind: &WriteKind,
        in_place: bool,
    ) {
        let given = match kind {
            WriteKind::Put(rhs) | WriteKind::Value(rhs) => self.origin_of(code, rhs.clone()),
            WriteKind::Constant | WriteKind::Step if !in_place => Origin::Plain,
            _ => return,
        };
        self.store(origin, given);
    }

    /// What is inside a value — an element, a field: what was put inside
    /// the containers it may be, its holder's own. Where nothing was put,
    /// nothing yet: the bottom, so that the readings climb to what stands
    /// still, and a change through it is not passed as safe
    /// (`Program::change`). A harness property's insides are its state.
    pub(super) fn inside_of(&self, origin: &Origin) -> Origin {
        match origin {
            Origin::Plain => Origin::Plain,
            Origin::Fresh { maker, sites } => {
                let mut held = Origin::Unset;
                for site in sites {
                    held = held.join(self.contents.get(site).cloned().unwrap_or(Origin::Unset));
                }
                held.handed_from(HELD, *maker)
            }
            Origin::Shared(..) => origin.clone(),
            // A harness object read by an index: a member nothing names.
            Origin::Object(Value::Object(..)) => Origin::Unknown,
            Origin::Object(_) | Origin::Product => Origin::Product,
            // Nothing yet holds nothing yet.
            Origin::Unset => Origin::Unset,
            Origin::Unknown => Origin::Unknown,
        }
    }

    /// `tokens` an expression, its first at index `base` of its file.
    fn origin_in(&mut self, code: usize, tokens: &[Token], base: usize) -> Origin {
        let mut t = tokens;
        let mut base = base;
        while t.len() >= 2
            && t[0].text == "("
            && matching_close(t, 0, "(", ")") == Some(t.len() - 1)
        {
            t = &t[1..t.len() - 1];
            base += 1;
        }
        let Some(first) = t.first() else {
            return Origin::Plain;
        };
        // A conditional, or a choice of operands: either side's value.
        if let Some((question, colon)) = conditional(t) {
            let then = self.origin_in(code, &t[question + 1..colon], base + question + 1);
            let other = self.origin_in(code, &t[colon + 1..], base + colon + 1);
            return then.join(other);
        }
        let choices = split_top(t, &["||", "&&", "??"]);
        if choices.len() > 1 {
            let mut joined = Origin::Unset;
            for (from, to) in choices {
                joined = joined.join(self.origin_in(code, &t[from..to], base + from));
            }
            return joined;
        }
        // Any other operator makes a plain value.
        if computes(t) {
            return Origin::Plain;
        }
        let site = (self.codes[code].doc, base);
        match first.text.as_str() {
            "[" if matching_close(t, 0, "[", "]") == Some(t.len() - 1) => {
                for (from, to) in split_top(&t[1..t.len() - 1], &[","]) {
                    let element = self.element_origin(code, base + 1 + from..base + 1 + to);
                    self.store_at(code, site, element);
                }
                self.made_at(site, Made::Array);
                return Origin::made(code, site);
            }
            "{" if matching_close(t, 0, "{", "}") == Some(t.len() - 1) => {
                for (from, to) in split_top(&t[1..t.len() - 1], &[","]) {
                    let field = &t[1 + from..1 + to];
                    let value_at = field
                        .iter()
                        .position(|tok| tok.text == ":")
                        .map_or(0, |c| c + 1);
                    let at = base + 1 + from + value_at;
                    let value = self.element_origin(code, at..base + 1 + to);
                    self.store_at(code, site, value);
                }
                self.made_at(site, Made::Object);
                return Origin::made(code, site);
            }
            // Constructed: a container nothing is known to be put inside yet.
            "new" => {
                let language = t
                    .get(1)
                    .is_some_and(|ty| CONSTRUCTED.contains(&ty.text.as_str()));
                self.made_at(
                    site,
                    if language {
                        Made::Constructed
                    } else {
                        Made::Borrowing
                    },
                );
                return Origin::made(code, site);
            }
            "function" => return Origin::Plain,
            _ => {}
        }
        if literal(&first.text) && t.len() == 1 {
            return Origin::Plain;
        }
        if function_body(t).is_some() {
            return Origin::Plain;
        }
        self.chain_origin(code, t, base)
    }

    /// A path and what follows it — calls, indexes, members — read left to
    /// right.
    fn chain_origin(&mut self, code: usize, t: &[Token], base: usize) -> Origin {
        let mut k;
        let mut current: Origin;
        // A path not yet resolved, so that a call can be resolved on it.
        let mut pending: Option<Path> = None;
        if is_name(&t[0].text) {
            let mut members = Vec::new();
            k = 1;
            while k + 1 < t.len() && dot(t, k) && is_name(&t[k + 1].text) {
                members.push(t[k + 1].text.clone());
                k += 2;
            }
            pending = Some(Path {
                head: Head::Name(t[0].text.clone()),
                members,
                at: base,
            });
            current = Origin::Unset;
        } else if literal(&t[0].text) || matches!(t[0].text.as_str(), "(" | "[" | "{") {
            let end = match t[0].text.as_str() {
                "(" => matching_close(t, 0, "(", ")"),
                "[" => matching_close(t, 0, "[", "]"),
                "{" => matching_close(t, 0, "{", "}"),
                _ => Some(0),
            };
            let Some(end) = end else {
                return Origin::Unknown;
            };
            current = self.origin_in(code, &t[..=end], base);
            k = end + 1;
        } else {
            return Origin::Unknown;
        }
        while k < t.len() {
            match t[k].text.as_str() {
                "(" => {
                    let Some(close) = matching_close(t, k, "(", ")") else {
                        return Origin::Unknown;
                    };
                    let args = base + k + 1..base + close;
                    current = match pending.take() {
                        Some(path) => self.call_origin(code, &path, args),
                        None => Origin::Unknown,
                    };
                    k = close + 1;
                }
                "[" => {
                    let Some(close) = matching_close(t, k, "[", "]") else {
                        return Origin::Unknown;
                    };
                    if let Some(path) = pending.take() {
                        current = self.path_origin(code, &path);
                    }
                    current = self.inside_of(&current);
                    k = close + 1;
                }
                "." | "?" if dot(t, k) => {
                    let name_at = if t[k].text == "?" { k + 2 } else { k + 1 };
                    let Some(name) = t.get(name_at).map(|tok| tok.text.clone()) else {
                        return Origin::Unknown;
                    };
                    let method = t.get(name_at + 1).is_some_and(|tok| tok.text == "(");
                    match pending.as_mut() {
                        Some(path) => path.members.push(name),
                        None if method => {
                            let close = matching_close(t, name_at + 1, "(", ")").unwrap_or(t.len());
                            let site = (self.codes[code].doc, base + name_at);
                            let args = base + name_at + 2..base + close;
                            current = self.method_origin(code, current, &name, site, args);
                            k = close + 1;
                            continue;
                        }
                        None => current = self.member_origin(current, &name),
                    }
                    k = name_at + 1;
                }
                _ => return Origin::Unknown,
            }
        }
        match pending {
            Some(path) => self.path_origin(code, &path),
            None => current,
        }
    }

    /// Where the value a path names in `code` came from.
    pub(super) fn path_origin(&mut self, code: usize, path: &Path) -> Origin {
        let mut origin = match &path.head {
            Head::Name(name) if name == "this" => self.this_origin(code, path.at),
            Head::Name(name) => self.name_origin(code, name, path.at),
            Head::Value => self.value_head_origin(code, path.at),
        };
        for name in &path.members {
            origin = self.member_origin(origin, name);
        }
        origin
    }

    /// A name's value: a local's or parameter's of `code` or of a code it
    /// was carved from, or what the file's scope names.
    fn name_origin(&mut self, code: usize, name: &str, at: usize) -> Origin {
        let mut current = Some(code);
        while let Some(c) = current {
            let (d, u, range, parent) = {
                let c = &self.codes[c];
                (c.doc, c.unit, c.range.clone(), c.parent)
            };
            if self.facts_of(d, u).within(&range).local(name, at).is_some()
                || (parent.is_none()
                    && self.docs[d].units[u]
                        .bound
                        .iter()
                        .any(|(_, b)| b.name == name))
            {
                return self.local_origin(c, name, at);
            }
            current = parent;
        }
        match self.head(code, name, at) {
            Value::Object(d, o) => Origin::Object(Value::Object(d, o)),
            Value::Prop(d, o, p) => self.prop_origin(d, o, &p),
            Value::Product(Some(ty)) if ty.chars().next().is_some_and(char::is_uppercase) => {
                Origin::Object(Value::Product(Some(ty)))
            }
            Value::Product(_) => Origin::Product,
            Value::Function(..) | Value::Global(_) => Origin::Plain,
            _ => Origin::Unknown,
        }
    }

    /// A harness property's value: the product object its type names, or
    /// the harness's own state.
    fn prop_origin(&mut self, d: usize, o: usize, p: &str) -> Origin {
        if d >= self.harness {
            return Origin::Product;
        }
        match self.prop_type(d, o, p) {
            Some(ty) if ty.chars().next().is_some_and(char::is_uppercase) => {
                return Origin::Object(Value::Product(Some(ty)));
            }
            Some(_) => return Origin::Shared(d, o, p.to_string()),
            None => {}
        }
        let key = (d, o, p.to_string());
        if let Some(known) = self.prop_origins.get(&key) {
            return known.clone();
        }
        self.prop_origins.insert(key.clone(), Origin::Unset);
        let given = self.given_origin(d, o, p);
        let origin = match given {
            // Given nothing by the harness: handed in by the product (`window`, `settingsDialog`).
            Origin::Unset => Origin::Product,
            // What the harness's own code made is the harness's state.
            Origin::Fresh { .. } | Origin::Plain => Origin::Shared(d, o, p.to_string()),
            other => other,
        };
        self.prop_origins.insert(key, origin.clone());
        origin
    }

    /// What a harness property typed `var` is given, as the harness's
    /// own state where the harness made it (`Program::assigned_values`).
    fn given_origin(&mut self, d: usize, o: usize, p: &str) -> Origin {
        // What the harness made and gave the property is the property's own state.
        let own = |origin: Origin| match origin {
            Origin::Fresh { .. } | Origin::Plain => Origin::Shared(d, o, p.to_string()),
            other => other,
        };
        let mut given = Origin::Unset;
        for assigned in self.assigned_values(d, o, p) {
            let origin = match assigned {
                Assigned::Spelled(sd, so, range) => self.object_value_origin(sd, so, range),
                Assigned::InCode(code, rhs) => self.origin_of(code, rhs),
                Assigned::Plain => Origin::Plain,
            };
            given = given.join(own(origin));
        }
        given
    }

    /// The origin of a value an object binds, at the object level of file
    /// `d`: a container or a plain value the file writes out, or what a path
    /// names.
    fn object_value_origin(&mut self, d: usize, o: usize, range: Range<usize>) -> Origin {
        let mut range = range;
        while range.len() >= 2
            && self.docs[d].tokens[range.start].text == "("
            && matching_close(&self.docs[d].tokens, range.start, "(", ")") == Some(range.end - 1)
        {
            range = range.start + 1..range.end - 1;
        }
        let tokens = &self.docs[d].tokens[range.clone()];
        let Some(first) = tokens.first() else {
            return Origin::Unset;
        };
        // A block: what it returns.
        if first.text == "{" && tokens.iter().any(|t| t.text == "return") {
            let returns = super::scan::facts(&self.docs[d].tokens, range).returns;
            let mut joined = Origin::Unset;
            for (value, _) in returns {
                joined = joined.join(self.object_value_origin(d, o, value));
            }
            return joined;
        }
        if matches!(first.text.as_str(), "null" | "undefined") && tokens.len() == 1 {
            return Origin::Unset;
        }
        if matches!(first.text.as_str(), "[" | "{" | "new")
            || (literal(&first.text) && tokens.len() == 1)
        {
            return Origin::Plain;
        }
        match self.path_value_at(d, o, range) {
            Some(Value::Object(vd, vo)) => Origin::Object(Value::Object(vd, vo)),
            Some(Value::Prop(pd, po, pp)) => self.prop_origin(pd, po, &pp),
            Some(Value::Product(_)) => Origin::Product,
            Some(Value::Function(..) | Value::Global(_)) => Origin::Plain,
            // A call or an expression the object's scope computes: of the product's when it reads the product.
            _ => Origin::Product,
        }
    }

    fn member_origin(&mut self, origin: Origin, name: &str) -> Origin {
        if builtins::PLAIN_MEMBERS.contains(&name)
            && matches!(
                origin,
                Origin::Plain | Origin::Fresh { .. } | Origin::Shared(..)
            )
            && self.language_member(&origin, name)
        {
            return Origin::Plain;
        }
        match origin {
            Origin::Object(Value::Object(d, o)) if d < self.harness => {
                match self.member(d, o, name) {
                    Some(Value::Object(md, mo)) => Origin::Object(Value::Object(md, mo)),
                    Some(Value::Prop(pd, po, p)) => self.prop_origin(pd, po, &p),
                    Some(Value::Function(..)) => Origin::Unknown,
                    // A property the harness object's type has of its own (`running`, `width`): its state.
                    _ => Origin::Shared(d, o, name.to_string()),
                }
            }
            Origin::Object(_) | Origin::Product => Origin::Product,
            Origin::Plain | Origin::Fresh { .. } | Origin::Shared(..) => self.inside_of(&origin),
            Origin::Unset => Origin::Unset,
            Origin::Unknown => Origin::Unknown,
        }
    }

    /// The value before the `.` at `dot`: the call, index or literal the
    /// path reads off.
    fn value_head_origin(&mut self, code: usize, dot_at: usize) -> Origin {
        let d = self.codes[code].doc;
        let tokens = &self.docs[d].tokens;
        let mut start = dot_at;
        // Back over `(…)` and `[…]` and the path before them.
        while let Some(before) = start.checked_sub(1) {
            match tokens[before].text.as_str() {
                ")" | "]" => {
                    let (open, close) = if tokens[before].text == ")" {
                        ("(", ")")
                    } else {
                        ("[", "]")
                    };
                    match opening(tokens, before, open, close) {
                        Some(at) => start = at,
                        None => return Origin::Unknown,
                    }
                }
                text if is_name(text) => {
                    start = before;
                    if start >= 1 && matches!(tokens[start - 1].text.as_str(), ".") {
                        start -= 1;
                        continue;
                    }
                    break;
                }
                text if literal(text) => {
                    start = before;
                    break;
                }
                _ => break,
            }
        }
        if start == dot_at {
            return Origin::Unknown;
        }
        self.origin_of(code, start..dot_at)
    }

    /// What a call answers; `args` are the tokens between its parentheses.
    fn call_origin(&mut self, code: usize, path: &Path, args: Range<usize>) -> Origin {
        let Some(method) = path.last().map(str::to_string) else {
            return Origin::Unknown;
        };
        let site = (self.codes[code].doc, path.at);
        let listed = self.arguments(code, &args);
        if let Some(head) = self.global_of(code, path) {
            let spread = self.spreads(code, &listed);
            return match builtins::global(&head, &method, listed.len(), spread) {
                Some((answers, made)) => {
                    self.made_at(site, made);
                    self.global_answer(code, answers, site, &listed)
                }
                None => Origin::Unknown,
            };
        }
        match self.resolve(code, path) {
            // The harness's function or the product's: what its own code returns, read as the caller's.
            Value::Function(fd, fu) => {
                if self.declared.contains_key(&(fd, fu)) {
                    return Origin::Unknown;
                }
                {
                    let callee = self.code_of(fd, fu);
                    self.return_origin(fd, fu).handed_from(callee, code)
                }
            }
            Value::Global(g) if builtins::PLAIN_FUNCTIONS.contains(&g.as_str()) => Origin::Plain,
            Value::Global(g) if builtins::NEW_FUNCTIONS.contains(&g.as_str()) => {
                self.made_at(site, Made::Constructed);
                Origin::made(code, site)
            }
            _ if path.members.is_empty() => Origin::Unknown,
            _ => {
                let receiver = path.receiver().unwrap_or_else(|| path.clone());
                let receiver_origin = self.path_origin(code, &receiver);
                self.method_origin(code, receiver_origin, &method, site, args)
            }
        }
    }

    /// Whether any of a call's arguments is spread, so that their count is
    /// not known.
    fn spreads(&self, code: usize, args: &[Range<usize>]) -> bool {
        let d = self.codes[code].doc;
        args.iter().any(|arg| {
            self.docs[d]
                .tokens
                .get(arg.start)
                .is_some_and(|t| t.text == "...")
        })
    }

    /// The elements of a call's argument list, `args` its tokens.
    fn arguments(&self, code: usize, args: &Range<usize>) -> Vec<Range<usize>> {
        let d = self.codes[code].doc;
        split_top(&self.docs[d].tokens[args.clone()], &[","])
            .into_iter()
            .map(|(from, to)| args.start + from..args.start + to)
            .collect()
    }

    /// What a global object's function answers, by its contract.
    fn global_answer(
        &mut self,
        code: usize,
        answers: GlobalAnswers,
        site: Site,
        args: &[Range<usize>],
    ) -> Origin {
        let first = |program: &mut Program| match args.first() {
            Some(arg) => program.origin_of(code, arg.clone()),
            None => Origin::Unknown,
        };
        match answers {
            GlobalAnswers::Plain => Origin::Plain,
            GlobalAnswers::NewPlain => {
                self.store_at(code, site, Origin::Plain);
                Origin::made(code, site)
            }
            GlobalAnswers::NewInsideOfFirst => {
                let first = first(self);
                let held = self.inside_of(&first);
                self.store_at(code, site, held);
                Origin::made(code, site)
            }
            GlobalAnswers::NewOfArguments => {
                for arg in args {
                    let element = self.element_origin(code, arg.clone());
                    self.store_at(code, site, element);
                }
                Origin::made(code, site)
            }
            GlobalAnswers::NewOfCallback(at) => {
                let answered = self.callback_answers(code, args[at].clone());
                self.store_at(code, site, answered);
                Origin::made(code, site)
            }
            GlobalAnswers::NewDeep => {
                self.store_at(code, site, Origin::made(HELD, site));
                Origin::made(code, site)
            }
            GlobalAnswers::First => first(self),
        }
    }

    /// What `method` called on a value of `receiver`'s origin answers, by
    /// its contract; `site` is where a new container it answers is made.
    fn method_origin(
        &mut self,
        code: usize,
        receiver: Origin,
        method: &str,
        site: Site,
        args: Range<usize>,
    ) -> Origin {
        if matches!(receiver, Origin::Object(_) | Origin::Product) {
            return Origin::Product;
        }
        if receiver == Origin::Unset {
            return Origin::Unset;
        }
        if !self.language_method(&receiver, method) {
            return Origin::Unknown;
        }
        let args = self.arguments(code, &args);
        let Some(answers) = builtins::method(method, args.len(), self.spreads(code, &args)) else {
            return Origin::Unknown;
        };
        // A method of the language's that answers a new container answers an array.
        self.made_at(site, Made::Array);
        match answers {
            Answers::Plain => Origin::Plain,
            Answers::NewPlain => {
                self.store_at(code, site, Origin::Plain);
                Origin::made(code, site)
            }
            Answers::NewInside => {
                let held = self.inside_of(&receiver);
                self.store_at(code, site, held);
                Origin::made(code, site)
            }
            Answers::NewJoined => {
                let held = self.inside_of(&receiver);
                self.store_at(code, site, held);
                for arg in args {
                    let element = self.element_origin(code, arg);
                    let inside = self.inside_of(&element);
                    self.store_at(code, site, element);
                    self.store_at(code, site, inside);
                }
                Origin::made(code, site)
            }
            Answers::NewOfCallback(at) => {
                let answered = self.callback_answers(code, args[at].clone());
                self.store_at(code, site, answered);
                Origin::made(code, site)
            }
            Answers::NewOfCallbackFlat(at) => {
                let answered = self.callback_answers(code, args[at].clone());
                let flattened = self.inside_of(&answered);
                self.store_at(code, site, answered);
                self.store_at(code, site, flattened);
                Origin::made(code, site)
            }
            Answers::Inside => self.inside_of(&receiver),
            Answers::Receiver => receiver,
        }
    }

    /// What a function handed to a call answers: a function spelled there
    /// answers its returned values, read as the code it is spelled in reads
    /// them — its parameters by what the call hands them (`param_origin`);
    /// a harness or product function named there answers what its code
    /// returns; anything else answers a value of no known origin.
    pub(super) fn callback_answers(&mut self, code: usize, arg: Range<usize>) -> Origin {
        let d = self.codes[code].doc;
        if let Some(body) = function_body(&self.docs[d].tokens[arg.clone()]) {
            let body = arg.start + body.start..arg.start + body.end;
            let tokens = &self.docs[d].tokens;
            let block = tokens[body.start].text == "{"
                && matching_close(tokens, body.start, "{", "}") == Some(body.end - 1);
            if !block {
                return self.origin_of(code, body);
            }
            let returns = super::scan::facts(tokens, body).returns;
            let mut answered = Origin::Unset;
            for (value, _) in returns {
                answered = answered.join(self.origin_of(code, value));
            }
            // Returning nothing answers `undefined`.
            return if answered == Origin::Unset {
                Origin::Plain
            } else {
                answered
            };
        }
        if self.names_plain_function(code, arg.clone()) {
            return Origin::Plain;
        }
        match self.named_callback(code, arg) {
            Some((fd, fu)) => {
                let callee = self.code_of(fd, fu);
                self.return_origin(fd, fu).handed_from(callee, code)
            }
            None => Origin::Unknown,
        }
    }

    /// Where a local of `code` came from: its value and every value given
    /// it after. What is put inside it is its containers' (`contents`).
    fn local_origin(&mut self, code: usize, name: &str, at: usize) -> Origin {
        let (d, u, range) = {
            let c = &self.codes[code];
            (c.doc, c.unit, c.range.clone())
        };
        let facts = self.facts_of(d, u).within(&range);
        let local = facts.local(name, at).cloned();
        let parameter = self.docs[d].units[u]
            .bound
            .iter()
            .find(|(_, b)| b.name == name)
            .cloned()
            .filter(|_| local.is_none() && self.codes[code].parent.is_none());
        let declaration = local
            .as_ref()
            .map(|l| l.at)
            .or_else(|| parameter.as_ref().map(|(_, b)| b.at))
            .unwrap_or(at);
        let key = (code, declaration);
        if let Some(known) = self.local_origins.get(&key) {
            return known.clone();
        }
        // Asked again while being read (`x = x.concat(…)`): nothing yet.
        self.local_origins.insert(key, Origin::Unset);
        let mut origin = Origin::Unset;
        let mut given = false;
        if let Some((i, bound)) = parameter {
            let handed = self
                .origin_bindings
                .get(&(d, u, i))
                .cloned()
                .unwrap_or(Origin::Unset);
            origin = match bound.depth {
                Some(depth) => self.inside_times(handed, depth),
                None => Origin::Unknown,
            };
            for (default, depth) in bound.defaults {
                let value = self.origin_of(code, default);
                origin = origin.join(self.inside_times(value, depth));
            }
            given = true;
        }
        for local in local.iter() {
            let Some(depth) = local.depth else {
                origin = origin.join(Origin::Unknown);
                given = true;
                continue;
            };
            let whole = if local.param {
                self.param_origin(code, local.at)
            } else if let Some(init) = local.init.clone() {
                self.origin_of(code, init)
            } else {
                continue;
            };
            // A name a pattern binds is what the value holds, as deep as the
            // pattern reaches, or a default where it holds nothing.
            let mut bound = self.inside_times(whole, depth);
            for (default, inside) in local.defaults.clone() {
                let given = self.origin_of(code, default);
                bound = bound.join(self.inside_times(given, inside));
            }
            origin = origin.join(bound);
            given = true;
        }
        for write in &facts.writes {
            if write.path.head != Head::Name(name.to_string()) || !write.path.members.is_empty() {
                continue;
            }
            if facts.local(name, write.path.at).map(|l| l.at) != local.as_ref().map(|l| l.at) {
                continue;
            }
            match &write.kind {
                WriteKind::Value(rhs) => {
                    origin = origin.join(self.origin_of(code, rhs.clone()));
                    given = true;
                }
                WriteKind::Constant => {
                    origin = origin.join(Origin::Plain);
                    given = true;
                }
                WriteKind::Step | WriteKind::Mutate | WriteKind::Put(_) => {}
            }
        }
        let origin = if given { origin } else { Origin::Unknown };
        self.local_origins.insert(key, origin.clone());
        origin
    }

    /// What `origin` holds, `times` containers deep.
    pub(super) fn inside_times(&mut self, origin: Origin, times: usize) -> Origin {
        (0..times).fold(origin, |held, _| self.inside_of(&held))
    }

    /// Where the value a function of the harness or the product returns came
    /// from, as its own code made or holds it.
    fn return_origin(&mut self, d: usize, u: usize) -> Origin {
        if let Some(known) = self.return_origins.get(&(d, u)) {
            return known.clone();
        }
        self.return_origins.insert((d, u), Origin::Unset);
        let code = self.code_of(d, u);
        let range = self.codes[code].range.clone();
        let returns = self.facts_of(d, u).within(&range).returns;
        let mut origin = Origin::Unset;
        for (value, _) in returns {
            origin = origin.join(self.origin_of(code, value));
        }
        if origin == Origin::Unset {
            origin = Origin::Plain;
        }
        self.return_origins.insert((d, u), origin.clone());
        origin
    }
}

/// Whether a `.` (or `?.`) stands at `k`.
fn dot(t: &[Token], k: usize) -> bool {
    t[k].text == "." || (t[k].text == "?" && t.get(k + 1).is_some_and(|n| n.text == "."))
}

pub(super) fn literal(text: &str) -> bool {
    matches!(text, "true" | "false" | "null" | "undefined")
        || text.starts_with('"')
        || text.starts_with('\'')
        || text.starts_with('`')
        || (text.starts_with('/') && text.len() > 1)
        || text.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// The `?` and `:` of a conditional at the top level of `t`.
fn conditional(t: &[Token]) -> Option<(usize, usize)> {
    let mut depth = 0i32;
    let mut question = None;
    let mut nested = 0;
    for (k, tok) in t.iter().enumerate() {
        match tok.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            "?" if depth == 0 && !t.get(k + 1).is_some_and(|n| n.text == ".") => {
                if question.is_none() {
                    question = Some(k);
                } else {
                    nested += 1;
                }
            }
            ":" if depth == 0 && question.is_some() => {
                if nested == 0 {
                    return question.map(|q| (q, k));
                }
                nested -= 1;
            }
            _ => {}
        }
    }
    None
}

/// The stretches of `t` between its top-level `separators`.
fn split_top(t: &[Token], separators: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut from = 0;
    for (k, tok) in t.iter().enumerate() {
        match tok.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            text if depth == 0 && separators.contains(&text) => {
                out.push((from, k));
                from = k + 1;
            }
            _ => {}
        }
    }
    if from < t.len() {
        out.push((from, t.len()));
    }
    out
}

/// Whether `t` computes a plain value at its top level: an arithmetic,
/// comparing or negating operator outside every bracket.
fn computes(t: &[Token]) -> bool {
    if matches!(
        t[0].text.as_str(),
        "!" | "-" | "+" | "typeof" | "void" | "~"
    ) {
        return true;
    }
    let mut depth = 0i32;
    for tok in t {
        match tok.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            "+" | "-" | "*" | "/" | "%" | "===" | "!==" | "==" | "!=" | "<" | ">" | "<=" | ">="
            | "instanceof" | "in" | "&" | "|" | "^" | "<<" | ">>"
                if depth == 0 =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// The index of the `open` that the `close` at `at` closes, read back.
fn opening(tokens: &[Token], at: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0;
    for k in (0..=at).rev() {
        if tokens[k].text == close {
            depth += 1;
        } else if tokens[k].text == open {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}
