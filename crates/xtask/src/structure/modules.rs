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
//! with the feature (`crate::HARNESS_FEATURE`), so the broken build is
//! the one no command in this repository runs.
//!
//! What counts as naming a type is what QML resolves: an object
//! declaration (`WindowHarness {`) and a typed property
//! (`property TabProbe probe`). A name inside a comment is a reference to
//! read, and there are many — the harness is where a verb is implemented,
//! and the product's comments say so.

use std::path::Path;

/// The product's QML, and the harness module whose names it may not use.
const PRODUCT: &str = "crates/platitude-app/src/ui";
const HARNESS_QMLDIR: &str = "crates/platitude-app/src/auto/qmldir";

/// One failure per product file that names a harness type, and how many
/// names were looked for.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let names = harness_types(root)?;
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
        }
    }
    Ok((failures, names.len()))
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
}
