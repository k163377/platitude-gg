//! The `Co-authored-by` lines a commit message carries.
//!
//! Split out of `details`, which is what asks git for them
//! (`%(trailers:key=Co-authored-by,…)`) and hands the packed field here.

/// Separates one trailer value from the next inside that field.
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

/// Splits the trailer field into one entry per credited person.
///
/// The shape git hands over is whatever the message wrote after the
/// colon. `Name <address>` is the convention every tool that reads these
/// follows, so the address is taken from the last angle-bracketed run;
/// anything else stays a name in full rather than being guessed at.
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

#[cfg(test)]
mod tests {
    use super::*;

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
        // that has none is a name, not a name with a broken address.
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
}
