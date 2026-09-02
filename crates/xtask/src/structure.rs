//! `cargo xtask structure` — the per-file length backstop of
//! .claude/rules/structure.md, counted by machine.
//!
//! **Code lines**: blank lines and comment-only lines do not count, the way
//! clippy counts a function for `too_many_lines`. A count that charged for
//! comments would pay a reader to delete them, and deleting them is not
//! what a long file needs.
//!
//! One number covers every file, and it is a backstop rather than a target:
//! what a file past it needs is a look at its design, which is a person's
//! call on the reason and not a machine's on the count. So the count only
//! has to be loud once, and a file has one of three standings:
//!
//! * **on the ledger** — .claude/rules-refs/structure.md 分割しない判断 holds
//!   a written reason not to split it, which is the rule's own escape hatch.
//!   No backstop applies and no baseline entry is kept: the entry is the
//!   whole of the standing, and what the file measures is nobody's to
//!   record (a length on the entry would be rewritten on every growth
//!   without anyone re-reading the reason). The check that remains is
//!   that the entry names a file that is still there.
//! * **in the baseline** — pinned at the length it had. It may shrink, and
//!   the baseline follows it down; it may not grow.
//! * **neither** — the backstop applies as written, so a file that crosses
//!   it for the first time fails on the run that first sees it.
//!
//! Two other things about how the tree is divided are counted here,
//! because they are the same shape of question and the same second of
//! work: the product's QML may not name a type from the verification
//! harness's module ([`modules`]), and the app's Rust may not look a
//! `PG_*` variable up outside the one module that owns them ([`env`]).
//!
//! The fn half of the same § is left to clippy's `too_many_lines`, which
//! already knows where functions begin and end.

mod env;
mod modules;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The length past which a file's design is the question, in code lines
/// (.claude/rules/structure.md §長さの閾値). It is clippy's proposed
/// `too_many_lines_in_file` default, counted the way that lint counts.
const BACKSTOP: usize = 1000;

/// Where the ratchet keeps what was already over when it went in.
const BASELINE: &str = "crates/xtask/structure-baseline.txt";
/// The written refusals to split, which outrank the backstop.
const LEDGER: &str = ".claude/rules-refs/structure.md";
/// The heading whose section holds them.
const LEDGER_SECTION: &str = "## 分割しない判断";
/// Where a failing count sends its reader.
const RULES: &str = ".claude/rules/structure.md";

const BASELINE_HEADER: &str = "\
# Baseline for `cargo xtask structure` — the files that were already past
# .claude/rules/structure.md's length backstop when the count went in.
#
# Each line pins one file at the length it had: `cargo xtask structure` fails
# if it grows past that number, and rewrites the number downwards when the
# file shrinks, so the list only ever loosens by being split. A file that
# drops under the backstop, or is split away entirely, leaves the list on the
# next run. Lowering a number by hand is fine; raising one is the thing this
# file exists to stop. A file that is deliberately not split belongs in
# .claude/rules-refs/structure.md 分割しない判断 instead — the ledger there
# outranks the backstop and needs no entry here.
#
# <code lines> <path from the workspace root>
";

/// One file counted: the code lines it is held to, and the physical lines
/// they sit in, because "long in comments" and "long in code" want
/// different answers and only the second is this check's business.
struct Counted {
    path: String,
    code: usize,
    physical: usize,
}

impl Counted {
    /// How long it is, in the terms a reader needs to pick a fix: the
    /// number it is held to, and how much of the file is comment.
    fn measured(&self) -> String {
        format!(
            "{} code lines in {} physical ({}% comment and blank)",
            self.code,
            self.physical,
            comment_share(self.physical, self.code)
        )
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (structure takes none)"));
    }
    let root = crate::tree::workspace_root();
    let ledger = read_ledger(&root)?;
    let counted = scan(&root)?;

    let mut failures = check_ledger(&ledger, &counted);

    let (ledgered, rest): (Vec<&Counted>, Vec<&Counted>) = counted
        .iter()
        .filter(|file| file.code > BACKSTOP)
        .partition(|file| ledger.iter().any(|entry| names(entry, &file.path)));
    let (pinned, over) = check_baseline(&root, &rest, counted.len())?;
    failures.extend(over);
    let (crossed, harness_types) = modules::check(&root)?;
    let (looked_up, app_files) = env::check(&root)?;
    let boundary_broken = crossed.len() + looked_up.len();
    failures.extend(crossed);
    failures.extend(looked_up);

    for failure in &failures {
        println!("structure: {failure}");
    }
    if failures.is_empty() {
        let code: usize = counted.iter().map(|file| file.code).sum();
        let physical: usize = counted.iter().map(|file| file.physical).sum();
        println!(
            "structure: {} files counted, {code} code lines in {physical} physical \
             ({}% comment and blank), {} on the ledger, {pinned} pinned by the baseline, \
             {harness_types} harness types out of the product's reach, \
             {app_files} app files off the environment — PASS",
            counted.len(),
            comment_share(physical, code),
            ledgered.len()
        );
        Ok(())
    } else if boundary_broken == failures.len() {
        Err(format!(
            "{boundary_broken} file(s) reaching for something a shipped build does not \
             carry (.claude/rules/app-ui.md)"
        ))
    } else {
        Err(format!(
            "{} file(s) over the length they are held to ({RULES} §長さの閾値)",
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
            .map(|file| (file.path.clone(), file.code))
            .collect();
        write_baseline(&baseline_path, &fresh)?;
        println!(
            "structure: {scanned} files counted; wrote {BASELINE} pinning the {} already \
             over the backstop — from here they may shrink, not grow",
            fresh.len()
        );
        return Ok((fresh.len(), Vec::new()));
    };

    let mut failures: Vec<String> = Vec::new();
    let mut next: BTreeMap<String, usize> = BTreeMap::new();
    for file in rest {
        match baseline.get(&file.path) {
            None => failures.push(format!(
                "{}: {}, {} past the {BACKSTOP}-line backstop — ask the design the question \
                 ({RULES} §長さの閾値): split the responsibility out, or record in {LEDGER} \
                 why it is not split",
                file.path,
                file.measured(),
                file.code - BACKSTOP,
            )),
            Some(&was) if file.code > was => {
                failures.push(format!(
                    "{}: {}, {} more than the {was} it is pinned at and {} past the \
                     {BACKSTOP}-line backstop — a pinned file may shrink, not grow \
                     ({RULES} §長さの閾値): split first, or record in {LEDGER} why it is \
                     not split",
                    file.path,
                    file.measured(),
                    file.code - was,
                    file.code - BACKSTOP,
                ));
                // Pinned where it was: a run that fails must not also raise
                // the bar it just failed against.
                next.insert(file.path.clone(), was);
            }
            Some(&was) => {
                next.insert(file.path.clone(), file.code.min(was));
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
/// All of them, not just the ones over the backstop: the run reports what
/// the tree costs in code and what it spends on comments, and a ledgered
/// file still owes its entry the fact that it is there.
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
        let text = String::from_utf8_lossy(&bytes);
        counted.push(Counted {
            path: relative(root, file),
            code: code_lines(&text),
            physical: text.lines().count(),
        });
    }
    Ok(counted)
}

/// The lines of a .rs or .qml file that carry code, counted the way clippy
/// counts a function body for `too_many_lines`: a line counts once it has
/// anything on it outside a `//` or `/* */` comment.
///
/// Both languages comment alike, so one counter answers for both. A `//`
/// inside a string literal ends the line early here, as it does in clippy —
/// what is left of the line is still code, so the line is still counted.
fn code_lines(text: &str) -> usize {
    let mut count = 0;
    let mut in_comment = false;
    for line in text.lines() {
        let mut rest = line;
        let mut has_code = false;
        loop {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }
            if in_comment {
                let Some(end) = rest.find("*/") else { break };
                rest = &rest[end + 2..];
                in_comment = false;
                continue;
            }
            let block = rest.find("/*").unwrap_or(rest.len());
            let line_comment = rest.find("//").unwrap_or(rest.len());
            has_code |= block > 0 && line_comment > 0;
            if block < line_comment {
                rest = &rest[block + 2..];
                in_comment = true;
                continue;
            }
            break;
        }
        if has_code {
            count += 1;
        }
    }
    count
}

/// What share of a file's lines is comment or blank, as whole percent.
fn comment_share(physical: usize, code: usize) -> usize {
    (physical.saturating_sub(code) * 100)
        .checked_div(physical)
        .unwrap_or(0)
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
                "{}: no `{LEDGER_SECTION}` section — the backstop reads its exemptions from \
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
/// file in passing and must not hand the whole file an exemption;
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
