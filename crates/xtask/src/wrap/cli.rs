//! The command: which documents are read, and what is said and written.

use std::path::{Path, PathBuf};

use super::document::{Rewrapped, rewrap, unended};
use crate::command::{self, Permission, Where};

pub(crate) static CHECK: command::Command = command::Command {
    id: "wrap.check",
    call: "wrap",
    purpose: "the markdown, held to prose filled to 120 columns and a line end at each file's end",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static FIX: command::Command = command::Command {
    id: "wrap.fix",
    call: "wrap --fix",
    purpose: "fill the markdown's prose to 120 columns and end each file on a line end, touching \
              nothing else",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&CHECK, &FIX];

/// The width a line of prose is filled to, in columns.
const WIDTH: usize = 120;

/// How much of a document this leaves as it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Left {
    /// Every byte.
    Whole,
    /// Its lines: the file still ends on a line end.
    Lines,
}

/// Markdown this leaves, whole or line by line, and why: a path from the
/// root, or a directory's with its `/`.
const LEFT: &[(&str, Left, &str)] = &[
    (
        "vendor/",
        Left::Whole,
        "the patched-in crates, whose documents — their upstream's text and the notes on what \
         this tree changed in it — stay as they were written",
    ),
    (
        ".claude/rules-refs/",
        Left::Lines,
        "one item to a line, so that a Grep for its name returns the item whole (CLAUDE.md \
         規約の置き場所)",
    ),
    (
        ".claude/skills/verify-ui/verbs.md",
        Left::Lines,
        "one verb to a line, so that a Grep for the verb returns its entry whole (the \
         verify-ui skill)",
    ),
];

/// What this leaves of a document, and why, if it leaves anything; `name`
/// is the path from the root.
fn left(name: &str) -> Option<(Left, &'static str)> {
    LEFT.iter()
        .find(|(path, _, _)| {
            if path.ends_with('/') {
                name.starts_with(path)
            } else {
                name == *path
            }
        })
        .map(|(_, kind, why)| (*kind, *why))
}

/// The document as the command makes it, or the reason it is left whole.
fn made(name: &str, text: &str) -> Result<Rewrapped, &'static str> {
    let left = left(name);
    if let Some((Left::Whole, why)) = left {
        return Err(why);
    }
    if let Some(why) = unreadable(text) {
        return Err(why);
    }
    if left.is_some() {
        let ended = unended(text);
        return Ok(Rewrapped {
            text: if ended {
                format!("{text}\n")
            } else {
                text.to_string()
            },
            changed: Vec::new(),
            kept: Vec::new(),
            ended,
        });
    }
    Ok(rewrap(text, WIDTH))
}

/// What the post-write hook says about a markdown file an edit just
/// wrote: the paragraphs it left unfilled, and a missing line end at its
/// end. Nothing for a file outside the tree this runner serves — a
/// session's memory, a scratch file, another seat, whose name the command
/// would read against this tree — nor under `target/`, nor one this
/// leaves whole ([`LEFT`], [`unreadable`]).
pub(crate) fn note(path: &str) -> Option<String> {
    if !path.to_ascii_lowercase().ends_with(".md") {
        return None;
    }
    let file = Path::new(path);
    let tree = file
        .ancestors()
        .skip(1)
        .find(|dir| dir.join(".git").exists())?;
    if spelled(tree) != spelled(&crate::tree::workspace_root()) {
        return None;
    }
    let name = relative(tree, file);
    // The build's and the throwaway trees' folder (`crate::temp`), which
    // git ignores and the command does not list.
    if name.starts_with("target/") {
        return None;
    }
    let text = std::fs::read_to_string(file).ok()?;
    said(&name, &text)
}

/// The note on one document, `name` from its tree's root.
fn said(name: &str, text: &str) -> Option<String> {
    let done = made(name, text).ok()?;
    let mut wrong = Vec::new();
    if let Some(first) = done.changed.first() {
        wrong.push(format!(
            "{} paragraph(s) not filled to {WIDTH} columns, the first at line {first}",
            done.changed.len()
        ));
    }
    if done.ended {
        wrong.push("no line end at the end of the file".to_string());
    }
    if wrong.is_empty() {
        return None;
    }
    Some(format!(
        "{name}: {} — `{} {name}` puts it right and touches nothing else.",
        wrong.join(", and "),
        FIX.line()
    ))
}

/// Why a document is left as it stands as a whole: a carriage return, or
/// a tab in a line's indent, which the block reading counts no columns
/// for. A tab anywhere else keeps only its own paragraph (`fill`).
fn unreadable(text: &str) -> Option<&'static str> {
    if text.contains('\r') {
        return Some("a carriage return, which this reads no line ends for");
    }
    text.lines()
        .any(|line| line[..line.len() - line.trim_start().len()].contains('\t'))
        .then_some("a tab in a line's indent, which this counts no columns for")
}

/// A path as two spellings of it compare: forward slashes, and on
/// Windows one case.
fn spelled(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let mut fix = false;
    let mut named: Vec<PathBuf> = Vec::new();
    let root = crate::tree::workspace_root();
    for arg in args {
        match arg.as_str() {
            "--fix" => fix = true,
            other if other.starts_with("--") => {
                return Err(format!(
                    "unknown option {other:?} (wrap takes --fix, and the documents to read)"
                ));
            }
            other => named.push(root.join(other)),
        }
    }
    let files = if named.is_empty() {
        documents(&root)?
    } else {
        named
    };
    let mut unwrapped = 0;
    let mut paragraphs = 0;
    let mut unended_files = 0;
    let mut read = 0;
    for file in &files {
        let name = relative(&root, file);
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        read += 1;
        let done = match made(&name, &text) {
            Ok(done) => done,
            Err(why) => {
                println!("wrap: {name}: left as it stands — {why}");
                continue;
            }
        };
        if let Some((Left::Lines, why)) = left(&name) {
            println!("wrap: {name}: lines left as they stand — {why}");
        }
        for line in &done.kept {
            println!(
                "wrap: {name}:{line}: paragraph kept as it stands — a line that could end on a \
                 hard break, a tab, or a span running onto the next line that joining would \
                 change"
            );
        }
        if done.changed.is_empty() && !done.ended {
            continue;
        }
        unwrapped += usize::from(!done.changed.is_empty());
        paragraphs += done.changed.len();
        unended_files += usize::from(done.ended);
        if fix {
            std::fs::write(file, &done.text).map_err(|e| format!("{}: {e}", file.display()))?;
        }
        say_of(&name, &done, fix);
    }
    if fix || (unwrapped == 0 && unended_files == 0) {
        println!(
            "wrap: {read} markdown file(s) read, {paragraphs} paragraph(s) in {unwrapped} file(s) \
             filled, {unended_files} file(s) given their last line end{} — PASS",
            if fix { "" } else { " (none needed)" }
        );
        return Ok(());
    }
    Err(format!(
        "{paragraphs} paragraph(s) in {unwrapped} file(s) are not filled to {WIDTH} columns, and \
         {unended_files} file(s) do not end on a line end: `{}` puts them right and touches \
         nothing else",
        FIX.line()
    ))
}

/// What one document needed, said as done with `--fix` and as owed
/// without.
fn say_of(name: &str, done: &Rewrapped, fix: bool) {
    if !done.changed.is_empty() {
        if fix {
            println!("wrap: {name}: {} paragraph(s) filled", done.changed.len());
        } else {
            let shown: Vec<String> = done.changed.iter().take(8).map(usize::to_string).collect();
            let more = done.changed.len().saturating_sub(shown.len());
            println!(
                "wrap: {name}: {} paragraph(s) not filled to {WIDTH} columns, at line(s) {}{}",
                done.changed.len(),
                shown.join(", "),
                if more > 0 {
                    format!(" and {more} more")
                } else {
                    String::new()
                }
            );
        }
    }
    if done.ended {
        if fix {
            println!("wrap: {name}: line end put at the end of the file");
        } else {
            println!("wrap: {name}: no line end at the end of the file");
        }
    }
}

/// Every markdown file git tracks or would add, from the root.
fn documents(root: &Path) -> Result<Vec<PathBuf>, String> {
    let listed = crate::subprocess::git_query(
        &root.to_string_lossy(),
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            ":(icase)*.md",
        ],
    )
    .ok_or_else(|| format!("{}: git ls-files did not answer", root.display()))?;
    let mut files: Vec<PathBuf> = listed
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(|name| root.join(name))
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

fn relative(root: &Path, file: &Path) -> String {
    file.trim_prefix(root).to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{left, note, said, unreadable};

    #[test]
    fn the_documents_left_are_named_by_path_or_by_directory() {
        assert!(left("vendor/patched/README.md").is_some());
        assert!(left(".claude/rules-refs/app-ui.md").is_some());
        assert!(left(".claude/skills/verify-ui/verbs.md").is_some());
        assert!(left(".claude/skills/verify-ui/SKILL.md").is_none());
        assert!(left("internal-docs/vendor/x.md").is_none());
        assert!(left("CLAUDE.md").is_none());
    }

    #[test]
    fn the_note_names_the_first_paragraph_and_the_line_that_fills_it() {
        let long = "語".repeat(70);
        let text = format!("# 見出し\n\n短い段落\n\n{long}\n");
        let note = said("internal-docs/x.md", &text).expect("a note");
        assert!(
            note.starts_with("internal-docs/x.md: 1 paragraph(s)"),
            "{note}"
        );
        assert!(note.contains("the first at line 5"), "{note}");
        assert!(
            note.contains("`cargo xtask wrap --fix internal-docs/x.md`"),
            "{note}"
        );
        assert_eq!(said("internal-docs/x.md", "# 見出し\n\n短い段落\n"), None);
        assert_eq!(said(".claude/rules-refs/core.md", &text), None);
        assert_eq!(
            said("internal-docs/x.md", &format!("{text}\t- 字下げ\n")),
            None
        );
    }

    /// A file whose lines are left still ends on a line end; one left whole
    /// is not touched at all.
    #[test]
    fn every_document_but_one_left_whole_ends_on_a_line_end() {
        let note = said("internal-docs/x.md", "短い段落").expect("a note");
        assert!(
            note.contains("no line end at the end of the file"),
            "{note}"
        );
        let long = "語".repeat(70);
        let note = said(".claude/rules-refs/core.md", &format!("- {long}")).expect("a note");
        assert!(!note.contains("paragraph"), "its lines are left: {note}");
        assert!(note.contains("no line end"), "{note}");
        assert_eq!(said("vendor/x/README.md", "upstream"), None);
        let done = super::made(".claude/skills/verify-ui/verbs.md", "- `a` 長い").expect("made");
        assert_eq!(done.text, "- `a` 長い\n");
        assert!(super::made("vendor/x/README.md", "upstream").is_err());
        assert!(
            super::made(".claude/rules-refs/core.md", "- a\r\n- b").is_err(),
            "a file whose lines end CRLF gets no LF beside them"
        );
    }

    /// Another tree's document — a seat's, a scratch repository's — is named
    /// from its own root, which the command would read against this one.
    #[test]
    fn a_document_of_another_tree_gets_no_note() {
        let yard = crate::yard::Yard::new("wrap-note");
        let tree = yard.join("other");
        std::fs::create_dir_all(tree.join(".git")).expect("a tree of its own");
        let file = tree.join("notes.md");
        std::fs::write(&file, format!("{}\n", "語".repeat(70))).expect("a document");
        assert_eq!(note(&file.to_string_lossy().replace('\\', "/")), None);
    }

    #[test]
    fn a_document_under_target_or_no_markdown_gets_no_note() {
        let root = crate::tree::workspace_root();
        let under_target = root.join("target").join("wrap-note").join("x.md");
        assert_eq!(
            note(&under_target.to_string_lossy().replace('\\', "/")),
            None
        );
        let source = root
            .join("crates")
            .join("xtask")
            .join("src")
            .join("main.rs");
        assert_eq!(note(&source.to_string_lossy().replace('\\', "/")), None);
    }

    #[test]
    fn a_carriage_return_or_a_tab_in_an_indent_leaves_the_document_whole() {
        assert!(unreadable("a\r\nb\n").is_some());
        assert!(unreadable("-\tx\n\tcode\n").is_some());
        assert!(unreadable("a\tb\n").is_none());
        assert!(unreadable("```\nx\n```\n").is_none());
    }
}
