//! Whether a member of a value the code holds is the language's own — a
//! method of an array or a string, its `length` — or the value's: the
//! language's contracts (`super::builtins`) answer for a member only where
//! the value is one the language made, and nothing gives it a member of
//! that name of its own (`x.slice = f`, `Object.assign(x, …)`, a prototype
//! set). What a container is, its maker's contract says, recorded where it
//! was made (`builtins::Made`): that a language function made it says
//! nothing of its members — `Object.create(p)`'s are `p`'s. A string or a
//! number, and a harness property given nothing else, are the language's
//! too. An object the code spells holds its members by its keys: none of
//! them is the language's. What a call does, what it answers, what it puts
//! inside its receiver and what it calls back are all asked here, so one
//! resolution answers for every judgement.

use std::ops::Range;

use super::builtins::{self, CONSTRUCTED, Made};
use super::origin::{Origin, Site, literal};
use super::program::{Program, Value};
use super::scan::{Head, Path, WriteKind};
use crate::lex::matching_close;

/// What holds members of its own: a container, by the site that made it; a
/// harness property's value; the plain values (a function given a member).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Holder {
    Site(Site),
    Shared(usize, usize, String),
    Plain,
}

/// The name of a member given by a name nothing here reads
/// (`Object.assign`): any.
pub(super) const ANY: &str = "*";

/// A value a harness property is given.
pub(super) enum Assigned {
    /// Spelled at the object level of file `d`'s object `o`: the property's
    /// own value, or an instance's binding of it.
    Spelled(usize, usize, Range<usize>),
    /// Assigned in code: the code, and the value's tokens.
    InCode(usize, Range<usize>),
    /// A literal, or a step.
    Plain,
}

/// The holders a value of `origin` is.
fn holders(origin: &Origin) -> Vec<Holder> {
    match origin {
        Origin::Fresh { sites, .. } => sites.iter().map(|site| Holder::Site(*site)).collect(),
        Origin::Shared(d, o, p) => vec![Holder::Shared(*d, *o, p.clone())],
        Origin::Plain => vec![Holder::Plain],
        _ => Vec::new(),
    }
}

impl Program {
    /// Records, for the next reading, that a value of `origin` is given a
    /// member `name` of its own ([`ANY`] for one nothing here names, and
    /// for a prototype: every member is then another's).
    pub(super) fn give(&mut self, origin: &Origin, name: &str) {
        let name = if name == "__proto__" { ANY } else { name };
        for holder in holders(origin) {
            self.members_collected
                .entry(holder)
                .or_default()
                .insert(name.to_string());
        }
    }

    /// What the container made at `site` is, as the contract that made it
    /// says (`builtins::Made`).
    pub(super) fn made_at(&mut self, site: Site, made: Made) {
        self.kinds.insert(site, made);
    }

    /// Whether `method` called on a value of `origin` is the language's own
    /// method: one its tables hold (`builtins::is_method`), of a value the
    /// language made, which holds no member of that name of its own.
    pub(super) fn language_method(&mut self, origin: &Origin, method: &str) -> bool {
        builtins::is_method(method) && self.language_member(origin, method)
    }

    /// Whether the member `name` of a value of `origin` is the language's.
    pub(super) fn language_member(&mut self, origin: &Origin, name: &str) -> bool {
        if holders(origin)
            .iter()
            .any(|holder| self.given(holder, name))
        {
            return false;
        }
        match origin {
            Origin::Plain => true,
            // Nothing yet: what it comes to hold decides.
            Origin::Unset => true,
            Origin::Fresh { sites, .. } => sites.iter().all(|site| self.site_by_language(*site)),
            Origin::Shared(d, o, p) => self.shared_by_language(*d, *o, p, name),
            _ => false,
        }
    }

    /// Whether the reading before found a member `name` given `holder`.
    fn given(&self, holder: &Holder, name: &str) -> bool {
        self.members
            .get(holder)
            .is_some_and(|names| names.contains(name) || names.contains(ANY))
    }

    /// Whether the container made at `site` is the language's, as what made
    /// it says (`made_at`): an array, a value read from JSON, a language
    /// constructor's. An object the code spells, one made from another, and
    /// one nothing recorded the making of are not.
    fn site_by_language(&self, site: Site) -> bool {
        matches!(
            self.kinds.get(&site),
            Some(Made::Array | Made::Json | Made::Constructed)
        )
    }

    /// Whether every value a harness property is given is the language's,
    /// with no member `name` of its own: a value type's always is. Asked
    /// again while being answered — a value given it reads it back —, what
    /// else it is given decides; an answer reached with none such pending is
    /// kept for the reading.
    fn shared_by_language(&mut self, d: usize, o: usize, p: &str, name: &str) -> bool {
        if self.prop_type(d, o, p).is_some() {
            return true;
        }
        let key = (d, o, p.to_string());
        if self.proving.contains(&key) {
            return true;
        }
        let known = (d, o, p.to_string(), name.to_string());
        if let Some(answer) = self.language_props.get(&known) {
            return *answer;
        }
        let first = self.proving.is_empty();
        self.proving.push(key);
        let mut all = true;
        for assigned in self.assigned_values(d, o, p) {
            all = all
                && match assigned {
                    Assigned::Plain => true,
                    Assigned::Spelled(sd, so, range) => {
                        self.spelled_by_language(sd, so, range, name)
                    }
                    Assigned::InCode(code, rhs) => {
                        let origin = self.origin_of(code, rhs);
                        self.language_member(&origin, name)
                    }
                };
        }
        self.proving.pop();
        if first {
            self.language_props.insert(known, all);
        }
        all
    }

    /// Whether a value spelled at an object's level is the language's: an
    /// array, a literal, nothing, a language constructor's value, another
    /// harness property's that is.
    fn spelled_by_language(&mut self, d: usize, o: usize, range: Range<usize>, name: &str) -> bool {
        let mut range = range;
        while range.len() >= 2
            && self.docs[d].tokens[range.start].text == "("
            && matching_close(&self.docs[d].tokens, range.start, "(", ")") == Some(range.end - 1)
        {
            range = range.start + 1..range.end - 1;
        }
        let tokens = &self.docs[d].tokens[range.clone()];
        match tokens.first().map(|t| t.text.as_str()) {
            Some("[") => true,
            Some("{") | None => false,
            Some("new") => tokens
                .get(1)
                .is_some_and(|t| CONSTRUCTED.contains(&t.text.as_str())),
            Some(first) if tokens.len() == 1 && literal(first) => true,
            Some(_) => match self.path_value_at(d, o, range) {
                Some(Value::Prop(pd, po, pp)) => {
                    !self.given(&Holder::Shared(pd, po, pp.clone()), name)
                        && self.shared_by_language(pd, po, &pp, name)
                }
                _ => false,
            },
        }
    }

    /// What a harness property typed `var` is given: its own value, the
    /// value each instance binds it to, and each value the harness's code
    /// assigns it.
    pub(super) fn assigned_values(&mut self, d: usize, o: usize, p: &str) -> Vec<Assigned> {
        let mut assigned = Vec::new();
        let value = self.docs[d].objects[o]
            .props
            .iter()
            .find(|prop| prop.name == p)
            .and_then(|prop| prop.value.clone());
        if let Some(range) = value {
            assigned.push(Assigned::Spelled(d, o, range));
        }
        for d2 in 0..self.harness {
            for o2 in 0..self.docs[d2].objects.len() {
                let object = &self.docs[d2].objects[o2];
                if o != 0 || object.ty != self.docs[d].name {
                    continue;
                }
                if let Some((_, range)) = object.bindings.iter().find(|(b, _)| b == p).cloned() {
                    assigned.push(Assigned::Spelled(d2, o2, range));
                }
            }
        }
        for d2 in 0..self.harness {
            for u2 in 0..self.docs[d2].units.len() {
                let unit_object = self.docs[d2].units[u2].object;
                let writes: Vec<(Path, WriteKind)> = {
                    let body = self.docs[d2].units[u2].body.clone();
                    self.facts_of(d2, u2)
                        .within(&body)
                        .writes
                        .into_iter()
                        .map(|w| (w.path, w.kind))
                        .collect()
                };
                for (path, kind) in writes {
                    let assigns = match (&path.head, path.members.as_slice()) {
                        (Head::Name(head), [member]) => {
                            member == p
                                && self.scope_value(d2, unit_object, head) == Value::Object(d, o)
                        }
                        (Head::Name(head), []) => {
                            head == p && d2 == d && (unit_object == o || o == 0)
                        }
                        _ => false,
                    };
                    if !assigns {
                        continue;
                    }
                    match kind {
                        WriteKind::Value(rhs) => {
                            let code = self.code_of(d2, u2);
                            assigned.push(Assigned::InCode(code, rhs));
                        }
                        WriteKind::Constant | WriteKind::Step => assigned.push(Assigned::Plain),
                        WriteKind::Mutate | WriteKind::Put(_) => {}
                    }
                }
            }
        }
        assigned
    }
}
