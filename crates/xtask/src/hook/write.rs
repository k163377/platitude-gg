//! The Write and Edit guards: where a write may land, and the notes an
//! edit gets after (seat re-claim, QML registration and fonts, doc shape).

use super::payload::string_field;
use super::seat;

/// PreToolUse(Write|Edit). One decision per call, so the first objection
/// is the answer.
pub(super) fn pre_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    let Some((decision, reason)) = second_test_binary(&path)
        .map(|reason| ("deny", reason))
        .or_else(|| seat::write_objection(input, &path))
    else {
        return Ok(());
    };
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"{decision}\",\"permissionDecisionReason\":\
         \"{reason}\"}}}}"
    );
    Ok(())
}

/// A new .rs directly under crates/platitude-core/tests/, which would be a
/// second test binary.
fn second_test_binary(path: &str) -> Option<String> {
    let rest = path.split("crates/platitude-core/tests/").nth(1)?;
    (rest.ends_with(".rs") && !rest.contains('/')).then(|| {
        "Integration tests are one binary: cargo test runs test binaries one \
         after another, so a file directly under tests/ becomes a second, \
         serialized binary and a second link. Add the test as a module under \
         crates/platitude-core/tests/it/ and register it in tests/it/main.rs \
         (.claude/rules/core.md)."
            .to_string()
    })
}

/// PostToolUse(Write|Edit). One JSON object is the whole answer, so every
/// note rides in a single additionalContext.
pub(super) fn post_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    let mut notes: Vec<String> = Vec::new();
    notes.extend(seat::reclaim(input, &path));
    notes.extend(qml_notes(&path)?);
    notes.extend(doc_notes(&path));
    if !notes.is_empty() {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
             \"additionalContext\":\"{}\"}}}}",
            notes.join(" ")
        );
    }
    Ok(())
}

/// The blocks an edit to this tree's markdown just tore off their list,
/// and the cap an always-loaded document just grew past (`crate::docs`).
/// Said right after the edit: a torn block looks fine in the source, and
/// only the turn that made it knows what it meant.
fn doc_notes(path: &str) -> Vec<String> {
    if !crate::docs::covers(path) {
        return Vec::new();
    }
    let Some(name) = path.rsplit('/').next() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let over = crate::docs::oversize(path, text.len());
    crate::docs::findings(&text)
        .into_iter()
        .chain(over)
        .map(|finding| format!("{name} {finding}"))
        .collect()
}

/// QML rules a change keeps missing: a file absent from qmldir or main.rs
/// fails to resolve only at runtime, and a wrong font form renders fine on
/// the machine that wrote it.
fn qml_notes(path: &str) -> Result<Vec<String>, String> {
    // Both QML modules (`platitude.ui`, `platitude.auto`) have their own
    // qmldir and are embedded from the one main.rs.
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
    // Only judge the registries that are actually there.
    if let Ok(qmldir) = std::fs::read_to_string(ui_dir.join("qmldir"))
        && !qmldir.contains(&file_name)
    {
        missing.push(format!("{module}qmldir"));
    }
    if let Some(src_dir) = ui_dir.parent()
        && let Ok(main_rs) = std::fs::read_to_string(src_dir.join("main.rs"))
        && !main_rs.contains(&file_name)
    {
        missing.push("main.rs (qrc::embed!)".to_string());
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
        // Theme.qml is where the weight token picks its per-OS value.
        if file_name != "Theme.qml" {
            notes.extend(qml_weight_notes(&content));
        }
    }
    Ok(notes)
}

/// The named weights past `Font.Normal`. The one the UI uses is Theme's,
/// which picks it per OS.
const MACHINE_WEIGHTS: [&str; 8] = [
    "Font.Thin",
    "Font.ExtraLight",
    "Font.Light",
    "Font.Medium",
    "Font.DemiBold",
    "Font.Bold",
    "Font.ExtraBold",
    "Font.Black",
];

/// デザイン規約 §タイポグラフィ's two weights, `Font.Normal` and
/// `Theme.fontWeightStrong`: a weight named anywhere else skips the token's
/// per-OS pick.
fn qml_weight_notes(content: &str) -> Vec<String> {
    let mut notes = Vec::new();
    for (number, line) in content.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        let named = MACHINE_WEIGHTS.iter().find(|weight| {
            code.match_indices(*weight).any(|(at, _)| {
                !code[at + weight.len()..].starts_with(|c: char| c.is_ascii_alphanumeric())
            })
        });
        if let Some(weight) = named {
            notes.push(format!(
                "line {}: the weight past Font.Normal is Theme.fontWeightStrong \
                 — {weight} skips its per-OS pick (Ubuntu's DemiBold draws \
                 Medium with fonts-noto-cjk-extra and Bold without; \
                 デザイン規約 §タイポグラフィ).",
                number + 1
            ));
        }
    }
    notes
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
    use super::{qml_font_notes, qml_weight_notes};

    #[test]
    fn flags_named_weights_past_normal() {
        let notes = qml_weight_notes(
            "Text {\n    font.weight: Font.DemiBold\n}\n\
             Text { font.weight: on ? Font.Bold : Font.Normal }\n",
        );
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(notes[0].contains("line 2") && notes[0].contains("Font.DemiBold"));
        assert!(notes[1].contains("line 4") && notes[1].contains("Font.Bold"));
    }

    #[test]
    fn accepts_normal_the_token_and_comments() {
        let notes = qml_weight_notes(
            "// Font.DemiBold in a comment is fine\n\
             Text { font.weight: on ? Theme.fontWeightStrong : Font.Normal }\n\
             Text { font.bold: modelData.bold }\n",
        );
        assert!(notes.is_empty(), "{notes:?}");
    }

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
