//! `cargo xtask docs` — the three ways an edit to this tree's markdown has
//! torn a block off the list it belonged to without saying so.
//!
//! All three survive review because nothing about them looks wrong in the
//! source: the words are all still there, in the order they were written,
//! and what changed is which block each line belongs to — which two of
//! them show only once rendered, and the third only to a reader who knows
//! how this tree writes a heading. What that costs is a sentence that no
//! longer reads and a rule nobody applies, so the count is over the
//! documents that are read as rules ([`ROOTS`] and [`LOOSE`]).
//!
//! Each is a shape this file's own history has actually taken:
//!
//! * [`Kind::OrphanIndent`] — an indented line under a block that closed
//!   the list above it, so it hangs at a bullet's indent with no bullet.
//!   One clause of デザイン規約 §チップ lived like this through three
//!   rewrites: each edit glued its head onto the paragraph that ended the
//!   block above and left its tail on a line of its own.
//! * [`Kind::HeadNoBlank`] — a heading with no blank line over it. Every
//!   heading here stands over one, so a heading that does not was made by
//!   an edit that spliced two blocks together — here, the same rewrite
//!   that left the clause above hanging.
//! * [`Kind::TableInList`] — a table written at column 0 inside a list.
//!   The table ends the list, so the list's own text after it goes on at
//!   an indent that continues nothing.
//!
//! The count reads structure only: how wide a line runs and
//! how a sentence is built are the writer's, and a machine that had an
//! opinion on either would be answering a question nobody asked it.

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

/// The trees whose markdown is read, and the one file outside them.
const ROOTS: [&str; 3] = ["internal-docs", ".claude/rules", ".claude/rules-refs"];
const LOOSE: [&str; 1] = ["CLAUDE.md"];

/// The bytes an always-loaded rule document may hold. CLAUDE.md is in
/// the context of every call a session makes, and a file under
/// .claude/rules in every call after its crate is first touched, so a
/// byte in either is paid again on every call that follows — which is
/// what makes a rule here dearer than the same rule anywhere else. What
/// a name can find (a type, a part, a command, a function) belongs in
/// .claude/rules-refs, which nothing loads unasked.
pub(crate) const CLAUDE_CAP: usize = 12 * 1024;
pub(crate) const RULE_CAP: usize = 8 * 1024;

/// The cap a document carries, when it carries one: CLAUDE.md and the
/// files directly under .claude/rules. Answers for a hook payload's
/// path (absolute, forward slashes) and for one relative to the root.
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

/// The finding for a capped document that has grown past its cap.
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
    for finding in &quoted.findings {
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
    if !quoted.findings.is_empty() {
        wrong.push(format!(
            "{} value cell(s) that do not quote the source: the values live in Theme.qml and \
             Metrics.qml, and the document prints what they say — change the value there and run \
             `{}`",
            quoted.findings.len(),
            SYNC.line()
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

/// Every document read once: its torn blocks, and its size against the
/// cap it carries, each finding named by the path relative to the root.
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

/// Write what the catalogue owns, when asked to.
///
/// The same order the value cells are held in: `--sync` writes, and the
/// findings are still said, so that drift reaches the person who has to
/// decide whether the declaration was the side that was wrong.
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

/// Hold the design document's value cells to the sources, writing them
/// when asked to.
///
/// The write happens ahead of the findings, which are still said:
/// `--sync` is how a value cell is corrected, and a run that
/// corrects one still says which cell it was, so that the drift reaches
/// the person who has to decide whether the source was the side that was
/// wrong.
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
        quoted.findings.len()
    );
    Ok(tokens::Quoted {
        findings: Vec::new(),
        rewritten: None,
        ..quoted
    })
}

/// The torn blocks of one document, as the sentences a reader is given.
/// The Write hook says the same thing about the one file an edit just
/// landed in, so a tear is answered in the turn that made it, ahead
/// of the next gate.
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

/// One place a block was torn off what it belonged to, at the line whose
/// reader is meant to look — the break itself, where it and the
/// symptom differ.
struct Break {
    line: usize,
    kind: Kind,
}

enum Kind {
    OrphanIndent,
    HeadNoBlank,
    /// The line the table left hanging, which is what shows the tear.
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
/// The walk carries one question: may an indented line be here — is a
/// list still open over it. A list opens on a bullet at any indent, and
/// only a block at **column 0** closes it. That asymmetry is the whole
/// of the accuracy: a table or a heading indented inside a list is part
/// of the list and closes nothing, and a count that let one close the
/// list would call every sound bullet under it an orphan.
///
/// What closes a list, then: a column-0 heading, a column-0 table row,
/// and a column-0 paragraph that starts its own block (a blank line
/// above it). A column-0 line with no blank above it is a lazy
/// continuation of the item it follows and leaves the list open — the
/// conservative reading, and the one that answers to what this tree
/// writes.
///
/// Fenced code is skipped whole: what is inside a fence is a sample of
/// something else's syntax, and its indents are its own. So is the YAML
/// front matter of .claude/rules, whose keys carry indented values.
fn breaks(text: &str) -> Vec<Break> {
    let mut found = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut at = front_matter_end(&lines);

    let mut fenced = false;
    let mut list_open = false;
    // The column-0 table that closed a list and has not been followed by
    // anything else that closes one: the finding an orphan under it earns.
    let mut table_closer: Option<usize> = None;
    let mut in_table = false;
    // The start of a document reads as a blank line: the first heading
    // has nothing above it to have been spliced onto.
    let mut blank_above = true;
    // One finding per closed region: everything hanging under one torn
    // block is one tear, however many blank lines fall through it.
    // Cleared at column 0, where the next block begins and the next
    // tear would be a different one.
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
            // Only the row that opens the table answers for it; the rows
            // under it are the same block and find the list already shut.
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

/// Where the document starts: past the YAML front matter when there is
/// one, and at the top when there is not.
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

/// A list marker. The marker needs its space too, which is what keeps
/// `---` and a `**bold**` lead out of the set.
fn is_bullet(bare: &str) -> bool {
    if let Some(rest) = bare.strip_prefix(['-', '*', '+']) {
        return rest.starts_with(' ');
    }
    let digits = bare.len() - bare.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    digits > 0 && bare[digits..].starts_with(['.', ')']) && bare[digits + 1..].starts_with(' ')
}

/// Every file under `dir` with that extension, however deep. Both halves
/// of this check walk a tree for one kind of file, and the command half
/// walks two.
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

    /// The shape デザイン規約 §チップ carried through three rewrites: a
    /// sentence whose head was glued onto the paragraph that ended the
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

    /// Only the first line of a run is named: a run of them is one tear,
    /// and a finding per line would bury it.
    #[test]
    fn a_run_of_orphaned_lines_is_one_finding() {
        let text = "段落。\n  ひとつ\n  ふたつ\n  みっつ\n";
        assert_eq!(kinds(text), vec![(2, "orphan")]);
    }

    /// And a blank line inside what one torn block left hanging is still
    /// that one block: a table named once and then again for its second
    /// paragraph would read as two tables to move.
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

    /// A second tear is a second finding: what separates them is a block
    /// at column 0 (blank lines inside one split nothing).
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

    /// The first heading has nothing above it to be swallowed by, and a
    /// `#` that opens a word is prose.
    #[test]
    fn the_first_line_and_a_hash_inside_a_word_are_left_alone() {
        assert!(kinds("# 見出し\n\n本文\n").is_empty());
        assert!(kinds("本文。\n#5 は番号であって見出しではない\n").is_empty());
    }

    /// A table written at column 0 inside a list ends the list, so what
    /// the list meant to go on saying hangs at an indent that continues
    /// nothing. The finding is at the table, which is what to move.
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

    /// A bullet with a table under it and nothing indented after is how
    /// this tree writes a table, and reads a dozen times over.
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

    /// The asymmetry the count rests on: a list is closed only by a block
    /// at column 0. A table or a heading indented inside a list is part
    /// of the list, and closing on one would call every bullet under it
    /// an orphan.
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

    /// What is inside a fence is a sample of something else's syntax, and
    /// its indents and hashes are its own.
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

    /// The front matter of .claude/rules carries indented values under a
    /// key, which is a list to YAML and an orphan to markdown.
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

    /// A marker needs its space, which is what keeps a thematic break and
    /// a bolded lead — how most paragraphs here open — out of the set.
    #[test]
    fn a_rule_and_a_bolded_lead_do_not_open_a_list() {
        let text = "---\n\n**強調で始まる段落**が続く。\n  字下げ\n";
        assert_eq!(kinds(text), vec![(4, "orphan")]);
    }

    /// A numbered list opens one the same way a bullet does.
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

    /// The caps sit on the documents every call loads — CLAUDE.md and
    /// the rules — and on nothing a session reads by choice, spelled as
    /// a hook's absolute path and as the check's relative one alike.
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
