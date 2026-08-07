//! Claude Code hook handlers (`cargo xtask hook <event>`).
//!
//! Wired from .claude/settings.json. Each handler reads the hook's JSON
//! payload from stdin and answers on stdout; printing nothing means "no
//! objection". These exist to make the rules that keep being forgotten
//! mechanical instead of attentional (CLAUDE.md 規約の置き場所).

use std::io::Read;

pub fn run(args: &[String]) -> Result<(), String> {
    let event = args.first().map(String::as_str).unwrap_or("");
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("failed to read hook payload: {e}"))?;
    match event {
        "pre-write" => pre_write(&input),
        "post-write" => post_write(&input),
        "session-start" => session_start(&input),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// PreToolUse(Write): a new .rs directly under crates/platitude-core/tests/
/// would become a second, serialized test binary — integration tests are one
/// binary by rule (tests/it/). Deny with the rule spelled out.
fn pre_write(input: &str) -> Result<(), String> {
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
             (CLAUDE.md ビルド・テスト).\"}}}}"
        );
    }
    Ok(())
}

/// PostToolUse(Write|Edit): rules a QML change keeps missing by
/// attention. A file absent from qmldir or main.rs silently fails to
/// resolve at runtime (qmldir directories only expose enumerated
/// types), and the font rules below dodge review because the wrong
/// form still renders fine on the machine it was written on.
fn post_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    if !path.ends_with(".qml") || !path.contains("crates/platitude-app/src/ui/") {
        return Ok(());
    }
    let Some(file_name) = path.rsplit('/').next().map(str::to_string) else {
        return Ok(());
    };
    let ui_dir = std::path::Path::new(&path)
        .parent()
        .ok_or("qml path has no parent")?;
    let mut notes: Vec<String> = Vec::new();
    let mut missing: Vec<&str> = Vec::new();
    // Missing registries are someone else's layout problem, not this hook's:
    // only judge the files that are actually there.
    if let Ok(qmldir) = std::fs::read_to_string(ui_dir.join("qmldir"))
        && !qmldir.contains(&file_name)
    {
        missing.push("src/ui/qmldir");
    }
    if let Some(src_dir) = ui_dir.parent()
        && let Ok(main_rs) = std::fs::read_to_string(src_dir.join("main.rs"))
        && !main_rs.contains(&file_name)
    {
        missing.push("main.rs (include_bytes_qml!)");
    }
    if !missing.is_empty() {
        notes.push(format!(
            "{file_name} is not registered in: {}. A QML component in a \
             qmldir directory is invisible unless enumerated there, and \
             unbundled unless embedded in main.rs (.claude/rules/app-ui.md).",
            missing.join(" and ")
        ));
    }
    if let Ok(content) = std::fs::read_to_string(&path) {
        notes.extend(qml_font_notes(&content));
    }
    if !notes.is_empty() {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
             \"additionalContext\":\"{}\"}}}}",
            notes.join(" ")
        );
    }
    Ok(())
}

/// The font rules of デザイン規約 §QML実装ルール, checked line by line:
/// pointSize drifts with each OS's logical DPI, and a family named
/// outside Theme skips the per-OS fallback chain Theme resolves — both
/// look right on the machine they were written on and break on another.
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
                 (デザイン規約 §QML実装ルール).",
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
                 the per-OS fallback chain (デザイン規約 §QML実装ルール).",
                number + 1
            ));
        }
    }
    notes
}

/// SessionStart: sessions opened in the primary checkout get the worktree
/// rule injected while worktree sessions stay quiet. Plain stdout becomes
/// session context for this event.
fn session_start(input: &str) -> Result<(), String> {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    if !cwd.replace('\\', "/").contains("/.claude/worktrees/") {
        println!(
            "This session runs in the primary checkout. Implementation work \
             belongs in a reused fixed-name worktree (`claude --worktree <name>`) \
             so parallel sessions do not fight over target/ and the release exe \
             — see CLAUDE.md ビルド・テスト. Document edits and review are fine here."
        );
    }
    Ok(())
}

/// Returns the first JSON string value for `key` in `input`, unescaped just
/// enough for paths (\\ \" \/). The payload is machine-produced JSON, so the
/// first occurrence of a key like "file_path" is the tool input's.
fn string_field(input: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let after_key = &input[input.find(&needle)? + needle.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let body = after_colon.strip_prefix('"')?;
    let mut value = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(value),
            '\\' => match chars.next()? {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                other => value.push(other),
            },
            other => value.push(other),
        }
    }
    None
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
