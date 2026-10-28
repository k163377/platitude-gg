//! What a value a QML load is handed names on disk, step by step as Qt 6
//! takes it: the string a literal spells, its escapes read as JS reads
//! them ([`string_value`]); the URL that string is — a query and a fragment
//! name no file, a scheme names no file of the tree, an escape of an
//! unreserved character is that character ([`url_path`]); that URL
//! resolved against the loading file's, its dot segments removed, and the
//! file it then is — the rest of its escapes read as UTF-8, spelled as the
//! tree spells it ([`file_of`]). Where a step is not certainly what Qt
//! does — a backslash, an escape of a separator or of `%`, an escape that
//! is no UTF-8 — it says so (`Err`), and no file is named.
//!
//! Measured with `qmltestrunner` (Qt 6.12, Windows): `Card%20View.qml`,
//! `Card View.qml`, `Card\u0020View.qml` and `Card\x20View.qml` load one
//! file; so do `%43ard.qml` and `Card.qml`, `Card.qml?x=1#f` and
//! `Card.qml`, `sub/%2E%2E/Card.qml` and `Card.qml`; `..\shared\Card.qml`,
//! `Card%25.qml` and `file:../Card.qml` load nothing.

use std::path::{Component, Path};

/// A string token's value, as JS reads it: its quotes (`"`, `'`, or a
/// template's backticks with nothing to fill in) taken off and its escapes
/// read. `None` for a template that fills one in, or an escape JS rejects.
pub(super) fn string_value(token: &str) -> Option<String> {
    let quote = token
        .chars()
        .next()
        .filter(|c| matches!(c, '"' | '\'' | '`'))?;
    let inner = token.strip_prefix(quote)?.strip_suffix(quote)?;
    if quote == '`' && inner.contains("${") {
        return None;
    }
    let mut value = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            value.push(c);
            continue;
        }
        match chars.next()? {
            'n' => value.push('\n'),
            't' => value.push('\t'),
            'r' => value.push('\r'),
            'b' => value.push('\u{8}'),
            'f' => value.push('\u{c}'),
            'v' => value.push('\u{b}'),
            '0' if !chars.peek().is_some_and(char::is_ascii_digit) => value.push('\0'),
            'x' => {
                let hex: String = [chars.next()?, chars.next()?].iter().collect();
                value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
            }
            'u' => value.push(unicode_escape(&mut chars)?),
            // A line continued: no character.
            '\n' | '\u{2028}' | '\u{2029}' => {}
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            other => value.push(other),
        }
    }
    Some(value)
}

/// The character a `\u` escape names — `\uXXXX`, a surrogate pair of two,
/// or `\u{X…}` —, read past the `u`.
fn unicode_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<char> {
    let unit = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>| -> Option<u32> {
        if chars.peek() == Some(&'{') {
            chars.next();
            let hex: String = chars.by_ref().take_while(|c| *c != '}').collect();
            return u32::from_str_radix(&hex, 16).ok();
        }
        let hex: String = [chars.next()?, chars.next()?, chars.next()?, chars.next()?]
            .iter()
            .collect();
        u32::from_str_radix(&hex, 16).ok()
    };
    let first = unit(chars)?;
    if (0xD800..0xDC00).contains(&first) {
        let mut ahead = chars.clone();
        if ahead.next() == Some('\\') && ahead.next() == Some('u') {
            let second = unit(&mut ahead)?;
            if (0xDC00..0xE000).contains(&second) {
                *chars = ahead;
                return char::from_u32(0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00));
            }
        }
        return None;
    }
    char::from_u32(first)
}

/// The path of the URL a load's string is, where it is a reference to a
/// file beside the loading one: its query and fragment taken off and each
/// escape of an unreserved character read as that character. `None` for
/// no file of the tree — no value, a resource the runner holds none of, an
/// image a provider draws; `Err` for a URL off the tree (a scheme, an
/// authority, a path from the root) or one Qt reads otherwise than as
/// written (a backslash, a `%` that escapes nothing).
pub(super) fn url_path(value: &str) -> Result<Option<String>, String> {
    let path = value.split(['#', '?']).next().unwrap_or_default();
    if path.is_empty()
        || value.starts_with("qrc:")
        || value.starts_with(":/")
        || value.starts_with("image:")
    {
        return Ok(None);
    }
    let scheme = path
        .split_once(':')
        .is_some_and(|(head, _)| !head.contains('/'));
    if scheme || path.starts_with('/') {
        return Err(format!("a load of {value}, off the tree"));
    }
    if path.contains('\\') {
        return Err(format!(
            "a load of {value}, whose backslash Qt reads as no separator"
        ));
    }
    let bytes = path.as_bytes();
    let mut out = String::new();
    let mut k = 0;
    while k < bytes.len() {
        if bytes[k] != b'%' {
            let c = path[k..].chars().next().unwrap_or_default();
            out.push(c);
            k += c.len_utf8();
            continue;
        }
        let byte = path
            .get(k + 1..k + 3)
            .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            .ok_or_else(|| format!("a load of {value}, whose % escapes nothing"))?;
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&path[k..k + 3]);
        }
        k += 3;
    }
    Ok(Some(out))
}

/// The file of the tree at `dir` a URL path ([`url_path`]) names, resolved
/// against the loading file's directory `base` (a workspace path): its dot
/// segments removed, its escapes read as UTF-8, and spelled as the tree
/// spells it where the file system finds it otherwise spelled (a case
/// folded on Windows). `Err` for a path that climbs out of the workspace,
/// an escape of `/`, `\` or `%` (Qt opens it otherwise than a path reads),
/// or an escape that is no UTF-8.
pub(super) fn file_of(dir: &Path, base: &Path, path: &str) -> Result<String, String> {
    let resolved =
        folded(&base.join(path)).ok_or_else(|| format!("{path} climbs out of the tree"))?;
    let lower = resolved.to_ascii_lowercase();
    if ["%2f", "%5c", "%25"]
        .iter()
        .any(|escape| lower.contains(escape))
    {
        return Err(format!(
            "a load of {path}, whose escaped separator or % Qt opens otherwise"
        ));
    }
    let decoded = percent_decoded(&resolved)
        .ok_or_else(|| format!("a load of {path}, whose escapes are no UTF-8"))?;
    Ok(as_the_tree_spells(dir, &decoded).unwrap_or(decoded))
}

/// `text` with each `%XX` read as a byte, the bytes as UTF-8.
fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut k = 0;
    while k < bytes.len() {
        if bytes[k] == b'%' {
            out.push(u8::from_str_radix(text.get(k + 1..k + 3)?, 16).ok()?);
            k += 3;
        } else {
            out.push(bytes[k]);
            k += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The workspace path the file system opens for `path`, where it opens one:
/// the tree's own spelling of each part.
fn as_the_tree_spells(dir: &Path, path: &str) -> Option<String> {
    let real = dir.join(path).canonicalize().ok()?;
    let root = dir.canonicalize().ok()?;
    let inside = real.strip_prefix(&root).ok()?;
    let spelled = inside.to_string_lossy().replace('\\', "/");
    (!spelled.is_empty()).then_some(spelled)
}

/// `path` as a workspace path, its `.` and `..` folded away; `None` for
/// one that climbs out of the workspace.
pub(super) fn folded(path: &Path) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::{file_of, string_value, url_path};
    use std::path::Path;

    /// A literal's value is what JS reads: each escape, a template with
    /// nothing to fill in; a template that fills one in, and a malformed
    /// escape, have no value here.
    #[test]
    fn a_literal_is_read_as_js_reads_it() {
        for (token, value) in [
            ("\"Card View.qml\"", "Card View.qml"),
            ("'Card\\u0020View.qml'", "Card View.qml"),
            ("\"Card\\x20View.qml\"", "Card View.qml"),
            ("\"Card\\u{20}View.qml\"", "Card View.qml"),
            ("`Card View.qml`", "Card View.qml"),
            ("'it\\'s.qml'", "it's.qml"),
            ("\"Caf\\u00e9.qml\"", "Café.qml"),
            ("\"\\uD83D\\uDE00.qml\"", "\u{1F600}.qml"),
            ("\"a\\\nb.qml\"", "ab.qml"),
        ] {
            assert_eq!(string_value(token).as_deref(), Some(value), "{token}");
        }
        for token in [
            "`Card${n}.qml`",
            "\"Card\\x2.qml\"",
            "\"\\uZZZZ\"",
            "name",
            "42",
        ] {
            assert_eq!(string_value(token), None, "{token}");
        }
    }

    /// A string is the URL Qt takes it as: one file however its unreserved
    /// characters are escaped, its query and fragment aside; a scheme, an
    /// authority, a path from the root, a backslash and a `%` that escapes
    /// nothing name no file of the tree.
    #[test]
    fn a_string_is_the_url_qt_takes_it_as() {
        for (value, path) in [
            ("../shared/Card.qml", "../shared/Card.qml"),
            ("../shared/%43ard.qml", "../shared/Card.qml"),
            ("../shared/Card.qml?x=1", "../shared/Card.qml"),
            ("../shared/Card.qml#frag", "../shared/Card.qml"),
            ("%2E%2E/shared/Card%2Eqml", "../shared/Card.qml"),
            ("../shared/Card%20View.qml", "../shared/Card%20View.qml"),
            ("../shared/Card View.qml", "../shared/Card View.qml"),
        ] {
            assert_eq!(url_path(value), Ok(Some(path.to_string())), "{value}");
        }
        for value in [
            "",
            "qrc:/Card.qml",
            ":/Card.qml",
            "image://provider/x",
            "?x=1",
        ] {
            assert_eq!(url_path(value), Ok(None), "{value}");
        }
        for value in [
            "file:../shared/Card.qml",
            "file:///tmp/Card.qml",
            "C:/tmp/Card.qml",
            "https://example.com/Card.qml",
            "//host/Card.qml",
            "/tmp/Card.qml",
            "..\\shared\\Card.qml",
            "../shared/Card%.qml",
            "../shared/Card%G0.qml",
        ] {
            assert!(url_path(value).is_err(), "{value}: {:?}", url_path(value));
        }
    }

    /// A URL path is the file its resolution names, its escapes read: one
    /// file however its separators and dot segments are written; an escape
    /// of a separator or of `%`, or one that is no UTF-8, names none.
    #[test]
    fn a_url_path_is_the_file_it_resolves_to() {
        let root = crate::yard::Yard::new("qml-urls");
        let base = Path::new("app/tests/qml");
        for (path, file) in [
            ("../../shared/Card%20View.qml", "app/shared/Card View.qml"),
            ("../../shared/Card View.qml", "app/shared/Card View.qml"),
            ("../../shared/x/../Card.qml", "app/shared/Card.qml"),
            ("../../shared//./Card.qml", "app/shared/Card.qml"),
            ("../../shared/Caf%C3%A9.qml", "app/shared/Café.qml"),
        ] {
            assert_eq!(file_of(&root, base, path), Ok(file.to_string()), "{path}");
        }
        for path in [
            "../../shared%2FCard.qml",
            "../../shared%5CCard.qml",
            "../../shared/Card%25.qml",
            "../../shared/Caf%E9.qml",
            "../../../../../Card.qml",
        ] {
            assert!(file_of(&root, base, path).is_err(), "{path}");
        }
        // Where the file system finds a file spelled otherwise (a case folded), the tree's spelling is the file.
        let shared = root.join("app/shared");
        std::fs::create_dir_all(&shared).expect("dir");
        std::fs::write(shared.join("Card.qml"), "Item {}\n").expect("file");
        let found = file_of(&root, base, "../../SHARED/Card.qml").expect("a file");
        if root.join("app/SHARED/Card.qml").exists() {
            assert_eq!(found, "app/shared/Card.qml");
        } else {
            assert_eq!(found, "app/SHARED/Card.qml");
        }
    }
}
