//! Which of the app's files may look a harness knob (`_PGG_*`) up,
//! counted by machine (.claude/rules/app-ui.md「Rust 側で `_PGG_*` を読む
//! のは `harness::knobs` だけ」). `harness::knobs` is behind the harness
//! feature, so a shipped window reads no knob; a stray lookup shows
//! nothing at run time, where the variable is simply not set.
//!
//! The prefix alone tells a reader (.claude/rules/code.md): the product's
//! own variables spell `PGG_` (`main.rs` reads `PGG_LOG` before there is a
//! harness to ask), and xtask's and the hooks' spell `PGGDEV_`, which no
//! file of the app reads — they reach its environment, since a run clears
//! only the knobs it inherits.
//!
//! The core's `settings::Env` reads the knobs without a `_PGG_` literal
//! (`Env::system().automated()`), so it is held to the same module.

use std::path::Path;

/// The crate whose Rust this covers.
const APP: &str = "crates/platitude-app/src";
/// The one module that may look one up. Spelled from inside the crate,
/// because a literal naming a crate root would read as this file depending
/// on it (`gate::graph`).
const READER: &str = "harness/knobs.rs";
/// The core's own reader over the same variables, held to the same module.
const READER_TYPE: &str = "settings::Env";
/// A harness knob's prefix.
const KNOB: &str = "_PGG_";
/// The prefix of xtask's and the hooks' own variables.
const TOOLING: &str = "PGGDEV_";

/// One failure per line naming a variable its file may not, and how many
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
        let reader = shown == READER;
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (number, line) in text.lines().enumerate() {
            if !reader && uses_reader(line) {
                failures.push(format!(
                    "{APP}/{shown}:{}: uses `{READER_TYPE}` — it reads the same variables without \
                     spelling one, so the only place in this crate that may hold it is `{READER}`. \
                     Put the answer on `Knobs` and hand that to whoever needs it \
                     (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
            let knobs = if reader {
                Vec::new()
            } else {
                named(line, KNOB)
            };
            for name in knobs {
                failures.push(format!(
                    "{APP}/{shown}:{}: names {name} — the only place in this crate that looks \
                     a `{KNOB}*` knob up is `{READER}`. Put the knob on `Knobs`, so a build \
                     without the harness reads no knob at all (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
            for name in named(line, TOOLING) {
                failures.push(format!(
                    "{APP}/{shown}:{}: names {name} — `{TOOLING}*` is what xtask and the hooks \
                     read for themselves, and nothing in the app reads one. A knob for the \
                     harness spells `{KNOB}` and lives on `Knobs` (.claude/rules/code.md)",
                    number + 1
                ));
            }
        }
    }
    Ok((failures, files.len()))
}

/// The variables starting with `prefix` that one line of Rust names as a
/// string literal; none on a comment line, where a name is a reference and
/// not a lookup. Holds while nothing in the crate pastes a name together
/// from halves.
fn named<'a>(line: &'a str, prefix: &str) -> Vec<&'a str> {
    let code = line.trim_start();
    if code.starts_with("//") {
        return Vec::new();
    }
    let quoted = format!("\"{prefix}");
    let mut found = Vec::new();
    let mut rest = code;
    while let Some(at) = rest.find(&quoted) {
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
        assert_eq!(
            named("    act: text(\"_PGG_AUTO_ACT\"),", KNOB),
            ["_PGG_AUTO_ACT"]
        );
        assert_eq!(
            named("        .env(\"_PGG_PERF_OID\", \"_PGG_PERF_FILE\")", KNOB),
            ["_PGG_PERF_OID", "_PGG_PERF_FILE"]
        );
        assert!(named("    /// `_PGG_MEM_REPORT=1`: the window drives it", KNOB).is_empty());
        assert!(named("    // _PGG_AUTO_ACT is read in knobs.rs", KNOB).is_empty());
        assert!(named("    let on = knobs().mem_report;", KNOB).is_empty());
        assert!(
            named("    std::env::var(\"PGG_LOG\")", KNOB).is_empty(),
            "the product's own variable is not a knob"
        );
    }

    #[test]
    fn a_tooling_variable_is_told_by_its_prefix_alone() {
        assert_eq!(
            named("    std::env::var(\"PGGDEV_STEP\")", TOOLING),
            ["PGGDEV_STEP"]
        );
        assert!(named("    std::env::var(\"_PGG_AUTO_ACT\")", TOOLING).is_empty());
        assert!(named("    std::env::var(\"PGG_LOG\")", TOOLING).is_empty());
        assert!(named("    // PGGDEV_STEP rides through to git", TOOLING).is_empty());
    }

    /// The rule the count is for: the tree it runs on passes it.
    #[test]
    fn the_app_reads_the_environment_in_one_place() {
        let (failures, files) = check(&crate::tree::workspace_root()).expect("scan the app");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(files > 1, "the app has more than one .rs file");
    }
}
