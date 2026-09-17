//! The `Co-authored-by` lines a commit message carries, and how they are
//! read out of one.
//!
//! Two callers, two starting points. A commit that exists is asked of git
//! (`%(trailers:key=Co-authored-by,…)` — `details`), which hands back a
//! packed field for [`parse_co_authors`]. A message still being typed has
//! no object to ask about, so [`co_authors_in`] reads the text itself.

/// What git is told to put between trailer values, and so what a packed
/// field is split on. A record separator: it cannot appear in a name or
/// an address, and it does not end a NUL-delimited record.
const TRAILER_SEP: char = '\u{1f}';

/// Someone the message credits alongside the author.
///
/// A commit object holds one author; everyone else is a `Co-authored-by`
/// trailer, which is a line in the message and nothing more. The name is
/// what git returns; the address is optional, because nothing stops a
/// trailer from carrying a bare name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoAuthor {
    pub name: String,
    /// Without the angle brackets; empty when the trailer had none.
    pub email: String,
}

/// The `Co-authored-by` trailers a message body carries, read the way
/// git reads them.
///
/// Nothing can be asked of git here: the commit editor's message is not
/// a commit yet, so there is no object to run `%(trailers)` against, and
/// the answer has to follow the box while it is being typed. So the rule
/// is written out — **the trailer block is the last paragraph, and only
/// if every line of it is a `key: value`**. That is what keeps a body
/// that says "I dropped the Co-authored-by: line" from crediting anyone:
/// a paragraph with prose in it is prose.
///
/// Continuation lines (indented) belong to the trailer above them, which
/// is why they do not disqualify the block.
pub fn co_authors_in(body: &str) -> Vec<CoAuthor> {
    let lines: Vec<&str> = body.trim_end().lines().collect();
    let start = lines
        .iter()
        .rposition(|line| line.trim().is_empty())
        .map_or(0, |blank| blank + 1);
    let block = &lines[start.min(lines.len())..];
    if block.is_empty() || !block.iter().all(|line| is_trailer_line(line)) {
        return Vec::new();
    }
    block
        .iter()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case("co-authored-by")
                .then(|| value.trim())
        })
        .flat_map(parse_co_authors)
        .collect()
}

/// Whether one line of the last paragraph can stand in a trailer block:
/// a `key: value` whose key is a word, or a continuation indented under
/// the one before it.
fn is_trailer_line(line: &str) -> bool {
    if line.starts_with(' ') || line.starts_with('\t') {
        return true;
    }
    let Some((key, _)) = line.split_once(':') else {
        return false;
    };
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Splits the trailer field into one entry per credited person.
///
/// The shape git hands over is whatever the message wrote after the
/// colon. `Name <address>` is the convention every tool that reads these
/// follows, so the address is taken from the last angle-bracketed run;
/// anything else stays a name in full.
pub fn parse_co_authors(field: &str) -> Vec<CoAuthor> {
    field
        .split(TRAILER_SEP)
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let Some(open) = entry.rfind('<') else {
                return Some(CoAuthor {
                    name: entry.to_string(),
                    email: String::new(),
                });
            };
            let rest = &entry[open + 1..];
            let Some(close) = rest.rfind('>') else {
                return Some(CoAuthor {
                    name: entry.to_string(),
                    email: String::new(),
                });
            };
            Some(CoAuthor {
                name: entry[..open].trim_end().to_string(),
                email: rest[..close].to_string(),
            })
        })
        .collect()
}

/// Splits an identity a person typed — `Name <address>`, or either half
/// on its own — into `(name, address)`. The address is the inside of the
/// last angle-bracketed run, or the whole text when there is none, and
/// it only counts as one with an `@` past its first character (an
/// address is the half with an `@` in it); the name is what stands
/// before that bracket. Unlike a co-author trailer ([`parse_co_authors`])
/// a bare word here is nobody: with no address there is nothing to file
/// under.
pub fn split_identity(text: &str) -> (String, String) {
    let text = text.trim();
    let open = text.rfind('<');
    let close = text.rfind('>');
    let inner = match (open, close) {
        (Some(o), Some(c)) if c > o => text[o + 1..c].trim(),
        _ => text,
    };
    let email = if inner.find('@').is_some_and(|at| at > 0) {
        inner.to_string()
    } else {
        String::new()
    };
    let name = match open {
        Some(o) if o > 0 => text[..o].trim().to_string(),
        _ => String::new(),
    };
    (name, email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_identity_splits_into_name_and_address() {
        assert_eq!(
            split_identity("Ada Lovelace <ada@example.com>"),
            ("Ada Lovelace".to_string(), "ada@example.com".to_string())
        );
        assert_eq!(
            split_identity("  ada@example.com  "),
            (String::new(), "ada@example.com".to_string())
        );
        assert_eq!(
            split_identity("<ada@example.com>"),
            (String::new(), "ada@example.com".to_string())
        );
        // A bare word is nobody: with no address there is nothing to
        // file under — and `@` cannot open one.
        assert_eq!(split_identity("Ada"), (String::new(), String::new()));
        assert_eq!(split_identity("@x"), (String::new(), String::new()));
        assert_eq!(split_identity(""), (String::new(), String::new()));
        // The last bracketed run is the address, brackets before it are
        // part of the name.
        assert_eq!(
            split_identity("Ada <of Lovelace> <ada@example.com>"),
            (
                "Ada <of Lovelace>".to_string(),
                "ada@example.com".to_string()
            )
        );
    }

    #[test]
    fn co_author_field_splits_on_the_unit_separator() {
        let got = parse_co_authors("Claude Opus 5 <noreply@anthropic.com>\u{1f}Bob <b@e.com>");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "Claude Opus 5");
        assert_eq!(got[0].email, "noreply@anthropic.com");
        assert_eq!(got[1].name, "Bob");
        assert_eq!(got[1].email, "b@e.com");
    }

    #[test]
    fn co_author_without_an_address_keeps_its_whole_name() {
        // Nothing in git requires the angle-bracket form, so a trailer
        // that has none is a name in full.
        let got = parse_co_authors("Nameless");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Nameless");
        assert_eq!(got[0].email, "");

        let unclosed = parse_co_authors("Half <open");
        assert_eq!(unclosed[0].name, "Half <open");
        assert_eq!(unclosed[0].email, "");
    }

    #[test]
    fn co_author_name_may_hold_angle_brackets_of_its_own() {
        // The address is the last bracketed run, so a name that contains
        // brackets keeps them.
        let got = parse_co_authors("A <B> C <c@e.com>");
        assert_eq!(got[0].name, "A <B> C");
        assert_eq!(got[0].email, "c@e.com");
    }

    #[test]
    fn no_trailer_yields_no_co_authors() {
        assert!(parse_co_authors("").is_empty());
        assert!(parse_co_authors("\u{1f}").is_empty());
    }

    #[test]
    fn a_body_credits_whoever_its_last_paragraph_names() {
        let got = co_authors_in(
            "Why this change.\n\nCo-authored-by: Claude Opus 5 <noreply@anthropic.com>\n\
             Signed-off-by: Demo <demo@example.com>\n",
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Claude Opus 5");
        assert_eq!(got[0].email, "noreply@anthropic.com");
    }

    #[test]
    fn a_paragraph_with_prose_in_it_credits_nobody() {
        // git's own rule, and the reason the paragraph is read whole: a
        // body that talks *about* the trailer has no trailers.
        assert!(
            co_authors_in("I dropped the Co-authored-by: Claude line by mistake.").is_empty(),
            "prose in the last paragraph is prose"
        );
    }

    #[test]
    fn a_body_that_is_only_trailers_still_credits() {
        // The subject is not part of what this reads, so a message whose
        // whole body is the trailer block has no blank line to look for.
        let got = co_authors_in("Co-authored-by: Bob <b@e.com>");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Bob");
    }

    #[test]
    fn an_empty_body_credits_nobody() {
        assert!(co_authors_in("").is_empty());
        assert!(co_authors_in("\n\n").is_empty());
    }
}
