//! Which of the core's functions match git's words against a literal of
//! their own, counted by machine (.claude/rules-refs/core.md「書き込みの
//! 失敗の分類を足す・変える前に」): a message the product reads is written
//! in the table and named, so `cargo xtask relied-messages` holds every one of
//! them to git's releases. One written beside its classifier is held to
//! nothing.
//!
//! A function reads git's words where it names `said`, `stderr`,
//! `stderr_utf8` or `failure_message`; in one, a string literal — or a
//! `format!` of one — handed to a matching method, or a literal written as
//! a match arm's pattern, is a message out of the table. The
//! machine formats asked for are parsed where git's words are not named,
//! and the tests — each `#[cfg(test)]` item — are passed over. A literal that is no word of git's in a
//! function that also reads them — a machine format's answer, a mark of
//! the product's own — says so on its line or in a comment right above the
//! lines it covers: `// not git's words: <what it is>`.

use std::path::Path;

/// The crate whose Rust this covers.
const CORE: &str = "crates/platitude-core/src";
/// The table, as a path under [`CORE`].
const TABLE: &str = "relied_messages.rs";

/// What names git's words in a function.
const READS_GITS_WORDS: [&str; 4] = ["said", "stderr", "stderr_utf8", "failure_message"];

/// The methods a message is matched with.
const MATCHING: [&str; 9] = [
    "contains",
    "starts_with",
    "ends_with",
    "strip_prefix",
    "strip_suffix",
    "split_once",
    "rsplit_once",
    "find",
    "rfind",
];

/// One failure per literal matched against git's words outside the table,
/// and how many functions read git's words.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let core = root.join(CORE);
    let mut files = Vec::new();
    super::collect(&core, &mut files)?;
    files.sort();
    files.retain(|path| path.extension().is_some_and(|e| e == "rs"));
    let mut failures = Vec::new();
    let mut readers = 0;
    for path in &files {
        let shown = super::relative(&core, path);
        if shown == TABLE || is_test_file(&shown) {
            continue;
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (lines, read) = strays(&text);
        readers += read;
        failures.extend(lines.into_iter().map(|number| {
            format!(
                "{CORE}/{shown}:{number}: matches git's words against a literal of its own — \
                 write them as a form in {CORE}/{TABLE} (a `Words` with its `- git:` source \
                 and `- witness:`, in a `Relied` beside the others that say the same) and name \
                 it, so `{}` holds it to git's releases \
                 (internal-docs/gitの扱い.md §git の文言に依存する判定), or mark a literal that is \
                 no word of git's `{}<what it is>`",
                crate::relied_messages::AUDIT.line(),
                NOT_GITS
            )
        }));
    }
    Ok((failures, readers))
}

/// One file's lines that match git's words against a literal, and how many
/// of its functions read git's words.
fn strays(text: &str) -> (Vec<usize>, usize) {
    let held = literal_consts(text);
    let mut lines = Vec::new();
    let mut readers = 0;
    for function in functions(text) {
        if !function
            .lines
            .iter()
            .any(|(_, code, _)| names_gits_words(code))
        {
            continue;
        }
        readers += 1;
        let mut said_not = false;
        for (number, code, raw) in &function.lines {
            if code.trim().is_empty() {
                said_not = says_not_gits(raw);
                continue;
            }
            let flagged =
                matches_a_literal(code) || held.iter().any(|name| matches_with(code, name));
            let excused = said_not || says_not_gits(raw);
            said_not &= flagged;
            if flagged && !excused {
                lines.push(*number);
            }
        }
    }
    (lines, readers)
}

fn is_test_file(shown: &str) -> bool {
    let name = shown.rsplit('/').next().unwrap_or(shown);
    name.ends_with("_tests.rs") || name == "tests.rs" || name == "testkit.rs"
}

/// A function's lines: its number, the code (comments and the inside of
/// literals blanked, the quotes kept) and the line as written.
struct Function {
    lines: Vec<(usize, String, String)>,
}

/// The mark a literal that is no word of git's carries.
const NOT_GITS: &str = "// not git's words: ";

fn says_not_gits(raw: &str) -> bool {
    raw.split_once(NOT_GITS)
        .is_some_and(|(_, why)| !why.trim().is_empty())
}

/// The file's functions outside its test items, an inner one inside the
/// one around it.
fn functions(text: &str) -> Vec<Function> {
    let mut found = Vec::new();
    let mut current: Option<(Function, i64, bool)> = None;
    for (number, line) in outside_tests(text) {
        let code = code_of(line);
        if current.is_none() && starts_a_function(&code) {
            current = Some((Function { lines: Vec::new() }, 0, false));
        }
        let Some((function, depth, opened)) = current.as_mut() else {
            continue;
        };
        function
            .lines
            .push((number, code.clone(), line.to_string()));
        let opening = code.matches('{').count() as i64;
        *depth += opening - code.matches('}').count() as i64;
        *opened |= opening > 0;
        // A trait's declaration has no body to read.
        if !*opened && code.trim_end().ends_with(';') {
            current = None;
            continue;
        }
        if *opened
            && *depth <= 0
            && let Some((function, _, _)) = current.take()
        {
            found.push(function);
        }
    }
    found
}

/// The file's lines, numbered, without its `#[cfg(test)]` items: the item
/// the attribute is on — a `mod tests;` or a block to its closing brace —
/// and nothing after it, so the code below a test module declared at the
/// top is still read.
fn outside_tests(text: &str) -> Vec<(usize, &str)> {
    let mut kept = Vec::new();
    // The depth of the test item being passed over, and whether it opened.
    let mut passing: Option<(i64, bool)> = None;
    for (index, line) in text.lines().enumerate() {
        let mut rest = line;
        if passing.is_none() {
            let Some(after) = line.trim_start().strip_prefix("#[cfg(test)]") else {
                kept.push((index + 1, line));
                continue;
            };
            rest = after;
        }
        let (depth, opened) = passing.get_or_insert((0, false));
        let code = code_of(rest);
        let item = without_attributes(&code);
        let opening = item.matches('{').count() as i64;
        *depth += opening - item.matches('}').count() as i64;
        *opened |= opening > 0;
        if (!*opened && item.trim_end().ends_with(';')) || (*opened && *depth <= 0) {
            passing = None;
        }
    }
    kept
}

/// `code` with the attributes it starts with cut off.
fn without_attributes(code: &str) -> &str {
    let mut rest = code.trim_start();
    while let Some(attribute) = rest.strip_prefix("#[") {
        let Some((_, after)) = attribute.split_once(']') else {
            return "";
        };
        rest = after.trim_start();
    }
    rest
}

fn starts_a_function(code: &str) -> bool {
    let Some((before, _)) = code.trim_start().split_once("fn ") else {
        return false;
    };
    before.split_whitespace().all(|word| {
        matches!(word, "pub" | "async" | "const" | "unsafe")
            || (word.starts_with("pub(") && word.ends_with(')'))
    })
}

/// The line with its comment cut off and every literal's inside blanked,
/// so braces and words in a string are not read as code — the literals
/// themselves are read off the line as written.
fn code_of(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => break,
            '"' => {
                out.push('"');
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => drop(chars.next()),
                        '"' => break,
                        _ => out.push('_'),
                    }
                }
                out.push('"');
            }
            '\'' if is_char_literal(&mut chars.clone()) => {
                out.push_str("' '");
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => drop(chars.next()),
                        '\'' => break,
                        _ => {}
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Whether what follows a `'` is a character (`'x'`, `'\''`) and not a
/// lifetime (`'a`).
fn is_char_literal(rest: &mut std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    match rest.next() {
        Some('\\') => true,
        Some(_) => rest.next() == Some('\''),
        None => false,
    }
}

fn names_gits_words(code: &str) -> bool {
    READS_GITS_WORDS.iter().any(|name| names_word(code, name))
}

/// Whether `code` matches against the const `name`: hands it to a matching
/// method, iterates it, makes it a match arm or the words of a refusal
/// (`except_when_it_says`) — not where it is an argument, a key, a tag.
fn matches_with(code: &str, name: &str) -> bool {
    let handed = MATCHING.iter().any(|method| {
        code.contains(&format!(".{method}({name}")) || code.contains(&format!(".{method}(&{name}"))
    });
    let trimmed = code.trim_start();
    let arm = trimmed.strip_prefix(name).is_some_and(|rest| {
        ["=>", "|", "if "]
            .iter()
            .any(|then| rest.trim_start().starts_with(then))
    });
    handed
        || arm
        || code.contains(&format!("{name}.iter()"))
        || code.contains(&format!("except_when_it_says(&{name}"))
        || code.contains(&format!("except_when_it_says({name}"))
}

/// Whether `code` names `name` as a whole word.
fn names_word(code: &str, name: &str) -> bool {
    code.match_indices(name).any(|(at, _)| {
        let before = code[..at].chars().next_back();
        let after = code[at + name.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// The file's `&str` and `[&str; N]` consts written with a literal: a
/// message kept beside its classifier rather than in the table, once a
/// function that reads git's words names one.
fn literal_consts(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut lines = outside_tests(text)
        .into_iter()
        .map(|(_, line)| code_of(line));
    while let Some(code) = lines.next() {
        let Some((_, rest)) = code.split_once("const ") else {
            continue;
        };
        let Some((name, kind)) = rest.split_once(':') else {
            continue;
        };
        let kind = kind.trim_start();
        if !(kind.starts_with("&str") || kind.starts_with("[&str")) {
            continue;
        }
        let mut initializer = kind.split_once('=').map_or("", |(_, v)| v).to_string();
        while !initializer.contains(';') {
            let Some(next) = lines.next() else { break };
            initializer.push_str(&next);
        }
        if initializer.contains('"') {
            names.push(name.trim().to_string());
        }
    }
    names
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// A literal, or a `format!` of one, handed to a matching method, or a
/// literal pattern of a match arm.
fn matches_a_literal(code: &str) -> bool {
    let called = MATCHING.iter().any(|method| {
        ["\"", "b\"", "format!(", "&format!("]
            .iter()
            .any(|literal| code.contains(&format!(".{method}({literal}")))
    });
    let trimmed = code.trim_start();
    let arm = trimmed.starts_with('"')
        && trimmed[1..].split_once('"').is_some_and(|(_, rest)| {
            let rest = rest.trim_start();
            rest.starts_with("=>") || rest.starts_with("if ") || rest.starts_with('|')
        });
    called || arm
}

#[cfg(test)]
mod tests {
    use super::{code_of, names_gits_words, strays};

    #[test]
    fn a_literal_matched_against_gits_words_is_found_and_a_named_message_is_not() {
        let text = "\
fn reads(said: &str) -> bool {
    said.contains(\"would be overwritten\") // a message out of the table
        || said.contains(crate::relied_messages::OVERWRITES_WORK)
}

fn parses(stdout: &str) -> Option<&str> {
    stdout.strip_prefix(\"worktree \")
}

fn sorts(why: &str, said: &str) -> u8 {
    match why {
        \"non-fast-forward\" => 1,
        crate::relied_messages::DELETION_PROHIBITED => 2,
        _ => 0,
    }
}

fn opens(said: &str, format: &str) -> u8 {
    let kind = match format {
        // not git's words: rev-parse's --show-object-format answer
        \"sha1\" => 1,
        \"sha256\" => 2,
        _ => 0,
    };
    if said.contains(\"x\") { // not git's words: the product's own mark
        return kind;
    }
    u8::from(said.contains(\"y\"))
}

trait Reads {
    fn told(&self, said: &str);
}

const GONE: [&str; 1] = [\"gitfile does not point\"];

fn kept_beside(stderr: &str) -> bool {
    GONE.iter().any(|words| stderr.contains(words))
}

#[cfg(test)]
#[path = \"tests.rs\"]
mod tests;

fn reads_its_answer(out: &GitOutput) -> bool {
    out.stderr_utf8().contains(\"not a git repository\")
}

fn names_the_ref(said: &str, name: &str) -> bool {
    said.contains(&format!(\"'{name}' not found\"))
}

#[cfg(test)]
fn for_the_tests(said: &str) -> bool {
    said.contains(\"passed over\")
}
";
        // `parses` names none of git's words, a trait's declaration has no
        // body, and a test item is passed over — the code after it is not.
        assert_eq!(strays(text), (vec![2, 12, 28, 38, 46, 50], 6));
    }

    #[test]
    fn a_word_in_a_string_or_a_comment_is_not_code() {
        assert_eq!(
            code_of("let a = \"{ said }\"; // stderr"),
            "let a = \"________\"; "
        );
        assert!(!names_gits_words(&code_of("let a = \"said\";")));
        assert!(names_gits_words("let said = out.stderr_utf8();"));
        assert!(!names_gits_words("let unsaid = 1;"));
        assert_eq!(
            code_of("fn a<'b>(c: &'b str) -> char { '{' }"),
            "fn a<'b>(c: &'b str) -> char { ' ' }"
        );
    }

    #[test]
    fn the_core_reads_no_message_of_its_own() {
        let (failures, readers) =
            super::check(&crate::tree::workspace_root()).expect("the core reads");
        assert!(readers > 0, "functions reading git's words are counted");
        assert_eq!(failures, Vec::<String>::new());
    }
}
