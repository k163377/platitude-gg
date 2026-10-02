//! Every text box the product declares hands its right-click to the
//! product's menu (.claude/rules-refs/app-ui.md「`FieldMenu` = 文字の欄の右クリック」): a
//! `TextField` or `TextArea` declared in the product's QML writes
//! `ContextMenu.menu: null` in its own body. The style hangs its own menu
//! on both, and a box that leaves the binding unwritten keeps it — Qt opens
//! that menu by itself on the right-click, beside the product's or alone.
//!
//! Only the box's own body counts: the line in an object nested inside it
//! sets that object's menu, and an assignment in a function body comes too
//! late (the binding is deferred, and an assigned `null` against a menu
//! not built yet changes nothing). A component built on a box
//! (`SlimField { … }`) carries its root's line and is not read again.
//!
//! Comments and string literals are taken out before anything is read.

use std::path::Path;

use super::popups::opens_object;

/// The product's QML.
const PRODUCT: &str = "crates/platitude-app/src/ui";
/// The style's types that carry its menu on the box itself.
const BOXES: &[&str] = &["TextField", "TextArea"];
/// The attached property, and the one value that keeps the style's away.
const ATTACHED: &str = "ContextMenu.menu";
const VALUE: &str = "null";
/// Where a failing line sends its reader.
const RULE: &str = ".claude/rules-refs/app-ui.md「`FieldMenu` = 文字の欄の右クリック」";

/// One failure per box that keeps the style's menu, and how many boxes
/// were read.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let product = root.join(PRODUCT);
    let mut files = Vec::new();
    super::collect(&product, &mut files)?;
    files.sort();
    files.retain(|path| path.extension().is_some_and(|e| e == "qml"));
    let mut failures = Vec::new();
    let mut boxes = 0;
    for path in &files {
        let shown = super::relative(&product, path);
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let read = read_boxes(&text);
        boxes += read.len();
        for (line, kind, handed) in read {
            if handed {
                continue;
            }
            failures.push(format!(
                "{PRODUCT}/{shown}:{line}: a {kind} that keeps the style's right-click menu — \
                 write `{ATTACHED}: {VALUE}` in its own body and hand the request to a \
                 `FieldMenuSeat`, or Qt opens the style's menu by itself ({RULE})"
            ));
        }
    }
    Ok((failures, boxes))
}

/// Each box declared in `text`: the line its body opens on, its type, and
/// whether its own body writes the line.
fn read_boxes(text: &str) -> Vec<(usize, &'static str, bool)> {
    let code = super::without_comments_and_strings(text);
    let chars: Vec<char> = code.chars().collect();
    let attached: Vec<char> = ATTACHED.chars().collect();
    // Per open brace: the box it opens (its index in `found`), if it is one.
    let mut stack: Vec<Option<usize>> = Vec::new();
    let mut found = Vec::new();
    let mut line = 1;
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\n' => line += 1,
            '{' => stack.push(box_opened(&chars, i).map(|kind| {
                found.push((line, kind, false));
                found.len() - 1
            })),
            '}' => {
                stack.pop();
            }
            _ => {
                let starts = i == 0 || !is_name(chars[i - 1]);
                let named = starts && chars.get(i..i + attached.len()) == Some(&attached[..]);
                // The innermost brace is the box's own body, or the line is another object's.
                if let Some(Some(at)) = stack.last()
                    && named
                    && sets_null(&chars, i + attached.len())
                {
                    found[*at].2 = true;
                }
            }
        }
    }
    found
}

/// The box type an object declaration at the brace `at` is, if it is one.
fn box_opened(chars: &[char], at: usize) -> Option<&'static str> {
    if !opens_object(chars, at) {
        return None;
    }
    let mut end = at;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_name(chars[start - 1]) {
        start -= 1;
    }
    let name: String = chars[start..end].iter().collect();
    let last = name.rsplit('.').next().unwrap_or(&name);
    BOXES.iter().copied().find(|kind| *kind == last)
}

/// Whether the property name ending at `from` is bound to `null`: a colon,
/// the value, and nothing more of a name after it.
fn sets_null(chars: &[char], from: usize) -> bool {
    let mut at = from;
    while chars.get(at).is_some_and(|c| c.is_whitespace()) {
        at += 1;
    }
    if chars.get(at) != Some(&':') {
        return false;
    }
    at += 1;
    while chars.get(at).is_some_and(|c| c.is_whitespace()) {
        at += 1;
    }
    let value: Vec<char> = VALUE.chars().collect();
    chars.get(at..at + value.len()) == Some(&value[..])
        && !chars.get(at + value.len()).copied().is_some_and(is_name)
}

fn is_name(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(text: &str) -> Vec<usize> {
        read_boxes(text)
            .into_iter()
            .filter(|(_, _, handed)| !handed)
            .map(|(line, _, _)| line)
            .collect()
    }

    #[test]
    fn a_box_that_writes_the_line_in_its_own_body_passes() {
        let text = "\
import QtQuick
TextField {
    id: form
    ContextMenu.menu: null
    ContextMenu.onRequested: position => seat.offer(position)
}
";
        assert_eq!(read_boxes(text), vec![(2, "TextField", true)]);
    }

    #[test]
    fn a_box_without_it_is_named_at_the_line_its_body_opens() {
        let text = "\
Item {
    TextArea {
        id: area
    }
}
";
        assert_eq!(kept(text), vec![2]);
    }

    #[test]
    fn the_line_in_a_nested_object_or_a_function_is_not_the_boxs() {
        let text = "\
TextField {
    Item {
        ContextMenu.menu: null
    }
    function drop() {
        ContextMenu.menu = null
    }
    Component.onCompleted: { ContextMenu.menu: null }
}
";
        assert_eq!(kept(text), vec![1]);
    }

    #[test]
    fn a_value_other_than_null_keeps_a_menu_handed_to_qt() {
        let text = "\
TextField {
    ContextMenu.menu: AppMenu {}
}
TextArea {
    ContextMenu.menu: nullish
}
";
        assert_eq!(kept(text), vec![1, 4]);
    }

    #[test]
    fn a_component_built_on_a_box_and_other_mentions_of_the_names_are_not_boxes() {
        let text = "\
SlimField {
    wrapMode: TextArea.Wrap
    readonly property TextField field: null
    T.TextFieldish {
    }
}
";
        assert!(read_boxes(text).is_empty());
    }

    #[test]
    fn every_text_box_in_the_product_keeps_the_styles_menu_away() {
        let (failures, boxes) = check(&crate::tree::workspace_root()).expect("scan the product");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(boxes > 1, "the product declares more than one text box");
    }

    #[test]
    fn a_qualified_box_counts_and_comments_are_not_read() {
        let text = "\
T.TextArea {
    // ContextMenu.menu: null
}
";
        assert_eq!(kept(text), vec![1]);
    }
}
