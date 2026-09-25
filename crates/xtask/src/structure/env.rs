//! Which of the app's files may look a `PGG_*` variable up, counted by
//! machine (.claude/rules/app-ui.md「Rust 側で `PGG_*` を読むのは
//! `harness::knobs` だけ」). `harness::knobs` is behind the harness
//! feature, so a shipped window reads no exported variable; a stray lookup
//! shows nothing at run time, where the variable is simply not set.
//!
//! `PGG_LOG` (`settings::NOT_AUTOMATION`) is read in `main.rs`, before
//! there is a harness to ask.
//!
//! The core's `settings::Env` reads the same set without a `PGG_` literal
//! (`Env::system().automated()`), so it is held to the same module.

use std::path::Path;

/// The crate whose Rust this covers.
const APP: &str = "crates/platitude-app/src";
/// The one module that may look one up, and the entry point with the one
/// variable it may name. Spelled from inside the crate, because a literal
/// naming a crate root would read as this file depending on it
/// (`gate::graph`).
const READER: &str = "harness/knobs.rs";
const ENTRY: &str = "main.rs";
const LOGGING: &str = "PGG_LOG";
/// The core's own reader over the same variables, held to the same module.
const READER_TYPE: &str = "settings::Env";

/// One failure per file naming a `PGG_*` variable it may not, and how many
/// files were read for them.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let app = root.join(APP);
    let mut files = Vec::new();
    super::collect(&app, &mut files)?;
    files.sort();
    files.retain(|path| path.extension().is_some_and(|e| e == "rs"));
    let mut failures = Vec::new();
    for path in &files {
        let shown = super::relative(&app, path);
        if shown == READER {
            continue;
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (number, line) in text.lines().enumerate() {
            if uses_reader(line) {
                failures.push(format!(
                    "{APP}/{shown}:{}: uses `{READER_TYPE}` — it reads the same variables without \
                     spelling one, so the only place in this crate that may hold it is `{READER}`. \
                     Put the answer on `Knobs` and hand that to whoever needs it \
                     (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
            for name in named(line) {
                if shown == ENTRY && name == LOGGING {
                    continue;
                }
                failures.push(format!(
                    "{APP}/{shown}:{}: names {name} — the only place in this crate that looks \
                     a `PGG_*` variable up is `{READER}` (and `{ENTRY}` for `{LOGGING}`, which \
                     says how loud to be). Put the knob on \
                     `Knobs`, so a build without the harness reads no environment \
                     at all (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
        }
    }
    Ok((failures, files.len()))
}

/// The `PGG_*` variables one line of Rust names as a string literal; none
/// on a comment line, where a name is a reference and not a lookup. Holds
/// while nothing in the crate pastes a name together from halves.
fn named(line: &str) -> Vec<&str> {
    let code = line.trim_start();
    if code.starts_with("//") {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut rest = code;
    while let Some(at) = rest.find("\"PGG_") {
        rest = &rest[at + 1..];
        let end = rest.find('"').unwrap_or(rest.len());
        found.push(&rest[..end]);
        rest = &rest[end..];
    }
    found
}

/// Whether one line of Rust reaches for the core's environment reader;
/// false on a comment line, as in [`named`].
fn uses_reader(line: &str) -> bool {
    let code = line.trim_start();
    !code.starts_with("//") && code.contains(READER_TYPE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_reader_is_a_lookup_and_a_comment_is_a_reference() {
        assert!(uses_reader(
            "        automated: platitude_core::settings::Env::system().automated(),"
        ));
        assert!(uses_reader("use platitude_core::settings::Env;"));
        assert!(!uses_reader(
            "    /// (`settings::Env::automated` — any knob)"
        ));
        assert!(!uses_reader("    let driving = knobs().automated;"));
    }

    #[test]
    fn a_literal_is_a_lookup_and_a_comment_is_a_reference() {
        assert_eq!(named("    act: text(\"PGG_AUTO_ACT\"),"), ["PGG_AUTO_ACT"]);
        assert_eq!(
            named("        .env(\"PGG_PERF_OID\", \"PGG_PERF_FILE\")"),
            ["PGG_PERF_OID", "PGG_PERF_FILE"]
        );
        assert!(named("    /// `PGG_MEM_REPORT=1`: the window drives it").is_empty());
        assert!(named("    // PGG_AUTO_ACT is read in knobs.rs").is_empty());
        assert!(named("    let on = knobs().mem_report;").is_empty());
    }

    /// The rule the count is for: the tree it runs on passes it.
    #[test]
    fn the_app_reads_the_environment_in_one_place() {
        let (failures, files) = check(&crate::tree::workspace_root()).expect("scan the app");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(files > 1, "the app has more than one .rs file");
    }
}
