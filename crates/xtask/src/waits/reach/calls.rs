//! What a call does: a function's code — the harness's or the product's,
//! read as the caller's, so a change it makes is judged by what it changes
//! whichever side spells it — a function of the machinery's by the reach it
//! declares, the product's Rust slots by their receivers, and the
//! language's and Qt's own by their tables. A function handed to a call
//! that calls it back is called too (`callback`).

use std::ops::Range;

use super::builtins::{self, CallsBack, Given};
use super::origin::{Origin, literal};
use super::parse::{UnitKind, is_name};
use super::product::{self, Does};
use super::program::{ArgContext, Effect, OPERATION, Program, Value};
use super::scan::{self, Head, Path};
use crate::lex::{Token, matching_close};

/// What the product answers for a call: a table's word, or the code of each
/// function so named.
enum Answer {
    Does(Does),
    Calls(Vec<usize>),
}

impl Program {
    /// What a call does, and which of its arguments are read in a context
    /// of their own.
    pub(super) fn call(
        &mut self,
        code: usize,
        path: &Path,
        args: usize,
    ) -> (Option<Effect>, Vec<(usize, ArgContext)>) {
        let spelled = path.spelled();
        let in_product = self.codes[code].doc >= self.harness;
        let callee = match path.receiver() {
            None => match &path.head {
                Head::Name(name) => self.head(code, name, path.at),
                Head::Value => Value::Unknown,
            },
            Some(receiver) => {
                let method = path.last().unwrap_or_default().to_string();
                let receiver_value = match self.resolve(code, &receiver) {
                    Value::Captured(_, Some(alias)) => *alias,
                    other => other,
                };
                match &receiver_value {
                    Value::Object(d, o) => match self.member(*d, *o, &method) {
                        Some(Value::Prop(pd, ..)) if pd >= self.harness => {
                            return (self.product_call(None, &method, &spelled), Vec::new());
                        }
                        Some(found) => found,
                        None if self.is_operation(*d, *o)
                            && matches!(method.as_str(), "start" | "stop" | "restart") =>
                        {
                            return (Some(Effect::Start), Vec::new());
                        }
                        None => {
                            let ty = Some(self.docs[*d].objects[*o].ty.clone());
                            return (self.product_call(ty, &method, &spelled), Vec::new());
                        }
                    },
                    // What it changes is its first argument's, judged by that argument's origin (`read`).
                    Value::Global(g) if builtins::global_puts(g, &method).is_some() => {
                        return (None, Vec::new());
                    }
                    Value::Global(g) => {
                        return match product::global(g, &method) {
                            Does::Read => (None, Vec::new()),
                            Does::Input => (Some(Effect::Input(spelled)), Vec::new()),
                            Does::Unknown => (Some(Effect::Unread(spelled)), Vec::new()),
                        };
                    }
                    Value::Prop(d, ..)
                        if *d < self.harness && self.held_function(code, &receiver, &method) =>
                    {
                        return (
                            Some(Effect::Unread(format!(
                                "{spelled}, a function a value holds"
                            ))),
                            Vec::new(),
                        );
                    }
                    Value::Prop(d, o, p) => {
                        let ty = self.prop_type(*d, *o, p);
                        return (self.product_call(ty, &method, &spelled), Vec::new());
                    }
                    Value::Product(ty) => {
                        return (self.product_call(ty.clone(), &method, &spelled), Vec::new());
                    }
                    // A function a value of the code's holds — an object it made, a harness property's — called: no
                    // table answers for what it does, unless it is the language's own (`members`).
                    Value::Local | Value::Captured(_, None)
                        if self.held_function(code, &receiver, &method) =>
                    {
                        return (
                            Some(Effect::Unread(format!(
                                "{spelled}, a function a value holds"
                            ))),
                            Vec::new(),
                        );
                    }
                    // Of unknown type, in the product: the product's.
                    _ if in_product => {
                        return (self.product_call(None, &method, &spelled), Vec::new());
                    }
                    // In the harness: an operation's API by its name, else the product's.
                    _ => match self.operation_api(&method) {
                        Some(found) => found,
                        None if matches!(method.as_str(), "start" | "stop" | "restart") => {
                            return (Some(Effect::Unread(spelled)), Vec::new());
                        }
                        None => return (self.product_call(None, &method, &spelled), Vec::new()),
                    },
                }
            }
        };
        self.callee_call(code, path, callee, args, spelled)
    }

    /// What calling `callee`, resolved for the call `path` in `code`, does.
    fn callee_call(
        &mut self,
        code: usize,
        path: &Path,
        callee: Value,
        args: usize,
        spelled: String,
    ) -> (Option<Effect>, Vec<(usize, ArgContext)>) {
        let machinery = self.is_machinery(self.codes[code].doc);
        match callee {
            Value::Function(d, u) if d < self.harness => {
                if let Some(declared) = self.declared.get(&(d, u)) {
                    let contexts = declared
                        .args
                        .iter()
                        .filter(|(i, _)| *i < args)
                        .copied()
                        .collect();
                    return (
                        Some(Effect::Declared(spelled, declared.wheres.clone())),
                        contexts,
                    );
                }
                let callee_code = self.code_of(d, u);
                (Some(Effect::Calls(vec![callee_code])), Vec::new())
            }
            // The product's own function: read as the caller's, as a helper of the harness's is.
            Value::Function(d, u) => (Some(Effect::Calls(vec![self.code_of(d, u)])), Vec::new()),
            Value::Global(g) if product::GLOBAL_FUNCTIONS.contains(&g.as_str()) => {
                (None, Vec::new())
            }
            // The machinery runs the functions it was handed, in the contexts it declared taking them in.
            Value::Local | Value::Captured(..) | Value::Prop(..) if machinery => (None, Vec::new()),
            // A local of the code's own given a function spelled in it: read where it is spelled.
            Value::Local
                if path.members.is_empty()
                    && matches!(&path.head, Head::Name(name) if self.local_function(code, name, path.at)) =>
            {
                (None, Vec::new())
            }
            _ => (Some(Effect::Unread(spelled)), Vec::new()),
        }
    }

    /// What `method` does on the product type `ty`, where the type answers
    /// for it: a function of its QML (or its base's), a signal it declares,
    /// a slot of its Rust.
    fn typed_call(&mut self, ty: &str, method: &str) -> Option<Answer> {
        if let Some(&t) = self.types.get(ty)
            && t >= self.harness
        {
            let mut current = Some(t);
            for _ in 0..8 {
                let Some(c) = current else {
                    break;
                };
                let doc = &self.docs[c];
                if let Some(u) = doc.units.iter().position(|unit| {
                    unit.object == 0 && unit.kind == UnitKind::Function && unit.name == method
                }) {
                    return Some(Answer::Calls(vec![self.code_of(c, u)]));
                }
                if doc
                    .objects
                    .first()
                    .is_some_and(|root| root.signals.iter().any(|s| s == method))
                {
                    return Some(Answer::Does(Does::Input));
                }
                current = doc
                    .objects
                    .first()
                    .and_then(|root| self.types.get(&root.ty))
                    .copied()
                    .filter(|b| *b >= self.harness && *b != c);
            }
            return None;
        }
        self.product.slot(ty, method).map(Answer::Does)
    }

    /// What `method` does on something of the product's whose type is not
    /// known, by its name: an input the tables or a signal name, a slot so
    /// named, or every function so named — read as the caller's.
    fn named_call(&mut self, method: &str) -> Answer {
        if product::INPUT.contains(&method) || self.product.is_signal(method) {
            return Answer::Does(Does::Input);
        }
        let slot = self.product.slot_named(method);
        if slot == Some(true) {
            return Answer::Does(Does::Input);
        }
        let defined = self.product.functions_named(method).to_vec();
        if !defined.is_empty() {
            let codes = defined
                .into_iter()
                .map(|(d, u)| self.code_of(d, u))
                .collect();
            return Answer::Calls(codes);
        }
        if slot == Some(false) || product::READ.contains(&method) {
            return Answer::Does(Does::Read);
        }
        Answer::Does(Does::Unknown)
    }

    /// A function of the operation's type by its name, for a receiver of
    /// unknown type.
    fn operation_api(&self, method: &str) -> Option<Value> {
        let t = self.harness_type(OPERATION)?;
        let u = self.docs[t].units.iter().position(|unit| {
            unit.object == 0 && unit.kind == UnitKind::Function && unit.name == method
        })?;
        Some(Value::Function(t, u))
    }

    /// A call into the product: by the receiver's type where it is known,
    /// by the method's name where it is not.
    fn product_call(&mut self, ty: Option<String>, method: &str, spelled: &str) -> Option<Effect> {
        let typed = match ty.as_deref() {
            Some(ty) => self.typed_call(ty, method),
            None => None,
        };
        match typed.unwrap_or_else(|| self.named_call(method)) {
            Answer::Calls(codes) => Some(Effect::Calls(codes)),
            Answer::Does(Does::Read) => None,
            Answer::Does(Does::Input) => Some(Effect::Input(spelled.to_string())),
            Answer::Does(Does::Unknown) => Some(Effect::Unread(spelled.to_string())),
        }
    }

    /// The global object a call's path names — `Object` of `Object.assign` —
    /// where its head is bound to that global where it is spelled, and not
    /// to a local of the same name: no global's contract answers for a local.
    pub(super) fn global_of(&mut self, code: usize, path: &Path) -> Option<String> {
        let (Head::Name(object), [_]) = (&path.head, path.members.as_slice()) else {
            return None;
        };
        if !product::GLOBALS.contains(&object.as_str()) {
            return None;
        }
        (self.head(code, object, path.at) == Value::Global(object.clone())).then(|| object.clone())
    }

    /// How the call `path` calls back a function it is handed, by its
    /// contract (`builtins::calls_back`).
    pub(super) fn callback_of(&mut self, code: usize, path: &Path) -> Option<CallsBack> {
        let method = path.last()?.to_string();
        if let Some(object) = self.global_of(code, path) {
            return builtins::global_calls_back(&object, &method);
        }
        let receiver = path.receiver()?;
        if self.held_function(code, &receiver, &method) {
            return None;
        }
        builtins::calls_back(&method)
    }

    /// Whether `method` of the value `receiver` names in `code` is a
    /// function the value holds rather than the language's own: the value
    /// is the code's or the harness's (made, plain, a property's) and not
    /// one the language made with no such member of its own (`members`).
    fn held_function(&mut self, code: usize, receiver: &Path, method: &str) -> bool {
        let origin = self.path_origin(code, receiver);
        matches!(
            origin,
            Origin::Fresh { .. } | Origin::Plain | Origin::Shared(..)
        ) && !self.language_method(&origin, method)
    }

    /// What the call `path` walks for the function it calls back: its
    /// receiver, or the first argument of a global's (`Array.from`).
    pub(super) fn walked_origin(
        &mut self,
        code: usize,
        path: &Path,
        args: &[Range<usize>],
    ) -> Origin {
        if self.global_of(code, path).is_some() {
            return match args.first() {
                Some(first) => self.origin_of(code, first.clone()),
                None => Origin::Unknown,
            };
        }
        match path.receiver() {
            Some(receiver) => self.path_origin(code, &receiver),
            None => Origin::Unknown,
        }
    }

    /// What an argument names, where it is one path.
    fn named(&mut self, code: usize, arg: Range<usize>) -> Option<Value> {
        let d = self.codes[code].doc;
        let facts = scan::facts(&self.docs[d].tokens, arg.clone());
        let (path, _) = facts.paths.first()?.clone();
        if arg.len() != 1 + 2 * path.members.len() {
            return None;
        }
        Some(self.resolve(code, &path))
    }

    /// The harness or product function an argument names, where it is one
    /// path to one the check reads (none of the machinery's).
    pub(super) fn named_callback(
        &mut self,
        code: usize,
        arg: Range<usize>,
    ) -> Option<(usize, usize)> {
        match self.named(code, arg)? {
            Value::Function(fd, fu) if !self.declared.contains_key(&(fd, fu)) => Some((fd, fu)),
            _ => None,
        }
    }

    /// Whether an argument names a global function that answers a plain
    /// value and changes nothing (`Number`): handed to a call, it is called
    /// to read.
    pub(super) fn names_plain_function(&mut self, code: usize, arg: Range<usize>) -> bool {
        matches!(
            self.named(code, arg),
            Some(Value::Global(g)) if builtins::PLAIN_FUNCTIONS.contains(&g.as_str())
        )
    }

    /// What a call in `code` does by the function it calls back, where that
    /// function is handed by name rather than spelled in place (a spelled
    /// one is read where it stands, its parameters given what the call hands
    /// them — `Program::param_origin`): the function, called with what the
    /// call hands it; a value that is no function the check reads is not
    /// followed. A literal (`replace`'s text) calls nothing back.
    pub(super) fn callback(
        &mut self,
        code: usize,
        path: &Path,
        args: &[Range<usize>],
    ) -> Option<Effect> {
        let CallsBack { at, given, .. } = self.callback_of(code, path)?;
        let arg = args.get(at)?.clone();
        let d = self.codes[code].doc;
        let tokens = &self.docs[d].tokens[arg.clone()];
        if function_body(tokens).is_some() || (tokens.len() == 1 && literal(&tokens[0].text)) {
            return None;
        }
        if self.names_plain_function(code, arg.clone()) {
            return None;
        }
        if let Some((fd, fu)) = self.named_callback(code, arg.clone()) {
            let handed = match given {
                Given::Elements => {
                    let walked = self.walked_origin(code, path, args);
                    self.inside_of(&walked)
                }
                Given::Unknown => Origin::Unknown,
            };
            let callee = self.code_of(fd, fu);
            self.hand_over(code, callee, &handed);
            return Some(Effect::Calls(vec![callee]));
        }
        if self.spelled_function(code, &arg) {
            return None;
        }
        Some(Effect::Unread(format!(
            "{} calling back what no code here answers for",
            path.spelled()
        )))
    }

    /// Whether an argument is one name of a local of `code` given a function
    /// spelled in the code — read where it is spelled.
    fn spelled_function(&mut self, code: usize, arg: &Range<usize>) -> bool {
        if arg.len() != 1 {
            return false;
        }
        let name = self.docs[self.codes[code].doc].tokens[arg.start]
            .text
            .clone();
        self.local_function(code, &name, arg.start)
    }

    /// Whether every binding of `name` in `code` is a local given a function
    /// spelled in the code, and never given anything else: what calling it
    /// does is that function's body, read where it stands.
    pub(super) fn local_function(&mut self, code: usize, name: &str, at: usize) -> bool {
        let (d, u, range) = {
            let c = &self.codes[code];
            (c.doc, c.unit, c.range.clone())
        };
        let facts = self.facts_of(d, u).within(&range);
        let binding = facts.local(name, at);
        binding.is_some_and(|local| {
            !local.param
                && local
                    .init
                    .as_ref()
                    .is_some_and(|init| function_body(&self.docs[d].tokens[init.clone()]).is_some())
        }) && !facts.writes.iter().any(|write| {
            write.path.head == Head::Name(name.to_string())
                && facts.local(name, write.path.at).map(|l| l.at) == binding.map(|l| l.at)
        })
    }
}

/// The body of a function spelled by `tokens` — an arrow (`x => …`,
/// `(a, b) => …`) or a `function` — relative to them; none for anything
/// else.
pub(super) fn function_body(tokens: &[Token]) -> Option<Range<usize>> {
    let first = tokens.first()?;
    if is_name(&first.text)
        && first.text != "function"
        && tokens.get(1).is_some_and(|t| t.text == "=>")
    {
        return Some(2..tokens.len());
    }
    if first.text == "(" {
        let close = matching_close(tokens, 0, "(", ")")?;
        return tokens
            .get(close + 1)
            .is_some_and(|t| t.text == "=>")
            .then_some(close + 2..tokens.len());
    }
    if first.text == "function" {
        let open = tokens.iter().position(|t| t.text == "(")?;
        let close = matching_close(tokens, open, "(", ")")?;
        let body = close + 1;
        let ends = tokens.get(body).is_some_and(|t| t.text == "{")
            && matching_close(tokens, body, "{", "}") == Some(tokens.len() - 1);
        return ends.then_some(body..tokens.len());
    }
    None
}
