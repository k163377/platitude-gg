//! The product as the harness's calls reach it: its QML functions and
//! signals by name, its Rust slots by the receiver Rust gives them, and the
//! language's and Qt's own methods by the tables below. Its functions are
//! read as their caller's code (`super::calls`), so what one changes is
//! judged as the harness's own code is (`super::origin`). A name none of
//! them answers for is `Unknown`, and a caller takes it as nothing safe.

use std::collections::HashMap;

use super::parse::Doc;

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
pub(super) const READ: &[&str] = &[
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
pub(super) const INPUT: &[&str] = &[
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
        }
    }

    /// What the slot `method` of the Rust type `ty` does, if it has one.
    pub fn slot(&self, ty: &str, method: &str) -> Option<Does> {
        self.typed_slots
            .get(&(ty.to_string(), method.to_string()))
            .map(|changes| if *changes { Does::Input } else { Does::Read })
    }

    /// Whether some Rust slot is named `method`, and whether some so named
    /// can change what it is called on.
    pub fn slot_named(&self, method: &str) -> Option<bool> {
        self.slots.get(method).copied()
    }

    /// The product's QML functions named `method`, as `(file, unit)`.
    pub fn functions_named(&self, method: &str) -> &[(usize, usize)] {
        self.functions.get(method).map_or(&[], Vec::as_slice)
    }

    /// Whether some object of the product declares a signal `name`.
    pub fn is_signal(&self, name: &str) -> bool {
        self.signals.contains_key(name)
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
