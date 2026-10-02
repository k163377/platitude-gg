//! `cargo xtask structure` — the per-file length backstop of
//! .claude/rules/structure.md §長さの閾値, counted by machine.
//!
//! **Code lines**: blank and comment-only lines do not count, as clippy
//! counts a function for `too_many_lines` — a count that charged for
//! comments would pay a reader to delete them.
//!
//! The backstop only has to be loud once, so a file has one of three
//! standings:
//!
//! * **on the ledger** — .claude/rules-refs/structure.md 分割しない判断 holds
//!   a written reason not to split it. No backstop and no baseline entry
//!   (a length kept for it would be rewritten on every growth without
//!   anyone re-reading the reason); the one check left is that the entry
//!   names a file still there.
//! * **in the baseline** — pinned at the length it had. It may shrink, and
//!   the baseline follows it down; it may not grow.
//! * **neither** — the backstop applies as written.
//!
//! Five other questions of how the tree is divided are counted here too:
//! the product's QML may not name a harness type ([`modules`]), the app's
//! Rust may not look a `PGG_*` variable up outside the module that owns
//! them ([`env`]), the product's QML closes no popup but its own
//! ([`popups`]), QML asks the graph, not a row's id, which row is this
//! window's working tree ([`wiprow`]), and every text box the product
//! declares keeps the style's right-click menu away ([`textboxes`]).
//!
//! The fn half of the same § is left to clippy's `too_many_lines`.

mod env;
mod modules;
mod popups;
mod textboxes;
mod wiprow;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::command::{self, Permission, Where};

pub(crate) static STRUCTURE: command::Command = command::Command {
    id: "structure.lengths",
    call: "structure",
    purpose: "the per-file length backstop, and what the tree spends on comments",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&STRUCTURE];

/// The length past which a file's design is the question, in code lines
/// (.claude/rules/structure.md §長さの閾値).
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
/// they sit in.
struct Counted {
    path: String,
    code: usize,
    physical: usize,
}

impl Counted {
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
    let (closed_for, product_files) = popups::check(&root)?;
    let (asked_the_id, qml_files) = wiprow::check(&root)?;
    let (styled_boxes, text_boxes) = textboxes::check(&root)?;
    let boundary_broken = crossed.len() + looked_up.len();
    let closing_for_others = closed_for.len();
    let menus_kept = styled_boxes.len();
    failures.extend(crossed);
    failures.extend(looked_up);
    failures.extend(closed_for);
    failures.extend(asked_the_id);
    failures.extend(styled_boxes);

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
             {app_files} app files off the environment, \
             {product_files} product files closing nothing but their own, \
             {qml_files} QML files asking the graph which row is this window's, \
             {text_boxes} text boxes keeping the style's menu away — PASS",
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
    } else if closing_for_others == failures.len() {
        Err(format!(
            "{closing_for_others} line(s) closing a popup that is not theirs to close \
             (.claude/rules-refs/app-ui.md「メニューを閉じるのは自分」)"
        ))
    } else if menus_kept == failures.len() {
        Err(format!(
            "{menus_kept} text box(es) keeping the style's right-click menu \
             (.claude/rules-refs/app-ui.md「`FieldMenu` = 文字の欄の右クリック」)"
        ))
    } else {
        Err(format!(
            "{} file(s) over the length they are held to ({RULES} §長さの閾値)",
            failures.len()
        ))
    }
}

/// The baseline ratchet over the files the ledger does not answer for:
/// how many files it pins, and one failure per file past its line.
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
             over the backstop — from here they may only shrink",
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
                     {BACKSTOP}-line backstop — a pinned file may only shrink \
                     ({RULES} §長さの閾値): split first, or record in {LEDGER} why it is \
                     not split",
                    file.path,
                    file.measured(),
                    file.code - was,
                    file.code - BACKSTOP,
                ));
                // A failing run leaves its bar where it stood.
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

/// Every .rs and .qml under crates/, counted — all of them, since the
/// totals and the ledger check need the files under the backstop too.
/// spike/ is outside the rule, as it is outside the workspace.
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
/// anything on it outside a `//` or `/* */` comment. A `//` inside a
/// string literal ends the line early, as in clippy; the line still counts.
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
            // Neither holds anything a person wrote (a crate-local target/
            // appears when cargo runs inside a crate).
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

/// The text with every comment and every string literal's contents turned
/// to spaces, line breaks kept so the lines still count.
fn without_comments_and_strings(text: &str) -> String {
    #[derive(PartialEq)]
    enum In {
        Code,
        Line,
        Block,
        Text(char),
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut state = In::Code;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match state {
            In::Code => match c {
                '/' if next == Some('/') => {
                    state = In::Line;
                    out.push_str("  ");
                    i += 2;
                    continue;
                }
                '/' if next == Some('*') => {
                    state = In::Block;
                    out.push_str("  ");
                    i += 2;
                    continue;
                }
                '"' | '\'' | '`' => {
                    state = In::Text(c);
                    out.push(c);
                }
                _ => out.push(c),
            },
            In::Line => {
                if c == '\n' {
                    state = In::Code;
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            }
            In::Block => {
                if c == '*' && next == Some('/') {
                    state = In::Code;
                    out.push_str("  ");
                    i += 2;
                    continue;
                }
                out.push(if c == '\n' { '\n' } else { ' ' });
            }
            In::Text(quote) => {
                if c == '\\' {
                    out.push_str("  ");
                    i += 2;
                    continue;
                }
                if c == quote {
                    state = In::Code;
                    out.push(c);
                } else {
                    out.push(if c == '\n' { '\n' } else { ' ' });
                }
            }
        }
        i += 1;
    }
    out
}

/// The ledger's entries: each names a file by the tail of its path that
/// tells it apart ([`names`]). Only the 分割しない判断 section counts — the
/// sections above it name files as examples of a trap.
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

/// The file one bullet exempts: the path it opens with in bold,
/// ``- **`path` …** — …``. A bullet naming a file in passing (about one
/// long fn in it) exempts nothing; a lost bold turns the count red, the
/// safe direction for a formatting slip.
fn ledger_bullet(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("- **")?.strip_prefix('`')?;
    let (raw, _) = rest.split_once('`')?;
    if !(raw.ends_with(".rs") || raw.ends_with(".qml")) {
        return None;
    }
    Some(raw.replace('\\', "/"))
}

/// That every entry still names a file: a split-away file's entry would
/// go on excusing a name.
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

/// Writes the baseline with LF endings, as .gitattributes pins the tree:
/// a CRLF rewrite would show as a diff on every Windows run.
fn write_baseline(path: &Path, entries: &BTreeMap<String, usize>) -> Result<(), String> {
    let mut text = String::from(BASELINE_HEADER);
    for (file, lines) in entries {
        text.push_str(&format!("{lines} {file}\n"));
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests;
