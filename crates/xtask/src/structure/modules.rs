//! The one boundary between the two QML modules, counted by machine.
//!
//! `platitude.ui` is the product and `platitude.auto` is the verification
//! harness, and a shipped build carries only the first (`platitude-app`
//! §features). So the product may not *name* a harness type: a static
//! type reference resolves at load time, and the build that has no
//! harness would fail to load `Main.qml` — no window at all, from a line
//! that reads perfectly well in the build everybody develops in.
//!
//! Nothing else here notices. Every xtask that starts the app builds it
//! with the feature (`crate::tree::HARNESS_FEATURE`), so the broken build is
//! the one no command in this repository runs.
//!
//! What counts as naming a type is what QML resolves: an object
//! declaration (`WindowHarness {`) and a typed property
//! (`property TabProbe probe`). A name inside a comment is a reference to
//! read, and there are many — the harness is where a verb is implemented,
//! and the product's comments say so.
//!
//! **The same boundary has a second half on the Rust side.** What a run
//! was told to do reaches QML through a singleton the feature registers
//! (`harness::singleton::Harness`), which is not a `platitude.auto` type
//! and so is not in the qmldir above. The product may not name it — a
//! singleton the engine cannot resolve fails the document that reads it,
//! the same way a missing type does — and may not ask `AppBackend` for
//! anything that lives on it either, which is what the move was for. Both
//! names are read out of the singleton's own source, so a property added
//! there is out of the product's reach on the run that adds it.

use std::path::Path;

/// The product's QML, and the harness module whose names it may not use.
const PRODUCT: &str = "crates/platitude-app/src/ui";
const HARNESS_QMLDIR: &str = "crates/platitude-app/src/auto/qmldir";
/// The QML-facing harness singleton, whose members say what the product
/// may no longer ask `AppBackend` for.
const HARNESS_SINGLETON: &str = "crates/platitude-app/src/harness/singleton.rs";
/// What the product calls it, and what it calls the object it may ask.
const SINGLETON: &str = "Harness";
const BACKEND: &str = "AppBackend";

/// One failure per product file that names a harness type, and how many
/// names were looked for.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let names = harness_types(root)?;
    let members = singleton_members(root)?;
    let mut failures = Vec::new();
    let product = root.join(PRODUCT);
    let entries = std::fs::read_dir(&product)
        .map_err(|e| format!("could not read {}: {e}", product.display()))?;
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| format!("could not read {}: {e}", product.display()))?
            .path();
        if path.extension().is_some_and(|e| e == "qml") {
            files.push(path);
        }
    }
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let shown = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (number, line) in text.lines().enumerate() {
            if let Some(named) = names.iter().find(|name| names_type(line, name)) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{} names the harness type {named} — a shipped build has no \
                     `platitude.auto` and would fail to load. Reach it through `HarnessSeat` \
                     instead (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
            if reads(line, SINGLETON, "") {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{} reads the harness singleton — a shipped build registers \
                     no `{SINGLETON}`, and a document that names one the engine cannot resolve \
                     does not load. The product asks its own properties and something else writes \
                     them (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
            if let Some(member) = members.iter().find(|member| reads(line, BACKEND, member)) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{} asks `{BACKEND}` for {member}, which is the harness's \
                     ({HARNESS_SINGLETON}). Nothing the product shows may turn on what a run was \
                     told to do (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
        }
    }
    Ok((failures, names.len() + members.len()))
}

/// Everything `Harness` puts in front of QML: the `qproperty!` names as
/// written, and the slots' own names in the camel case
/// `ConvertToCamelCase` gives them.
///
/// Read out of the source rather than listed here, so a knob added to the
/// harness is out of the product's reach without anybody remembering to
/// add it twice.
///
/// A function only counts behind `#[qslot]` — the file's own `Default` is
/// a `fn` too, and QML has never heard of it.
fn singleton_members(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(HARNESS_SINGLETON);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut members = Vec::new();
    let mut is_slot = false;
    for line in text.lines() {
        let code = line.trim_start();
        if let Some(rest) = code.strip_prefix("qproperty!(\"")
            && let Some(name) = rest.split('"').next()
        {
            members.push(name.to_string());
        } else if is_slot
            && let Some(rest) = code.strip_prefix("fn ")
            && let Some(name) = rest.split('(').next()
        {
            members.push(camel(name));
        }
        // The attribute and the signature it belongs to are a line apart,
        // and nothing else stands between them.
        is_slot = code == "#[qslot]";
    }
    if members.is_empty() {
        return Err(format!("{HARNESS_SINGLETON} puts nothing in front of QML"));
    }
    Ok(members)
}

/// `open_session_count` → `openSessionCount`, the name Qt registers the
/// slot under (`#[qobject(ConvertToCamelCase)]`).
fn camel(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut rising = false;
    for ch in snake.chars() {
        if ch == '_' {
            rising = true;
        } else if rising {
            out.extend(ch.to_uppercase());
            rising = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Whether one line of QML reads `<object>.<member>` — or, with an empty
/// member, reads anything at all off `object`.
///
/// Not on a comment line: the product's comments say where a verb is
/// implemented and what writes a property, and both name these constantly.
/// **Both ends of the name have to end**: `WindowHarness.qml` — the file a
/// seat loads by URL — is not `Harness.`, and
/// `AppBackend.autoFetchMinutes` — the application's own setting — is not
/// `AppBackend.autoAct`.
fn reads(line: &str, object: &str, member: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") {
        return false;
    }
    let looked_for = if member.is_empty() {
        format!("{object}.")
    } else {
        format!("{object}.{member}")
    };
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut at = 0;
    while let Some(found) = code[at..].find(&looked_for) {
        let start = at + found;
        let end = start + looked_for.len();
        let before_ends = code[..start].chars().next_back().is_none_or(|c| !word(c));
        let after_ends = member.is_empty() || !code[end..].starts_with(word);
        if before_ends && after_ends {
            return true;
        }
        at = end;
    }
    false
}

/// The type names `platitude.auto` exports, off its own qmldir.
fn harness_types(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(HARNESS_QMLDIR);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let names: Vec<String> = text
        .lines()
        .filter(|line| !line.starts_with("module ") && !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next().map(str::to_string))
        .collect();
    if names.is_empty() {
        return Err(format!("{HARNESS_QMLDIR} lists no types"));
    }
    Ok(names)
}

/// Whether one line of QML names `type` the way the engine resolves —
/// an object declaration or a typed property, not a mention in prose.
fn names_type(line: &str, type_name: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") || code.starts_with("///") {
        return false;
    }
    let Some(rest) = code.strip_prefix(type_name) else {
        return declares_property(code, type_name);
    };
    // `Foo {` and `Foo{` are declarations; `Foobar {` is a different type.
    matches!(rest.trim_start().chars().next(), Some('{'))
        && rest.chars().next().is_none_or(|c| c == ' ' || c == '{')
}

/// `property Foo bar` / `required property Foo bar` / `readonly property
/// Foo bar` — the other way a QML file has to have the type resolved.
fn declares_property(code: &str, type_name: &str) -> bool {
    let mut words = code
        .split_whitespace()
        .skip_while(|word| matches!(*word, "required" | "readonly" | "default" | "final"));
    words.next() == Some("property") && words.next() == Some(type_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declaration_and_a_typed_property_are_naming_the_type() {
        assert!(names_type("    WindowHarness {", "WindowHarness"));
        assert!(names_type("WindowHarness{", "WindowHarness"));
        assert!(names_type("    property TabProbe probe", "TabProbe"));
        assert!(names_type(
            "    required property TabProbe probe",
            "TabProbe"
        ));
        assert!(names_type(
            "    readonly property TabProbe probe: null",
            "TabProbe"
        ));
    }

    #[test]
    fn prose_and_neighbouring_names_are_not() {
        // The product's comments point at the harness constantly.
        assert!(!names_type(
            "    // reached by `WindowHarness`",
            "WindowHarness"
        ));
        assert!(!names_type(
            "    /// Automation: WindowHarness reads this",
            "WindowHarness"
        ));
        // A longer name that merely starts with one.
        assert!(!names_type("    TabProbeSeat {", "TabProbe"));
        // The property's own name, not its type.
        assert!(!names_type("    property var tabProbe", "TabProbe"));
        // A call through a seat says nothing about the type.
        assert!(!names_type("    harness.ask().begin()", "WindowHarness"));
    }

    #[test]
    fn a_read_off_the_singleton_is_one_and_a_longer_name_is_not() {
        assert!(reads(
            "        active: Harness.autoAct !== \"\"",
            "Harness",
            ""
        ));
        // The seat is the product's own type and shares the first word, and
        // the file it loads by URL ends in the same eight characters.
        assert!(!reads("    HarnessSeat {", "Harness", ""));
        assert!(!reads("        part: \"WindowHarness.qml\"", "Harness", ""));
        assert!(reads(
            "        if (AppBackend.autoAct === \"delete-gone\")",
            "AppBackend",
            "autoAct"
        ));
        // The application's own auto-fetch setting, which merely starts the
        // same way.
        assert!(!reads(
            "        fetchField.text = AppBackend.autoFetchMinutes > 0",
            "AppBackend",
            "autoAct"
        ));
        assert!(!reads(
            "    /// written by the harness (`Harness.autoAct`)",
            "Harness",
            ""
        ));
    }

    #[test]
    fn a_slot_is_looked_for_under_the_name_qt_registers_it_by() {
        assert_eq!(camel("open_session_count"), "openSessionCount");
        assert_eq!(camel("report"), "report");
    }

    /// What the singleton actually puts in front of QML, and what it only
    /// looks like it does.
    #[test]
    fn only_the_slots_and_the_properties_are_read_off_the_singleton() {
        let members = singleton_members(&crate::tree::workspace_root()).expect("read the harness");
        assert!(members.iter().any(|name| name == "autoAct"));
        assert!(members.iter().any(|name| name == "openSessionCount"));
        assert!(
            !members.iter().any(|name| name == "default"),
            "the file's own `Default` is a `fn` QML has never heard of"
        );
    }

    /// The rule the count is for: the tree it runs on passes it.
    #[test]
    fn the_product_reaches_for_nothing_a_shipped_build_lacks() {
        let (failures, names) = check(&crate::tree::workspace_root()).expect("scan the product");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(
            names > 1,
            "the harness puts more than one name out of reach"
        );
    }
}
