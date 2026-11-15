//! Every mark the product draws by hand stands on `InkCanvas`
//! (.claude/rules-refs/app-ui.md「`Canvas` は生まれたフレームにインクを持た
//! ない」): a `Canvas` declared anywhere but at the root of `InkCanvas.qml`
//! is a mark `Ink.owed` never counts, so a picture taken in the turn it is
//! born shows its seat empty and the shot waits for nothing.
//!
//! A declaration is an object whose type's last segment is `Canvas`
//! (`QtQuick.Canvas` and an inline `component …: Canvas` alike).
//!
//! Comments and string literals are taken out before anything is read.

use std::path::Path;

use super::popups::{identifier_before, opens_object};

/// The product's QML.
const PRODUCT: &str = "crates/platitude-app/src/ui";
/// The one file whose root is Qt's own `Canvas`, from the product's root.
const BASE: &str = "InkCanvas.qml";
/// The type it stands for.
const CANVAS: &str = "Canvas";
/// Where a failing line sends its reader.
pub(super) const RULE: &str =
    ".claude/rules-refs/app-ui.md「`Canvas` は生まれたフレームにインクを持たない」";

/// One failure per bare `Canvas`, and how many product files were read.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let product = root.join(PRODUCT);
    let mut files = Vec::new();
    super::collect(&product, &mut files)?;
    files.sort();
    files.retain(|path| path.extension().is_some_and(|e| e == "qml"));
    let mut failures = Vec::new();
    for path in &files {
        let shown = super::relative(&product, path);
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for line in bare_canvases(&text, shown == BASE) {
            failures.push(format!(
                "{PRODUCT}/{shown}:{line}: a bare `{CANVAS}` — root the mark in `InkCanvas`, or \
                 `Ink.owed` never counts it and a picture taken before its first paint shows \
                 its seat empty ({RULE})"
            ));
        }
    }
    Ok((failures, files.len()))
}

/// The line of each `Canvas` declared in `text`; the base's own root is
/// left out where `base` says the text is the base.
fn bare_canvases(text: &str, base: bool) -> Vec<usize> {
    let code = super::without_comments_and_strings(text);
    let chars: Vec<char> = code.chars().collect();
    let mut depth = 0usize;
    let mut line = 1;
    let mut found = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\n' => line += 1,
            '{' => {
                if declares_canvas(&chars, i) && !(base && depth == 0) {
                    found.push(line);
                }
                depth += 1;
            }
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    found
}

/// Whether the brace at `at` opens a `Canvas` object.
fn declares_canvas(chars: &[char], at: usize) -> bool {
    opens_object(chars, at)
        && identifier_before(chars, at).is_some_and(|name| name.rsplit('.').next() == Some(CANVAS))
}

#[cfg(test)]
mod tests {
    use super::bare_canvases;

    #[test]
    fn a_canvas_is_found_wherever_it_is_declared() {
        let nested = "Item {\n    Canvas {\n        onPaint: {}\n    }\n}\n";
        assert_eq!(bare_canvases(nested, false), [2]);
        let rooted = "import QtQuick\nCanvas {\n}\n";
        assert_eq!(bare_canvases(rooted, false), [2]);
        let qualified = "Item {\n    QtQuick.Canvas { }\n}\n";
        assert_eq!(bare_canvases(qualified, false), [2]);
        let inline = "Item {\n    component Mark: Canvas {\n    }\n}\n";
        assert_eq!(bare_canvases(inline, false), [2]);
    }

    #[test]
    fn a_mark_on_ink_canvas_and_the_base_s_own_root_pass() {
        let marked = "Item {\n    InkCanvas {\n        onPaint: {}\n    }\n}\n";
        assert_eq!(bare_canvases(marked, false), Vec::<usize>::new());
        let named =
            "Item {\n    // a Canvas { in a comment\n    property string s: \"Canvas {\"\n}\n";
        assert_eq!(bare_canvases(named, false), Vec::<usize>::new());
        let base = "Canvas {\n    id: canvas\n}\n";
        assert_eq!(bare_canvases(base, true), Vec::<usize>::new());
        let nested_in_base = "Canvas {\n    Canvas { }\n}\n";
        assert_eq!(bare_canvases(nested_in_base, true), [2]);
    }
}
