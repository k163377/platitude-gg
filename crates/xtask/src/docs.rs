//! `cargo xtask docs` — the caps of the always-loaded documents, the
//! quotations held in `tokens` and `commands`, and here the ways an edit
//! tears a block off the list it belonged to ([`Kind`], each told in
//! `Kind::say`) in the documents read as rules ([`ROOTS`] and [`LOOSE`]).
//! A tear keeps every word in order, so review does not see it.
//!
//! The count reads structure only: line width and sentence shape are the
//! writer's.

mod commands;
mod tokens;

use std::path::{Path, PathBuf};

use crate::command::{self, Permission, Where};

pub(crate) static CHECK: command::Command = command::Command {
    id: "docs.check",
    call: "docs",
    purpose: "the rule documents' torn blocks, quoted values and command spans",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static SYNC: command::Command = command::Command {
    id: "docs.sync",
    call: "docs --sync",
    purpose: "write the generated cells and spans from the sources they quote",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&CHECK, &SYNC];

const ROOTS: [&str; 3] = ["internal-docs", ".claude/rules", ".claude/rules-refs"];
const LOOSE: [&str; 1] = ["CLAUDE.md"];

/// The caps of the always-loaded documents (CLAUDE.md
/// §規約の置き場所と本ファイルの運用).
pub(crate) const CLAUDE_CAP: usize = 12 * 1024;
pub(crate) const RULE_CAP: usize = 8 * 1024;

/// The cap a document carries, if any. Takes a hook payload's path
/// (absolute, forward slashes) or one relative to the root.
pub(crate) fn cap_of(path: &str) -> Option<usize> {
    if path == "CLAUDE.md" || path.ends_with("/CLAUDE.md") {
        Some(CLAUDE_CAP)
    } else if path.ends_with(".md")
        && (path.starts_with(".claude/rules/") || path.contains("/.claude/rules/"))
    {
        Some(RULE_CAP)
    } else {
        None
    }
}

pub(crate) fn oversize(path: &str, bytes: usize) -> Option<String> {
    let cap = cap_of(path)?;
    (bytes > cap).then(|| {
        format!(
            "OVER-CAP: {bytes} bytes against a cap of {cap} — this document is read again by \
             every call after it, so cut what it says: what a name can find (a type, a part, \
             a command, a function) goes to .claude/rules-refs, and only what applies whatever \
             file is touched stays (CLAUDE.md 規約の置き場所)"
        )
    })
}

pub fn run(args: &[String]) -> Result<(), String> {
    let mut sync = false;
    for arg in args {
        match arg.as_str() {
            "--sync" => sync = true,
            other => {
                return Err(format!(
                    "unknown option {other:?} (docs takes --sync, or nothing to read only)"
                ));
            }
        }
    }
    let root = crate::tree::workspace_root();
    let quoted = quote(&root, sync)?;
    let held = commands::hold(&root).and_then(|held| written(&root, held, sync))?;
    let mut files = Vec::new();
    for dir in ROOTS {
        under(&root.join(dir), "md", &mut files)?;
    }
    files.extend(LOOSE.iter().map(|name| root.join(name)));
    files.sort();

    let (torn, over) = read_each(&root, &files)?;

    for finding in torn.iter().chain(&over) {
        println!("docs: {finding}");
    }
    for finding in quoted.writable.iter().chain(&quoted.findings) {
        println!("docs: {}: {finding}", tokens::DOCUMENT);
    }
    for finding in held.writable.iter().chain(&held.findings) {
        println!("docs: {finding}");
    }

    let mut wrong: Vec<String> = Vec::new();
    if !torn.is_empty() {
        wrong.push(format!(
            "{} torn block(s): every word is still there and in the order it was written, which \
             is why these read as fine — so read each one where it stands and put it back into \
             the block it belongs to",
            torn.len()
        ));
    }
    if !over.is_empty() {
        wrong.push(format!(
            "{} always-loaded document(s) over cap: every byte of one is read again by every \
             call after it, so what a name can find goes to .claude/rules-refs and only what \
             applies whatever file is touched stays",
            over.len()
        ));
    }
    if !quoted.writable.is_empty() {
        wrong.push(format!(
            "{} value cell(s) that do not quote the source: the values live in Theme.qml and \
             Metrics.qml, and the document prints what they say — change the value there and run \
             `{}`",
            quoted.writable.len(),
            SYNC.line()
        ));
    }
    if !quoted.findings.is_empty() {
        wrong.push(format!(
            "{} token(s) the design document's tables name and neither Theme.qml nor Metrics.qml \
             declares — no sync writes these: fix the row, or the token",
            quoted.findings.len()
        ));
    }
    if !held.writable.is_empty() {
        wrong.push(format!(
            "{} generated command reference(s) that do not quote the catalogue: a command is \
             declared beside the code that runs it, and `{}` writes what quotes it — change \
             the declaration",
            held.writable.len(),
            SYNC.line()
        ));
    }
    if !held.findings.is_empty() {
        wrong.push(format!(
            "{} command reference(s) no sync can answer: a reference to an id nobody \
             declares, a line written out where a reference belongs, or a verb one list \
             carries and another does not",
            held.findings.len()
        ));
    }
    if wrong.is_empty() {
        println!(
            "docs: {} markdown files scanned, every block still in the list it belongs to, {} \
             value cell(s) quoting Theme.qml and Metrics.qml, {} command span(s) and {} \
             mention(s) quoting the catalogue — PASS",
            files.len(),
            quoted.cells,
            held.spans,
            held.mentions
        );
        Ok(())
    } else {
        Err(wrong.join("; and "))
    }
}

fn read_each(root: &Path, files: &[PathBuf]) -> Result<(Vec<String>, Vec<String>), String> {
    let mut torn: Vec<String> = Vec::new();
    let mut over: Vec<String> = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let relative = file
            .strip_prefix(root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        for found in breaks(&text) {
            torn.push(format!("{relative}:{}: {}", found.line, found.kind.say()));
        }
        if let Some(finding) = oversize(&relative, text.len()) {
            over.push(format!("{relative}: {finding}"));
        }
    }
    Ok((torn, over))
}

/// With `--sync`, write what the catalogue owns. What the write answers
/// is cleared; `findings`, which a person has to answer, are still said.
fn written(root: &Path, held: commands::Held, sync: bool) -> Result<commands::Held, String> {
    if !sync || held.rewritten.is_empty() {
        return Ok(held);
    }
    for (path, text) in &held.rewritten {
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        println!(
            "docs: {} rewritten — its command references now quote the catalogue",
            path.strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        );
    }
    Ok(commands::Held {
        writable: Vec::new(),
        rewritten: Vec::new(),
        ..held
    })
}

/// With `--sync`, write the value cells. What the write answers is
/// cleared; a token the source does not declare is still said.
fn quote(root: &Path, sync: bool) -> Result<tokens::Quoted, String> {
    let quoted = tokens::quote(root)?;
    if !sync {
        return Ok(quoted);
    }
    let Some(rewritten) = &quoted.rewritten else {
        return Ok(quoted);
    };
    let path = root.join(tokens::DOCUMENT);
    std::fs::write(&path, rewritten).map_err(|e| format!("{}: {e}", tokens::DOCUMENT))?;
    println!(
        "docs: {} rewritten — {} value cell(s) now quote the source",
        tokens::DOCUMENT,
        quoted.writable.len()
    );
    Ok(tokens::Quoted {
        writable: Vec::new(),
        rewritten: None,
        ..quoted
    })
}

/// The torn blocks of one document as sentences — what the post-write
/// hook says about the file an edit just landed in.
pub(crate) fn findings(text: &str) -> Vec<String> {
    breaks(text)
        .into_iter()
        .map(|found| format!("line {}: {}", found.line, found.kind.say()))
        .collect()
}

/// Whether a path is one of the documents this reads, spelled the way a
/// hook payload spells it — absolute, with forward slashes.
pub(crate) fn covers(path: &str) -> bool {
    path.ends_with(".md")
        && (ROOTS.iter().any(|root| path.contains(&format!("/{root}/")))
            || LOOSE.iter().any(|name| path.ends_with(&format!("/{name}"))))
}

/// One tear, at the line to fix — the break itself, where it and the
/// symptom differ.
struct Break {
    line: usize,
    kind: Kind,
}

enum Kind {
    OrphanIndent,
    HeadNoBlank,
    /// The line the table left hanging.
    TableInList {
        hanging: usize,
    },
}

impl Kind {
    fn say(&self) -> String {
        match self {
            Self::OrphanIndent => "ORPHAN-INDENT: this line is indented under a block that \
                 closed the list above it, so it hangs at a bullet's indent with no bullet — \
                 either wrap it into the paragraph it ends or give it back its bullet"
                .to_string(),
            Self::HeadNoBlank => "HEAD-NO-BLANK: a heading with no blank line over it. Every \
                 heading in this tree stands over one, so a heading that does not is the mark \
                 of an edit that spliced two blocks together — read what is above it before \
                 putting the blank line back"
                .to_string(),
            Self::TableInList { hanging } => format!(
                "TABLE-IN-LIST: this table is written at column 0 inside a list, so it ends \
                 the list and line {hanging} goes on at an indent that continues nothing — \
                 indent the table to the list's own text"
            ),
        }
    }
}

/// The torn blocks of one document.
///
/// The walk carries one question: is a list still open over an indented
/// line. A list opens on a bullet at any indent and only a column-0 block
/// closes it — a table or heading indented inside a list is part of it,
/// and letting one close the list would call every sound bullet under it
/// an orphan. A column-0 line with no blank above is a lazy continuation
/// and leaves the list open.
///
/// Fenced code and the YAML front matter of .claude/rules are skipped:
/// their indents are their own.
fn breaks(text: &str) -> Vec<Break> {
    let mut found = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut at = front_matter_end(&lines);

    let mut fenced = false;
    let mut list_open = false;
    // The column-0 table that closed a list, while nothing else has closed
    // one since: an orphan under it is a `TableInList`.
    let mut table_closer: Option<usize> = None;
    let mut in_table = false;
    // The start reads as a blank line: the first heading was spliced onto
    // nothing.
    let mut blank_above = true;
    // One finding per torn block, however many blank lines fall through
    // what hangs under it; cleared at column 0, where the next block begins.
    let mut orphaned = false;

    while at < lines.len() {
        let line = lines[at];
        let number = at + 1;
        at += 1;
        let bare = line.trim_start();

        if bare.starts_with("```") {
            fenced = !fenced;
            blank_above = false;
            continue;
        }
        if fenced {
            blank_above = false;
            continue;
        }
        if bare.is_empty() {
            blank_above = true;
            in_table = false;
            continue;
        }
        if line.starts_with([' ', '\t']) {
            if !list_open && !orphaned {
                orphaned = true;
                found.push(match table_closer {
                    Some(table) => Break {
                        line: table,
                        kind: Kind::TableInList { hanging: number },
                    },
                    None => Break {
                        line: number,
                        kind: Kind::OrphanIndent,
                    },
                });
            }
            blank_above = false;
            continue;
        }

        orphaned = false;
        if is_heading(bare) {
            if !blank_above {
                found.push(Break {
                    line: number,
                    kind: Kind::HeadNoBlank,
                });
            }
            list_open = false;
            table_closer = None;
            in_table = false;
        } else if bare.starts_with('|') {
            // Only the opening row answers for the table; the rows under it
            // find the list already shut.
            if !in_table {
                table_closer = list_open.then_some(number);
                in_table = true;
            }
            list_open = false;
        } else if is_bullet(bare) {
            list_open = true;
            table_closer = None;
            in_table = false;
        } else {
            in_table = false;
            if blank_above {
                list_open = false;
                table_closer = None;
            }
        }
        blank_above = false;
    }
    found
}

fn front_matter_end(lines: &[&str]) -> usize {
    if lines.first().map(|line| line.trim_end()) != Some("---") {
        return 0;
    }
    lines[1..]
        .iter()
        .position(|line| line.trim_end() == "---")
        .map_or(0, |close| close + 2)
}

/// An ATX heading: the hashes need a space after them, so a `#` that
/// opens a word is prose.
fn is_heading(bare: &str) -> bool {
    let rest = bare.trim_start_matches('#');
    let hashes = bare.len() - rest.len();
    (1..=6).contains(&hashes) && (rest.is_empty() || rest.starts_with(' '))
}

/// A list marker. It needs its space too, which keeps `---` and a
/// `**bold**` lead out.
fn is_bullet(bare: &str) -> bool {
    if let Some(rest) = bare.strip_prefix(['-', '*', '+']) {
        return rest.starts_with(' ');
    }
    let digits = bare.len() - bare.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    digits > 0 && bare[digits..].starts_with(['.', ')']) && bare[digits + 1..].starts_with(' ')
}

/// Every file under `dir` with that extension, however deep.
pub(crate) fn under(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.is_dir() {
            under(&path, extension, out)?;
        } else if path.extension().is_some_and(|ext| ext == extension) {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Kind, breaks, covers};

    fn kinds(text: &str) -> Vec<(usize, &'static str)> {
        breaks(text)
            .into_iter()
            .map(|found| {
                let name = match found.kind {
                    Kind::OrphanIndent => "orphan",
                    Kind::HeadNoBlank => "head",
                    Kind::TableInList { .. } => "table",
                };
                (found.line, name)
            })
            .collect()
    }

    /// A sentence whose head was glued onto the paragraph that ended the
    /// block above it and whose tail kept a bullet's indent.
    #[test]
    fn a_line_left_at_a_bullets_indent_under_a_paragraph_is_named() {
        let text = "\
- 印は 2 つ

**チップは枠の内側余白だけが 1 ギャップ**で、バッジが名前から
離れて立つ。重なったチップの一覧も同じ姿になり、hover の地色と
  クリックが外れる
- 突き合わせは ref join 1 本
";
        assert_eq!(kinds(text), vec![(5, "orphan")]);
    }

    /// A finding per line would bury the tear.
    #[test]
    fn a_run_of_orphaned_lines_is_one_finding() {
        let text = "段落。\n  ひとつ\n  ふたつ\n  みっつ\n";
        assert_eq!(kinds(text), vec![(2, "orphan")]);
    }

    /// A table named again for its second paragraph would read as two
    /// tables to move.
    #[test]
    fn a_blank_line_through_the_hanging_text_does_not_earn_a_second_finding() {
        let table = "\
- 印は状態で出入りする:

| 先端を | drop |
|---|---|
| このブランチだけ | 長押し |

  **見るのは先端であってその行ではない。**

  範囲に merge は無い。
";
        assert_eq!(kinds(table), vec![(3, "table")]);
        assert_eq!(kinds("段落。\n  ひとつ\n\n  ふたつ\n"), vec![(2, "orphan")]);
    }

    /// What separates two tears is a column-0 block, not a blank line.
    #[test]
    fn two_blocks_that_each_tore_something_are_two_findings() {
        let text = "段落ひとつ。\n  字下げ\n段落ふたつ。\n\n  字下げ\n";
        assert_eq!(kinds(text), vec![(2, "orphan"), (5, "orphan")]);
    }

    #[test]
    fn a_heading_with_no_blank_line_over_it_is_named() {
        let text = "\
# 見出し

- 箇条
  続き
### 行が読む答えはどこから来るか

本文。
";
        assert_eq!(kinds(text), vec![(5, "head")]);
    }

    #[test]
    fn the_first_line_and_a_hash_inside_a_word_are_left_alone() {
        assert!(kinds("# 見出し\n\n本文\n").is_empty());
        assert!(kinds("本文。\n#5 は番号であって見出しではない\n").is_empty());
    }

    /// The finding is at the table, which is what to move.
    #[test]
    fn a_column_zero_table_that_leaves_the_list_hanging_is_named() {
        let text = "\
- 印は状態で出入りする:

| 先端を | drop | 理由 |
|---|---|---|
| このブランチだけ | 長押し | reflog しか届かない |

  **見るのは先端であってその行ではない。**
";
        assert_eq!(kinds(text), vec![(3, "table")]);
        let Kind::TableInList { hanging } = breaks(text).remove(0).kind else {
            panic!("the finding names the line the table left hanging");
        };
        assert_eq!(hanging, 7);
    }

    /// How this tree usually writes a table under a bullet.
    #[test]
    fn a_table_under_a_bullet_with_nothing_hanging_after_it_passes() {
        let text = "\
- 層は 3 つ:

| 層 | 何が入るか |
|---|---|
| 基礎 | 色・字 |

次の段落。

- 次の箇条
  その続き
";
        assert!(kinds(text).is_empty(), "{:?}", kinds(text));
    }

    /// The asymmetry `breaks` rests on.
    #[test]
    fn an_indented_table_or_heading_inside_a_list_closes_nothing() {
        let text = "\
- 層は 3 つ:

  | 層 | 何が入るか |
  |---|---|
  | 基礎 | 色・字 |

  #### 個別の層
  その続き
- 次の箇条
  その続き
";
        assert!(kinds(text).is_empty(), "{:?}", kinds(text));
    }

    #[test]
    fn fenced_code_is_read_as_none_of_this() {
        let text = "\
段落。

```
paths:
  - \"crates/**\"
# not a heading
```

段落。
";
        assert!(kinds(text).is_empty(), "{:?}", kinds(text));
    }

    #[test]
    fn yaml_front_matter_is_skipped_and_the_document_starts_after_it() {
        let text = "\
---
paths:
  - \"crates/platitude-app/**\"
---

# platitude-app 規約

本文。
";
        assert!(kinds(text).is_empty(), "{:?}", kinds(text));
    }

    #[test]
    fn a_rule_and_a_bolded_lead_do_not_open_a_list() {
        let text = "---\n\n**強調で始まる段落**が続く。\n  字下げ\n";
        assert_eq!(kinds(text), vec![(4, "orphan")]);
    }

    #[test]
    fn a_numbered_item_opens_a_list() {
        assert!(kinds("1. まず書く\n   その続き\n2. 次\n").is_empty());
    }

    #[test]
    fn the_documents_read_are_the_ones_named() {
        assert!(covers("C:/p/internal-docs/デザイン規約.md"));
        assert!(covers("/p/.claude/rules/core.md"));
        assert!(covers("/p/.claude/rules-refs/app-ui.md"));
        assert!(covers("/p/CLAUDE.md"));
        assert!(!covers("/p/ci/baseline/perf-windows-x64.md"));
        assert!(!covers("/p/internal-docs/notes.txt"));
    }

    /// In both spellings: a hook's absolute path and the check's relative one.
    #[test]
    fn the_caps_sit_on_the_always_loaded_documents() {
        use super::{CLAUDE_CAP, RULE_CAP, cap_of, oversize};
        assert_eq!(cap_of("C:/p/CLAUDE.md"), Some(CLAUDE_CAP));
        assert_eq!(cap_of("CLAUDE.md"), Some(CLAUDE_CAP));
        assert_eq!(cap_of("/p/.claude/rules/app-ui.md"), Some(RULE_CAP));
        assert_eq!(cap_of(".claude/rules/core.md"), Some(RULE_CAP));
        assert_eq!(cap_of("/p/.claude/rules-refs/app-ui.md"), None);
        assert_eq!(cap_of(".claude/rules-refs/core.md"), None);
        assert_eq!(cap_of("internal-docs/デザイン規約.md"), None);
        assert_eq!(cap_of("/p/.claude/rules/notes.txt"), None);
        assert!(oversize(".claude/rules/core.md", RULE_CAP).is_none());
        let over = oversize(".claude/rules/core.md", RULE_CAP + 1).unwrap();
        assert!(over.starts_with("OVER-CAP: "), "{over}");
        assert!(over.contains("rules-refs"), "{over}");
        assert!(oversize("internal-docs/x.md", usize::MAX).is_none());
    }
}
