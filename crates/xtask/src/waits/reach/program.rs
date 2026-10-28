//! The harness read as one program: each name resolved to what it stands
//! for, each call to what it calls, and each stretch of code to what it
//! does ([`Effect`]) — a harness function's effects being its caller's, a
//! function of the machinery's answering by the reach it declares
//! (`// reach(…)`), and the product answering by `super::product`.

use std::collections::HashMap;
use std::ops::Range;

use super::builtins;
use super::origin::{Origin, Site};
use super::parse::{Doc, UnitKind, is_name};
use super::product::{self, Product};
use super::scan::{Facts, Head, Path, WriteKind};
use super::{Context, Declared};

/// What a name stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Value {
    /// A harness object: its file and its index there.
    Object(usize, usize),
    /// A property of a harness object whose value is no harness object.
    Prop(usize, usize, String),
    /// A harness or product function: file and unit.
    Function(usize, usize),
    /// Something of the product's, with its type where it is known.
    Product(Option<String>),
    /// A global object of the language's or Qt's (`Math`, `Qt`).
    Global(String),
    /// A value of the stretch's own.
    Local,
    /// A local or parameter of a stretch this one was carved out of,
    /// with the harness object it stands for where it is one.
    Captured(String, Option<Box<Value>>),
    Unknown,
}

/// What a stretch of code does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Effect {
    /// A harness object's property given a value; `config` where the code
    /// starts that object's operation after it (a start condition).
    Write {
        target: (usize, usize, String),
        step: bool,
        config: bool,
    },
    /// A local of the stretch it was carved from, written from inside.
    Smuggle(String),
    /// The product written or pressed.
    Input(String),
    /// A value of the stretch it was carved from, read inside.
    Captured(String),
    /// A harness function's code, whose effects are the caller's.
    Calls(usize),
    /// A function of the machinery's, by the reach it declares.
    Declared(String, Vec<Context>),
    /// An operation started, stopped or restarted.
    Start,
    /// A call or a write this check cannot follow.
    Unread(String),
}

/// A stretch of code: a function's or handler's body, or an argument
/// carved out of one for a context of its own.
pub(super) struct Code {
    pub doc: usize,
    pub unit: usize,
    pub range: Range<usize>,
    pub parent: Option<usize>,
    /// What it does, each with its line; filled by [`Program::read`].
    pub effects: Vec<(Effect, usize)>,
    /// The paths it holds against a count, resolved.
    pub compares: Vec<(Value, String, usize)>,
}

/// A stretch read in a context of its own.
pub(super) struct Root {
    pub code: usize,
    pub context: Context,
}

pub(super) struct Program {
    pub docs: Vec<Doc>,
    /// `docs[..harness]` are the harness's; the rest the product's.
    pub harness: usize,
    pub machinery: Vec<&'static str>,
    pub codes: Vec<Code>,
    pub roots: Vec<Root>,
    /// The top code of each unit, by `(doc, unit)`.
    top: HashMap<(usize, usize), usize>,
    facts: HashMap<(usize, usize), Facts>,
    pub(super) types: HashMap<String, usize>,
    pub(super) product: Product,
    pub declared: HashMap<(usize, usize), Declared>,
    /// Markers that are not where a declaration may stand, or that do not read.
    pub misdeclared: Vec<(usize, usize)>,
    prop_values: std::cell::RefCell<PropValues>,
    /// What each caller hands a harness function's parameter, by `(doc,
    /// unit, index)`: the one object every call names, or `None` where calls
    /// differ — read in a first pass, used by the second ([`Program::again`]).
    bindings: HashMap<(usize, usize, usize), Option<Value>>,
    /// The same, as the reading under way finds it.
    collected: HashMap<(usize, usize, usize), Option<Value>>,
    /// Where what each caller hands a harness function's parameter came
    /// from, joined over the callers — read and used as `bindings` are.
    pub(super) origin_bindings: HashMap<(usize, usize, usize), Origin>,
    origin_collected: HashMap<(usize, usize, usize), Origin>,
    /// Each local's and each function's return's origin, once read in this
    /// reading (`super::origin`).
    pub(super) local_origins: HashMap<(usize, String), Origin>,
    pub(super) return_origins: HashMap<(usize, usize), Origin>,
    pub(super) prop_origins: HashMap<(usize, usize, String), Origin>,
    /// What each container holds, by the site that made it, as the reading
    /// before found every put — read by this reading (`super::origin`).
    pub(super) contents: HashMap<Site, Origin>,
    /// The same, as the reading under way puts it.
    pub(super) contents_collected: HashMap<Site, Origin>,
}

/// The harness object each property holds, by `(doc, object, name)`, once
/// read; `None` for one that holds none.
type PropValues = HashMap<(usize, usize, String), Option<(usize, usize)>>;

/// The operation's type, whose functions are the operations' API.
pub(super) const OPERATION: &str = "StageTimer";

impl Program {
    pub fn new(
        docs: Vec<Doc>,
        harness: usize,
        slots: Vec<(String, String, bool)>,
        machinery: Vec<&'static str>,
    ) -> Program {
        let mut types = HashMap::new();
        for (d, doc) in docs.iter().enumerate() {
            types.entry(doc.name.clone()).or_insert(d);
        }
        let product = Product::new(&docs, harness, slots);
        let mut program = Program {
            docs,
            harness,
            machinery,
            codes: Vec::new(),
            roots: Vec::new(),
            top: HashMap::new(),
            facts: HashMap::new(),
            types,
            product,
            declared: HashMap::new(),
            misdeclared: Vec::new(),
            prop_values: std::cell::RefCell::new(HashMap::new()),
            bindings: HashMap::new(),
            collected: HashMap::new(),
            origin_bindings: HashMap::new(),
            origin_collected: HashMap::new(),
            local_origins: HashMap::new(),
            return_origins: HashMap::new(),
            prop_origins: HashMap::new(),
            contents: HashMap::new(),
            contents_collected: HashMap::new(),
        };
        program.declarations();
        program
    }

    pub fn is_machinery(&self, d: usize) -> bool {
        let name = format!("{}.qml", self.docs[d].name);
        self.machinery.iter().any(|m| *m == name)
    }

    /// Reads each function's `// reach(…)` marker.
    fn declarations(&mut self) {
        for d in 0..self.harness {
            for u in 0..self.docs[d].units.len() {
                let unit = &self.docs[d].units[u];
                let Some(marker) = marker_above(&self.docs[d].lines, unit.line) else {
                    continue;
                };
                match (unit.kind, Declared::read(&marker)) {
                    (UnitKind::Function, Some(declared)) if self.is_machinery(d) => {
                        self.declared.insert((d, u), declared);
                    }
                    _ => self.misdeclared.push((d, unit.line)),
                }
            }
        }
    }

    pub(super) fn facts_of(&mut self, d: usize, u: usize) -> &Facts {
        if !self.facts.contains_key(&(d, u)) {
            let doc = &self.docs[d];
            let facts = super::scan::facts(&doc.tokens, doc.units[u].body.clone());
            self.facts.insert((d, u), facts);
        }
        &self.facts[&(d, u)]
    }

    /// The top code of a unit, read once.
    pub fn code_of(&mut self, d: usize, u: usize) -> usize {
        if let Some(code) = self.top.get(&(d, u)) {
            return *code;
        }
        let range = self.docs[d].units[u].body.clone();
        let code = self.new_code(d, u, range, None);
        self.top.insert((d, u), code);
        self.read(code);
        code
    }

    /// Reads the program again from nothing, keeping what the first reading
    /// learned of the parameters each call hands on (`bindings`): a code read
    /// before all its callers were seen is read with all of them now.
    pub fn again(&mut self) {
        self.bindings = std::mem::take(&mut self.collected);
        self.origin_bindings = std::mem::take(&mut self.origin_collected);
        self.contents = std::mem::take(&mut self.contents_collected);
        self.local_origins.clear();
        self.return_origins.clear();
        self.prop_origins.clear();
        self.codes.clear();
        self.roots.clear();
        self.top.clear();
    }

    /// Whether the reading just made bound a parameter, or put something
    /// inside a container, otherwise than the one before it: then it is read
    /// again.
    pub fn learned(&self) -> bool {
        self.collected != self.bindings
            || self.origin_collected != self.origin_bindings
            || self.contents_collected != self.contents
    }

    /// What an argument's tokens stand for in `code`, where they are one
    /// path to an object.
    fn arg_value(&mut self, code: usize, d: usize, range: Range<usize>) -> Option<Value> {
        let facts = super::scan::facts(&self.docs[d].tokens, range.clone());
        let (path, _) = facts.paths.first()?.clone();
        if range.len() != 1 + 2 * path.members.len() {
            return None;
        }
        match self.resolve(code, &path) {
            Value::Captured(_, Some(alias)) => Some(*alias),
            value => Some(self.objectified(value)).filter(object_value),
        }
    }

    /// A harness property that holds an object of the product's, as that
    /// object.
    fn objectified(&self, value: Value) -> Value {
        match value {
            Value::Prop(d, o, p) => match self.prop_type(d, o, &p) {
                Some(ty) => Value::Product(Some(ty)),
                None => Value::Prop(d, o, p),
            },
            other => other,
        }
    }

    /// Records what a call hands parameter `i` of `(d, u)`.
    fn bind(&mut self, d: usize, u: usize, i: usize, value: Option<Value>) {
        match self.collected.get(&(d, u, i)) {
            None => {
                self.collected.insert((d, u, i), value);
            }
            Some(known) if *known != value => {
                self.collected.insert((d, u, i), None);
            }
            Some(_) => {}
        }
    }

    fn new_code(
        &mut self,
        d: usize,
        u: usize,
        range: Range<usize>,
        parent: Option<usize>,
    ) -> usize {
        self.codes.push(Code {
            doc: d,
            unit: u,
            range,
            parent,
            effects: Vec::new(),
            compares: Vec::new(),
        });
        self.codes.len() - 1
    }

    // ---- Names ----

    /// Whether `object` is an operation: its type, or its type's type, is
    /// the operation's.
    pub fn is_operation(&self, d: usize, o: usize) -> bool {
        let mut ty = self.docs[d].objects[o].ty.clone();
        for _ in 0..8 {
            if ty == OPERATION {
                return true;
            }
            let Some(&t) = self.types.get(&ty) else {
                return false;
            };
            if t >= self.harness {
                return false;
            }
            ty = self.docs[t]
                .objects
                .first()
                .map(|o| o.ty.clone())
                .unwrap_or_default();
        }
        false
    }

    /// The harness file a type is, if it is one.
    pub(super) fn harness_type(&self, ty: &str) -> Option<usize> {
        self.types.get(ty).copied().filter(|t| *t < self.harness)
    }

    /// `name` on the object: a property, a function, or one of its type's.
    pub(super) fn member(&self, d: usize, o: usize, name: &str) -> Option<Value> {
        let object = &self.docs[d].objects[o];
        if object.props.iter().any(|p| p.name == name) {
            return Some(match self.prop_value(d, o, name) {
                Some((pd, po)) => Value::Object(pd, po),
                None => Value::Prop(d, o, name.to_string()),
            });
        }
        if let Some(u) = self.docs[d].units.iter().position(|unit| {
            unit.object == o && unit.kind == UnitKind::Function && unit.name == name
        }) {
            return Some(Value::Function(d, u));
        }
        let t = *self.types.get(&object.ty)?;
        if t == d {
            return None;
        }
        // The instance binds what its type declares (`driver: driver`).
        if object.bindings.iter().any(|(b, _)| b == name)
            && self.docs[t].objects[0].props.iter().any(|p| p.name == name)
        {
            return Some(match self.bound(d, o, name) {
                Some((bd, bo)) => Value::Object(bd, bo),
                None => Value::Prop(t, 0, name.to_string()),
            });
        }
        self.member(t, 0, name)
    }

    /// The object a binding on `(d, o)` names, if it names one.
    fn bound(&self, d: usize, o: usize, name: &str) -> Option<(usize, usize)> {
        let (_, range) = self.docs[d].objects[o]
            .bindings
            .iter()
            .find(|(b, _)| b == name)?;
        self.path_object(d, o, range.clone())
    }

    /// The product type a property holds, where it is known: its declared
    /// type, or its value's.
    pub(super) fn prop_type(&self, d: usize, o: usize, name: &str) -> Option<String> {
        let prop = self.docs[d].objects[o]
            .props
            .iter()
            .find(|p| p.name == name)?;
        // A value type (`int`, `string`) is named too: what holds one holds a value, not an object.
        if !matches!(prop.ty.as_str(), "var" | "alias" | "variant" | "QtObject")
            && self.harness_type(&prop.ty).is_none()
        {
            return Some(prop.ty.clone());
        }
        match self.path_value(d, o, prop.value.clone()?)? {
            Value::Product(ty) => ty,
            Value::Prop(pd, po, p) if (pd, po, p.as_str()) != (d, o, name) => {
                self.prop_type(pd, po, &p)
            }
            _ => None,
        }
    }

    /// The harness object a property holds, if it holds one: its value's
    /// path, its type, or the value every instance binds it to.
    fn prop_value(&self, d: usize, o: usize, name: &str) -> Option<(usize, usize)> {
        let key = (d, o, name.to_string());
        if let Some(known) = self.prop_values.borrow().get(&key) {
            return *known;
        }
        self.prop_values.borrow_mut().insert(key.clone(), None);
        let found = self.prop_value_read(d, o, name);
        self.prop_values.borrow_mut().insert(key, found);
        found
    }

    fn prop_value_read(&self, d: usize, o: usize, name: &str) -> Option<(usize, usize)> {
        let prop = self.docs[d].objects[o]
            .props
            .iter()
            .find(|p| p.name == name)?;
        if let Some(range) = &prop.value {
            return self.path_object(d, o, range.clone());
        }
        if let Some(t) = self.harness_type(&prop.ty) {
            return Some((t, 0));
        }
        if o != 0 {
            return None;
        }
        // Bound by every instance of this file's type.
        let mut found: Option<(usize, usize)> = None;
        for d2 in 0..self.harness {
            for (o2, object) in self.docs[d2].objects.iter().enumerate() {
                if object.ty != self.docs[d].name {
                    continue;
                }
                let value = self.bound(d2, o2, name)?;
                if found.is_some_and(|f| f != value) {
                    return None;
                }
                found = Some(value);
            }
        }
        found
    }

    /// The harness object a value's tokens name, where they are one path.
    fn path_object(&self, d: usize, o: usize, range: Range<usize>) -> Option<(usize, usize)> {
        match self.path_value(d, o, range)? {
            Value::Object(vd, vo) => Some((vd, vo)),
            _ => None,
        }
    }

    /// What a value's tokens stand for, where they are one path.
    fn path_value(&self, d: usize, o: usize, range: Range<usize>) -> Option<Value> {
        let tokens = &self.docs[d].tokens[range];
        let first = tokens.first()?;
        if !is_name(&first.text) {
            return None;
        }
        let mut names = vec![first.text.as_str()];
        let mut k = 1;
        while k + 1 < tokens.len() + 1 && tokens.get(k).is_some_and(|t| t.text == ".") {
            names.push(tokens.get(k + 1)?.text.as_str());
            k += 2;
        }
        if k != tokens.len() {
            return None;
        }
        let mut value = self.scope_name(d, o, names[0]);
        for name in &names[1..] {
            value = self.member_of(&value, name);
        }
        Some(value)
    }

    /// A name as code of object `o` sees it, past its own locals: the
    /// object's members, the file's ids, the root's members, the globals.
    /// `scope_name` for the origin's reading.
    pub(super) fn scope_value(&self, d: usize, o: usize, name: &str) -> Value {
        self.scope_name(d, o, name)
    }

    /// `path_value` for the origin's reading.
    pub(super) fn path_value_at(&self, d: usize, o: usize, range: Range<usize>) -> Option<Value> {
        self.path_value(d, o, range)
    }

    fn scope_name(&self, d: usize, o: usize, name: &str) -> Value {
        // An id before the scope object's own names: `driver: driver` binds the id.
        if let Some(i) = self.docs[d]
            .objects
            .iter()
            .position(|object| object.id.as_deref() == Some(name))
        {
            return Value::Object(d, i);
        }
        if let Some(value) = self.member(d, o, name) {
            return value;
        }
        if o != 0
            && let Some(value) = self.member(d, 0, name)
        {
            return value;
        }
        if let Some(t) = self.harness_type(name)
            && d < self.harness
        {
            // A singleton of the harness's (`Awaited`).
            return Value::Object(t, 0);
        }
        if product::GLOBALS.contains(&name) || product::GLOBAL_FUNCTIONS.contains(&name) {
            return Value::Global(name.to_string());
        }
        if name.chars().next().is_some_and(char::is_uppercase) {
            // A singleton of the product's (`PageSettled`, `Tips`).
            return Value::Product(Some(name.to_string()));
        }
        if d >= self.harness || name == "parent" {
            return Value::Product(None);
        }
        Value::Unknown
    }

    /// `value.name`.
    fn member_of(&self, value: &Value, name: &str) -> Value {
        match value {
            Value::Object(d, o) => match self.member(*d, *o, name) {
                Some(Value::Prop(pd, po, p)) if pd >= self.harness => {
                    Value::Product(self.prop_type(pd, po, &p))
                }
                Some(found) => found,
                None => self.product_member(Some(self.docs[*d].objects[*o].ty.clone()), name),
            },
            Value::Prop(d, o, p) => self.product_member(self.prop_type(*d, *o, p), name),
            Value::Product(ty) => self.product_member(ty.clone(), name),
            Value::Global(g) => Value::Global(format!("{g}.{name}")),
            _ => Value::Unknown,
        }
    }

    /// `name` on something of the product's of type `ty`: a function of its
    /// QML, or a value of the type its property declares.
    fn product_member(&self, ty: Option<String>, name: &str) -> Value {
        let Some(t) = ty.as_deref().and_then(|ty| self.types.get(ty)).copied() else {
            return Value::Product(None);
        };
        match self.member(t, 0, name) {
            Some(Value::Prop(pd, po, p)) => Value::Product(self.prop_type(pd, po, &p)),
            // A function of its own, or an object of its own an alias names (`filesWalk: fileWalk`).
            Some(found @ (Value::Function(..) | Value::Object(..))) => found,
            _ => Value::Product(None),
        }
    }

    /// What the head of a path stands for in `code`.
    pub(super) fn head(&mut self, code: usize, name: &str) -> Value {
        let mut current = Some(code);
        let mut own = true;
        while let Some(c) = current {
            let (d, u, range, parent) = {
                let c = &self.codes[c];
                (c.doc, c.unit, c.range.clone(), c.parent)
            };
            let param = parent
                .is_none()
                .then(|| self.docs[d].units[u].params.iter().position(|p| p == name))
                .flatten();
            // A parameter every caller hands the same object stands for that object.
            if let Some(Some(bound)) = param.and_then(|i| self.bindings.get(&(d, u, i))).cloned() {
                return if own {
                    bound
                } else {
                    Value::Captured(name.to_string(), Some(Box::new(bound)))
                };
            }
            let is_param = param.is_some();
            let local = self
                .facts_of(d, u)
                .within(&range)
                .locals
                .into_iter()
                .find(|l| l.name == name);
            if is_param || local.is_some() {
                // A local that names an object — the harness's or the product's — stands for it: what is read
                // through it is read when it is read. One holding a value holds what was read before.
                let alias = local
                    .and_then(|l| l.init)
                    .and_then(|init| {
                        let o = self.docs[d].units[u].object;
                        self.path_value(d, o, init)
                    })
                    .map(|v| self.objectified(v))
                    .filter(object_value);
                return match (own, alias) {
                    (true, Some(alias)) => alias,
                    (true, None) => Value::Local,
                    (false, alias) => Value::Captured(name.to_string(), alias.map(Box::new)),
                };
            }
            own = false;
            current = parent;
        }
        let c = &self.codes[code];
        let o = self.docs[c.doc].units[c.unit].object;
        self.scope_name(c.doc, o, name)
    }

    /// What a path stands for, member by member.
    pub(super) fn resolve(&mut self, code: usize, path: &Path) -> Value {
        let mut value = match &path.head {
            Head::Name(name) => self.head(code, name),
            Head::Value => Value::Unknown,
        };
        for name in &path.members {
            value = match value {
                Value::Captured(_, Some(alias)) => self.member_of(&alias, name),
                other => self.member_of(&other, name),
            };
        }
        value
    }

    // ---- Effects ----

    /// Fills in what `code` does: its own facts, those of the arguments
    /// carved out of it left to their own codes.
    fn read(&mut self, code: usize) {
        let (d, u, range) = {
            let c = &self.codes[code];
            (c.doc, c.unit, c.range.clone())
        };
        let facts = self.facts_of(d, u).within(&range);
        let mut carved: Vec<Range<usize>> = Vec::new();
        let mut effects = Vec::new();
        // Calls first: they say which arguments are stretches of their own — an arrow a local holds among them,
        // which is carved where it is spelled, so its calls, met before, are the carved code's.
        let mut called = Vec::new();
        for call in &facts.calls {
            if carved.iter().any(|r| r.contains(&call.path.at)) {
                continue;
            }
            let (effect, contexts) = self.call(code, &call.path, call.args.len());
            if let Some(Effect::Calls(callee)) = &effect {
                self.hand_on(code, *callee, &call.args);
            }
            self.puts(code, &call.path, &call.args);
            // A global function that changes the value it is handed first (`Object.assign`) changes it in place.
            if let (Head::Name(object), [method]) = (&call.path.head, call.path.members.as_slice())
                && builtins::global_puts(object, method).is_some()
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
                    && let Value::Captured(_, None) = self.head(code, name)
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
        let tokens = &self.docs[d].tokens[range.clone()];
        let path = {
            let facts = super::scan::facts(&self.docs[d].tokens, range.clone());
            let (path, _) = facts.paths.first()?.clone();
            let length = 1 + 2 * path.members.len();
            (tokens.len() == length).then_some(path)?
        };
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
                if !arrow(&self.docs[d].tokens[init.clone()]) {
                    return None;
                }
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

    /// Records what a call in `code` hands each parameter of the harness
    /// function whose code is `callee`: the object it names, and where its
    /// value came from.
    fn hand_on(&mut self, code: usize, callee: usize, args: &[Range<usize>]) {
        let d = self.codes[code].doc;
        let (cd, cu) = (self.codes[callee].doc, self.codes[callee].unit);
        for (i, arg) in args.iter().enumerate() {
            let value = self.arg_value(code, d, arg.clone());
            self.bind(cd, cu, i, value);
            // A value the caller made, handed down, is the callee's to build as the caller's own.
            let origin = match self.origin_of(code, arg.clone()) {
                made @ Origin::Fresh { maker, .. } if maker == code => made.handed_to(callee),
                other => other,
            };
            let joined = self
                .origin_collected
                .remove(&(cd, cu, i))
                .unwrap_or(Origin::Unset)
                .join(origin);
            self.origin_collected.insert((cd, cu, i), joined);
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
                if arrow(&self.docs[d].tokens[range.clone()]) {
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
                    if arrow(&self.docs[d].tokens[value.clone()]) {
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
            if self.is_local(code, name) {
                return None;
            }
            return match self.head(code, name) {
                Value::Captured(..) => Some(Effect::Smuggle(name.clone())),
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
        // What is changed: the value itself, changed in place, or the one whose member is given a value.
        let (changed, member) = if in_place {
            (path.clone(), None)
        } else {
            (path.receiver()?, path.members.last().cloned())
        };
        let origin = self.path_origin(code, &changed);
        self.written_into(code, &origin, kind, in_place);
        self.change(code, origin, member, path, step, starts)
    }

    /// What changing a value of `origin` does — in place, or its `member`
    /// given a value.
    pub(super) fn change(
        &mut self,
        code: usize,
        origin: Origin,
        member: Option<String>,
        path: &Path,
        step: bool,
        starts: &[(Value, usize)],
    ) -> Option<Effect> {
        match origin {
            // Holds nothing to change.
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

    fn is_local(&mut self, code: usize, name: &str) -> bool {
        let (d, u, range, parent) = {
            let c = &self.codes[code];
            (c.doc, c.unit, c.range.clone(), c.parent)
        };
        (parent.is_none() && self.docs[d].units[u].params.iter().any(|p| p == name))
            || self
                .facts_of(d, u)
                .within(&range)
                .locals
                .iter()
                .any(|l| l.name == name)
    }
}

/// How a declared function takes an argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArgContext {
    /// A function run in that context.
    Of(Context),
    /// An object whose `when` and `report` are readings (`StageTimer.complete`).
    Spec,
}

/// Whether a value is known to be an object — the harness's, or the
/// product's of an object type — and not a value read off one.
fn object_value(value: &Value) -> bool {
    match value {
        Value::Object(..) => true,
        Value::Product(Some(ty)) => ty.chars().next().is_some_and(char::is_uppercase),
        _ => false,
    }
}

/// Whether `tokens` are a function: an arrow or a `function`.
fn arrow(tokens: &[crate::lex::Token]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    if first.text == "function" {
        return true;
    }
    if is_name(&first.text) && tokens.get(1).is_some_and(|t| t.text == "=>") {
        return true;
    }
    first.text == "("
        && crate::lex::matching_close(tokens, 0, "(", ")")
            .is_some_and(|close| tokens.get(close + 1).is_some_and(|t| t.text == "=>"))
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

/// The `// reach(…)` marker in the comment right above `line`.
fn marker_above(lines: &[String], line: usize) -> Option<String> {
    let mut k = line.checked_sub(1)?;
    while k > 0 {
        let text = lines.get(k - 1)?.trim();
        if !text.starts_with("//") {
            return None;
        }
        if let Some(rest) = text.strip_prefix("// reach(") {
            return Some(rest.split(')').next().unwrap_or_default().to_string());
        }
        k -= 1;
    }
    None
}
