//! The table as the product writes it: each `pub(crate) const NAME: Words`
//! (a form) in `platitude-core/src/relied_messages.rs`, with the `- git:` /
//! `- openssh:` and `- witness:` lines of the doc comment above it, and
//! each `Relied` (a message) held to name forms of it.

use std::path::Path;

/// The product's table of git's words.
pub(super) const TABLE: &str = "crates/platitude-core/src/relied_messages.rs";

/// One form the product reads, and where its words come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Form {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) sources: Vec<Source>,
    /// Test paths in the integration binary; empty where the table says
    /// `none`.
    pub(super) witnesses: Vec<String>,
}

/// A file of a supplier's tree, and words on a line of it (any case).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Source {
    pub(super) supplier: Supplier,
    pub(super) path: String,
    pub(super) words: String,
}

impl Source {
    /// Whether `line` of `path` is one this source names.
    pub(super) fn names(&self, path: &str, line: &str) -> bool {
        self.path == path && line.to_lowercase().contains(&self.words.to_lowercase())
    }
}

/// Whose tree a source is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Supplier {
    Git,
    OpenSsh,
}

impl Supplier {
    pub(super) fn word(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::OpenSsh => "openssh",
        }
    }

    pub(super) fn from_word(word: &str) -> Option<Self> {
        match word {
            "git" => Some(Self::Git),
            "openssh" => Some(Self::OpenSsh),
            _ => None,
        }
    }
}

pub(super) fn read(root: &Path) -> Result<Vec<Form>, String> {
    let text = std::fs::read_to_string(root.join(TABLE)).map_err(|e| format!("{TABLE}: {e}"))?;
    parse(&text).map_err(|e| format!("{TABLE}: {e}"))
}

/// Every form, in the table's order — each `Words` const. A form names at
/// least one source and says what witnesses it, `none` included: a line
/// left out is the table forgetting, not deciding. A message (a `Relied`
/// const) carries none of its own and is held to name forms of the table.
fn parse(text: &str) -> Result<Vec<Form>, String> {
    let mut forms: Vec<Form> = Vec::new();
    let mut messages: Vec<(String, Vec<String>)> = Vec::new();
    let mut doc: Vec<&str> = Vec::new();
    let mut lines = text.lines().enumerate();
    while let Some((at, line)) = lines.next() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("///") {
            doc.push(rest.trim());
            continue;
        }
        let Some(rest) = trimmed
            .strip_prefix("pub(crate) const ")
            .filter(|rest| !rest.starts_with("fn "))
        else {
            if !trimmed.starts_with("#[") {
                doc.clear();
            }
            continue;
        };
        let (name, rest) = rest
            .split_once(':')
            .ok_or_else(|| format!("line {}: an entry has a type", at + 1))?;
        let name = name.trim();
        let mut item = rest.trim().to_string();
        while !item.ends_with(';') {
            let (_, next) = lines
                .next()
                .ok_or_else(|| format!("{name}: the file ends inside its value"))?;
            item.push_str(next.trim());
        }
        let item = item.trim_end_matches(';');
        if forms.iter().any(|f| f.name == name) || messages.iter().any(|(n, _)| n == name) {
            return Err(format!("{name} is written twice"));
        }
        if let Some(literal) = item
            .strip_prefix("Words =")
            .and_then(|value| value.trim().strip_prefix("Words("))
            .and_then(|value| value.strip_suffix(')'))
        {
            let value = unquote(literal.trim())
                .ok_or_else(|| format!("{name}: its words are not one string literal"))?;
            forms.push(form_of(name, value, &doc)?);
        } else if let Some(group) = item.strip_prefix("Relied =").map(str::trim) {
            let members = group
                .strip_prefix("Relied::any_case(&[")
                .or_else(|| group.strip_prefix("Relied::any(&["))
                .and_then(|members| members.strip_suffix("])"))
                .ok_or_else(|| format!("{name}: a message is `Relied::any(&[…])`"))?;
            let tagged = doc.iter().any(|line| {
                ["- git: ", "- openssh: ", "- witness: "]
                    .iter()
                    .any(|tag| line.starts_with(tag))
            });
            if tagged {
                return Err(format!(
                    "{name}: a message's sources and witnesses are its forms'"
                ));
            }
            let members = members
                .split(',')
                .map(str::trim)
                .filter(|member| !member.is_empty())
                .map(str::to_string)
                .collect();
            messages.push((name.to_string(), members));
        } else {
            return Err(format!(
                "line {}: {name} is neither a `Words` form nor a `Relied` message",
                at + 1
            ));
        }
        doc.clear();
    }
    for (name, members) in &messages {
        let strays: Vec<&str> = members
            .iter()
            .filter(|member| !forms.iter().any(|form| &form.name == *member))
            .map(String::as_str)
            .collect();
        if members.is_empty() || !strays.is_empty() {
            return Err(format!(
                "{name} names what is no form of the table: {}",
                strays.join(", ")
            ));
        }
    }
    if forms.is_empty() {
        return Err("no form in it".to_string());
    }
    Ok(forms)
}

fn form_of(name: &str, value: String, doc: &[&str]) -> Result<Form, String> {
    let mut sources = Vec::new();
    let mut witnesses = Vec::new();
    let mut said_none = false;
    for line in doc {
        let Some(item) = line.strip_prefix("- ") else {
            continue;
        };
        let Some((key, rest)) = item.split_once(": ") else {
            continue;
        };
        if key == "witness" {
            if rest.starts_with("none") {
                said_none = true;
            } else {
                let [test] = ticked(rest)[..] else {
                    return Err(format!("{name}: a witness is one `test::path`: {line}"));
                };
                witnesses.push(test.to_string());
            }
        } else if let Some(supplier) = Supplier::from_word(key) {
            let [path, words] = ticked(rest)[..] else {
                return Err(format!(
                    "{name}: a source is a `file` and the `words` on its line: {line}"
                ));
            };
            sources.push(Source {
                supplier,
                path: path.to_string(),
                words: words.to_string(),
            });
        }
    }
    if sources.is_empty() {
        return Err(format!("{name} names no source (`- git: `file` `words``)"));
    }
    if witnesses.is_empty() != said_none {
        return Err(format!(
            "{name} says what witnesses it once: tests, or `- witness: none — <why>`"
        ));
    }
    Ok(Form {
        name: name.to_string(),
        value,
        sources,
        witnesses,
    })
}

/// The spans between backticks, in order.
fn ticked(text: &str) -> Vec<&str> {
    text.split('`').skip(1).step_by(2).collect()
}

/// The text of one Rust string literal, its escapes undone.
fn unquote(literal: &str) -> Option<String> {
    let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            if c == '"' {
                return None;
            }
            out.push(c);
            continue;
        }
        out.push(match chars.next()? {
            'n' => '\n',
            't' => '\t',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            _ => return None,
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{Supplier, parse, unquote};

    const TABLE: &str = r#"//! The module's words.

// ---- A group

/// A lock another git holds.
///
/// - git: `lockfile.c` `Unable to create '%s.lock'`
/// - witness: `commit_integration::a_lock`
/// - witness: `stage_integration::refusals::a_lock`
pub(crate) const LOCK: Words = Words(".lock': File exists");

/// Advice, on a line of its own.
///
/// - git: `builtin/worktree.c` `use 'remove -f -f'`
/// - openssh: `sshconnect.c` `Host key verification failed`
/// - witness: none — nothing reaches it
pub(crate) const ADVICE: Words =
    Words("\nuse 'remove -f -f'");

/// Either of them, as a message.
pub(crate) const EITHER: Relied =
    Relied::any(&[LOCK, ADVICE]);

impl Words {
    pub(crate) const fn text(self) -> &'static str {
        self.0
    }
}
"#;

    #[test]
    fn each_form_carries_its_sources_and_witnesses() {
        let forms = parse(TABLE).expect("a table");
        assert_eq!(forms.len(), 2);
        assert_eq!(forms[0].name, "LOCK");
        assert_eq!(forms[0].value, ".lock': File exists");
        assert_eq!(forms[0].sources[0].path, "lockfile.c");
        assert_eq!(forms[0].sources[0].words, "Unable to create '%s.lock'");
        assert_eq!(
            forms[0].witnesses,
            [
                "commit_integration::a_lock",
                "stage_integration::refusals::a_lock"
            ]
        );
        assert_eq!(
            forms[1].value, "\nuse 'remove -f -f'",
            "the value spans lines"
        );
        assert_eq!(forms[1].sources[1].supplier, Supplier::OpenSsh);
        assert!(forms[1].witnesses.is_empty());
    }

    #[test]
    fn a_form_that_forgets_its_source_or_its_witness_is_refused() {
        let unsourced = "/// - witness: none — x\npub(crate) const A: Words = Words(\"a\");\n";
        assert!(parse(unsourced).unwrap_err().contains("names no source"));
        let unwitnessed = "/// - git: `a.c` `a`\npub(crate) const A: Words = Words(\"a\");\n";
        assert!(
            parse(unwitnessed)
                .unwrap_err()
                .contains("witnesses it once")
        );
        let twice = "/// - git: `a.c` `a`\n/// - witness: none\npub(crate) const A: Words = Words(\"a\");\n\
                     /// - git: `a.c` `a`\n/// - witness: none\npub(crate) const A: Words = Words(\"a\");\n";
        assert!(parse(twice).unwrap_err().contains("twice"));
    }

    /// A message is its forms: it names forms of the table and carries no
    /// source of its own.
    #[test]
    fn a_message_names_forms_and_nothing_else() {
        let form = "/// - git: `a.c` `a`\n/// - witness: none\npub(crate) const A: Words = Words(\"a\");\n";
        let stray = format!("{form}pub(crate) const M: Relied = Relied::any(&[A, B]);\n");
        assert!(
            parse(&stray)
                .unwrap_err()
                .contains("no form of the table: B")
        );
        let sourced = format!(
            "{form}/// - git: `a.c` `a`\npub(crate) const M: Relied = Relied::any_case(&[A]);\n"
        );
        assert!(parse(&sourced).unwrap_err().contains("its forms'"));
        let plain = format!("{form}pub(crate) const M: Relied = Relied::any_case(&[A]);\n");
        assert_eq!(parse(&plain).expect("a table").len(), 1);
    }

    #[test]
    fn a_literal_is_read_as_rust_reads_it() {
        assert_eq!(unquote(r#""a\"b\\c\n""#).as_deref(), Some("a\"b\\c\n"));
        assert_eq!(unquote(r#""a" + "b""#), None);
    }

    #[test]
    fn the_products_table_reads() {
        let root = crate::tree::workspace_root();
        let forms = super::read(&root).expect("the product's table");
        assert!(forms.iter().all(|form| !form.sources.is_empty()));
    }
}
