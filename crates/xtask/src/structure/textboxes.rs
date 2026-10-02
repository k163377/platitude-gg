//! Every text box the product declares hands its right-click and its menu
//! key to the product's menu (.claude/rules-refs/app-ui.md「`FieldMenu` =
//! 文字の欄の右クリック」): a `TextField` or `TextArea` declared in the
//! product's QML writes `ContextMenu.menu: null` and `Keys.onMenuPressed` in
//! its own body. The style hangs its own menu on both, and a box that leaves
//! the binding unwritten keeps it — Qt opens that menu by itself on the
//! right-click, beside the product's or alone. And a box that leaves the key
//! unanswered opens nothing for it where the platform raises no request of
//! its own (xcb raises one at the pointer, Windows none for the menu key
//! that Qt can be sure of), or passes the key up to the list it sits in,
//! which opens the list's menu instead (デザイン規約 §メニュー のキーボード).
//!
//! Only the box's own body counts: the line in an object nested inside it
//! belongs to that object, and an assignment in a function body comes too
//! late (the binding is deferred, and an assigned `null` against a menu
//! not built yet changes nothing). A component built on a box
//! (`SlimField { … }`) carries its root's lines and is not read again.
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
/// The handler that answers the menu key on the box itself.
const KEY: &str = "Keys.onMenuPressed";
/// Where a failing line sends its reader.
const RULE: &str = ".claude/rules-refs/app-ui.md「`FieldMenu` = 文字の欄の右クリック」";

/// One box as its own body declares it.
#[derive(Debug, PartialEq, Eq)]
struct Boxed {
    /// The line its body opens on.
    line: usize,
    kind: &'static str,
    /// `ContextMenu.menu: null` is written.
    handed: bool,
    /// `Keys.onMenuPressed` is bound.
    keyed: bool,
}

/// One failure per line a box leaves out, and how many boxes were read.
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
        for found in read {
            let Boxed {
                line,
                kind,
                handed,
                keyed,
            } = found;
            if !handed {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{line}: a {kind} that keeps the style's right-click menu — \
                     write `{ATTACHED}: {VALUE}` in its own body and hand the request to a \
                     `FieldMenuSeat`, or Qt opens the style's menu by itself ({RULE})"
                ));
            }
            if !keyed {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{line}: a {kind} that leaves the menu key unanswered — \
                     bind `{KEY}` in its own body to its `FieldMenuSeat`, or the key opens \
                     nothing here, or the menu of the list it sits in ({RULE})"
                ));
            }
        }
    }
    Ok((failures, boxes))
}

/// Each box declared in `text`, and which of the two lines its own body
/// writes.
fn read_boxes(text: &str) -> Vec<Boxed> {
    let code = super::without_comments_and_strings(text);
    let chars: Vec<char> = code.chars().collect();
    let attached: Vec<char> = ATTACHED.chars().collect();
    let key: Vec<char> = KEY.chars().collect();
    // Per open brace: the box it opens (its index in `found`), if it is one.
    let mut stack: Vec<Option<usize>> = Vec::new();
    let mut found: Vec<Boxed> = Vec::new();
    let mut line = 1;
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\n' => line += 1,
            '{' => stack.push(box_opened(&chars, i).map(|kind| {
                found.push(Boxed {
                    line,
                    kind,
                    handed: false,
                    keyed: false,
                });
                found.len() - 1
            })),
            '}' => {
                stack.pop();
            }
            _ => {
                // The innermost brace is the box's own body, or the line is another object's.
                let Some(Some(at)) = stack.last() else {
                    continue;
                };
                let starts = i == 0 || !is_name(chars[i - 1]);
                if starts
                    && chars.get(i..i + attached.len()) == Some(&attached[..])
                    && sets_null(&chars, i + attached.len())
                {
                    found[*at].handed = true;
                }
                if starts
                    && chars.get(i..i + key.len()) == Some(&key[..])
                    && binds(&chars, i + key.len())
                {
                    found[*at].keyed = true;
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

/// Whether the property name ending at `from` is bound at all: the name
/// ends there and a colon follows.
fn binds(chars: &[char], from: usize) -> bool {
    if chars.get(from).copied().is_some_and(is_name) {
        return false;
    }
    let mut at = from;
    while chars.get(at).is_some_and(|c| c.is_whitespace()) {
        at += 1;
    }
    chars.get(at) == Some(&':')
}

/// Whether the property name ending at `from` is bound to `null`: a colon,
/// the value, and nothing more of a name after it.
fn sets_null(chars: &[char], from: usize) -> bool {
    if !binds(chars, from) {
        return false;
    }
    let mut at = from;
    while chars.get(at) != Some(&':') {
        at += 1;
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

    /// The lines of the boxes that leave out the menu, and of those that
    /// leave out the key.
    fn kept(text: &str) -> (Vec<usize>, Vec<usize>) {
        let read = read_boxes(text);
        let menu = read.iter().filter(|b| !b.handed).map(|b| b.line).collect();
        let key = read.iter().filter(|b| !b.keyed).map(|b| b.line).collect();
        (menu, key)
    }

    #[test]
    fn a_box_that_writes_both_lines_in_its_own_body_passes() {
        let text = "\
import QtQuick
TextField {
    id: form
    ContextMenu.menu: null
    ContextMenu.onRequested: position => seat.offer(position)
    Keys.onMenuPressed: event => event.accepted = seat.offer()
}
";
        assert_eq!(
            read_boxes(text),
            vec![Boxed {
                line: 2,
                kind: "TextField",
                handed: true,
                keyed: true
            }]
        );
    }

    #[test]
    fn a_box_without_them_is_named_at_the_line_its_body_opens() {
        let text = "\
Item {
    TextArea {
        id: area
    }
}
";
        assert_eq!(kept(text), (vec![2], vec![2]));
    }

    #[test]
    fn the_lines_in_a_nested_object_or_a_function_are_not_the_boxs() {
        let text = "\
TextField {
    Item {
        ContextMenu.menu: null
        Keys.onMenuPressed: event => {}
    }
    function drop() {
        ContextMenu.menu = null
    }
    Component.onCompleted: { ContextMenu.menu: null }
}
";
        assert_eq!(kept(text), (vec![1], vec![1]));
    }

    #[test]
    fn a_value_other_than_null_keeps_a_menu_handed_to_qt() {
        let text = "\
TextField {
    ContextMenu.menu: AppMenu {}
    Keys.onMenuPressed: seat.offer()
}
TextArea {
    ContextMenu.menu: nullish
    Keys.onMenuPressed: seat.offer()
}
";
        assert_eq!(kept(text), (vec![1, 5], vec![]));
    }

    #[test]
    fn the_key_is_its_own_line_and_another_handler_is_not_it() {
        let text = "\
TextField {
    ContextMenu.menu: null
    Keys.onPressed: event => seat.offer()
    Keys.onMenuPressedLater: seat.offer()
}
";
        assert_eq!(kept(text), (vec![], vec![1]));
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
    fn every_text_box_in_the_product_hands_its_menu_and_its_key_to_the_products() {
        let (failures, boxes) = check(&crate::tree::workspace_root()).expect("scan the product");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(boxes > 1, "the product declares more than one text box");
    }

    #[test]
    fn a_qualified_box_counts_and_comments_are_not_read() {
        let text = "\
T.TextArea {
    // ContextMenu.menu: null
    // Keys.onMenuPressed: seat.offer()
}
";
        assert_eq!(kept(text), (vec![1], vec![1]));
    }
}
