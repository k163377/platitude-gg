//! Where the tooling may stand a tree under the throwaway folder
//! (`crate::temp::root`), counted by machine: in a base the day-old sweep
//! reads (`verify::sweep_yesterdays_runs`) — a test's tree in a
//! `crate::yard::Yard` — or in one of the few trees bounded some other way
//! ([`OUTSIDE_THE_SWEEP`]). A tree anywhere else stands for good once what
//! made it is killed, or once a test panics short of its own removal.
//!
//! Read off the spelling, where each path is decided. Counted here, on
//! every gate, and not by a unit test: a test is chosen by what reads the
//! changed file (`gate::graph`), and nothing reads the file a new spelling
//! lands in — a file's literal naming its own tree is no edge.

use std::path::Path;

/// The crate whose Rust this covers.
const XTASK: &str = "crates/xtask/src";

/// What stands under the folder outside every base the sweep reads, each
/// bounded some other way: the file spelling it, from inside [`XTASK`],
/// and the name it joins — `None` for the folder itself. A row nothing
/// spells any more is a failure: an excuse outliving its tree would
/// excuse the next.
const OUTSIDE_THE_SWEEP: [(&str, Option<&str>); 5] = [
    // The sweep itself, reading every base.
    ("verify/ownership.rs", None),
    // Asks whether a path is in the folder, and makes nothing.
    ("hook/seat.rs", None),
    // Held to the newest few, and a gone taker's listing taken
    // (`forget_all_but_the_newest`, `forget_what_gone_takers_left`).
    ("linux/witness.rs", Some("pgg-engine")),
    // One name, emptied as each run starts.
    ("relied_messages/master.rs", Some("relied-messages-master")),
    // The container's cache: one name, in the container's own `/tmp`.
    ("relied_messages/sources.rs", Some("relied-messages")),
];

/// The folder as code reaches it — made, and only placed — split so that
/// this file is not one of the spellings it finds.
const FOLDER: [&str; 2] = [concat!("temp::", "root()"), concat!("temp::", "place()")];

/// What one spelling of the folder joins to it first.
#[derive(Debug, PartialEq)]
enum Joined {
    /// Nothing: the folder is used as it is.
    Folder,
    /// A name: a literal, or a `&str` constant of the same file.
    Name(String),
    /// Anything else, as written — a name this count cannot read.
    Unread(String),
}

/// One failure per spelling standing a tree outside every base the sweep
/// reads, or naming it so it cannot be read, and per excuse nothing
/// spells; and how many files were read.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let xtask = root.join(XTASK);
    let mut files = Vec::new();
    super::collect(&xtask, &mut files)?;
    files.sort();
    files.retain(|path| path.extension().is_some_and(|e| e == "rs"));
    let mut failures = Vec::new();
    let mut excused = vec![false; OUTSIDE_THE_SWEEP.len()];
    for path in &files {
        let shown = super::relative(&xtask, path);
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (line, joined) in under_the_folder(&text) {
            let named = match &joined {
                Joined::Folder => None,
                Joined::Name(name) if crate::verify::RUN_BASES.contains(&name.as_str()) => {
                    continue;
                }
                Joined::Name(name) => Some(name.as_str()),
                Joined::Unread(written) => {
                    failures.push(format!(
                        "{XTASK}/{shown}:{line}: joins `{written}` to the throwaway folder, which \
                         this count cannot read as a name — spell the base as a literal or as a \
                         `&str` constant of this file, so it can be held to the bases a sweep \
                         reads"
                    ));
                    continue;
                }
            };
            match OUTSIDE_THE_SWEEP
                .iter()
                .position(|&(file, base)| file == shown && base == named)
            {
                Some(row) => excused[row] = true,
                None => failures.push(format!(
                    "{XTASK}/{shown}:{line}: stands a tree in {} under the throwaway folder, which \
                     no sweep reads — what a killed run or a panicking test leaves there stays for \
                     good. A test's tree goes in `crate::yard::Yard`; a run's, in a base \
                     `verify::ownership::RUN_BASES` names",
                    shown_as(named)
                )),
            }
        }
    }
    for (&(file, base), _) in OUTSIDE_THE_SWEEP
        .iter()
        .zip(&excused)
        .filter(|(_, seen)| !**seen)
    {
        failures.push(format!(
            "{XTASK}/{file}: excused for {} under the throwaway folder, and spells it no more — \
             take the row out of `structure::throwaway::OUTSIDE_THE_SWEEP`",
            shown_as(base)
        ));
    }
    Ok((failures, files.len()))
}

fn shown_as(base: Option<&str>) -> String {
    base.map_or_else(
        || "the folder itself".to_string(),
        |name| format!("`{name}`"),
    )
}

/// Each spelling of the folder in `text`, by line, and what it joins
/// first. None on a comment line, where a path is a reference.
fn under_the_folder(text: &str) -> Vec<(usize, Joined)> {
    let mut found: Vec<(usize, usize, Joined)> = FOLDER
        .iter()
        .flat_map(|folder| {
            text.match_indices(folder).filter_map(|(at, _)| {
                let start = text[..at].rfind('\n').map_or(0, |end| end + 1);
                if text[start..at].trim_start().starts_with("//") {
                    return None;
                }
                let line = text[..at].matches('\n').count() + 1;
                Some((at, line, joined(text, &text[at + folder.len()..])))
            })
        })
        .collect();
    found.sort_by_key(|&(at, _, _)| at);
    found
        .into_iter()
        .map(|(_, line, joined)| (line, joined))
        .collect()
}

/// What `after` — the text past one spelling of the folder in `text` —
/// joins to it first ([`under_the_folder`]).
fn joined(text: &str, after: &str) -> Joined {
    let Some(argument) = after.trim_start().strip_prefix(".join(") else {
        return Joined::Folder;
    };
    let argument = argument.trim_start();
    if let Some(literal) = argument.strip_prefix('"') {
        return Joined::Name(literal.split('"').next().unwrap_or_default().to_string());
    }
    // An inline `mod tests` reaches the file's own constant through
    // `super::`; in a file of its own the constant is not here, and the
    // name goes unread.
    let local = argument
        .strip_prefix("super::")
        .or_else(|| argument.strip_prefix("self::"))
        .unwrap_or(argument);
    let constant: String = local
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let value = text
        .split_once(&format!("const {constant}: &str = \""))
        .and_then(|(_, rest)| rest.split('"').next())
        .filter(|_| !constant.is_empty() && local[constant.len()..].starts_with(')'));
    match value {
        Some(value) => Joined::Name(value.to_string()),
        None => Joined::Unread(argument.lines().next().unwrap_or_default().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{FOLDER, Joined, check, under_the_folder};

    /// Each way code joins a name to the folder — a literal, a chain
    /// broken over lines, a constant of the file's own (from its tests
    /// too), after a URL on the same line — the uses that join nothing,
    /// and the names this count cannot read: another module's constant, a
    /// name made at run time.
    #[test]
    fn a_spelling_is_read_for_the_name_it_joins_and_a_comment_is_a_reference() {
        let text = "const BASE: &str = \"pgg-kept\";\n\
                    let a = crate::ROOT.join(\"pgg-verify\");\n\
                    let b = crate::ROOT\n        .join(\"pgg-split\")\n        .join(\"x\");\n\
                    let c = crate::ROOT.join(BASE);\n\
                    let d = crate::ROOT.join(crate::yard::BASE);\n\
                    let e = crate::ROOT.join(format!(\"note-{}\", pid));\n\
                    let f = claim_dir(crate::ROOT, \"flat\");\n\
                    /// See `crate::ROOT.join(\"pgg-doc\")`.\n\
                    let g = (\"https://x\", crate::ROOT.join(\"pgg-after-a-url\"));\n\
                    let h = crate::PLACE.to_string_lossy();\n\
                    let i = crate::ROOT.join(super::BASE);\n"
            .replace("ROOT", FOLDER[0])
            .replace("PLACE", FOLDER[1]);
        let name = |name: &str| Joined::Name(name.to_string());
        let unread = |written: &str| Joined::Unread(written.to_string());
        assert_eq!(
            under_the_folder(&text),
            [
                (2, name("pgg-verify")),
                (3, name("pgg-split")),
                (6, name("pgg-kept")),
                (7, unread("crate::yard::BASE);")),
                (8, unread("format!(\"note-{}\", pid));")),
                (9, Joined::Folder),
                (11, name("pgg-after-a-url")),
                (12, Joined::Folder),
                (13, name("pgg-kept")),
            ]
        );
    }

    /// The rule the count is for: the tree it runs on passes it.
    #[test]
    fn every_tree_the_tooling_stands_under_the_folder_is_one_a_sweep_reads() {
        let (failures, files) = check(&crate::tree::workspace_root()).expect("scan xtask");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(files > 1, "xtask has more than one .rs file");
    }
}
