//! The Write and Edit guards: where a write may land, and the notes an
//! edit gets after (seat re-claim, QML registration and fonts, doc shape,
//! prose width).
//!
//! Not guarded, on purpose: an Edit that quotes more `old_string` than it
//! needs. One deny costs an order of magnitude more than a tighter quote
//! saves, and a non-blocking note or an always-loaded rule line loses too,
//! since what it adds rides on every call. Do not propose it again without
//! a new measurement. Nor an edit that rewrites what the same session
//! wrote: nothing known before the write tells such a rethink apart.

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
         \"{}\"}}}}",
        super::payload::printable(&reason)
    );
    Ok(())
}

/// A new .rs directly under crates/platitude-core/tests/, which would be a
/// second test binary.
///
/// The path stays spelled whole although nothing here reads that tree: the
/// gate's graph takes the string for an edge (`graph::literal_paths`), and a
/// reach is narrowed by moving code into leaves, never by hiding a path.
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
/// note rides in a single additionalContext — made printable, since a note
/// quotes the edited file (a label's expression keeps its quotes) and a raw
/// quote leaves no JSON for the session to be told.
pub(super) fn post_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    let mut notes: Vec<String> = Vec::new();
    notes.extend(seat::reclaim(input, &path));
    notes.extend(qml_notes(&path)?);
    notes.extend(doc_notes(&path));
    notes.extend(crate::wrap::note(&path));
    if !notes.is_empty() {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
             \"additionalContext\":\"{}\"}}}}",
            super::payload::printable(&notes.join(" "))
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
/// fails to resolve only at runtime, so does a singleton read without its
/// module's import, and a wrong font form renders fine on the machine that
/// wrote it.
fn qml_notes(path: &str) -> Result<Vec<String>, String> {
    // Every QML module — the product's (`platitude.ui`, and each in a
    // directory under it) and the harness's (`platitude.auto`) — has its own
    // qmldir beside its files, and all of them are embedded from the one
    // main.rs.
    const APP: &str = "crates/platitude-app/";
    let module = ["src/ui/", "src/auto/"]
        .into_iter()
        .find(|dir| path.contains(&format!("{APP}{dir}")));
    let (Some(module), true) = (module, path.ends_with(".qml")) else {
        return Ok(Vec::new());
    };
    let Some(file_name) = path.rsplit('/').next().map(str::to_string) else {
        return Ok(Vec::new());
    };
    let Some(at) = path.rfind(APP) else {
        return Ok(Vec::new());
    };
    let (root, shown) = (&path[..at], &path[at..]);
    let dir = std::path::Path::new(path)
        .parent()
        .ok_or("qml path has no parent")?;
    let mut notes: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    // Only judge the registries that are actually there.
    if let Ok(qmldir) = std::fs::read_to_string(dir.join("qmldir"))
        && !qmldir.contains(&file_name)
    {
        let dir_shown = shown.rsplit_once('/').map_or(shown, |(dir, _)| dir);
        missing.push(format!("{}/qmldir", &dir_shown[APP.len()..]));
    }
    if let Ok(main_rs) = std::fs::read_to_string(format!("{root}{APP}src/main.rs"))
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
        // The same reading `cargo xtask structure` makes of the imports.
        notes.extend(crate::structure::singletons::notes(
            std::path::Path::new(root),
            shown,
            &content,
        ));
        notes.extend(qml_font_notes(&content));
        // Theme.qml is where the weight token picks its per-OS value.
        if file_name != "Theme.qml" {
            notes.extend(qml_weight_notes(&content));
        }
        // The product draws input with PlainTextLabel alone; the same reading
        // `cargo xtask structure` makes, said while the edit is still in hand.
        if module == "src/ui/" {
            notes.extend(
                crate::structure::labels::findings(&file_name, &content)
                    .into_iter()
                    .map(|finding| format!("line {}: {}", finding.line, finding.what)),
            );
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
    use super::{qml_font_notes, qml_notes, qml_weight_notes};

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

    /// A singleton's file stands a directory below the product's, its
    /// registry beside it and main.rs two up; a file there that its qmldir
    /// does not declare hears what the module holds, and a file reading a
    /// singleton unimported hears so, as each is written.
    #[test]
    fn a_module_under_ui_is_judged_by_its_own_qmldir_and_main_rs() {
        let root = crate::yard::Yard::new("hook-write-modules");
        let app = root.join("crates/platitude-app/src");
        let write = |path: &str, text: &str| {
            let file = app.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("the fixture's dirs");
            std::fs::write(file, text).expect("the fixture");
        };
        write("main.rs", "qrc::embed!(\"ui/words/Words.qml\");\n");
        write(
            "ui/qmldir",
            "module platitude.ui\nsingleton Theme 1.0 Theme.qml\n",
        );
        write(
            "ui/words/qmldir",
            "module platitude.ui.words\nsingleton Words 1.0 Words.qml\n",
        );
        write("ui/words/Words.qml", "pragma Singleton\nQtObject {}\n");
        write("ui/words/Stray.qml", "Item {}\n");
        write("ui/Reader.qml", "Item {\n    width: Words.lead\n}\n");
        let at = |path: &str| format!("{}/{path}", app.to_string_lossy().replace('\\', "/"));
        let kept = qml_notes(&at("ui/words/Words.qml")).expect("notes");
        assert!(kept.is_empty(), "{kept:?}");
        let stray = qml_notes(&at("ui/words/Stray.qml")).expect("notes");
        assert_eq!(stray.len(), 2, "{stray:?}");
        assert!(
            stray[0].contains("src/ui/words/qmldir and main.rs"),
            "{stray:?}"
        );
        assert!(
            stray[1].contains(
                "ui/words/Stray.qml is not declared by the module's qmldir — a singleton is \
                 declared there"
            ),
            "{stray:?}"
        );
        let reader = qml_notes(&at("ui/Reader.qml")).expect("notes");
        assert!(
            reader.iter().any(|note| note
                .contains("ui/Reader.qml:2: reads Words without importing platitude.ui.words")),
            "{reader:?}"
        );
    }
}
