//! `cargo xtask structure` — the per-file line ceilings of
//! .claude/rules/structure.md, counted by machine.
//!
//! The ceilings are a ratchet, not a sweep. Some forty files were already
//! past them when this went in, and failing all of them at once would have
//! forced exactly the one commit structure.md forbids: a tree-wide reformat
//! that moves everything and shows nothing. So a file has one of three
//! standings, and only the third is a ceiling in the plain sense:
//!
//! * **on the ledger** — .claude/rules-refs/structure.md 分割しない判断 holds
//!   a written reason not to split it, which is the rule's own escape hatch.
//!   No ceiling applies, and no baseline entry is kept for it.
//! * **in the baseline** — over the ceiling from before the count existed,
//!   pinned at the number it had. It may shrink, and the baseline follows it
//!   down; it may not grow, which is structure.md's 上限超過ファイルへ追記しない
//!   with a machine behind it.
//! * **neither** — the ceiling applies as written, so a file that crosses it
//!   for the first time fails on the run that first sees it.
//!
//! Physical lines, not code lines: that is what structure.md says, and a
//! count anybody can reproduce with an editor's line number is worth more
//! here than one that argues about comments. The fn ceiling is the other
//! half of the same § and is left to clippy's `too_many_lines`, which
//! already knows where functions begin and end.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Ceilings in physical lines (.claude/rules/structure.md §上限).
const SRC_CEILING: usize = 500;
const TESTS_CEILING: usize = 1000;

/// Where the ratchet keeps what was already over when it went in.
const BASELINE: &str = "crates/xtask/structure-baseline.txt";
/// The written refusals to split, which outrank the ceiling.
const LEDGER: &str = ".claude/rules-refs/structure.md";
/// The heading whose section holds them.
const LEDGER_SECTION: &str = "## 分割しない判断";
/// Where a failing count sends its reader.
const RULES: &str = ".claude/rules/structure.md";

const BASELINE_HEADER: &str = "\
# Baseline for `cargo xtask structure` — the files that were already past
# .claude/rules/structure.md's line ceiling when the count went in.
#
# Each line pins one file at the length it had: `cargo xtask structure` fails
# if it grows past that number, and rewrites the number downwards when the
# file shrinks, so the list only ever loosens by being split. A file that
# drops under the ceiling, or is split away entirely, leaves the list on the
# next run. Lowering a number by hand is fine; raising one is the thing this
# file exists to stop. A file that is deliberately not split belongs in
# .claude/rules-refs/structure.md 分割しない判断 instead — the ledger there
# outranks the ceiling and needs no entry here.
#
# <physical lines> <path from the workspace root>
";

/// One file counted past the ceiling that applies to it.
struct Over {
    path: String,
    lines: usize,
    ceiling: usize,
}

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (structure takes none)"));
    }
    let root = crate::workspace_root();
    let ledger = ledger_paths(&root)?;
    let (scanned, over) = scan(&root)?;
    let (ledgered, rest): (Vec<Over>, Vec<Over>) = over
        .into_iter()
        .partition(|file| ledger.iter().any(|tail| names(tail, &file.path)));

    let baseline_path = root.join(BASELINE);
    let Some(baseline) = read_baseline(&baseline_path)? else {
        let fresh: BTreeMap<String, usize> = rest
            .iter()
            .map(|file| (file.path.clone(), file.lines))
            .collect();
        write_baseline(&baseline_path, &fresh)?;
        println!(
            "structure: {scanned} files counted; wrote {BASELINE} pinning the {} already \
             over their ceiling — from here they may shrink, not grow",
            fresh.len()
        );
        return Ok(());
    };

    let mut failures: Vec<String> = Vec::new();
    let mut next: BTreeMap<String, usize> = BTreeMap::new();
    for file in &rest {
        match baseline.get(&file.path) {
            None => failures.push(format!(
                "{}: {} lines, {} past the {}-line ceiling — split the responsibility out \
                 ({RULES} §分割), or record in {LEDGER} why it is not split",
                file.path,
                file.lines,
                file.lines - file.ceiling,
                file.ceiling
            )),
            Some(&was) if file.lines > was => {
                failures.push(format!(
                    "{}: {} lines, {} more than the {was} it is pinned at and {} past the \
                     {}-line ceiling — 上限超過ファイルへ追記しない ({RULES} §上限): split \
                     first, or put the addition where it belongs",
                    file.path,
                    file.lines,
                    file.lines - was,
                    file.lines - file.ceiling,
                    file.ceiling
                ));
                // Pinned where it was: a run that fails must not also raise
                // the bar it just failed against.
                next.insert(file.path.clone(), was);
            }
            Some(&was) => {
                next.insert(file.path.clone(), file.lines.min(was));
            }
        }
    }

    if next != baseline {
        let mut shrunk = 0;
        for (path, lines) in &next {
            if baseline.get(path).is_some_and(|was| lines < was) {
                shrunk += 1;
            }
        }
        let gone = baseline
            .keys()
            .filter(|path| !next.contains_key(*path))
            .count();
        write_baseline(&baseline_path, &next)?;
        println!(
            "structure: {BASELINE} followed the tree down ({shrunk} shorter, {gone} off the \
             list) — commit it with the change that earned it"
        );
    }

    for failure in &failures {
        println!("structure: {failure}");
    }
    if failures.is_empty() {
        println!(
            "structure: {scanned} files counted, {} on the ledger, {} pinned by the baseline \
             — PASS",
            ledgered.len(),
            next.len()
        );
        Ok(())
    } else {
        Err(format!(
            "{} file(s) past the line ceiling ({RULES} §上限)",
            failures.len()
        ))
    }
}

/// Every .rs and .qml under crates/, and the ones past their ceiling.
///
/// crates/ is the whole of what the rule covers: spike/ is throwaway Phase 0
/// reference code the workspace already excludes, and target/ is not walked
/// even where a stray per-crate one appears.
fn scan(root: &Path) -> Result<(usize, Vec<Over>), String> {
    let mut files = Vec::new();
    collect(&root.join("crates"), &mut files)?;
    files.sort();
    let mut over = Vec::new();
    for file in &files {
        let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let lines = String::from_utf8_lossy(&bytes).lines().count();
        let path = relative(root, file);
        let ceiling = if path.contains("/tests/") {
            TESTS_CEILING
        } else {
            SRC_CEILING
        };
        if lines > ceiling {
            over.push(Over {
                path,
                lines,
                ceiling,
            });
        }
    }
    Ok((files.len(), over))
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            // Neither holds anything a person wrote: a crate-local target/
            // appears the moment somebody runs cargo from inside a crate,
            // and a dot-directory belongs to tooling.
            if name != "target" && !name.starts_with('.') {
                collect(&path, out)?;
            }
        } else if name.ends_with(".rs") || name.ends_with(".qml") {
            out.push(path);
        }
    }
    Ok(())
}

fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The files the ledger already answers for.
///
/// Ledger entries name a file by however much of its tail tells it apart
/// (`ui/AutoActDriver.qml`), so these match as path suffixes. Only the
/// 分割しない判断 section counts — the sections above it name files as
/// examples of a trap, not as permission to be long, and reading the whole
/// document would quietly excuse them.
fn ledger_paths(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(LEDGER);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let section = text.split(LEDGER_SECTION).nth(1).ok_or_else(|| {
        format!(
            "{}: no `{LEDGER_SECTION}` section — the ceiling reads its exemptions from there, \
             so a renamed heading would silently withdraw every one of them",
            path.display()
        )
    })?;
    Ok(ledger_entries(section))
}

/// The files a ledger section exempts, up to the next heading.
///
/// A bullet exempts the file it is *about*, and that is the one it opens
/// with in bold: ``- **`path`(N 行)は割らない** — …``. The same section
/// also carries bullets about one long function inside a file, which name
/// their file in passing and must not hand the whole file a ceiling
/// exemption; leading on the file in bold is what tells the two apart.
/// Losing the bold costs an exemption and turns the count red, which is
/// the direction a formatting slip should fail in.
fn ledger_entries(section: &str) -> Vec<String> {
    let section = section.split("\n## ").next().unwrap_or(section);
    section
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("- **"))
        .filter_map(|lead| lead.strip_prefix('`'))
        .filter_map(|rest| rest.split('`').next())
        .filter(|token| token.ends_with(".rs") || token.ends_with(".qml"))
        .map(|token| token.replace('\\', "/"))
        .collect()
}

/// Whether a ledger entry names this file — as the whole path, or as the
/// tail of it that starts at a path separator (so `nav.rs` never stands for
/// `models/nav.rs`'s neighbour `graph_nav.rs`).
fn names(entry: &str, path: &str) -> bool {
    path == entry || path.ends_with(&format!("/{entry}"))
}

fn read_baseline(path: &Path) -> Result<Option<BTreeMap<String, usize>>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let mut entries = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = index + 1;
        let (lines, file) = line.split_once(' ').ok_or_else(|| {
            format!(
                "{}:{at}: expected `<lines> <path>`, got {line:?}",
                path.display()
            )
        })?;
        let lines = lines.parse::<usize>().map_err(|e| {
            format!(
                "{}:{at}: {lines:?} is not a line count: {e}",
                path.display()
            )
        })?;
        entries.insert(file.trim().to_string(), lines);
    }
    Ok(Some(entries))
}

/// Writes the baseline with LF endings, which .gitattributes pins the tree
/// to on all three OSes — a CRLF rewrite here would show up as a diff on
/// every Windows run.
fn write_baseline(path: &Path, entries: &BTreeMap<String, usize>) -> Result<(), String> {
    let mut text = String::from(BASELINE_HEADER);
    for (file, lines) in entries {
        text.push_str(&format!("{lines} {file}\n"));
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{ledger_entries, names, read_baseline, write_baseline};
    use std::collections::BTreeMap;

    #[test]
    fn reads_the_ledger_down_to_the_next_heading() {
        let section = "(台帳 — 行が消えたら分割済み)\n\n\
             - **`ui/AutoActDriver.qml`(1769 行)は割らない** — `runAutoAct()` の分岐\n\
             - **`crates/x/src/a.rs` も同じ理由**\n\
             \n## 次の見出し\n\n- **`ui/NotHere.qml` は別の話**\n";
        assert_eq!(
            ledger_entries(section),
            ["ui/AutoActDriver.qml", "crates/x/src/a.rs"]
        );
    }

    #[test]
    fn reads_no_entry_out_of_prose_that_names_no_file() {
        assert_eq!(
            ledger_entries("- `runAutoAct()` の分岐は割らない\n"),
            [""; 0]
        );
    }

    #[test]
    fn reads_no_file_exemption_out_of_a_bullet_about_one_function() {
        // The fn ceiling keeps its entries in the same section; a file
        // named there in passing keeps its own ceiling.
        let section = "\n\
             - `parse/diff/parse.rs` の `parse_patch` は 196 行(上限 100)\n\
             - **`ui/Pane.qml`(700 行)は割らない** — 理由\n";
        assert_eq!(ledger_entries(section), ["ui/Pane.qml"]);
    }

    #[test]
    fn matches_a_ledger_entry_only_at_a_path_boundary() {
        assert!(names(
            "ui/AutoActDriver.qml",
            "crates/a/src/ui/AutoActDriver.qml"
        ));
        assert!(names("crates/a/src/nav.rs", "crates/a/src/nav.rs"));
        assert!(!names("nav.rs", "crates/a/src/graph_nav.rs"));
        assert!(!names("ui/Pane.qml", "crates/a/src/ui/PaneX.qml"));
    }

    #[test]
    fn round_trips_a_baseline_through_the_file_it_writes() {
        let dir = std::env::temp_dir().join("pg-structure-baseline-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("baseline.txt");
        let entries = BTreeMap::from([
            ("crates/a/src/b.rs".to_string(), 501),
            ("crates/a/tests/it/c.rs".to_string(), 1200),
        ]);
        write_baseline(&path, &entries).unwrap();
        assert_eq!(read_baseline(&path).unwrap(), Some(entries));
        // The header is comments, and the body is one file per line.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains('\r'), "baseline must stay LF: {text:?}");
        assert!(text.ends_with("501 crates/a/src/b.rs\n1200 crates/a/tests/it/c.rs\n"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_a_missing_baseline_as_the_first_run() {
        let path = std::env::temp_dir().join("pg-structure-baseline-absent.txt");
        let _ = std::fs::remove_file(&path);
        assert_eq!(read_baseline(&path).unwrap(), None);
    }
}
