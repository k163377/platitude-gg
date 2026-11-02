//! The `Co-authored-by` lines a commit message carries. For a commit, git
//! is asked (`%(trailers:…)` in `details`) and [`parse_co_authors`] splits
//! the packed field; a message still being typed has no object to ask
//! about, so [`co_authors_in`] reads the text itself — git's rule copied,
//! since `git interpret-trailers` would be a process per keystroke. The
//! commit editor's `+N` follows it as the text is typed.

/// What git is told to put between trailer values (the unit separator):
/// it cannot appear in a name or an address, and is not the NUL that ends
/// a record.
const TRAILER_SEP: char = '\u{1f}';

/// Someone a `Co-authored-by` trailer credits alongside the author. The
/// address is optional: nothing stops a trailer carrying a bare name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoAuthor {
    pub name: String,
    /// Without the angle brackets; empty when the trailer had none.
    pub email: String,
}

/// The trailers git writes itself; one in the last paragraph lets prose
/// share the block (`trailer.c` `git_generated_prefixes`).
const GIT_GENERATED: [&str; 2] = ["Signed-off-by: ", "(cherry picked from commit "];

/// The `Co-authored-by` trailers a message body carries, by git's rule
/// (`trailer.c` `find_trailer_block_start`; fixed against git's own read in
/// the `details_diff` integration test): the trailer block is the last
/// paragraph, if every line in it is a trailer or a continuation of one —
/// or, with a git-generated trailer among them, if trailers are at least a
/// quarter of its lines. So a body that mentions "Co-authored-by:" in prose
/// credits nobody. `#` lines are passed over, as git reads them as comments.
pub fn co_authors_in(body: &str) -> Vec<CoAuthor> {
    let lines: Vec<&str> = body.lines().collect();
    let Some(start) = trailer_block_start(&lines) else {
        return Vec::new();
    };
    // A continuation joins its trailer's value with one space (git's
    // `unfold_value`); a `#` line ends the trailer before it, so an indented
    // line after one continues nothing (`trailer_info_get`).
    let mut values = Vec::new();
    let mut open: Option<String> = None;
    for line in &lines[start..] {
        if line.starts_with('#') {
            values.extend(open.take());
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            if let Some(value) = open.as_mut() {
                value.push(' ');
                value.push_str(line.trim());
            }
            continue;
        }
        values.extend(open.take());
        if let Some(at) = separator(line)
            && line[..at].trim().eq_ignore_ascii_case("co-authored-by")
        {
            open = Some(line[at + 1..].trim().to_string());
        }
    }
    values.extend(open);
    values
        .iter()
        .flat_map(|value| parse_co_authors(value))
        .collect()
}

/// Where the last paragraph starts, if git takes it for the trailer block.
/// Read from the bottom as git does; the top of the body stands where the
/// blank line after the subject would.
fn trailer_block_start(lines: &[&str]) -> Option<usize> {
    let (mut trailers, mut others, mut continuations) = (0usize, 0usize, 0usize);
    let mut generated = false;
    let mut only_spaces = true;
    let mut start = 0;
    for (at, line) in lines.iter().enumerate().rev() {
        if line.starts_with('#') {
            others += continuations;
            continuations = 0;
            continue;
        }
        if line.trim().is_empty() {
            if only_spaces {
                continue;
            }
            start = at + 1;
            break;
        }
        only_spaces = false;
        if GIT_GENERATED.iter().any(|prefix| line.starts_with(prefix)) {
            trailers += 1;
            continuations = 0;
            generated = true;
        } else if line.starts_with(|c: char| c.is_ascii_whitespace()) {
            continuations += 1;
        } else if separator(line).is_some() {
            trailers += 1;
            continuations = 0;
        } else {
            others += 1 + continuations;
            continuations = 0;
        }
    }
    if only_spaces {
        return None;
    }
    others += continuations;
    ((generated && trailers * 3 >= others) || (trailers > 0 && others == 0)).then_some(start)
}

/// Where a trailer line's `:` stands (`trailer.c` `find_separator`): a key
/// of ASCII letters, digits and `-`, then optional blanks — no `_`, and no
/// key at all is no trailer.
fn separator(line: &str) -> Option<usize> {
    let mut blanks = false;
    for (at, c) in line.char_indices() {
        if c == ':' {
            return (at > 0).then_some(at);
        }
        if !blanks && (c.is_ascii_alphanumeric() || c == '-') {
            continue;
        }
        if at > 0 && (c == ' ' || c == '\t') {
            blanks = true;
            continue;
        }
        return None;
    }
    None
}

/// Splits a packed trailer field into one entry per person. The address
/// is the last angle-bracketed run (the `Name <address>` convention);
/// without one the whole entry is the name.
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

/// Splits a typed identity — `Name <address>`, or either half alone —
/// into `(name, address)`. The address is the last bracketed run (or the
/// whole text) and counts only with an `@` past its first character; the
/// name is what precedes that bracket. Unlike [`parse_co_authors`], a bare
/// word is nobody: with no address there is nothing to file under.
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
        // A bare word is nobody, and `@` cannot open an address.
        assert_eq!(split_identity("Ada"), (String::new(), String::new()));
        assert_eq!(split_identity("@x"), (String::new(), String::new()));
        assert_eq!(split_identity(""), (String::new(), String::new()));
        // Earlier brackets belong to the name.
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
        assert!(
            co_authors_in("I dropped the Co-authored-by: Claude line by mistake.").is_empty(),
            "prose in the last paragraph is prose"
        );
    }

    #[test]
    fn a_body_that_is_only_trailers_still_credits() {
        // The body excludes the subject, so a trailer-only body has no
        // blank line to look for.
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
