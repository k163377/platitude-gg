//! The shape of a QML file as the reach check reads it: its objects (a
//! type, an `id`, the properties and signals it declares, the values it
//! binds) and the code they hold, each function and handler a unit of its
//! own with its tokens kept for [`super::scan`]. A member the reading
//! cannot place is named (`unread`), not skipped: code past it would go
//! unread.

use std::ops::Range;

use crate::lex::{Lang, Token, matching_close, tokens};

/// One QML file.
pub(super) struct Doc {
    pub path: String,
    /// The type the file declares: its name without `.qml`.
    pub name: String,
    pub tokens: Vec<Token>,
    /// The file as written, for the role a function declares above itself.
    pub lines: Vec<String>,
    /// The root object first; then the others in the order they open.
    pub objects: Vec<Object>,
    pub units: Vec<Unit>,
    /// Lines of members the reading could not place.
    pub unread: Vec<usize>,
}

pub(super) struct Object {
    /// The type's last name (`Automation.PageStage` is `PageStage`).
    pub ty: String,
    pub id: Option<String>,
    pub props: Vec<Prop>,
    pub signals: Vec<String>,
    /// The values the object binds to names it does not declare
    /// (`driver: driver`), by the name.
    pub bindings: Vec<(String, Range<usize>)>,
}

pub(super) struct Prop {
    pub name: String,
    /// The declared type's last name: `var`, `int`, `Item`, `alias`.
    pub ty: String,
    /// Tokens of its value, when it is given one that is no object.
    pub value: Option<Range<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UnitKind {
    Function,
    Handler,
}

/// A function or a handler.
pub(super) struct Unit {
    pub object: usize,
    pub kind: UnitKind,
    pub name: String,
    pub params: Vec<String>,
    /// Its body: the tokens between its braces, or its expression.
    pub body: Range<usize>,
    pub line: usize,
}

pub(super) fn parse(path: &str, text: &str) -> Doc {
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .trim_end_matches(".qml")
        .to_string();
    let mut doc = Doc {
        path: path.to_string(),
        name,
        tokens: tokens(text, Lang::Qml),
        lines: text.lines().map(str::to_string).collect(),
        objects: Vec::new(),
        units: Vec::new(),
        unread: Vec::new(),
    };
    let mut at = 0;
    while doc
        .tokens
        .get(at)
        .is_some_and(|t| matches!(t.text.as_str(), "import" | "pragma"))
    {
        let line = doc.tokens[at].line;
        while doc.tokens.get(at).is_some_and(|t| t.line == line) {
            at += 1;
        }
    }
    match type_open(&doc.tokens, at) {
        Some(open) => {
            let close = object(&mut doc, at, open);
            if close + 1 < doc.tokens.len() {
                let line = doc.tokens[close + 1].line;
                doc.unread.push(line);
            }
        }
        None => {
            let line = doc.tokens.get(at).map_or(1, |t| t.line);
            doc.unread.push(line);
        }
    }
    doc
}

pub(super) fn is_name(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
}

fn is_type(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_uppercase)
}

/// The `{` of an object whose type path starts at `at`
/// (`Type {`, `Module.Type {`).
fn type_open(tokens: &[Token], at: usize) -> Option<usize> {
    let mut k = at;
    if !tokens.get(k).is_some_and(|t| is_type(&t.text)) {
        return None;
    }
    while tokens.get(k + 1).is_some_and(|t| t.text == ".")
        && tokens.get(k + 2).is_some_and(|t| is_type(&t.text))
    {
        k += 2;
    }
    tokens
        .get(k + 1)
        .is_some_and(|t| t.text == "{")
        .then_some(k + 1)
}

/// Reads the object whose type starts at `at` and whose `{` is `open`;
/// answers its `}` (the last token where it never closes).
fn object(doc: &mut Doc, at: usize, open: usize) -> usize {
    let close = matching_close(&doc.tokens, open, "{", "}");
    let end = close.unwrap_or(doc.tokens.len());
    if close.is_none() {
        doc.unread.push(doc.tokens[open].line);
    }
    let index = doc.objects.len();
    // `Behavior on x {` is a `Behavior`; `Module.Type {` a `Type`.
    let on = open >= 2 && doc.tokens[open - 2].text == "on";
    doc.objects.push(Object {
        ty: doc.tokens[if on { at } else { open - 1 }].text.clone(),
        id: None,
        props: Vec::new(),
        signals: Vec::new(),
        bindings: Vec::new(),
    });
    members(doc, index, open + 1, end);
    end.min(doc.tokens.len().saturating_sub(1))
}

fn members(doc: &mut Doc, object_at: usize, from: usize, end: usize) {
    let mut at = from;
    while at < end {
        let text = doc.tokens[at].text.clone();
        let next = doc.tokens.get(at + 1).map(|t| t.text.clone());
        at = match text.as_str() {
            ";" | "," => at + 1,
            "id" if next.as_deref() == Some(":") => {
                doc.objects[object_at].id = doc.tokens.get(at + 2).map(|t| t.text.clone());
                at + 3
            }
            "property" | "readonly" | "required" | "default" | "final" | "virtual" | "override" => {
                property(doc, object_at, at, end)
            }
            "signal" if next.as_deref().is_some_and(is_name) => {
                doc.objects[object_at]
                    .signals
                    .push(next.clone().unwrap_or_default());
                if doc.tokens.get(at + 2).is_some_and(|t| t.text == "(") {
                    matching_close(&doc.tokens, at + 2, "(", ")").map_or(end, |c| c + 1)
                } else {
                    at + 2
                }
            }
            "function" => function(doc, object_at, at, end),
            "enum" => skip_braced(doc, at, end),
            "component" => {
                // `component Name: Type { … }` — a type of its own, read as an object.
                match type_open(&doc.tokens, at + 3) {
                    Some(open) => object(doc, at + 3, open) + 1,
                    None => unread(doc, at, end),
                }
            }
            _ => {
                if let Some(open) = type_open(&doc.tokens, at) {
                    object(doc, at, open) + 1
                } else if is_name(&text) {
                    binding(doc, object_at, at, end)
                } else {
                    unread(doc, at, end)
                }
            }
        };
    }
}

/// Names the line at `at` unread and goes on from the next one.
fn unread(doc: &mut Doc, at: usize, end: usize) -> usize {
    let line = doc.tokens[at].line;
    doc.unread.push(line);
    let mut k = at;
    while k < end && doc.tokens[k].line == line {
        k += 1;
    }
    k
}

fn skip_braced(doc: &mut Doc, at: usize, end: usize) -> usize {
    let open = (at..end).find(|k| doc.tokens[*k].text == "{");
    match open.and_then(|o| matching_close(&doc.tokens, o, "{", "}")) {
        Some(close) => close + 1,
        None => unread(doc, at, end),
    }
}

/// `[modifiers] property <type> name [: value]`.
fn property(doc: &mut Doc, object_at: usize, at: usize, end: usize) -> usize {
    let mut k = at;
    while k < end && doc.tokens[k].text != "property" {
        k += 1;
    }
    k += 1;
    // The type: a name, a dotted path, or `list<T>`.
    let mut ty = doc
        .tokens
        .get(k)
        .map(|t| t.text.clone())
        .unwrap_or_default();
    k += 1;
    while doc.tokens.get(k).is_some_and(|t| t.text == ".") {
        ty = doc
            .tokens
            .get(k + 1)
            .map(|t| t.text.clone())
            .unwrap_or_default();
        k += 2;
    }
    if doc.tokens.get(k).is_some_and(|t| t.text == "<") {
        while k < end && doc.tokens[k].text != ">" {
            k += 1;
        }
        k += 1;
    }
    let Some(name) = doc.tokens.get(k).map(|t| t.text.clone()) else {
        return end;
    };
    k += 1;
    let mut prop = Prop {
        name: name.clone(),
        ty,
        value: None,
    };
    if doc.tokens.get(k).is_some_and(|t| t.text == ":") {
        let (after, range) = value(doc, object_at, &name, k + 1, end);
        prop.value = range;
        doc.objects[object_at].props.push(prop);
        return after;
    }
    doc.objects[object_at].props.push(prop);
    k
}

/// `function name(params)[: type] { body }`.
fn function(doc: &mut Doc, object_at: usize, at: usize, end: usize) -> usize {
    let name = doc
        .tokens
        .get(at + 1)
        .map(|t| t.text.clone())
        .unwrap_or_default();
    let open_paren = at + 2;
    if !doc.tokens.get(open_paren).is_some_and(|t| t.text == "(") {
        return unread(doc, at, end);
    }
    let Some(close_paren) = matching_close(&doc.tokens, open_paren, "(", ")") else {
        return unread(doc, at, end);
    };
    let params = params(&doc.tokens[open_paren + 1..close_paren]);
    let Some(open) = (close_paren + 1..end).find(|k| doc.tokens[*k].text == "{") else {
        return unread(doc, at, end);
    };
    let Some(close) = matching_close(&doc.tokens, open, "{", "}") else {
        return unread(doc, at, end);
    };
    doc.units.push(Unit {
        object: object_at,
        kind: UnitKind::Function,
        name,
        params,
        body: open + 1..close,
        line: doc.tokens[at].line,
    });
    close + 1
}

/// The names a parameter list declares: each parameter's first name, or
/// the names a destructuring one takes apart; a type after `:` and a
/// default after `=` are no name.
pub(super) fn params(tokens: &[Token]) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut taking = true;
    for token in tokens {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            "," if depth == 0 => taking = true,
            ":" | "=" if depth == 0 => taking = false,
            "..." => {}
            text if taking && is_name(text) => {
                out.push(text.to_string());
                if depth == 0 {
                    taking = false;
                }
            }
            _ => {}
        }
    }
    out
}

/// `name: value` (a handler when the name is `on…`), or `Type on name {`.
fn binding(doc: &mut Doc, object_at: usize, at: usize, end: usize) -> usize {
    let mut k = at;
    let mut name = doc.tokens[at].text.clone();
    while doc.tokens.get(k + 1).is_some_and(|t| t.text == ".")
        && doc.tokens.get(k + 2).is_some_and(|t| is_name(&t.text))
    {
        name = format!("{name}.{}", doc.tokens[k + 2].text);
        k += 2;
    }
    match doc.tokens.get(k + 1).map(|t| t.text.as_str()) {
        Some(":") => {
            let (after, range) = value(doc, object_at, &name, k + 2, end);
            if let Some(range) = range
                && !handler_name(&name)
            {
                doc.objects[object_at].bindings.push((name, range));
            }
            after
        }
        // `Behavior on x { … }`, `NumberAnimation on y { … }`.
        Some("on") if doc.tokens.get(k + 3).is_some_and(|t| t.text == "{") => {
            object(doc, at, k + 3) + 1
        }
        _ => unread(doc, at, end),
    }
}

fn handler_name(name: &str) -> bool {
    let last = name.rsplit('.').next().unwrap_or(name);
    last.strip_prefix("on")
        .is_some_and(|rest| rest.chars().next().is_some_and(char::is_uppercase))
}

/// The value bound to `name` from `at`: an object, a list, a handler's
/// code or an expression. Answers where reading goes on, and the
/// expression's tokens when it is one.
fn value(
    doc: &mut Doc,
    object_at: usize,
    name: &str,
    at: usize,
    end: usize,
) -> (usize, Option<Range<usize>>) {
    if let Some(open) = type_open(&doc.tokens, at) {
        return (object(doc, at, open) + 1, None);
    }
    let first = doc
        .tokens
        .get(at)
        .map(|t| t.text.clone())
        .unwrap_or_default();
    if first == "[" {
        let Some(close) = matching_close(&doc.tokens, at, "[", "]") else {
            return (unread(doc, at, end), None);
        };
        let has_objects = (at + 1..close).any(|k| type_open(&doc.tokens, k).is_some());
        if has_objects {
            let mut k = at + 1;
            while k < close {
                if let Some(open) = type_open(&doc.tokens, k) {
                    k = object(doc, k, open) + 1;
                } else {
                    k += 1;
                }
            }
            return (close + 1, None);
        }
        return (close + 1, Some(at..close + 1));
    }
    if handler_name(name) {
        return handler(doc, object_at, name, at, end);
    }
    if first == "{" {
        let Some(close) = matching_close(&doc.tokens, at, "{", "}") else {
            return (unread(doc, at, end), None);
        };
        return (close + 1, Some(at..close + 1));
    }
    let stop = expression_end(&doc.tokens, at, end);
    (stop, Some(at..stop))
}

/// A handler's code: a block, a function, an arrow, or an expression.
fn handler(
    doc: &mut Doc,
    object_at: usize,
    name: &str,
    at: usize,
    end: usize,
) -> (usize, Option<Range<usize>>) {
    let line = doc.tokens[at].line;
    let mut params_of = Vec::new();
    let mut start = at;
    if doc.tokens[at].text == "function" {
        let Some(open_paren) = (at..end).find(|k| doc.tokens[*k].text == "(") else {
            return (unread(doc, at, end), None);
        };
        let Some(close_paren) = matching_close(&doc.tokens, open_paren, "(", ")") else {
            return (unread(doc, at, end), None);
        };
        params_of = params(&doc.tokens[open_paren + 1..close_paren]);
        start = close_paren + 1;
    } else if doc.tokens[at].text == "("
        && let Some(close_paren) = matching_close(&doc.tokens, at, "(", ")")
        && doc
            .tokens
            .get(close_paren + 1)
            .is_some_and(|t| t.text == "=>")
    {
        params_of = params(&doc.tokens[at + 1..close_paren]);
        start = close_paren + 2;
    } else if is_name(&doc.tokens[at].text)
        && doc.tokens.get(at + 1).is_some_and(|t| t.text == "=>")
    {
        params_of = vec![doc.tokens[at].text.clone()];
        start = at + 2;
    }
    let (body, after) = if doc.tokens.get(start).is_some_and(|t| t.text == "{") {
        match matching_close(&doc.tokens, start, "{", "}") {
            Some(close) => (start + 1..close, close + 1),
            None => return (unread(doc, at, end), None),
        }
    } else {
        let stop = expression_end(&doc.tokens, start, end);
        (start..stop, stop)
    };
    doc.units.push(Unit {
        object: object_at,
        kind: UnitKind::Handler,
        name: name.to_string(),
        params: params_of,
        body,
        line,
    });
    (after, None)
}

/// Where an expression starting at `at` ends: at a `,` / `;` or a closing
/// bracket of its enclosing list, or at a line break no operator carries
/// across.
pub(super) fn expression_end(tokens: &[Token], at: usize, end: usize) -> usize {
    let mut depth = 0usize;
    let mut k = at;
    while k < end {
        let text = tokens[k].text.as_str();
        if k > at
            && depth == 0
            && tokens[k].line != tokens[k - 1].line
            && !carries_on(&tokens[k - 1].text)
            && !carried_from(text)
        {
            return k;
        }
        match text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                if depth == 0 {
                    return k;
                }
                depth -= 1;
            }
            "," | ";" if depth == 0 => return k,
            _ => {}
        }
        k += 1;
    }
    end
}

/// Whether a line ending on `text` goes on to the next.
fn carries_on(text: &str) -> bool {
    if text.starts_with('`') && text.ends_with("${") {
        return true;
    }
    if is_name(text)
        || text.starts_with('"')
        || text.starts_with('\'')
        || text.starts_with('`')
        || text.starts_with('}')
        || text.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        return false;
    }
    !matches!(text, ")" | "]" | "}" | "++" | "--")
}

/// Whether a line starting on `text` continues the one before.
fn carried_from(text: &str) -> bool {
    text.starts_with('}') && text.len() > 1
        || matches!(
            text,
            "." | "?"
                | ":"
                | "+"
                | "-"
                | "*"
                | "/"
                | "%"
                | "&&"
                | "||"
                | "??"
                | "==="
                | "!=="
                | "=="
                | "!="
                | "<"
                | ">"
                | "<="
                | ">="
                | "&"
                | "|"
                | "^"
        )
}
