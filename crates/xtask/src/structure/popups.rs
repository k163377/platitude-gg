//! Who closes a popup in the product, counted by machine
//! (.claude/rules-refs/app-ui.md「メニューを閉じるのは自分」): a body that
//! closes two things is a set spelled by hand, and the next entrance added
//! is the one it misses.
//!
//! Four shapes are counted, all in the product's own QML:
//!
//! * a function or handler body that closes more than one thing —
//!   `close()` and `dismiss()` alike, however they are qualified;
//! * a popup component declaring its own `dismiss()`, which silently
//!   shadows Qt's (the one a row calls to take every menu level down);
//! * any component declaring its own `close()`: it exists to be closed
//!   from outside, the start of a hand-spelled set;
//! * a card (`AppCard` at the root) setting its own `closePolicy`: the
//!   base spells it, Escape only while the keyboard is in the card
//!   (.claude/rules-refs/app-ui.md「hover のカードは窓の `Shortcut` を塞がない」).
//!
//! The bars (`AskBar`, `FindBar`, `NoticeBar`) keep their `dismiss()`: they
//! are items with a verb of their own.
//!
//! Comments and string literals are taken out before anything is read.

use std::collections::BTreeMap;
use std::path::Path;

/// The product's QML.
const PRODUCT: &str = "crates/platitude-app/src/ui";
/// The root types under which `close()` and `dismiss()` are already Qt's.
const POPUPS: &[&str] = &["Popup", "Menu", "Dialog", "Drawer", "AppMenu", "AppDialog"];
/// What closes a popup, whichever it is called on.
const CLOSERS: &[&str] = &["close", "dismiss"];
/// Where a failing line sends its reader.
const RULE: &str = ".claude/rules-refs/app-ui.md「メニューを閉じるのは自分」";
/// The base whose `closePolicy` is its own to spell.
const CARD: &str = "AppCard";
/// Where a card's failing line sends its reader.
const CARD_RULE: &str =
    ".claude/rules-refs/app-ui.md「hover のカードは窓の `Shortcut` を塞がない」";

/// One failure per line that closes what is not its to close, and how many
/// product files were read.
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
        for finding in findings(&text) {
            failures.push(format!(
                "{PRODUCT}/{shown}:{}: {}",
                finding.line, finding.what
            ));
        }
    }
    Ok((failures, files.len()))
}

struct Finding {
    line: usize,
    what: String,
}

/// Everything one file does about closing that the rule forbids, in line
/// order.
fn findings(text: &str) -> Vec<Finding> {
    let code = super::without_comments_and_strings(text);
    let root = root_type(&code);
    let popup = root.as_deref().is_some_and(|name| POPUPS.contains(&name));
    let card = root.as_deref() == Some(CARD);
    let depths = depths_at_line_starts(&code);
    let mut found = Vec::new();
    for (number, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();
        if card && depths.get(number) == Some(&1) && trimmed.starts_with("closePolicy:") {
            found.push(Finding {
                line: number + 1,
                what: format!(
                    "sets `closePolicy` on an `{CARD}` — the base spells it: Escape only while the \
                     keyboard is in the card, since a popup standing with `CloseOnEscape` keeps every \
                     window `Shortcut` out (Ctrl+F, F5, the bars' Escape) yet hears Escape only from \
                     the keyboard it holds. A card the pointer leaving closes says \
                     `closesOnPressOutside: false` ({CARD_RULE})"
                ),
            });
        }
        if declares(trimmed, "close") {
            found.push(Finding {
                line: number + 1,
                what: format!(
                    "declares a `close()` of its own — a popup already has Qt's, and a component \
                     that hands one out exists to be named and closed from outside, which is where \
                     a hand-spelled set of entrances begins. A popup takes itself down ({RULE})"
                ),
            });
        }
        if popup && declares(trimmed, "dismiss") {
            found.push(Finding {
                line: number + 1,
                what: format!(
                    "declares `dismiss()` on a {} — Qt's own `dismiss()` is what a row calls to take \
                     every level of a nested menu down, and a QML function of the same name shadows \
                     it silently ({RULE})",
                    root.as_deref().unwrap_or("popup")
                ),
            });
        }
    }
    found.extend(bodies_closing_twice(&code));
    found.sort_by_key(|finding| finding.line);
    found
}

/// Whether a line of code declares a function of that name.
fn declares(trimmed: &str, name: &str) -> bool {
    trimmed
        .strip_prefix("function ")
        .map(str::trim_start)
        .and_then(|rest| rest.strip_prefix(name))
        .is_some_and(|rest| rest.trim_start().starts_with('('))
}

/// How many braces are open where each line starts — 1 is the root object's
/// own body.
fn depths_at_line_starts(code: &str) -> Vec<usize> {
    let mut depth: usize = 0;
    code.lines()
        .map(|line| {
            let at = depth;
            for c in line.chars() {
                match c {
                    '{' => depth += 1,
                    '}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            at
        })
        .collect()
}

/// The type the file's root object is declared as: the identifier before
/// the first brace at depth zero, past any `pragma` and `import` lines.
fn root_type(code: &str) -> Option<String> {
    let chars: Vec<char> = code.chars().collect();
    let at = chars.iter().position(|&c| c == '{')?;
    let name = identifier_before(&chars, at)?;
    Some(name.rsplit('.').next().unwrap_or(&name).to_string())
}

/// What a brace opens: an object declaration (`AppMenu {`), a function or
/// handler body, or a block nested inside one of those bodies.
enum Block {
    Object,
    /// The calls that close something, as they read, with their lines.
    Body(Vec<(usize, String)>),
    Nested,
}

/// One finding per body that closes more than one thing — a brace-less
/// handler on one line counts as a body of its own.
fn bodies_closing_twice(code: &str) -> Vec<Finding> {
    let chars: Vec<char> = code.chars().collect();
    let mut stack: Vec<Block> = Vec::new();
    let mut loose: BTreeMap<usize, Vec<(usize, String)>> = BTreeMap::new();
    let mut found = Vec::new();
    let mut line = 1;
    let mut balanced = true;
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\n' => line += 1,
            '{' => stack.push(if opens_object(&chars, i) {
                Block::Object
            } else if matches!(stack.last(), None | Some(Block::Object)) {
                Block::Body(Vec::new())
            } else {
                Block::Nested
            }),
            '}' => match stack.pop() {
                Some(Block::Body(calls)) if calls.len() > 1 => found.push(fanned_out(&calls)),
                Some(_) => {}
                None => balanced = false,
            },
            _ => {
                if let Some(call) = call_at(&chars, i) {
                    match stack.iter_mut().rev().find_map(|block| match block {
                        Block::Body(calls) => Some(calls),
                        _ => None,
                    }) {
                        Some(calls) => calls.push((line, call)),
                        None => loose.entry(line).or_default().push((line, call)),
                    }
                }
            }
        }
    }
    if !stack.is_empty() {
        balanced = false;
    }
    if !balanced {
        found.push(Finding {
            line: 1,
            what: "braces do not balance once comments and strings are taken out, so the \
                   count could not read this file — a brace in a regular expression is the \
                   usual reason; spell it another way"
                .to_string(),
        });
    }
    found.extend(
        loose
            .into_values()
            .filter(|calls| calls.len() > 1)
            .map(|calls| fanned_out(&calls)),
    );
    found
}

fn fanned_out(calls: &[(usize, String)]) -> Finding {
    let named: Vec<&str> = calls.iter().map(|(_, call)| call.as_str()).collect();
    Finding {
        line: calls[1].0,
        what: format!(
            "closes {} things in one body ({}) — a popup closes itself: a row says `dismiss()` \
             once on its own menu and Qt takes every level down, and a menu standing for an \
             answer reads the answer and goes. A set spelled out by hand here is the one the \
             next entrance is left out of ({RULE})",
            named.len(),
            named.join(", ")
        ),
    }
}

/// Whether the brace at `at` opens an object declaration: the token before
/// it is an identifier whose last segment starts with a capital.
pub(super) fn opens_object(chars: &[char], at: usize) -> bool {
    identifier_before(chars, at).is_some_and(|name| {
        name.rsplit('.')
            .next()
            .and_then(|last| last.chars().next())
            .is_some_and(char::is_uppercase)
    })
}

/// The identifier (dots included) that ends just before `at`, whitespace
/// between them allowed.
fn identifier_before(chars: &[char], at: usize) -> Option<String> {
    let mut end = at;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_name(chars[start - 1]) {
        start -= 1;
    }
    (start < end).then(|| chars[start..end].iter().collect())
}

fn is_name(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

/// The closing call that starts at `at`, spelled with its receiver
/// (`commitMenu.close()`), or nothing: not part of a longer name, and not
/// the declaration `function close()`.
fn call_at(chars: &[char], at: usize) -> Option<String> {
    if at > 0 && is_name(chars[at - 1]) && chars[at - 1] != '.' {
        return None;
    }
    let name = CLOSERS.iter().find(|name| {
        let spelled: Vec<char> = format!("{name}()").chars().collect();
        chars.get(at..at + spelled.len()) == Some(&spelled[..])
    })?;
    if identifier_before(chars, at).as_deref() == Some("function") {
        return None;
    }
    let receiver = if at > 0 && chars[at - 1] == '.' {
        identifier_before(chars, at - 1).unwrap_or_default()
    } else {
        String::new()
    };
    Some(if receiver.is_empty() {
        format!("{name}()")
    } else {
        format!("{receiver}.{name}()")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<usize> {
        findings(text).iter().map(|finding| finding.line).collect()
    }

    #[test]
    fn two_closes_in_one_body_are_one_finding_at_the_second() {
        let text = "\
Item {
    function shutBoth() {
        commitMenu.close()
        stashMenu.close()
    }
}
";
        let found = findings(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 4);
        assert!(
            found[0]
                .what
                .contains("commitMenu.close(), stashMenu.close()")
        );
    }

    #[test]
    fn one_close_per_handler_is_each_handlers_own() {
        let text = "\
Item {
    AppMenuItem {
        onHeld: {
            stashMenu.dismiss()
            rowMenu.dropped()
        }
    }
    AppMenuItem {
        onHeld: {
            commitMenu.dismiss()
        }
    }
    onVisibleChanged: if (!visible) card.close()
}
";
        assert!(lines(text).is_empty());
    }

    #[test]
    fn a_dismiss_and_a_close_in_one_body_are_still_two() {
        let text = "\
AppMenu {
    AppMenuItem {
        onHeld: {
            card.close()
            card.closeRequested()
            host.dismiss()
        }
    }
}
";
        assert_eq!(lines(text), [6]);
    }

    #[test]
    fn if_and_else_in_one_body_still_close_twice() {
        let text = "\
Item {
    function answer(which) {
        if (which === 1) {
            refMenu.close()
        } else {
            commitMenu.close()
        }
    }
}
";
        assert_eq!(lines(text), [6]);
    }

    #[test]
    fn a_braceless_handler_that_closes_twice_is_counted() {
        let text = "\
Item {
    onPicked: sidebar ? refMenu.close() : commitMenu.close()
}
";
        assert_eq!(lines(text), [2]);
    }

    #[test]
    fn a_close_in_a_comment_or_a_string_is_a_name_to_read() {
        let text = "\
Item {
    // `TopBar.closeRequested` → `root.close()` is what keeps the gate; card.close() goes with it
    /* menu.close()
       host.close() */
    property string words: \"close() and dismiss()\"
    function one() {
        target.close() // the other close() is only said here
    }
}
";
        assert!(lines(text).is_empty());
    }

    #[test]
    fn a_popup_may_not_shadow_qts_dismiss_but_a_bar_keeps_its_own() {
        let card = "\
import QtQuick

AppMenu {
    id: card
    function dismiss() {
        card.close()
    }
}
";
        let found = findings(card);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 5);
        assert!(found[0].what.contains("AppMenu"));
        let bar = "\
Item {
    function dismiss() {
        bar.open = false
    }
}
";
        assert!(lines(bar).is_empty());
    }

    #[test]
    fn no_component_hands_out_a_close_of_its_own() {
        let text = "\
pragma ComponentBehavior: Bound

import QtQuick

Item {
    /// A write that landed has nothing left for the menu to catch.
    function close() {
        refMenu.close()
    }
}
";
        assert_eq!(lines(text), [7]);
    }

    #[test]
    fn a_body_inside_an_object_inside_a_body_is_its_own() {
        let text = "\
Item {
    function reach() {
        first.close()
    }
    property Timer keep: Timer {
        onTriggered: {
            if (!lit)
                card.close()
        }
    }
}
";
        assert!(lines(text).is_empty());
    }

    #[test]
    fn a_card_leaves_its_close_policy_to_the_base() {
        let card = "\
import QtQuick
import QtQuick.Controls.Fusion

AppCard {
    id: card
    // closePolicy: said in a comment is only read
    closePolicy: Popup.CloseOnEscape
    tracksPointer: true
}
";
        let found = findings(card);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 7);
        assert!(found[0].what.contains("closesOnPressOutside: false"));
        let spelled = "\
AppCard {
    closesOnPressOutside: false
}
";
        assert!(lines(spelled).is_empty());
    }

    #[test]
    fn a_popup_inside_a_card_and_a_menu_keep_their_own_policy() {
        let nested = "\
AppCard {
    contentItem: Item {
        Popup {
            closePolicy: Popup.CloseOnEscape
        }
    }
}
";
        assert!(lines(nested).is_empty());
        let menu = "\
AppMenu {
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
}
";
        assert!(lines(menu).is_empty());
    }

    #[test]
    fn the_root_type_is_read_past_pragma_and_imports() {
        assert_eq!(
            root_type("pragma ComponentBehavior: Bound\nimport QtQuick\n\nAppDialog {\n}\n")
                .as_deref(),
            Some("AppDialog")
        );
        assert_eq!(root_type("Controls.Menu {\n}\n").as_deref(), Some("Menu"));
        assert_eq!(root_type("import QtQuick\n"), None);
    }

    #[test]
    fn unbalanced_braces_are_said_rather_than_swallowed() {
        let found = findings("Item {\n    function f() {\n");
        assert_eq!(found.len(), 1);
        assert!(found[0].what.contains("do not balance"));
    }

    /// The rule the count is for: the tree it runs on passes it.
    #[test]
    fn the_product_closes_one_thing_per_body() {
        let (failures, files) = check(&crate::tree::workspace_root()).expect("scan the product");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(files > 1, "the product has more than one .qml file");
    }
}
