//! Step input identities: a file's object id, except that a Rust or QML
//! file is identified by its code as the language's lexer cuts it
//! (`crate::lex::normalized`), and a directory by its entries so
//! identified — Markdown left out of both. A comment reworded, a doc line
//! added or a blank line moved between tokens changes no key, so the
//! greens stand; a literal's every character, a name, a number and a
//! line end the code may hang a statement on are the key's, since beyond
//! the lexical layer nothing is read for meaning. The steps keyed on it
//! are the ones that build or run the app, which no comment reaches
//! (`plan::owed`); a tool that reads the sources keys on the bytes, and a
//! run of a Rust program over sources it reads as text (`qmltest`) on the
//! Rust files' code and every other file's bytes.

use std::collections::BTreeMap;
use std::io::Write;

/// The files whose id is their code, not their bytes.
fn read_for_code(path: &str) -> bool {
    path.ends_with(".rs") || path.ends_with(".qml")
}

/// The listing twice over, with blobs' ids replaced by the ids of their
/// code (`code_id`): every Rust and QML blob's (`code`), and the Rust
/// blobs' alone (`rust`). Read in one `git cat-file --batch` from `here` —
/// the tree under test, at the revision the listing is of.
pub(super) struct CodeListings {
    pub code: Vec<u8>,
    pub rust: Vec<u8>,
}

pub(super) fn with_code_ids(here: &str, listing: &[u8]) -> Result<CodeListings, String> {
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
        return Ok(CodeListings {
            code: listing.to_vec(),
            rust: listing.to_vec(),
        });
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
    let rewritten = |of: fn(&str) -> bool| {
        let mut out = Vec::with_capacity(listing.len());
        for entry in &entries {
            if entry.is_empty() {
                continue;
            }
            let rewritten = entry.split_once('\t').and_then(|(head, path)| {
                let mut fields = head.split(' ');
                let (mode, kind, id) = (fields.next()?, fields.next()?, fields.next()?);
                let code = by_blob.get(id).filter(|_| of(path))?;
                Some(format!("{mode} {kind} {code}\t{path}"))
            });
            out.extend_from_slice(rewritten.as_deref().unwrap_or(entry).as_bytes());
            out.push(0);
        }
        out
    };
    Ok(CodeListings {
        code: rewritten(read_for_code),
        rust: rewritten(|path| path.ends_with(".rs")),
    })
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

/// The id of a file's code: a fingerprint of its normalized text
/// (`crate::lex::normalized`), or of its every byte where the lexical
/// reading stopped short — no two inputs then share a key. `code:` /
/// `bytes:` so a listing cannot mistake either for a blob's.
pub(super) fn code_id(text: &str, rust: bool) -> String {
    let lang = if rust {
        crate::lex::Lang::Rust
    } else {
        crate::lex::Lang::Qml
    };
    let (prefix, keyed) = match crate::lex::normalized(text, lang) {
        Some(normalized) => ("code", normalized),
        None => ("bytes", text.to_string()),
    };
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in keyed.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{prefix}:{hash:016x}")
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
    use super::{code_id, code_ids_from_batch, from_listing};

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

    /// What the key must tell apart: a literal's own blank line or
    /// trailing space, a `//` that is text, and a line end a statement
    /// may hang on. What it must not: a comment, and spacing between
    /// tokens.
    #[test]
    fn a_literals_spacing_and_a_statements_line_end_are_the_code_ids_and_a_comment_is_not() {
        let one = "let s = r\"a\n\nb\";\n";
        let other = "let s = r\"a\nb\";\n";
        assert_ne!(code_id(one, true), code_id(other, true));
        let spaced = "x: `a \n b`\n";
        let tight = "x: `a\n b`\n";
        assert_ne!(code_id(spaced, false), code_id(tight, false));
        assert_ne!(
            code_id("let u = \"https://x\";\n", true),
            code_id("let u = \"https:\";\n", true)
        );
        assert_ne!(code_id("return\nx\n", false), code_id("return x\n", false));
        assert_eq!(code_id("a /* c */ b\n", true), code_id("a   b\n", true));
        assert_eq!(code_id("a\n\n\nb // c\n", false), code_id("a\nb\n", false));
    }

    /// A file the reading stops short on is keyed on its bytes: a comment
    /// moved in it moves the key, and its key says so.
    #[test]
    fn a_file_read_short_is_keyed_on_its_bytes() {
        let open = "Item {\n    text: `a ${ b\n}\n";
        assert!(
            code_id(open, false).starts_with("bytes:"),
            "{}",
            code_id(open, false)
        );
        assert_ne!(
            code_id(open, false),
            code_id("// c\nItem {\n    text: `a ${ b\n}\n", false)
        );
        assert!(code_id("Item {}\n", false).starts_with("code:"));
    }

    #[test]
    fn a_batch_of_blobs_answers_one_code_id_each() {
        let batch = b"id1 blob 9\nfn a() {}\nid2 blob 15\n// c\nfn a() {}\n";
        let ids = code_ids_from_batch(batch, &[true, true]).expect("two blobs");
        assert_eq!(ids[0], ids[1]);
        assert_eq!(ids[0], code_id("fn a() {}", true));
    }
}
