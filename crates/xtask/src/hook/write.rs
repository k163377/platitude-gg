//! The Write and Edit guards: where a new test file may land, the QML
//! rules a change keeps missing by attention, and the seat claim an edit
//! into an unclaimed seat puts back.

use super::payload::string_field;
use super::seat;

/// PreToolUse(Write): a new .rs directly under crates/platitude-core/tests/
/// would become a second, serialized test binary — integration tests are one
/// binary by rule (tests/it/).
pub(super) fn pre_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    let Some(rest) = path.split("crates/platitude-core/tests/").nth(1) else {
        return Ok(());
    };
    if rest.ends_with(".rs") && !rest.contains('/') {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
             \"Integration tests are one binary: cargo test runs test binaries \
             one after another, so a file directly under tests/ becomes a second, \
             serialized binary and a second link. Add the test as a module under \
             crates/platitude-core/tests/it/ and register it in tests/it/main.rs \
             (.claude/rules/core.md).\"}}}}"
        );
    }
    Ok(())
}

/// PostToolUse(Write|Edit): the seat re-claim, then the QML rules a
/// change keeps missing by attention. One JSON object is the whole
/// answer, so every note this call has rides out in a single
/// additionalContext.
pub(super) fn post_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    let mut notes: Vec<String> = Vec::new();
    notes.extend(seat::reclaim(input, &path));
    notes.extend(qml_notes(&path)?);
    if !notes.is_empty() {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
             \"additionalContext\":\"{}\"}}}}",
            notes.join(" ")
        );
    }
    Ok(())
}

/// Rules a QML change keeps missing by attention. A file absent from
/// qmldir or main.rs silently fails to resolve at runtime (qmldir
/// directories only expose enumerated types), and the font rules below
/// dodge review because the wrong form still renders fine on the machine
/// it was written on.
fn qml_notes(path: &str) -> Result<Vec<String>, String> {
    // Both QML modules: `src/ui` is `platitude.ui`, `src/auto` is the
    // verification harness's `platitude.auto`. Each has its own qmldir,
    // and both are embedded from the one main.rs.
    let module = ["src/ui/", "src/auto/"]
        .into_iter()
        .find(|dir| path.contains(&format!("crates/platitude-app/{dir}")));
    let (Some(module), true) = (module, path.ends_with(".qml")) else {
        return Ok(Vec::new());
    };
    let Some(file_name) = path.rsplit('/').next().map(str::to_string) else {
        return Ok(Vec::new());
    };
    let ui_dir = std::path::Path::new(path)
        .parent()
        .ok_or("qml path has no parent")?;
    let mut notes: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    // Missing registries are someone else's layout problem, not this hook's:
    // only judge the files that are actually there.
    if let Ok(qmldir) = std::fs::read_to_string(ui_dir.join("qmldir"))
        && !qmldir.contains(&file_name)
    {
        missing.push(format!("{module}qmldir"));
    }
    if let Some(src_dir) = ui_dir.parent()
        && let Ok(main_rs) = std::fs::read_to_string(src_dir.join("main.rs"))
        && !main_rs.contains(&file_name)
    {
        missing.push("main.rs (include_bytes_qml!)".to_string());
    }
    if !missing.is_empty() {
        notes.push(format!(
            "{file_name} is not registered in: {}. A QML component in a \
             qmldir directory is invisible unless enumerated there, and \
             unbundled unless embedded in main.rs (.claude/rules/app-ui.md).",
            missing.join(" and ")
        ));
    }
    if let Ok(content) = std::fs::read_to_string(path) {
        notes.extend(qml_font_notes(&content));
    }
    Ok(notes)
}

/// The font rules of デザイン規約 §QML 実装ルール, checked line by line.
fn qml_font_notes(content: &str) -> Vec<String> {
    let mut notes = Vec::new();
    for (number, line) in content.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        if code.contains("font.pointSize") {
            notes.push(format!(
                "line {}: font.pointSize drifts with each OS's logical DPI; \
                 use font.pixelSize with a Theme token \
                 (デザイン規約 §QML 実装ルール).",
                number + 1
            ));
        }
        let Some(value) = code.split("font.family").nth(1) else {
            continue;
        };
        let Some(value) = value.trim_start().strip_prefix(':') else {
            continue;
        };
        if value.contains('"') || !value.contains("Theme.") {
            notes.push(format!(
                "line {}: font.family may only take a family Theme resolved \
                 (Theme.uiFamily / Theme.monoFamily) — anything else skips \
                 the per-OS fallback chain (デザイン規約 §QML 実装ルール).",
                number + 1
            ));
        }
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::qml_font_notes;

    #[test]
    fn flags_point_size_and_families_named_outside_theme() {
        let notes =
            qml_font_notes("Text {\n    font.pointSize: 12\n    font.family: \"Segoe UI\"\n}\n");
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(notes[0].contains("line 2"));
        assert!(notes[1].contains("line 3"));
    }

    #[test]
    fn accepts_theme_resolved_families_and_comments() {
        let notes = qml_font_notes(
            "// font.pointSize in a comment is fine\n\
             Text { font.family: Theme.monoFamily }\n\
             Text { font.family: code ? Theme.monoFamily : Theme.uiFamily }\n",
        );
        assert!(notes.is_empty(), "{notes:?}");
    }
}
