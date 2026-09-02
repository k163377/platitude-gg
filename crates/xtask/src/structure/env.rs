//! Which of the app's files may look a `PG_*` variable up, counted by
//! machine.
//!
//! The verification harness is a thing the product can be built without
//! (`platitude-app` §features), and what keeps that true on the Rust side
//! is that the app asks one module what is driving it
//! (`harness::knobs`) instead of asking `std::env` wherever the answer is
//! wanted. That module is behind the feature, so a build without it never
//! looks a `PG_*` variable up — which is what stops a variable somebody
//! happens to have exported from reaching a shipped window.
//!
//! It is one line to lose and nothing catches it at run time — on the
//! machine that would have noticed, the variable is simply not set — so it
//! is counted here instead (.claude/rules/app-ui.md).
//!
//! `PG_LOG` is the exception, and deliberately: it says how loud to be
//! rather than who is driving (`settings::NOT_AUTOMATION`), and it is read
//! before there is a harness to ask.

use std::path::Path;

/// The crate whose Rust this covers.
const APP: &str = "crates/platitude-app/src";
/// The one module that may look one up, and the entry point with the one
/// variable it may name. Spelled from inside the crate, because a literal
/// naming a crate root would read as this file depending on it
/// (`gate::graph`).
const READER: &str = "harness/knobs.rs";
const ENTRY: &str = "main.rs";
const LOGGING: &str = "PG_LOG";

/// One failure per file naming a `PG_*` variable it may not, and how many
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
            for name in named(line) {
                if shown == ENTRY && name == LOGGING {
                    continue;
                }
                failures.push(format!(
                    "{APP}/{shown}:{}: names {name} — the only place in this crate that looks \
                     a `PG_*` variable up is `{READER}` (and `{ENTRY}` for `{LOGGING}`, which \
                     says how loud to be rather than who is driving). Put the knob on \
                     `Knobs` instead, so a build without the harness reads no environment \
                     at all (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
        }
    }
    Ok((failures, files.len()))
}

/// The `PG_*` variables one line of Rust names as a string literal.
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
    while let Some(at) = rest.find("\"PG_") {
        // Past the opening quote, then up to the closing one.
        rest = &rest[at + 1..];
        let end = rest.find('"').unwrap_or(rest.len());
        found.push(&rest[..end]);
        rest = &rest[end..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_literal_is_a_lookup_and_a_comment_is_a_reference() {
        assert_eq!(named("    act: text(\"PG_AUTO_ACT\"),"), ["PG_AUTO_ACT"]);
        assert_eq!(
            named("        .env(\"PG_PERF_OID\", \"PG_PERF_FILE\")"),
            ["PG_PERF_OID", "PG_PERF_FILE"]
        );
        assert!(named("    /// `PG_MEM_REPORT=1`: the window drives it").is_empty());
        assert!(named("    // PG_AUTO_ACT is read in knobs.rs").is_empty());
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
