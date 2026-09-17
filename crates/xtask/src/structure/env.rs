//! Which of the app's files may look a `PGG_*` variable up, counted by
//! machine.
//!
//! The verification harness is a thing the product can be built without
//! (`platitude-app` §features), and what keeps that true on the Rust side
//! is that the app asks one module what is driving it
//! (`harness::knobs`). That module is behind the feature, so a build
//! without it never looks a `PGG_*` variable up — which is what stops a
//! variable somebody happens to have exported from reaching a shipped
//! window.
//!
//! It is one line to lose and nothing catches it at run time — on the
//! machine that would have noticed, the variable is simply not set — so it
//! is counted here instead (.claude/rules/app-ui.md).
//!
//! `PGG_LOG` is the exception, and deliberately: it says how loud to be
//! (`settings::NOT_AUTOMATION`), and it is read
//! before there is a harness to ask.
//!
//! **Spelling a variable is not the only way to read one.** The core's
//! `settings::Env` is a reader over the same set — `Env::system().automated()`
//! answers "is anything driving this" without a `PGG_` literal anywhere — so
//! a shipped window that happened to have one exported would have gone on
//! answering yes through it while the count above passed. That reader is
//! held to the same one module.

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

/// The `PGG_*` variables one line of Rust names as a string literal.
///
/// None on a comment line: the crate's comments point at the protocol
/// constantly, and a name to read is not a lookup. What is left is close
/// enough to a literal to hold the rule — a variable has to be spelled
/// somewhere to be read, and nothing in this crate spells one by pasting
/// two halves together.
fn named(line: &str) -> Vec<&str> {
    let code = line.trim_start();
    if code.starts_with("//") {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut rest = code;
    while let Some(at) = rest.find("\"PGG_") {
        // Past the opening quote, then up to the closing one.
        rest = &rest[at + 1..];
        let end = rest.find('"').unwrap_or(rest.len());
        found.push(&rest[..end]);
        rest = &rest[end..];
    }
    found
}

/// Whether one line of Rust reaches for the core's environment reader.
///
/// None on a comment line, for the reason [`named`] gives: the crate's
/// comments point at `Env::automated` where they explain what a knob
/// means, and a name to read is not a lookup.
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
