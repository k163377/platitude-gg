//! Step input identities: a file's object id, except that a Rust or QML
//! file is identified by what it says once its comments are gone, and a
//! directory by its entries so identified — Markdown left out of both. A
//! comment reworded, a doc line added or a blank line moved changes no
//! key, so the greens stand; a string, a name or a number moved changes
//! the key, since nothing is read for meaning beyond the comments.

use std::collections::BTreeMap;
use std::io::Write;

/// The files whose id is their code, not their bytes.
fn read_for_code(path: &str) -> bool {
    path.ends_with(".rs") || path.ends_with(".qml")
}

/// The listing with every Rust and QML blob's id replaced by the id of
/// its code (`code_id`), read in one `git cat-file --batch` from `here` —
/// the tree under test, at the revision the listing is of.
pub(super) fn with_code_ids(here: &str, listing: &[u8]) -> Result<Vec<u8>, String> {
    let text = String::from_utf8_lossy(listing).into_owned();
    let entries: Vec<&str> = text.split('\0').collect();
    let wanted: Vec<(&str, &str)> = entries
        .iter()
        .filter_map(|entry| {
            let (head, path) = entry.split_once('\t')?;
            let mut fields = head.split(' ');
            let (_, kind, id) = (fields.next()?, fields.next()?, fields.next()?);
            (kind == "blob" && read_for_code(path)).then_some((id, path))
        })
        .collect();
    if wanted.is_empty() {
        return Ok(listing.to_vec());
    }
    let mut child = std::process::Command::new("git")
        .arg("-C")
        .arg(here)
        .args(["cat-file", "--batch"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("git cat-file: {e}"))?;
    let asked: String = wanted.iter().map(|(id, _)| format!("{id}\n")).collect();
    // The writer goes to a thread: git answers as it reads, and a pipe
    // full both ways would stand still.
    let mut stdin = child.stdin.take().ok_or("git cat-file: no stdin")?;
    let writer = std::thread::spawn(move || stdin.write_all(asked.as_bytes()));
    let output = child
        .wait_with_output()
        .map_err(|e| format!("git cat-file: {e}"))?;
    writer
        .join()
        .map_err(|_| "git cat-file: the writer panicked".to_string())?
        .map_err(|e| format!("git cat-file: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git cat-file --batch failed in {here}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let rust: Vec<bool> = wanted
        .iter()
        .map(|(_, path)| path.ends_with(".rs"))
        .collect();
    let code_ids = code_ids_from_batch(&output.stdout, &rust)?;
    let mut by_blob: BTreeMap<&str, String> = BTreeMap::new();
    for ((id, _), code) in wanted.iter().zip(code_ids) {
        by_blob.insert(id, code);
    }
    let mut out = Vec::with_capacity(listing.len());
    for entry in entries {
        if entry.is_empty() {
            continue;
        }
        let rewritten = entry.split_once('\t').and_then(|(head, path)| {
            let mut fields = head.split(' ');
            let (mode, kind, id) = (fields.next()?, fields.next()?, fields.next()?);
            let code = by_blob.get(id)?;
            Some(format!("{mode} {kind} {code}\t{path}"))
        });
        out.extend_from_slice(rewritten.as_deref().unwrap_or(entry).as_bytes());
        out.push(0);
    }
    Ok(out)
}

/// The code ids of the blobs as `git cat-file --batch` printed them —
/// `<id> blob <size>\n`, the bytes, `\n` — one per entry of `rust`, which
/// says whether each is Rust (else QML).
fn code_ids_from_batch(batch: &[u8], rust: &[bool]) -> Result<Vec<String>, String> {
    let mut ids = Vec::with_capacity(rust.len());
    let mut at = 0;
    while ids.len() < rust.len() {
        let rest = batch
            .get(at..)
            .ok_or("git cat-file --batch: the output ends before its last entry")?;
        let end = rest
            .iter()
            .position(|b| *b == b'\n')
            .ok_or("git cat-file --batch: a header cut short")?;
        let header = String::from_utf8_lossy(&batch[at..at + end]);
        let size: usize = header
            .split(' ')
            .nth(2)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("git cat-file --batch: {header:?}"))?;
        let start = at + end + 1;
        let body = batch
            .get(start..start + size)
            .ok_or("git cat-file --batch: a body cut short")?;
        ids.push(code_id(&String::from_utf8_lossy(body), rust[ids.len()]));
        at = start + size + 1;
    }
    Ok(ids)
}

/// The id of a file's code: a fingerprint of its text with the comments
/// out and the blank lines and trailing spaces gone. `code:` so a listing
/// cannot mistake it for a blob's.
pub(super) fn code_id(text: &str, rust: bool) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for line in without_comments(text, rust).lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        for byte in line.bytes().chain(std::iter::once(b'\n')) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("code:{hash:016x}")
}

/// The text with its comments taken out — `//` to the end of the line,
/// `/* */` (nested, as Rust allows) — and everything else kept as it is,
/// strings first of all: `"…"` with its escapes; for Rust (`rust`) its
/// `r#"…"#`, `b"…"` and char literals; for QML `'…'` and `` `…` ``. A `//`
/// inside any of those is text, not a comment.
pub(super) fn without_comments(text: &str, rust: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            let mut depth = 1;
            i += 2;
            while i < chars.len() && depth > 0 {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }
        if let Some(end) = string_end(&chars, i, rust) {
            out.extend(&chars[i..end]);
            i = end;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Where the string starting at `at` ends (one past its closing quote), or
/// `None` where no string starts there. A raw string `r#"…"#` ends at the
/// matching hashes; `"…"`, `'…'` and `` `…` `` at their quote, an escaped
/// one skipped. In Rust a `'` is a char literal only when a quote closes
/// it within a character or an escape — `'a` of a lifetime opens nothing.
fn string_end(chars: &[char], at: usize, rust: bool) -> Option<usize> {
    let c = chars[at];
    let before = at.checked_sub(1).map(|p| chars[p]);
    let word_before = before.is_some_and(|b| b.is_alphanumeric() || b == '_');
    if !rust {
        return match c {
            '"' | '\'' | '`' => quoted_end(chars, at, c),
            _ => None,
        };
    }
    // r"…" / r#"…"# / br"…" / br#"…"#
    if (c == 'r' || (c == 'b' && chars.get(at + 1) == Some(&'r'))) && !word_before {
        let mut j = at + if c == 'b' { 2 } else { 1 };
        let mut hashes = 0;
        while chars.get(j) == Some(&'#') {
            hashes += 1;
            j += 1;
        }
        if chars.get(j) == Some(&'"') {
            j += 1;
            loop {
                let quote = chars[j..].iter().position(|ch| *ch == '"')?;
                j += quote + 1;
                if chars[j..]
                    .iter()
                    .take(hashes)
                    .filter(|ch| **ch == '#')
                    .count()
                    == hashes
                {
                    return Some(j + hashes);
                }
            }
        }
        return None;
    }
    if c == 'b' && chars.get(at + 1) == Some(&'"') && !word_before {
        return quoted_end(chars, at + 1, '"');
    }
    if c == '"' {
        return quoted_end(chars, at, c);
    }
    if c == '\'' {
        // A char literal closes within one character or one escape.
        return if chars.get(at + 1) == Some(&'\\') {
            chars[at + 2..]
                .iter()
                .position(|ch| *ch == '\'')
                .map(|p| at + 3 + p)
        } else if chars.get(at + 2) == Some(&'\'') {
            Some(at + 3)
        } else {
            None
        };
    }
    None
}

fn quoted_end(chars: &[char], open: usize, quote: char) -> Option<usize> {
    let mut j = open + 1;
    while j < chars.len() {
        match chars[j] {
            '\\' => j += 2,
            ch if ch == quote => return Some(j + 1),
            _ => j += 1,
        }
    }
    None
}

pub(super) fn from_listing(listing: &[u8]) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut directories: BTreeMap<String, String> = BTreeMap::new();
    for entry in String::from_utf8_lossy(listing).split('\0') {
        let Some((head, path)) = entry.split_once('\t') else {
            continue;
        };
        let mut fields = head.split(' ');
        let (Some(mode), Some(kind), Some(id)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if kind == "tree" || super::graph::is_markdown(path) {
            continue;
        }
        files.insert(path.to_string(), id.to_string());
        let mut parent = path;
        while let Some((dir, _)) = parent.rsplit_once('/') {
            // Retain modes, names and gitlinks as a Git tree would.
            let contents = directories.entry(dir.to_string()).or_default();
            contents.push_str(&format!("{mode} {kind} {id}\t{path}\0"));
            parent = dir;
        }
    }
    for (dir, contents) in directories {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in contents.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
        files.insert(dir, format!("files:{hash:016x}"));
    }
    files
}

#[cfg(test)]
mod tests {
    use super::{code_id, code_ids_from_batch, from_listing, without_comments};

    #[test]
    fn directory_inputs_keep_names_modes_and_gitlinks_but_not_markdown() {
        let base = "100644 blob a\tsrc/code.rs\0";
        let ids = from_listing(base.as_bytes());
        let docs = format!("040000 tree changed\tsrc\0{base}100644 blob b\tsrc/README.MD\0");
        assert_eq!(ids, from_listing(docs.as_bytes()));
        for moved in [
            "100755 blob a\tsrc/code.rs\0",
            "100644 blob b\tsrc/code.rs\0",
            "100644 blob a\tsrc/renamed.rs\0",
            "160000 commit a\tsrc/code.rs\0",
        ] {
            assert_ne!(ids["src"], from_listing(moved.as_bytes())["src"]);
        }
        assert!(from_listing(b"100644 blob a\tdocs/only.md\0").is_empty());
    }

    #[test]
    fn comments_and_blank_lines_are_no_part_of_a_files_code_id() {
        let code = "fn a() {\n    let x = 1;\n}\n";
        let commented =
            "// a doc\nfn a() {\n\n    let x = 1; // one\n    /* block\n       comment */\n}\n";
        assert_eq!(code_id(code, true), code_id(commented, true));
        assert_ne!(
            code_id(code, true),
            code_id("fn a() {\n    let x = 2;\n}\n", true)
        );
        let qml = "Item {\n    width: 1 // wide\n}\n";
        assert_eq!(
            code_id(qml, false),
            code_id("Item {\n    width: 1\n}\n", false)
        );
    }

    #[test]
    fn a_slash_pair_inside_a_string_or_char_is_text() {
        let rust = [
            "let u = \"https://x\";\n",
            "let r = r#\"a // b\"#;\n",
            "let r = r\"a // b\";\n",
            "let b = b\"a // b\";\n",
            "let c = '/'; let d = \"//\"; // gone\n",
            "let e = \"a \\\" // b\";\n",
        ];
        let qml = ["x: \"https://x\"\n", "x: 'https://x'\n", "x: `https://x`\n"];
        for (kept, is_rust) in rust
            .iter()
            .map(|k| (*k, true))
            .chain(qml.iter().map(|k| (*k, false)))
        {
            let without = without_comments(kept, is_rust);
            assert!(without.contains("//"), "{kept:?} -> {without:?}");
            assert!(!without.contains("gone"), "{kept:?} -> {without:?}");
            assert!(
                without.trim_end().ends_with(';')
                    || without.trim_end().ends_with('`')
                    || without.trim_end().ends_with('"')
                    || without.trim_end().ends_with('\''),
                "{kept:?} -> {without:?}"
            );
        }
        // A lifetime opens no string, so the comment after it goes.
        let lifetime = "fn f<'a>(x: &'a str) {} // gone\n";
        assert_eq!(
            without_comments(lifetime, true),
            "fn f<'a>(x: &'a str) {} \n"
        );
        // Nested block comments, as Rust reads them.
        assert_eq!(without_comments("a /* x /* y */ z */ b", true), "a  b");
    }

    #[test]
    fn a_batch_of_blobs_answers_one_code_id_each() {
        let batch = b"id1 blob 9\nfn a() {}\nid2 blob 15\n// c\nfn a() {}\n";
        let ids = code_ids_from_batch(batch, &[true, true]).expect("two blobs");
        assert_eq!(ids[0], ids[1]);
        assert_eq!(ids[0], code_id("fn a() {}", true));
    }
}
