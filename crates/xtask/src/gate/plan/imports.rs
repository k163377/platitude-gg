//! What QML loads off the tree by a relative path, read off each load the
//! text holds: an import statement's string — `import "dir"`,
//! `import 'file.js' as X`, a JS file's `.import "file.js" as X`, a module's
//! `import … from "./m.mjs"` —, a value written to a property the QML types
//! load from (`source`: bound, assigned — inside parentheses or not, by a
//! name or a computed key —, given as a field), and what a function that
//! loads is handed (`Qt.createComponent`, `setSource`, `Qt.resolvedUrl`,
//! `Qt.include`, `import(…)`). The text is read as the engine's lexer reads
//! it (`crate::lex`), and a write is told from a local declared, a
//! parameter, a pattern's binding or a conditional's branch by where it
//! stands (`written`), so quoting, spacing, line breaks, comments and
//! parentheses that change no meaning do not change what is read.
//!
//! Where a load is is one question; what it loads is another: a load's
//! value is worked out as far as it is written out — strings, joined by
//! `+`, chosen by `?:`, `||` or `??`, handed through `Qt.resolvedUrl`
//! (`values`) —, and the file that value names on disk is worked out as Qt
//! works it out (`super::urls`: the string's escapes, the URL, its
//! resolution, the file). A string naming a `.qml`, `.js` or `.mjs` file
//! loads it too, by whatever route. A directory's `qmldir` adds the files
//! it names. A module import (`import QtQuick`, `import platitude.ui`)
//! resolves on the import path instead — what a runner stages there is its
//! own to name (`crate::qmltest`).
//!
//! What cannot be followed is not dropped: a path the tree does not hold is
//! kept as it is named, so that its appearing moves a key; a load whose
//! value or file nothing here works out, and a text the lexer could not
//! read to its end, leave what is loaded unnamed (`Loaded::unbounded`), and
//! no key answers for a run that reads them.

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;

use super::under;
use super::urls::{file_of, string_value, url_path};
use crate::lex::{Lang, Token, blanked, matching_close, tokens};

/// What the QML and JS under a directory load off the tree.
#[derive(Debug, Default)]
pub(super) struct Loaded {
    /// The files and directories they load, as workspace paths.
    pub paths: BTreeSet<String>,
    /// Why what they load cannot all be named, where it cannot: the first
    /// load nothing here follows, and the file it stands in.
    pub unbounded: Option<String>,
}

/// A URL path a text names ([`url_path`]): by a load, or by a string that
/// only names a file a load may read by any route — which names nothing
/// where its file cannot be worked out.
#[derive(Debug, PartialEq, Eq)]
struct Named {
    path: String,
    load: bool,
}

/// The properties a QML type loads a file from.
const LOADED_FROM: [&str; 3] = ["source", "fragmentShader", "vertexShader"];

/// The functions that load what their first argument names, and the one
/// that makes a component out of text, whose imports nothing here reads.
const LOADING: [&str; 4] = ["createComponent", "setSource", "resolvedUrl", "include"];
const FROM_TEXT: &str = "createQmlObject";

/// What the QML and JS files under `from` (workspace paths) load off the
/// tree at `dir` by relative path, and what that loads in turn: a
/// directory as itself — an import of one reads its qmldir and every type
/// in it —, a file as itself. Nothing under `from`, which is read already.
pub(super) fn loaded_from(dir: &Path, from: &[String]) -> Loaded {
    let mut loaded = Loaded::default();
    let mut queue = Vec::new();
    for path in from {
        enqueue(dir, path, &mut queue, &mut loaded);
    }
    let mut read = BTreeSet::new();
    while let Some(file) = queue.pop() {
        if !read.insert(file.clone()) {
            continue;
        }
        let base = Path::new(&file).parent().unwrap_or(Path::new(""));
        // A QML file implicitly imports its own directory. Key the
        // directory too: a new sibling type or qmldir can change resolution.
        if file.ends_with(".qml") {
            let directory = base.to_string_lossy().replace('\\', "/");
            if !directory.is_empty()
                && !from.iter().any(|f| under(&directory, f))
                && loaded.paths.insert(directory.clone())
            {
                enqueue(dir, &directory, &mut queue, &mut loaded);
            }
        }
        let named = match std::fs::read_to_string(dir.join(&file)) {
            Ok(text) if file.ends_with("qmldir") => qmldir_files(&text),
            Ok(text) => loads(&text),
            Err(_) => Err("a file that is no text".to_string()),
        };
        let named = match named {
            Ok(named) => named,
            Err(why) => {
                unbounded(&mut loaded, &file, why);
                continue;
            }
        };
        for Named { path, .. } in named {
            let target = match file_of(dir, base, &path) {
                Ok(target) => target,
                Err(why) => {
                    unbounded(&mut loaded, &file, why);
                    continue;
                }
            };
            if loaded.paths.contains(&target) || from.iter().any(|f| under(&target, f)) {
                continue;
            }
            // Kept as named where the tree does not hold it: its appearing moves the key.
            enqueue(dir, &target, &mut queue, &mut loaded);
            loaded.paths.insert(target);
        }
    }
    loaded
}

/// Notes the first reason what is loaded cannot all be named.
fn unbounded(loaded: &mut Loaded, file: &str, why: String) {
    if loaded.unbounded.is_none() {
        loaded.unbounded = Some(format!("{file}: {why}"));
    }
}

/// The files at `path` a load reads on: the file itself, or every QML and
/// JS file under the directory and its `qmldir` (deeper than an import
/// reads, which costs an input too many at most). None where the tree holds
/// nothing at `path`: a path is keyed as absent by whoever names it.
fn enqueue(dir: &Path, path: &str, queue: &mut Vec<String>, loaded: &mut Loaded) {
    match sources(dir, path) {
        Ok(files) => queue.extend(files),
        Err(why) => unbounded(loaded, path, why),
    }
}

/// Walk every source and qmldir, including nested module definitions. An unreadable tree is not an empty tree.
fn sources(dir: &Path, path: &str) -> Result<Vec<String>, String> {
    let full = dir.join(path);
    let metadata = match std::fs::symlink_metadata(&full) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot inspect {}: {error}", full.display())),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("a symbolic link is not a source closure: {path}"));
    }
    if !metadata.is_dir() {
        return if path.ends_with("qmldir")
            || [".qml", ".js", ".mjs"]
                .iter()
                .any(|end| path.ends_with(end))
        {
            Ok(vec![path.to_string()])
        } else {
            Ok(Vec::new())
        };
    }
    let mut found = Vec::new();
    let entries = std::fs::read_dir(&full)
        .map_err(|error| format!("cannot list {}: {error}", full.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot list {}: {error}", full.display()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("a source name is not UTF-8 under {}", full.display()))?;
        found.extend(sources(
            dir,
            &format!("{}/{name}", path.trim_end_matches('/')),
        )?);
    }
    Ok(found)
}

/// What a load is handed.
enum Given {
    /// The value's tokens.
    Value(Range<usize>),
    /// A value nothing here reads: a property added to, or written by a
    /// destructuring or a loop.
    Unread(&'static str),
}

/// The URL paths a QML or JS text names, as written: what each load names,
/// and each string naming a QML or JS file. `Err` where what a load names
/// is not known — a value nothing here works out, a URL off the tree, a
/// component made from text — or the text could not be read to its end, so
/// that what stands past the stop is not known.
fn loads(text: &str) -> Result<Vec<Named>, String> {
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
                // A string naming a file of QML or JS, which a load may read by any route.
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
/// where it is written: assigned (`=`, inside parentheses or not, the
/// target itself parenthesized), added to, bound or given as a field, or
/// the target of a destructuring or a loop. `None` where the name is no
/// property written: a local declared, a parameter or its type, a
/// pattern's binding, a conditional's branch, a label, a read.
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

/// Whether `k` stands inside a binding pattern.
fn in_pattern(tokens: &[Token], k: usize) -> bool {
    innermost_open(tokens, k).is_some_and(|open| is_pattern(tokens, open))
}

/// Whether `k` stands in a parameter list.
fn in_parameters(tokens: &[Token], k: usize) -> bool {
    innermost_open(tokens, k).is_some_and(|open| is_parameter_list(tokens, open))
}

/// Whether `k` stands in a `for (…)` head.
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

/// Whether a token is a name.
fn is_name(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// Whether a token is a string literal.
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
/// value nothing here works out, and for more than a handful of joinings.
fn values(t: &[Token]) -> Option<Vec<String>> {
    let mut t = t;
    while t.len() >= 2 && t[0].text == "(" && matching_close(t, 0, "(", ")") == Some(t.len() - 1) {
        t = &t[1..t.len() - 1];
    }
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

/// The files a `qmldir` names, as URL paths relative to it: the last word
/// of each type, singleton, internal or script line that ends in a QML or
/// JS file; `Err` for one off the tree.
fn qmldir_files(text: &str) -> Result<Vec<Named>, String> {
    let mut out = Vec::new();
    let words = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_whitespace().last())
        .filter(|word| {
            [".qml", ".js", ".mjs"]
                .iter()
                .any(|end| word.ends_with(end))
        });
    for word in words {
        if let Some(path) = url_path(word)? {
            out.push(Named { path, load: true });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{Named, loaded_from, loads, qmldir_files};
    use std::path::Path;

    #[test]
    fn nested_module_definitions_and_failed_walks_cannot_drop_inputs() {
        let root = crate::yard::Yard::new("qml-nested-modules");
        std::fs::create_dir_all(root.join("tests/sub")).unwrap();
        std::fs::create_dir_all(root.join("shared")).unwrap();
        std::fs::write(
            root.join("tests/sub/qmldir"),
            "Card 1.0 ../../shared/Card.qml\n",
        )
        .unwrap();
        std::fs::write(root.join("shared/Card.qml"), "import QtQuick\nItem {}\n").unwrap();
        let loaded = loaded_from(&root, &["tests".into()]);
        assert!(loaded.unbounded.is_none(), "{loaded:?}");
        assert!(loaded.paths.contains("shared/Card.qml"), "{loaded:?}");

        // A platform-independent metadata error: an invalid path must not be mistaken for an absent file.
        let failed = loaded_from(&root, &["tests/\0".into()]);
        assert!(
            failed.unbounded.is_some(),
            "a failed walk cannot keep a cache key: {failed:?}"
        );
    }

    #[test]
    fn implicit_siblings_and_their_module_are_inputs() {
        let root = crate::yard::Yard::new("qml-neighbours");
        for (path, text) in [
            (
                "tests/tst_a.qml",
                "import QtQuick\nLoader { source: '../cards/Card.qml' }",
            ),
            ("cards/Card.qml", "import QtQuick\nNeighbour {}"),
            ("cards/Neighbour.qml", "import QtQuick\nItem {}"),
            ("cards/qmldir", "Neighbour 1.0 Neighbour.qml"),
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        let got = loaded_from(&root, &["tests".into()]);
        assert!(got.unbounded.is_none(), "{got:?}");
        assert!(
            got.paths
                .iter()
                .any(|p| super::under("cards/Neighbour.qml", p)),
            "{got:?}"
        );
        assert!(
            got.paths.iter().any(|p| super::under("cards/qmldir", p)),
            "{got:?}"
        );
    }

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

    #[test]
    fn a_qmldir_names_the_files_of_its_types() {
        let text = "module platitude.ui\n# a comment.qml\nsingleton Theme 1.0 Theme.qml\n\
                    internal Seat ../seat/Seat.qml\nCard 1.0 Card%20View.qml\nscript 1.0 tools.js\nplugin x\n";
        let paths: Vec<String> = qmldir_files(text)
            .expect("on the tree")
            .into_iter()
            .map(|n| n.path)
            .collect();
        assert_eq!(
            paths,
            [
                "Theme.qml",
                "../seat/Seat.qml",
                "Card%20View.qml",
                "tools.js"
            ]
        );
    }

    /// The tests' loads, followed as far as they lead: a directory, a
    /// helper beside them, what the directory imports and its qmldir names
    /// in turn — none of it under the tests' own directory, which is read
    /// already —, the file a URL names on disk, a helper the tree does not
    /// hold kept as named; a load nothing follows leaves what is loaded
    /// unbounded, and a directory the tree does not hold loads nothing.
    #[test]
    fn what_the_tests_load_is_followed_through_what_it_loads() {
        let root = crate::yard::Yard::new("qml-imports");
        let write = |path: &str, text: &str| {
            let full = root.join(path);
            std::fs::create_dir_all(full.parent().expect("a parent")).expect("a directory");
            std::fs::write(full, text).expect("a file");
        };
        write(
            "app/tests/qml/tst_a.qml",
            "import \"../../src/auto\" as A\nimport 'painted.js' as P\nimport \"../gone\"\n\
             Loader { source: '../../src/' + 'cards/Card%20View.qml' }\n",
        );
        write(
            "app/tests/qml/painted.js",
            ".import '../../src/shared/ink.js' as Ink\n",
        );
        write("app/src/auto/Driver.qml", "import \"../lib\"\nItem {}\n");
        write("app/src/auto/qmldir", "Seat 1.0 ../seat/Seat.qml\n");
        write("app/src/seat/Seat.qml", "Item {}\n");
        write("app/src/lib/Lib.qml", "Item {}\n");
        write("app/src/cards/Card View.qml", "Item {}\n");
        write("app/src/shared/ink.js", "function ink() {}\n");
        write("app/src/ui/Untouched.qml", "Item {}\n");
        let loaded = loaded_from(&root, &["app/tests/qml".to_string()]);
        let expected: Vec<&str> = vec![
            "app/src/auto",
            "app/src/cards",
            "app/src/cards/Card View.qml",
            "app/src/lib",
            "app/src/seat",
            "app/src/seat/Seat.qml",
            "app/src/shared/ink.js",
            "app/tests/gone",
        ];
        assert_eq!(
            loaded.paths.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(loaded.unbounded, None);
        for (file, text) in [
            (
                "app/tests/out/tst_b.qml",
                "import '../../../../outside.js' as Out\n",
            ),
            (
                "app/tests/named/tst_c.qml",
                "Loader { source: '../' + card }\n",
            ),
            (
                "app/tests/escaped/tst_d.qml",
                "Loader { source: '../shared%2FCard.qml' }\n",
            ),
        ] {
            write(file, text);
            let dir = Path::new(file).parent().expect("dir").display().to_string();
            let loaded = loaded_from(&root, &[dir.replace('\\', "/")]);
            assert!(
                loaded
                    .unbounded
                    .as_deref()
                    .is_some_and(|why| why.starts_with(file)),
                "{file}: {loaded:?}"
            );
        }
        let loaded = loaded_from(&root, &["app/tests/none".to_string()]);
        assert!(
            loaded.paths.is_empty() && loaded.unbounded.is_none(),
            "{loaded:?}"
        );
        let odd = root.join("app/tests/odd/tst_odd.qml");
        std::fs::create_dir_all(odd.parent().expect("parent")).expect("dir");
        std::fs::write(&odd, [0xff, 0xfe, 0x00]).expect("odd");
        let loaded = loaded_from(&root, &["app/tests/odd".to_string()]);
        assert!(loaded.unbounded.is_some(), "{loaded:?}");
    }

    /// Every URL path `tst_loadspellings` spells, read off its own text,
    /// names one of the two files Qt reads there: the file each spelling's
    /// run compares against is the one the reader keys it on.
    #[test]
    fn each_spelling_of_the_spelling_test_names_the_file_qt_reads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let file = "crates/platitude-app/tests/qml/tst_loadspellings.qml";
        let text = std::fs::read_to_string(root.join(file)).expect("the spelling test");
        let base = Path::new(file).parent().expect("its directory");
        let plain = "crates/platitude-app/tests/loads/Plain.qml";
        let words = "crates/platitude-app/tests/loads/Two Words.qml";
        let named: Vec<(String, String)> = loads(&text)
            .expect("read to its end")
            .into_iter()
            .filter(|n| n.load)
            .map(|n| {
                let target = super::file_of(&root, base, &n.path).expect("a file of the tree");
                (n.path, target)
            })
            .collect();
        assert_eq!(
            named,
            [
                ("../loads/Plain.qml", plain),
                ("../loads/./Plain.qml", plain),
                ("../loads/Two%20Words.qml", words),
                ("../loads/Two Words.qml", words),
            ]
            .map(|(path, target)| (path.to_string(), target.to_string()))
        );
    }

    /// The tree's own QML tests load what the reader follows: the harness's
    /// module and the files `tst_loadspellings` loads, every load named, so
    /// their stamps keep.
    #[test]
    fn the_trees_qml_tests_load_what_is_followed() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let loaded = loaded_from(&root, &["crates/platitude-app/tests/qml".to_string()]);
        for path in [
            "crates/platitude-app/src/auto",
            "crates/platitude-app/tests/loads/Plain.qml",
            "crates/platitude-app/tests/loads/Two Words.qml",
        ] {
            assert!(loaded.paths.contains(path), "{path}: {loaded:?}");
        }
        assert!(
            loaded
                .paths
                .iter()
                .all(|path| path.starts_with("crates/platitude-app/")),
            "{loaded:?}"
        );
        assert_eq!(loaded.unbounded, None);
    }
}
