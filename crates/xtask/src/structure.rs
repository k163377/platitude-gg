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
//!   No ceiling applies and no baseline entry is kept: the entry is the
//!   whole of the standing, and what the file measures is nobody's to
//!   record. The entry did carry the length once, as a second ceiling that
//!   ratcheted — the point being that growth would land an edit on the line
//!   carrying the reason. What it landed instead was a number rewritten
//!   every time a verb was added to a driver, which is not a reading of the
//!   reason (2026-08-29 ユーザー判断). The check that remains is that the
//!   entry names a file that is still there.
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

/// One file counted, next to the ceiling that applies to it.
struct Counted {
    path: String,
    lines: usize,
    ceiling: usize,
}

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (structure takes none)"));
    }
    let root = crate::workspace_root();
    let ledger = read_ledger(&root)?;
    let counted = scan(&root)?;

    let mut failures = check_ledger(&ledger, &counted);

    let (ledgered, rest): (Vec<&Counted>, Vec<&Counted>) = counted
        .iter()
        .filter(|file| file.lines > file.ceiling)
        .partition(|file| ledger.iter().any(|entry| names(entry, &file.path)));
    let (pinned, over) = check_baseline(&root, &rest, counted.len())?;
    failures.extend(over);

    for failure in &failures {
        println!("structure: {failure}");
    }
    if failures.is_empty() {
        println!(
            "structure: {} files counted, {} on the ledger, {pinned} pinned by the baseline \
             — PASS",
            counted.len(),
            ledgered.len()
        );
        Ok(())
    } else {
        Err(format!(
            "{} file(s) over the length they are held to ({RULES} §上限)",
            failures.len()
        ))
    }
}

/// The baseline ratchet, over the files the ledger does not answer for.
///
/// Gives back how many files the baseline pins and one failure per file
/// that crossed a line it may not cross.
fn check_baseline(
    root: &Path,
    rest: &[&Counted],
    scanned: usize,
) -> Result<(usize, Vec<String>), String> {
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
        return Ok((fresh.len(), Vec::new()));
    };

    let mut failures: Vec<String> = Vec::new();
    let mut next: BTreeMap<String, usize> = BTreeMap::new();
    for file in rest {
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
    Ok((next.len(), failures))
}

/// Every .rs and .qml under crates/, counted.
///
/// All of them, not just the ones over a ceiling: a ledgered file under the
/// ceiling still owes its entry the length that entry records.
///
/// crates/ is the whole of what the rule covers: spike/ is throwaway Phase 0
/// reference code the workspace already excludes, and target/ is not walked
/// even where a stray per-crate one appears.
fn scan(root: &Path) -> Result<Vec<Counted>, String> {
    let mut files = Vec::new();
    collect(&root.join("crates"), &mut files)?;
    files.sort();
    let mut counted = Vec::with_capacity(files.len());
    for file in &files {
        let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let lines = String::from_utf8_lossy(&bytes).lines().count();
        let path = relative(root, file);
        let ceiling = if path.contains("/tests/") {
            TESTS_CEILING
        } else {
            SRC_CEILING
        };
        counted.push(Counted {
            path,
            lines,
            ceiling,
        });
    }
    Ok(counted)
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

/// The ledger, its text, and the entries that exempt a file.
///
/// The text comes back with them because a shrink rewrites the numbers in
/// place, and the offsets the entries carry are into this string.
///
/// Ledger entries name a file by however much of its tail tells it apart
/// (`ui/AutoActDriver.qml`), so these match as path suffixes. Only the
/// 分割しない判断 section counts — the sections above it name files as
/// examples of a trap, not as permission to be long, and reading the whole
/// document would quietly excuse them.
fn read_ledger(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(LEDGER);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let at = text
        .find(LEDGER_SECTION)
        .map(|found| found + LEDGER_SECTION.len())
        .ok_or_else(|| {
            format!(
                "{}: no `{LEDGER_SECTION}` section — the ceiling reads its exemptions from \
                 there, so a renamed heading would silently withdraw every one of them",
                path.display()
            )
        })?;
    Ok(ledger_entries(&text[at..]))
}

/// The entries a ledger section holds, up to the next heading.
fn ledger_entries(section: &str) -> Vec<String> {
    section
        .split("\n## ")
        .next()
        .unwrap_or(section)
        .lines()
        .filter_map(ledger_bullet)
        .collect()
}

/// The file one bullet exempts, if it is one of the bullets that do.
///
/// A bullet exempts the file it is *about*, and that is the one it opens
/// with in bold: ``- **`path` は割らない** — …``. The same section also
/// carries bullets about one long function inside a file, which name their
/// file in passing and must not hand the whole file a ceiling exemption;
/// leading on the file in bold is what tells the two apart. Losing the bold
/// costs an exemption and turns the count red, which is the direction a
/// formatting slip should fail in.
fn ledger_bullet(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("- **")?.strip_prefix('`')?;
    let (raw, _) = rest.split_once('`')?;
    if !(raw.ends_with(".rs") || raw.ends_with(".qml")) {
        return None;
    }
    Some(raw.replace('\\', "/"))
}

/// That every entry still names a file, which is the one thing about a
/// permanent exemption a machine can hold: a file that was split away
/// leaves its entry behind, and the entry goes on excusing a name.
fn check_ledger(ledger: &[String], counted: &[Counted]) -> Vec<String> {
    ledger
        .iter()
        .filter(|entry| !counted.iter().any(|file| names(entry, &file.path)))
        .map(|entry| {
            format!(
                "{LEDGER} 分割しない判断: `{}` names no file under crates/ — if it was split \
                 away, the entry goes with it (行が消えたら分割済み)",
                entry
            )
        })
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
mod tests;
