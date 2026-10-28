//! What a call does: a harness function's code, a function of the
//! machinery's by the reach it declares, the product's by its QML and its
//! Rust slots, and the language's and Qt's own by their tables.

use super::builtins;
use super::parse::UnitKind;
use super::product::{self, Does};
use super::program::{ArgContext, Effect, OPERATION, Program, Value};
use super::scan::{Head, Path};

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
        let machinery = self.is_machinery(self.codes[code].doc);
        let callee = match path.receiver() {
            None => match &path.head {
                Head::Name(name) => self.head(code, name),
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
                    Value::Prop(d, o, p) => {
                        let ty = self.prop_type(*d, *o, p);
                        return (self.product_call(ty, &method, &spelled), Vec::new());
                    }
                    Value::Product(ty) => {
                        return (self.product_call(ty.clone(), &method, &spelled), Vec::new());
                    }
                    // Of unknown type: an operation's API by its name, else the product's.
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
                (Some(Effect::Calls(callee_code)), Vec::new())
            }
            Value::Function(d, u) => match self.product.function(&self.docs, d, u) {
                Does::Read => (None, Vec::new()),
                _ => (Some(Effect::Input(spelled)), Vec::new()),
            },
            Value::Global(g) if product::GLOBAL_FUNCTIONS.contains(&g.as_str()) => {
                (None, Vec::new())
            }
            // The machinery runs the functions it was handed, in the contexts it declared taking them in.
            Value::Local | Value::Captured(..) | Value::Prop(..) if machinery => (None, Vec::new()),
            _ => (Some(Effect::Unread(spelled)), Vec::new()),
        }
    }

    /// What `method` does on the product type `ty`, where the type answers
    /// for it: a function of its QML (or its base's), a signal it declares,
    /// a slot of its Rust.
    fn typed_call(&self, ty: &str, method: &str) -> Option<Does> {
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
                    return Some(self.product.function(&self.docs, c, u));
                }
                if doc
                    .objects
                    .first()
                    .is_some_and(|root| root.signals.iter().any(|s| s == method))
                {
                    return Some(Does::Input);
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
        self.product.slot(ty, method)
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
    fn product_call(&self, ty: Option<String>, method: &str, spelled: &str) -> Option<Effect> {
        let typed = ty.as_deref().and_then(|ty| self.typed_call(ty, method));
        match typed.unwrap_or_else(|| self.product.does(&self.docs, method)) {
            Does::Read => None,
            Does::Input => Some(Effect::Input(spelled.to_string())),
            Does::Unknown => Some(Effect::Unread(spelled.to_string())),
        }
    }
}
