//! What a build of a file reads, apart from what its tests read: an item
//! under `#[cfg(test)]` — an inline test module, a helper, a `use` of its
//! own — is compiled into no binary but a test's, so what it alone names
//! is no input of a step that runs the binary (`Graph::built_inputs`).
//!
//! The item's end is read leaning short: a reading that stops early keeps
//! a read the build does not make (an input too many), and only one that
//! ran past the item could drop a read the build makes.

use std::collections::BTreeSet;
use std::path::Path;

use super::{Graph, Named, named_in, strip_comments};
use crate::lex::{Kind, Lang, lexemes};

/// The attribute that leaves its item out of every build but a test's,
/// spaces aside. Any other predicate naming `test` (`any(test, …)`) keeps
/// its item.
const TEST_ONLY: &str = "#[cfg(test)]";

/// What only `file`'s `#[cfg(test)]` items name, out of what the whole
/// file names (`named`); `None` where that is nothing.
pub(super) fn test_only(
    g: &Graph,
    root: &Path,
    file: &str,
    raw: &str,
    named: &Named,
) -> Option<BTreeSet<String>> {
    let built = without_test_items(raw);
    if built == raw {
        return None;
    }
    let kept = named_in(g, root, file, &built, &strip_comments(&built));
    let kept: BTreeSet<&String> = kept.all().collect();
    let only: BTreeSet<String> = named.all().filter(|t| !kept.contains(t)).cloned().collect();
    (!only.is_empty()).then_some(only)
}

/// `text` with every item under `#[cfg(test)]` blanked to spaces, the
/// attribute included and the line ends kept. Read by the language's
/// lexer (`crate::lex`), so an attribute or a bracket in a comment or a
/// literal counts for nothing; a file it cannot read whole is kept whole.
pub(super) fn without_test_items(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let (pieces, unread) = lexemes(&chars, Lang::Rust);
    if unread.is_some() {
        return text.to_string();
    }
    let mut code = vec![false; chars.len()];
    for piece in pieces.iter().filter(|p| p.kind == Kind::Code) {
        code[piece.start..piece.end].fill(true);
    }
    let mut blank = vec![false; chars.len()];
    let mut at = 0;
    while at < chars.len() {
        match attribute_end(&chars, &code, at) {
            Some(after) => {
                let end = item_end(&chars, &code, after);
                blank[at..end].fill(true);
                at = end;
            }
            None => at += 1,
        }
    }
    chars
        .iter()
        .zip(blank)
        .map(|(c, blanked)| if blanked && *c != '\n' { ' ' } else { *c })
        .collect()
}

/// Where the `#[cfg(test)]` starting at `at` ends, if one does: every
/// character of it code, whitespace between them allowed.
fn attribute_end(chars: &[char], code: &[bool], at: usize) -> Option<usize> {
    let mut wanted = TEST_ONLY.chars().peekable();
    let mut i = at;
    while let Some(next) = wanted.peek() {
        let c = *chars.get(i)?;
        if !code[i] {
            return None;
        }
        if c == *next {
            wanted.next();
        } else if !(c.is_whitespace() && i > at) {
            return None;
        }
        i += 1;
    }
    Some(i)
}

/// Where the item from `from` ends: past the first `;` or `,` at its own
/// depth, past the `}` that closes a block it opened there, or before the
/// bracket that closes the block it stands in — counted in code alone.
fn item_end(chars: &[char], code: &[bool], from: usize) -> usize {
    let mut depth = 0usize;
    for (i, c) in chars.iter().enumerate().skip(from) {
        if !code[i] {
            continue;
        }
        match c {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => {
                let Some(inner) = depth.checked_sub(1) else {
                    return i;
                };
                depth = inner;
                if depth == 0 && *c == '}' {
                    return i + 1;
                }
            }
            ';' | ',' if depth == 0 => return i + 1,
            _ => {}
        }
    }
    chars.len()
}

#[cfg(test)]
mod tests {
    use super::without_test_items;

    /// What a run of the binary reads: the code it compiles and the files
    /// that code opens, not what only its tests read, nor what a file it
    /// opens would compile with — while the closure a test selection keys
    /// on still holds both.
    #[test]
    fn a_built_closure_leaves_out_the_tests_reads_and_stops_at_what_it_opens() {
        let root = crate::yard::Yard::new("graph-built");
        let write = |path: &str, text: &str| {
            let full = root.join(path);
            std::fs::create_dir_all(full.parent().expect("a parent")).expect("a directory");
            std::fs::write(full, text).expect("a file");
        };
        let src = "crates/probe/src";
        write(
            &format!("{src}/main.rs"),
            "mod opened;\nmod run;\nmod shared;\nmod tested;\nfn main() {}\n",
        );
        write(
            &format!("{src}/run.rs"),
            "use crate::shared;\npub fn run() { shared::s(\"data/pin.txt\"); }\n\
             #[cfg(test)]\nmod tests {\n    use crate::tested;\n    \
             #[test]\n    fn t() { tested::t(\"data/fixture.txt\"); }\n}\n",
        );
        write(
            &format!("{src}/shared.rs"),
            "pub fn s(_: &str) { let _ = \"src/opened.rs\"; }\n",
        );
        write(
            &format!("{src}/opened.rs"),
            "use crate::tested;\npub fn o() { tested::t(\"\"); }\n",
        );
        write(&format!("{src}/tested.rs"), "pub fn t(_: &str) {}\n");
        write("crates/probe/data/pin.txt", "1\n");
        write("crates/probe/data/fixture.txt", "1\n");
        let g = super::super::build(&root).expect("the probe's graph");
        let run = vec![format!("{src}/run.rs")];
        let built = g.built_inputs(&run);
        for file in ["run.rs", "shared.rs", "opened.rs"] {
            assert!(
                built.contains(&format!("{src}/{file}")),
                "{file}: {built:?}"
            );
        }
        assert!(built.contains("crates/probe/data/pin.txt"), "{built:?}");
        for file in [
            format!("{src}/tested.rs"),
            "crates/probe/data/fixture.txt".into(),
        ] {
            assert!(!built.contains(&file), "{file}: {built:?}");
        }
        let tested = g.inputs(&run);
        for file in [
            format!("{src}/tested.rs"),
            "crates/probe/data/fixture.txt".into(),
        ] {
            assert!(tested.contains(&file), "{file}: {tested:?}");
        }
    }

    /// The code a build reads, as the build reads it: the items under the
    /// attribute go, with their strings and comments; the lines stay.
    #[test]
    fn a_test_only_item_is_blanked_and_the_rest_stands() {
        let text = "use crate::kept;\n#[cfg(test)]\nuse crate::gone;\n\
                    fn f() { kept::x(); }\n\
                    #[cfg(test)]\nmod tests {\n    use super::*;\n    \
                    fn t() { crate::gone::y(\"a/b.rs\"); }\n}\n\
                    fn g() -> u8 { 1 }\n";
        let built = without_test_items(text);
        assert_eq!(built.lines().count(), text.lines().count());
        for kept in [
            "use crate::kept;",
            "fn f() { kept::x(); }",
            "fn g() -> u8 { 1 }",
        ] {
            assert!(built.contains(kept), "{kept}: {built}");
        }
        for gone in ["gone", "tests", "a/b.rs", "cfg"] {
            assert!(!built.contains(gone), "{gone}: {built}");
        }
    }

    /// An item ends where it ends, whatever it is: a field or an arm
    /// before its comma, a statement at its `;`, the last thing in a block
    /// before the block's `}` — never past it into what the build reads.
    #[test]
    fn an_item_ends_at_its_own_end_and_not_past_it() {
        let field = "struct S {\n    #[cfg(test)]\n    a: Gone,\n    b: Kept,\n}\n";
        let built = without_test_items(field);
        assert!(
            built.contains("b: Kept,") && !built.contains("Gone"),
            "{built}"
        );
        let last = "fn f() {\n    #[cfg(test)]\n    gone()\n}\nfn kept() {}\n";
        let built = without_test_items(last);
        assert!(
            built.contains("}\nfn kept() {}") && !built.contains("gone"),
            "{built}"
        );
        let statement = "fn f() {\n    #[cfg(test)]\n    let x = [1; 2];\n    kept();\n}\n";
        let built = without_test_items(statement);
        assert!(
            built.contains("kept();") && !built.contains("let x"),
            "{built}"
        );
        let attributed =
            "#[cfg(test)]\n#[allow(dead_code)]\nfn gone(a: (u8, u8)) {}\nfn kept() {}\n";
        let built = without_test_items(attributed);
        assert!(
            built.contains("fn kept() {}") && !built.contains("gone"),
            "{built}"
        );
    }

    /// What is no `#[cfg(test)]` keeps its item: the attribute in a comment
    /// or a string, another predicate, and a brace in a literal does not
    /// end anything.
    #[test]
    fn only_the_attribute_itself_blanks_and_only_code_counts() {
        let kept = "// #[cfg(test)]\nfn a() {}\nconst S: &str = \"#[cfg(test)]\";\n\
                    #[cfg(not(test))]\nfn b() {}\n#[cfg(any(test, unix))]\nfn c() {}\n";
        assert_eq!(without_test_items(kept), kept);
        let spaced = "#[ cfg( test ) ]\nfn gone() { let _ = '}'; let _ = \"}\"; }\nfn kept() {}\n";
        let built = without_test_items(spaced);
        assert!(
            built.contains("fn kept() {}") && !built.contains("gone"),
            "{built}"
        );
    }
}
