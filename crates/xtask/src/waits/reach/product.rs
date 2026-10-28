//! What a call into the product does, for the harness code that makes it:
//! the product's own QML functions read for what they write, its Rust
//! slots by the receiver Rust gives them, and the language's and Qt's own
//! methods by the table below. A name none of them answers for is
//! `Unknown`, and a caller takes it as nothing safe.

use std::collections::HashMap;

use super::parse::Doc;
use super::scan::{self, Head};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Does {
    /// Reads, and changes nothing.
    Read,
    /// Changes the product: an input.
    Input,
    /// Nothing answers for it.
    Unknown,
}

/// The language's and Qt's own methods that only read, by name.
const READ: &[&str] = &[
    // Values: strings, arrays, numbers, objects.
    "concat",
    "slice",
    "indexOf",
    "lastIndexOf",
    "includes",
    "join",
    "map",
    "filter",
    "some",
    "every",
    "find",
    "findIndex",
    "reduce",
    "forEach",
    "keys",
    "values",
    "entries",
    "split",
    "substring",
    "substr",
    "startsWith",
    "endsWith",
    "toLowerCase",
    "toUpperCase",
    "trim",
    "replace",
    "replaceAll",
    "match",
    "test",
    "exec",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "padStart",
    "padEnd",
    "repeat",
    "toFixed",
    "toString",
    "localeCompare",
    "flat",
    "flatMap",
    "isArray",
    "isInteger",
    "fromCodePoint",
    "stringify",
    "parse",
    "hasOwnProperty",
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "sort",
    "reverse",
    "fill",
    // Qt items, views and metrics.
    "itemAt",
    "itemAtIndex",
    "indexAt",
    "mapToItem",
    "mapFromItem",
    "mapToGlobal",
    "mapFromGlobal",
    "childAt",
    "contains",
    "tightBoundingRect",
    "boundingRect",
    "advanceWidth",
    "elidedText",
    "colorEqual",
    "point",
    "rect",
    "size",
    "rgba",
    "hsla",
    "lighter",
    "darker",
    "isImageLoaded",
    "positionToRectangle",
    // A translated string's placeholders filled in.
    "arg",
];

/// The language's and Qt's own methods that change the product, by name:
/// a view moved, a focus taken, a popup opened, a timer started, a file
/// written, an event loop told to go.
const INPUT: &[&str] = &[
    "forceActiveFocus",
    "positionViewAtIndex",
    "positionViewAtBeginning",
    "positionViewAtEnd",
    "forceLayout",
    "update",
    "requestUpdate",
    "scheduleUpdate",
    "open",
    "close",
    "popup",
    "select",
    "selectAll",
    "deselect",
    "insert",
    "remove",
    "append",
    "setProperty",
    "move",
    "flick",
    "cancelFlick",
    "returnToBounds",
    "grabToImage",
    "saveToFile",
    "toggle",
    "click",
    "animateClick",
    "trigger",
    "start",
    "stop",
    "restart",
    "quit",
    "accept",
    "reject",
    "requested",
];

/// The product's code, by the names a harness call reaches it under.
pub(super) struct Product {
    /// Each QML function of the product, by name: the files and units that
    /// define one so called.
    functions: HashMap<String, Vec<(usize, usize)>>,
    signals: HashMap<String, ()>,
    /// Each Rust slot, by the name QML calls it: whether some slot so named
    /// can change what it is called on.
    slots: HashMap<String, bool>,
    /// The same, by the type that declares it.
    typed_slots: HashMap<(String, String), bool>,
    /// Each product file, by the type it declares.
    types: HashMap<String, usize>,
    /// What each name was found to do, once asked.
    known: std::cell::RefCell<HashMap<String, Does>>,
    /// What each function was found to do, once read.
    read_functions: std::cell::RefCell<HashMap<(usize, usize), Does>>,
}

impl Product {
    /// `docs[from..]` are the product's QML files; `slots` its Rust slots
    /// as `(type, QML name, changes)`.
    pub fn new(docs: &[Doc], from: usize, slots: Vec<(String, String, bool)>) -> Product {
        let mut functions: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut signals = HashMap::new();
        for (d, doc) in docs.iter().enumerate().skip(from) {
            for (u, unit) in doc.units.iter().enumerate() {
                if unit.kind == super::parse::UnitKind::Function {
                    functions.entry(unit.name.clone()).or_default().push((d, u));
                }
            }
            for object in &doc.objects {
                for signal in &object.signals {
                    signals.insert(signal.clone(), ());
                }
            }
        }
        let mut by_name: HashMap<String, bool> = HashMap::new();
        let mut typed_slots: HashMap<(String, String), bool> = HashMap::new();
        for (ty, name, changes) in slots {
            let entry = by_name.entry(name.clone()).or_insert(false);
            *entry = *entry || changes;
            let entry = typed_slots.entry((ty, name)).or_insert(false);
            *entry = *entry || changes;
        }
        Product {
            functions,
            signals,
            slots: by_name,
            typed_slots,
            types: docs
                .iter()
                .enumerate()
                .skip(from)
                .map(|(d, doc)| (doc.name.clone(), d))
                .collect(),
            known: std::cell::RefCell::new(HashMap::new()),
            read_functions: std::cell::RefCell::new(HashMap::new()),
        }
    }

    /// What the slot `method` of the Rust type `ty` does, if it has one.
    pub fn slot(&self, ty: &str, method: &str) -> Option<Does> {
        self.typed_slots
            .get(&(ty.to_string(), method.to_string()))
            .map(|changes| if *changes { Does::Input } else { Does::Read })
    }

    /// What calling `method` on something of the product's does.
    pub fn does(&self, docs: &[Doc], method: &str) -> Does {
        if let Some(known) = self.known.borrow().get(method) {
            return *known;
        }
        // Asked again while being read (a function that calls itself): read so far.
        self.known
            .borrow_mut()
            .insert(method.to_string(), Does::Read);
        let found = self.read(docs, method);
        self.known.borrow_mut().insert(method.to_string(), found);
        found
    }

    fn read(&self, docs: &[Doc], method: &str) -> Does {
        let mut found = Vec::new();
        if INPUT.contains(&method) || self.signals.contains_key(method) {
            return Does::Input;
        }
        if let Some(changes) = self.slots.get(method) {
            found.push(if *changes { Does::Input } else { Does::Read });
        }
        if let Some(defined) = self.functions.get(method) {
            for &(d, u) in defined {
                found.push(self.function(docs, d, u));
            }
        }
        if found.is_empty() {
            return if READ.contains(&method) {
                Does::Read
            } else {
                Does::Unknown
            };
        }
        if found.contains(&Does::Input) || found.contains(&Does::Unknown) {
            Does::Input
        } else {
            Does::Read
        }
    }

    /// What one product function does: an input where it writes anything
    /// not its own or calls what is one; a call it cannot follow counts as
    /// an input — the product's own code is not read on trust.
    pub fn function(&self, docs: &[Doc], d: usize, u: usize) -> Does {
        if let Some(known) = self.read_functions.borrow().get(&(d, u)) {
            return *known;
        }
        self.read_functions.borrow_mut().insert((d, u), Does::Read);
        let found = self.function_read(docs, d, u);
        self.read_functions.borrow_mut().insert((d, u), found);
        found
    }

    /// What a call inside file `d` does where the file itself answers for
    /// it: a function of its own called bare, or one of the object an id of
    /// its names — that object's own, or its type's. `None` where the file
    /// does not answer, and the call is read by its name.
    fn in_doc(&self, docs: &[Doc], d: usize, path: &scan::Path, method: &str) -> Option<Does> {
        let doc = &docs[d];
        let object = match (&path.head, path.members.len()) {
            (Head::Name(_), 0) => None,
            (Head::Name(head), 1) => Some(
                doc.objects
                    .iter()
                    .position(|o| o.id.as_deref() == Some(head.as_str()))?,
            ),
            _ => return None,
        };
        let own: Vec<(usize, usize)> = doc
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| {
                unit.kind == super::parse::UnitKind::Function
                    && unit.name == method
                    && object.is_none_or(|o| unit.object == o)
            })
            .map(|(u, _)| (d, u))
            .collect();
        let typed: Vec<(usize, usize)> = object
            .and_then(|o| self.types.get(&doc.objects[o].ty))
            .map(|&t| {
                docs[t]
                    .units
                    .iter()
                    .enumerate()
                    .filter(|(_, unit)| {
                        unit.kind == super::parse::UnitKind::Function
                            && unit.object == 0
                            && unit.name == method
                    })
                    .map(|(u, _)| (t, u))
                    .collect()
            })
            .unwrap_or_default();
        let found: Vec<(usize, usize)> = if own.is_empty() { typed } else { own };
        if found.is_empty() {
            return None;
        }
        let any_input = found
            .into_iter()
            .any(|(fd, fu)| self.function(docs, fd, fu) != Does::Read);
        Some(if any_input { Does::Input } else { Does::Read })
    }

    fn function_read(&self, docs: &[Doc], d: usize, u: usize) -> Does {
        let doc = &docs[d];
        let unit = &doc.units[u];
        let facts = scan::facts(&doc.tokens, unit.body.clone());
        let own = |name: &str| {
            unit.params.iter().any(|p| p == name) || facts.locals.iter().any(|l| l.name == name)
        };
        for write in &facts.writes {
            match &write.path.head {
                Head::Name(name) if own(name) && write.path.members.is_empty() => {}
                _ => return Does::Input,
            }
        }
        for call in &facts.calls {
            let Some(method) = call.path.last() else {
                return Does::Input;
            };
            if call.path.members.is_empty() {
                let Head::Name(name) = &call.path.head else {
                    return Does::Input;
                };
                if own(name) {
                    return Does::Input;
                }
                if GLOBAL_FUNCTIONS.contains(&name.as_str()) {
                    continue;
                }
            } else if let Head::Name(head) = &call.path.head
                && GLOBALS.contains(&head.as_str())
            {
                match global(head, method) {
                    Does::Read => continue,
                    _ => return Does::Input,
                }
            }
            if let Some(found) = self.in_doc(docs, d, &call.path, method) {
                match found {
                    Does::Read => continue,
                    _ => return Does::Input,
                }
            }
            match self.does(docs, method) {
                Does::Read => {}
                _ => return Does::Input,
            }
        }
        Does::Read
    }
}

/// The global objects the language and Qt give every file.
pub(super) const GLOBALS: &[&str] = &[
    "Math", "JSON", "Object", "Array", "Number", "String", "Date", "console", "Qt", "Boolean",
    "RegExp", "Symbol",
];

/// The global functions, which read.
pub(super) const GLOBAL_FUNCTIONS: &[&str] = &[
    "Number",
    "String",
    "Boolean",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "Array",
    "Error",
    "TypeError",
    "RangeError",
    "Map",
    "Set",
    "Date",
    "qsTr",
    "qsTranslate",
    "qsTrId",
];

/// What `object.method` does for a global object.
pub(super) fn global(object: &str, method: &str) -> Does {
    match (object, method) {
        ("Qt", "callLater" | "quit" | "exit" | "openUrlExternally") => Does::Input,
        ("Object", "assign" | "defineProperty" | "freeze") => Does::Input,
        ("Qt" | "Math" | "JSON" | "Object" | "Array" | "Number" | "String" | "Boolean", _) => {
            Does::Read
        }
        ("console", _) => Does::Read,
        // A second clock beside the run's own: read, and named by the clock rule.
        ("Date", "now") => Does::Read,
        _ => Does::Unknown,
    }
}

/// Each `#[qslot]` of `text` (a Rust file of the app) as `(type, QML name,
/// changes)`: a slot on `&mut self` changes its object; one on `&self`
/// does too where its body borrows mutably, sets a cell or says a change.
pub(super) fn slots(text: &str) -> Vec<(String, String, bool)> {
    let tokens = crate::lex::tokens(text, crate::lex::Lang::Rust);
    let mut out = Vec::new();
    let mut implemented = String::new();
    for (at, token) in tokens.iter().enumerate() {
        if token.text == "impl"
            && let Some(name) = tokens.get(at + 1)
        {
            implemented = name.text.clone();
        }
        if token.text != "qslot" || at < 2 || tokens[at - 2].text != "#" {
            continue;
        }
        let Some(fn_at) = (at..tokens.len().min(at + 12)).find(|k| tokens[*k].text == "fn") else {
            continue;
        };
        let Some(name) = tokens.get(fn_at + 1) else {
            continue;
        };
        let mutable = tokens.get(fn_at + 3).is_some_and(|t| t.text == "&")
            && tokens.get(fn_at + 4).is_some_and(|t| t.text == "mut");
        let Some(open) = (fn_at..tokens.len()).find(|k| tokens[*k].text == "{") else {
            continue;
        };
        let close = crate::lex::matching_close(&tokens, open, "{", "}").unwrap_or(tokens.len());
        let changes = mutable
            || tokens[open..close].iter().any(|t| {
                matches!(
                    t.text.as_str(),
                    "borrow_mut" | "set" | "replace" | "take" | "emit" | "invoke_method"
                ) || t.text.ends_with("_changed")
            });
        out.push((implemented.clone(), camel(&name.text), changes));
    }
    out
}

fn camel(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in snake.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}
