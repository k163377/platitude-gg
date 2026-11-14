//! The URL paths a QML or JS text names, read off each load it holds: an
//! import statement's string — `import "dir"`, `import 'file.js' as X`, a
//! JS file's `.import "file.js" as X`, a module's `import … from "./m.mjs"`
//! —, a value written to a property the QML types load from (`LOADED_FROM`:
//! bound, assigned, given as a field), and what a function that loads is
//! handed (`Qt.createComponent`, `setSource`, `Qt.resolvedUrl`,
//! `Qt.include`, `import(…)`, a `StackView`'s `push` / `replace`). The text
//! is read as the engine's lexer reads it (`crate::lex`), and a write is
//! told from a name that only names the property by where it stands
//! (`written`), so quoting, spacing, line breaks, comments and parentheses
//! that change no meaning do not change what is read.
//!
//! Where a load is is one question; what it loads is another: a load's
//! value is worked out as far as it is written out — strings, joined by
//! `+`, chosen by `?:`, `||` or `??`, handed through `Qt.resolvedUrl`
//! (`values`) —, and the URL path that value names is read as Qt reads the
//! string (`super::urls`: its escapes, the URL). A string naming a `.qml`,
//! `.js`, `.mjs` or `.qsb` file loads it too, by whatever route.
//!
//! `waits::reach` does the jobs of `conditional`, `top_level` and
//! `value_end` with its own `conditional`, `split_top` and `expression_end`,
//! which read optional chaining, an empty last stretch and a line's
//! continuation otherwise, so the two are not one.

use std::ops::Range;

use super::urls::{string_value, url_path};
use crate::lex::{Lang, Token, blanked, is_name, matching_close, tokens, unparenthesized};

/// A URL path a text names ([`url_path`]): by a load, or by a string that
/// only names a file a load may read by any route. `imports::loaded_from`
/// follows both alike.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Named {
    pub path: String,
    pub load: bool,
}

/// The properties a QML type loads a file from.
const LOADED_FROM: [&str; 3] = ["source", "fragmentShader", "vertexShader"];

/// The functions that load what their first argument names, and the one
/// that makes a component out of text, whose imports nothing here reads.
const LOADING: [&str; 4] = ["createComponent", "setSource", "resolvedUrl", "include"];
const FROM_TEXT: &str = "createQmlObject";

/// What a load is handed.
enum Given {
    /// The value's tokens.
    Value(Range<usize>),
    /// A value nothing here reads: a property added to, or written by a
    /// destructuring or a loop.
    Unread(&'static str),
}

/// The URL paths a QML or JS text names, as written: what each load names,
/// and each string naming a QML, JS or compiled shader file. `Err` where
/// what it names is not known — a value nothing here works out, a URL off
/// the tree, a component made from text, an `XMLHttpRequest` — or the text
/// could not be read to its end, so that what stands past the stop is not
/// known.
pub(super) fn loads(text: &str) -> Result<Vec<Named>, String> {
    if blanked(text, Lang::Qml, true).1.is_some() {
        return Err("a text read short".to_string());
    }
    let tokens = tokens(text, Lang::Qml);
    // XHR can read arbitrary URLs assembled at runtime. No file closure
    // is claimed for such a test; do not confuse it with an absent load.
    if tokens
        .windows(2)
        .any(|t| t[0].text == "new" && t[1].text == "XMLHttpRequest")
    {
        return Err("an XMLHttpRequest can read a URL not named here".into());
    }
    let stacks = object_ids(&tokens, "StackView");
    let mut out: Vec<Named> = Vec::new();
    let mut take = |path: String, load: bool| {
        if let Some(had) = out.iter_mut().find(|named| named.path == path) {
            had.load |= load;
        } else {
            out.push(Named { path, load });
        }
    };
    // Past a load's value, its strings read as its value, not again as names of their own.
    let mut past = 0;
    for k in 0..tokens.len() {
        let given = match load_at(&tokens, k, &stacks)? {
            Some(Given::Value(range)) => range,
            Some(Given::Unread(how)) => {
                return Err(format!(
                    "{} {how}, whose value nothing reads",
                    tokens[k].text
                ));
            }
            None => {
                if k >= past
                    && let Some(value) = string_value(&tokens[k].text)
                    && [".qml", ".js", ".mjs", ".qsb"]
                        .iter()
                        .any(|end| value.ends_with(end))
                    && let Some(path) = url_path(&value)?
                {
                    take(path, false);
                }
                continue;
            }
        };
        past = past.max(given.end);
        let spelled = spelled(&tokens[given.clone()]);
        let values = values(&tokens[given])
            .ok_or_else(|| format!("a load of {spelled}, which nothing works out"))?;
        for value in values {
            if let Some(path) = url_path(&value)? {
                take(path, true);
            }
        }
    }
    Ok(out)
}

/// The load at `k`, where one is: an import, a call of a loading function,
/// a write to a property a type loads from.
fn load_at(tokens: &[Token], k: usize, stacks: &[String]) -> Result<Option<Given>, String> {
    let text = tokens[k].text.as_str();
    let next = tokens.get(k + 1).map(|t| t.text.as_str());
    // An import names its path by a string; a module's name is no path.
    if matches!(text, "import" | "from") && next.is_some_and(is_string) {
        return Ok(Some(Given::Value(k + 1..k + 2)));
    }
    if text == "import" && next == Some("(") {
        return Ok(Some(Given::Value(argument(tokens, k + 1)?)));
    }
    let Some((name, after)) = property_name(tokens, k) else {
        return Ok(None);
    };
    let called = tokens.get(after).is_some_and(|t| t.text == "(");
    if called
        && matches!(name.as_str(), "push" | "replace")
        && k >= 2
        && tokens[k - 1].text == "."
        && stacks.contains(&tokens[k - 2].text)
    {
        return Ok(Some(Given::Value(argument(tokens, after)?)));
    }
    if called && LOADING.contains(&name.as_str()) {
        return Ok(Some(Given::Value(argument(tokens, after)?)));
    }
    if called && name == FROM_TEXT {
        return Err("a component made from text, whose imports nothing reads".to_string());
    }
    if LOADED_FROM.contains(&name.as_str()) {
        if name == "source" && not_a_load(tokens, k) {
            return Ok(None);
        }
        return Ok(written(tokens, k, after));
    }
    Ok(None)
}

/// IDs whose declared Qt type supplies a URL-taking stack operation.
fn object_ids(tokens: &[Token], ty: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for k in 0..tokens.len().saturating_sub(1) {
        if tokens[k].text != ty || tokens[k + 1].text != "{" {
            continue;
        }
        let Some(close) = matching_close(tokens, k + 1, "{", "}") else {
            continue;
        };
        for j in k + 2..close.saturating_sub(2) {
            if tokens[j].text == "id"
                && tokens[j + 1].text == ":"
                && innermost_open(tokens, j) == Some(k + 1)
            {
                ids.push(tokens[j + 2].text.clone());
            }
        }
    }
    ids
}

/// An attached Drag source is an object, and declaring a property does
/// not itself read its URL. A consumer of that property is read separately.
fn not_a_load(tokens: &[Token], k: usize) -> bool {
    (k >= 2 && tokens[k - 2].text == "property")
        || (k >= 2 && tokens[k - 2].text == "Drag" && tokens[k - 1].text == ".")
        || innermost_open(tokens, k).is_some_and(|open| open > 0 && tokens[open - 1].text == "Drag")
}

/// The property a token at `k` names, and the index past it: a name, a
/// computed key's string (`x["source"]`) or a field's (`{"source": …}`).
fn property_name(tokens: &[Token], k: usize) -> Option<(String, usize)> {
    let text = tokens[k].text.as_str();
    if is_name(text) {
        return Some((text.to_string(), k + 1));
    }
    let value = string_value(text)?;
    let before = k.checked_sub(1).map(|b| tokens[b].text.as_str());
    let next = tokens.get(k + 1).map(|t| t.text.as_str());
    let keyed = before == Some("[")
        && next == Some("]")
        && k.checked_sub(2).is_some_and(|b| {
            matches!(tokens[b].text.as_str(), ")" | "]") || is_name(&tokens[b].text)
        });
    if keyed {
        return Some((value, k + 2));
    }
    (next == Some(":")).then_some((value, k + 1))
}

/// What a property named at `k` (its name ending before `after`) is given,
/// where it is written: assigned (`=`, inside parentheses or not), added
/// to, bound or given as a field, or the target of a destructuring or a
/// loop. `None` where the name is no property written: a local declared, a
/// parameter or its type, a pattern's binding, a conditional's branch, a
/// label, a read.
fn written(tokens: &[Token], k: usize, after: usize) -> Option<Given> {
    // A computed key stands inside its own brackets: where it stands is where they do.
    let keyed = after > k + 1;
    let k = if keyed { k - 1 } else { k };
    let before = k.checked_sub(1).map(|b| tokens[b].text.as_str());
    let member = keyed || matches!(before, Some("."));
    // Parentheses around the target change nothing: `(x.source) = v`.
    let mut op = after;
    while tokens.get(op).is_some_and(|t| t.text == ")")
        && tokens
            .get(op + 1)
            .is_some_and(|t| t.text == ")" || t.text.ends_with('='))
    {
        op += 1;
    }
    let operator = tokens.get(op).map(|t| t.text.as_str())?;
    match operator {
        "=" if in_pattern(tokens, k) => {
            member.then_some(Given::Unread("written by a destructuring"))
        }
        "=" if !member && (declared(tokens, k) || in_parameters(tokens, k)) => None,
        "=" => Some(Given::Value(op + 1..value_end(tokens, op + 1))),
        "+=" | "||=" | "&&=" | "??="
            if member || !(declared(tokens, k) || in_parameters(tokens, k)) =>
        {
            Some(Given::Unread("added to"))
        }
        ":" if op == after => {
            if matches!(before, Some("?" | "case")) || in_conditional(tokens, k) {
                return None;
            }
            if member {
                return Some(Given::Value(op + 1..value_end(tokens, op + 1)));
            }
            match innermost_open(tokens, k) {
                None => Some(Given::Value(op + 1..value_end(tokens, op + 1))),
                Some(open) if tokens[open].text == "{" && !is_pattern(tokens, open) => {
                    Some(Given::Value(op + 1..value_end(tokens, op + 1)))
                }
                Some(_) => None,
            }
        }
        "," | "]" | "}" if member && in_pattern(tokens, k) => {
            Some(Given::Unread("written by a destructuring"))
        }
        "of" | "in" if member && in_loop_head(tokens, k) => {
            Some(Given::Unread("written by a loop"))
        }
        _ => None,
    }
}

/// The innermost bracket still open before `k`.
fn innermost_open(tokens: &[Token], k: usize) -> Option<usize> {
    let mut depth = 0usize;
    for j in (0..k).rev() {
        match tokens[j].text.as_str() {
            ")" | "]" | "}" => depth += 1,
            "(" | "[" | "{" if depth == 0 => return Some(j),
            "(" | "[" | "{" => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Whether the `(` at `open` opens a parameter list: a function's, an
/// arrow's, a method's.
fn is_parameter_list(tokens: &[Token], open: usize) -> bool {
    if tokens[open].text != "(" {
        return false;
    }
    let before = open.checked_sub(1).map(|b| tokens[b].text.as_str());
    let further = open.checked_sub(2).map(|b| tokens[b].text.as_str());
    let close = matching_close(tokens, open, "(", ")");
    let then = close
        .and_then(|c| tokens.get(c + 1))
        .map(|t| t.text.as_str());
    before == Some("function")
        || (before.is_some_and(is_name) && further == Some("function"))
        || then == Some("=>")
        || (then == Some("{")
            && before.is_some_and(|b| {
                is_name(b) && !matches!(b, "if" | "while" | "for" | "switch" | "with" | "catch")
            }))
}

/// Whether the `[` or `{` at `open` is a binding pattern: a destructuring
/// assigned or declared, a parameter's, a loop's, or one inside another.
fn is_pattern(tokens: &[Token], open: usize) -> bool {
    let (opening, closing) = match tokens[open].text.as_str() {
        "[" => ("[", "]"),
        "{" => ("{", "}"),
        _ => return false,
    };
    let then = matching_close(tokens, open, opening, closing)
        .and_then(|c| tokens.get(c + 1))
        .map(|t| t.text.as_str());
    if matches!(then, Some("=" | "of" | "in")) {
        return true;
    }
    let before = open.checked_sub(1).map(|b| tokens[b].text.as_str());
    if matches!(before, Some("const" | "let" | "var")) {
        return true;
    }
    if matches!(before, Some("=")) {
        return false;
    }
    match innermost_open(tokens, open) {
        Some(outer) if tokens[outer].text == "(" => is_parameter_list(tokens, outer),
        Some(outer) => is_pattern(tokens, outer),
        None => false,
    }
}

fn in_pattern(tokens: &[Token], k: usize) -> bool {
    innermost_open(tokens, k).is_some_and(|open| is_pattern(tokens, open))
}

fn in_parameters(tokens: &[Token], k: usize) -> bool {
    innermost_open(tokens, k).is_some_and(|open| is_parameter_list(tokens, open))
}

fn in_loop_head(tokens: &[Token], k: usize) -> bool {
    innermost_open(tokens, k).is_some_and(|open| {
        tokens[open].text == "(" && open.checked_sub(1).is_some_and(|b| tokens[b].text == "for")
    })
}

/// Whether the name at `k` is a local declared: after `const`, `let` or
/// `var`, or after a `,` of a declaration's list.
fn declared(tokens: &[Token], k: usize) -> bool {
    let before = k.checked_sub(1).map(|b| tokens[b].text.as_str());
    if matches!(before, Some("const" | "let" | "var")) {
        return true;
    }
    if before != Some(",") {
        return false;
    }
    let mut depth = 0usize;
    for j in (0..k - 1).rev() {
        match tokens[j].text.as_str() {
            ")" | "]" | "}" => depth += 1,
            "(" | "[" | "{" if depth == 0 => return false,
            "(" | "[" | "{" => depth -= 1,
            ";" if depth == 0 => return false,
            "const" | "let" | "var" if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// Whether the name at `k` stands between a conditional's `?` and its `:`.
fn in_conditional(tokens: &[Token], k: usize) -> bool {
    let mut depth = 0usize;
    let mut open = 0isize;
    for j in (0..k).rev() {
        match tokens[j].text.as_str() {
            ")" | "]" | "}" => depth += 1,
            "(" | "[" | "{" if depth == 0 => break,
            "(" | "[" | "{" => depth -= 1,
            ";" | "," if depth == 0 => break,
            "?" if depth == 0 => open += 1,
            ":" if depth == 0 => open -= 1,
            _ => {}
        }
    }
    open > 0
}

fn is_string(text: &str) -> bool {
    text.starts_with(['"', '\'', '`'])
}

/// The first argument of the call whose `(` is at `open`; `Err` for a call
/// that does not close.
fn argument(tokens: &[Token], open: usize) -> Result<Range<usize>, String> {
    let close = matching_close(tokens, open, "(", ")").ok_or("a call that does not close")?;
    let end = top_level(&tokens[open + 1..close], &[","])
        .first()
        .map_or(open + 1, |(_, to)| open + 1 + to);
    Ok(open + 1..end)
}

/// Where a value starting at `from` ends: at the `,`, `;` or closing bracket
/// the stretch it stands in ends at, or at the end of its line where nothing
/// carries it on — a bracket still open, an operator ending the line or
/// opening the next.
fn value_end(tokens: &[Token], from: usize) -> usize {
    let carries = |text: &str| {
        matches!(
            text,
            "+" | "-"
                | "*"
                | "/"
                | "%"
                | "?"
                | ":"
                | "||"
                | "&&"
                | "??"
                | "."
                | "?."
                | "="
                | "=="
                | "==="
                | "!="
                | "!=="
                | "<"
                | ">"
                | "<="
                | ">="
        )
    };
    let mut depth = 0usize;
    let mut k = from;
    while let Some(token) = tokens.get(k) {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" | "," | ";" if depth == 0 => return k,
            ")" | "]" | "}" => depth -= 1,
            _ => {}
        }
        let next = tokens.get(k + 1);
        if depth == 0
            && next.is_some_and(|n| n.line > token.line && !carries(&n.text))
            && !carries(&token.text)
        {
            return k + 1;
        }
        k += 1;
    }
    k
}

/// The tokens as written, for a reason.
fn spelled(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The strings an expression can be, where it is written out: a string, a
/// template with nothing to fill in, strings joined by `+`, either side of a
/// `?:` or of `||` and `??`, what `Qt.resolvedUrl` is handed. `None` for
/// anything else — a name, a call, a template that fills one in — whose
/// value nothing here works out, and for joinings past 64 strings.
fn values(t: &[Token]) -> Option<Vec<String>> {
    let t = &t[unparenthesized(t, 0..t.len())];
    if let Some((question, colon)) = conditional(t) {
        let mut either = values(&t[question + 1..colon])?;
        either.extend(values(&t[colon + 1..])?);
        return Some(either);
    }
    let choices = top_level(t, &["||", "??"]);
    if choices.len() > 1 {
        let mut either = Vec::new();
        for (from, to) in choices {
            either.extend(values(&t[from..to])?);
        }
        return Some(either);
    }
    let parts = top_level(t, &["+"]);
    if parts.len() > 1 {
        let mut joined = vec![String::new()];
        for (from, to) in parts {
            let part = values(&t[from..to])?;
            joined = joined
                .iter()
                .flat_map(|head| part.iter().map(move |tail| format!("{head}{tail}")))
                .collect();
            if joined.len() > 64 {
                return None;
            }
        }
        return Some(joined);
    }
    match t {
        [one] => string_value(&one.text).map(|value| vec![value]),
        [qt, dot, resolved, open, .., close]
            if qt.text == "Qt"
                && dot.text == "."
                && resolved.text == "resolvedUrl"
                && open.text == "("
                && close.text == ")"
                && matching_close(t, 3, "(", ")") == Some(t.len() - 1) =>
        {
            values(&t[4..t.len() - 1])
        }
        _ => None,
    }
}

/// The `?` and `:` of a conditional at the top level of `t`.
fn conditional(t: &[Token]) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut question = None;
    let mut nested = 0usize;
    for (k, token) in t.iter().enumerate() {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "?" if depth == 0 && question.is_none() => question = Some(k),
            "?" if depth == 0 => nested += 1,
            ":" if depth == 0 && question.is_some() && nested == 0 => {
                return question.map(|q| (q, k));
            }
            ":" if depth == 0 && question.is_some() => nested -= 1,
            _ => {}
        }
    }
    None
}

/// The stretches of `t` between its top-level `separators`.
fn top_level(t: &[Token], separators: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut from = 0;
    for (k, token) in t.iter().enumerate() {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            text if depth == 0 && separators.contains(&text) => {
                out.push((from, k));
                from = k + 1;
            }
            _ => {}
        }
    }
    out.push((from, t.len()));
    out
}

#[cfg(test)]
mod tests {
    use super::{Named, loads};

    #[test]
    fn a_property_name_alone_is_not_a_file_load() {
        for text in [
            "Item { id: item; Drag.source: item }",
            "Item { id: item; Drag { source: item } }",
            "Item { property url source: item.url }",
        ] {
            assert_eq!(named(text), Ok(Vec::new()), "{text}");
        }
    }

    #[test]
    fn files_and_unresolved_file_readers_are_not_silently_dropped() {
        assert!(loads("const component = '../../outside/Card.qml'").is_ok());
        assert!(loads("const component = 'file:///outside/Card.qml'").is_err());
        assert_eq!(
            named("ShaderEffect { fragmentShader: 'paint.frag.qsb' }"),
            Ok(vec!["paint.frag.qsb".into()])
        );
        assert!(loads("const request = new XMLHttpRequest()").is_err());
        assert!(loads("StackView { id: stack }\nfunction f(url) { stack.push(url) }").is_err());
    }

    /// The paths a text's loads name, or why they cannot be named.
    fn named(text: &str) -> Result<Vec<String>, String> {
        loads(text).map(|named| {
            named
                .into_iter()
                .filter(|n| n.load)
                .map(|n| n.path)
                .collect()
        })
    }

    /// The same imports in every spelling Qt takes — either quote, broken
    /// across lines, a comment between the words — read alike; a commented
    /// import, a module import and a resource are none; a file a string
    /// names is loaded too.
    #[test]
    fn an_import_is_read_by_its_tokens_in_every_spelling() {
        let text = "import QtQuick\nimport platitude.ui\nimport \"../../src/auto\" as Automation\n\
                    import\n    '../shared/helper.js' /* the answer */\n    as Helper\n\
                    import \"painted.js\" as Painted // beside it\n  .import 'inner.js' as Inner\n\
                    import { mark } from './mark.mjs'\n// import \"commented.js\" as Gone\n\
                    /* import 'blocked.js' as Gone */\nimport \"qrc:/x.qml\"\n\
                    Loader { source: 'Card.qml' }\nText { text: \"plain words\" }\n\
                    import 'esc\\'aped.js' as E\nproperty string named: \"Named.qml\"\n";
        assert_eq!(
            loads(text).expect("read to its end"),
            [
                ("../../src/auto", true),
                ("../shared/helper.js", true),
                ("painted.js", true),
                ("inner.js", true),
                ("./mark.mjs", true),
                ("Card.qml", true),
                ("esc'aped.js", true),
                ("Named.qml", false),
            ]
            .map(|(path, load)| Named {
                path: path.to_string(),
                load
            })
        );
        assert_eq!(
            named("import 'a.js' as A\n"),
            named("import \"a.js\"\n    as A\n")
        );
        assert!(
            loads("import 'a.js' as A\nproperty string s: \"open\n").is_err(),
            "a text read short loads what nothing can say"
        );
    }

    /// What a load is handed is worked out however it is written — a
    /// string, strings joined, either side of a choice, a template with
    /// nothing to fill in, through `Qt.resolvedUrl`, bound or assigned, a
    /// component made or a source set — and every spelling of one target
    /// names that target.
    #[test]
    fn a_load_names_its_target_however_its_value_is_written() {
        for spelled in [
            "Loader { source: \"../../shared/Card.qml\" }",
            "Loader { source: \"../../shared/\" + \"Card.qml\" }",
            "Loader { source: '../../' + ('shared/' + 'Card' + '.qml') }",
            "Loader { source: Qt.resolvedUrl(`../../shared/Card.qml`) }",
            "Loader {\n    source: true\n        ? \"../../shared/Card.qml\"\n        : '../../shared/' + 'Card.qml'\n}",
            "Loader { id: l }\nfunction f() { l.source = \"../../shared/\" + \"Card.qml\" }",
            "function f() { return Qt.createComponent('../../shared/' + 'Card.qml') }",
            "function f(l) { l.setSource(\"../../shared/\" + \"Card.qml\", {}) }",
            "function f(l) { l[\"setSource\"](\"../../shared/Card.qml\") }",
            "Loader { source: \"\" || \"../../shared/Card.qml\" }",
            "Loader { source: \"../../shared/%43ard.qml?at=1#top\" }",
            "Loader { source: \"../../shared/Card\\u002Eqml\" }",
            "function f(c) { return c.createObject(null, {\"source\": \"../../shared/Card.qml\"}) }",
        ] {
            assert_eq!(
                named(spelled),
                Ok(vec!["../../shared/Card.qml".to_string()]),
                "{spelled}"
            );
        }
        assert_eq!(
            named("Loader { source: cond ? 'A.qml' : 'B.qml' }"),
            Ok(vec!["A.qml".to_string(), "B.qml".to_string()])
        );
        assert_eq!(
            named("Image { source: \"qrc:/icons/a.svg\" }\nLoader { source: \"\" }"),
            Ok(Vec::new())
        );
    }

    /// A write to `source` is a load whatever wraps it — parentheses around
    /// the assignment, its value or its target, a computed key, an operand
    /// of another expression —, and a name that only names a local, a
    /// parameter, a pattern's binding or a conditional's branch is none:
    /// were it read as a load, its value, which nothing works out, would
    /// leave the load unnamed.
    #[test]
    fn a_write_to_source_is_told_from_a_name_that_only_names_it() {
        let value = "\"../../shared/\" + \"Card.qml\"";
        for wrapped in [
            "card.source = V",
            "(card.source = V)",
            "((card.source = V))",
            "card.source = (V)",
            "(card.source) = V",
            "card[\"source\"] = V",
            "card['source'] = (V)",
            "void (card.source = V)",
            "ready && (card.source = V)",
            "f(card.source = V)",
            "if (card.source = V) {}",
            "source = V",
            "(source = V)",
        ] {
            let text = format!(
                "Loader {{ id: card }}\nComponent.onCompleted: {{ {} }}\n",
                wrapped.replace('V', value)
            );
            assert_eq!(
                named(&text),
                Ok(vec!["../../shared/Card.qml".to_string()]),
                "{wrapped}"
            );
        }
        for only_named in [
            "const source = where",
            "let a = 1, source = where",
            "for (let i = 0, source = where; i < 1; i++) {}",
            "function f(source = where) {}",
            "function f(source: url) {}",
            "const f = (a, source = where) => a",
            "const {source = where} = o",
            "const {source: s = where} = o",
            "({source = where} = o)",
            "const [source = where] = o",
            "function f({source = where}) {}",
            "const x = ready ? source : where",
            "switch (x) { case source: break }",
            "compare(card.source, where)",
            "const s = card.source",
        ] {
            let text = format!("Item {{\n    function f() {{ {only_named} }}\n}}\n");
            assert_eq!(named(&text), Ok(Vec::new()), "{only_named}");
        }
    }

    /// A load whose value is not written out — a name, a call, a template
    /// that fills one in —, a property added to or written by a
    /// destructuring or a loop, a URL off the tree or read otherwise than
    /// as written, and a component made from text name no target: what
    /// they load is not known.
    #[test]
    fn a_load_nothing_works_out_is_no_target() {
        for spelled in [
            "Loader { source: \"../../shared/\" + name + \".qml\" }",
            "Loader { source: root.card }",
            "Loader { source: `../../shared/${name}.qml` }",
            "Loader { source: pick(\"Card.qml\") }",
            "function f(l) { l.source = where }",
            "function f(l) { l.source += \".qml\" }",
            "function f(l) { [l.source] = [\"Card.qml\"] }",
            "function f(l) { ({a: l.source} = {a: \"Card.qml\"}) }",
            "function f(l) { for (l.source of [\"Card.qml\"]) {} }",
            "function f() { return Qt.createComponent(where) }",
            "Loader { source: \"file:///tmp/Card.qml\" }",
            "Loader { source: \"/tmp/Card.qml\" }",
            "Loader { source: \"..\\\\shared\\\\Card.qml\" }",
            "Loader { source: \"Card%.qml\" }",
            "import '/abs'\n",
            "function f(o) { return Qt.createQmlObject('import \"x\"; Item {}', o) }",
        ] {
            assert!(loads(spelled).is_err(), "{spelled}: {:?}", loads(spelled));
        }
    }
}
