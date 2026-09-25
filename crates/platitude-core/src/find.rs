//! What one typed line matches, over the commits already on screen.
//!
//! Runs no git: it reads the rows the graph window already holds, so
//! nothing outside the loaded window is found (`session::DEFAULT_LOG_LIMIT`),
//! and the bar says so.
//!
//! The line is taken literally (never split on spaces), and each field
//! answers only the query shapes it can hold. Deliberately out: the
//! description and a stash's `stash@{n}`, which would light a row for a
//! reason nothing on it shows, and the domain half of an address, which
//! would light every row (デザイン規約 §コミットを探す).

/// One typed line, ready to be asked of a row.
///
/// Built once per keystroke; the folding and the shape tests are done
/// here so that the per-row work is comparisons only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The line as typed, ASCII-folded, whitespace kept exactly (`fix ` asks
    /// for the word, not the prefix).
    needle: String,
    /// The line could be the start of an object name.
    oid_prefix: bool,
}

/// The fields of one row a search can look at, all already in memory.
///
/// Prose and names are searched anywhere inside, identifiers only from
/// their start.
#[derive(Debug, Default, Clone, Copy)]
pub struct Row<'a> {
    /// Full object name in lowercase hex. Empty for a row that is not a
    /// commit (the working-tree row), which matches nothing at all.
    pub oid_hex: &'a str,
    /// The first line of the message; the description is not searched
    /// (module note).
    pub subject: &'a str,
    /// The author, then whoever the message credits. Prose.
    pub people: &'a [&'a str],
    /// Their addresses. Identifiers.
    pub addresses: &'a [&'a str],
    /// Branches, tags and the HEAD marker — the row's chips. Names.
    pub tokens: &'a [&'a str],
}

impl Query {
    /// Reads a typed line. `None` when there is nothing to search for:
    /// empty, or whitespace only — taken literally, whitespace would light
    /// nearly every row.
    pub fn new(text: &str) -> Option<Self> {
        if text.chars().all(char::is_whitespace) {
            return None;
        }
        let needle = text.to_ascii_lowercase();
        let oid_prefix =
            needle.len() >= MIN_OID_PREFIX && needle.bytes().all(|b| b.is_ascii_hexdigit());
        Some(Self { needle, oid_prefix })
    }

    /// The line as typed (folded), for whoever has to show it back.
    pub fn text(&self) -> &str {
        &self.needle
    }

    /// Whether this row is one of the answers.
    pub fn matches(&self, row: &Row<'_>) -> bool {
        // The caller blanks a non-commit's id (`Row::oid_hex`): the
        // working-tree row's all-zero id would otherwise answer `0000`.
        if row.oid_hex.is_empty() {
            return false;
        }
        self.at_start_of(row.oid_hex)
            || self.inside(row.subject)
            || row.people.iter().any(|p| self.inside(p))
            || row.addresses.iter().any(|a| self.starts(a))
            || row.tokens.iter().any(|t| self.inside(t))
    }

    /// From the start, ignoring case.
    fn starts(&self, hay: &str) -> bool {
        hay.as_bytes()
            .get(..self.needle.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(self.needle.as_bytes()))
    }

    /// Anywhere inside, ignoring ASCII case only — a Unicode table is not
    /// worth it on a per-keystroke path, and CJK has no case.
    fn inside(&self, hay: &str) -> bool {
        contains_folded(hay, &self.needle)
    }

    /// From the start only, as git resolves abbreviations, and only when
    /// the line could be an object name at all. Inside random hex a short
    /// run would light rows for no reason a reader could see.
    fn at_start_of(&self, oid_hex: &str) -> bool {
        self.oid_prefix && oid_hex.starts_with(&self.needle)
    }
}

/// Shortest run of hex read as an object name — the shortest git resolves
/// (`rev-parse --short` floors at 4). Three would find commits by an id no
/// git command accepts.
const MIN_OID_PREFIX: usize = 4;

/// Case-folded substring test over bytes: valid UTF-8 never starts with a
/// continuation byte, so a byte-level hit is always a real substring hit.
fn contains_folded(hay: &str, needle_folded: &str) -> bool {
    let (h, n) = (hay.as_bytes(), needle_folded.as_bytes());
    if n.is_empty() {
        return true;
    }
    if n.len() > h.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OID: &str = "3f2a1b0c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5f40";

    fn row<'a>(
        subject: &'a str,
        people: &'a [&'a str],
        addresses: &'a [&'a str],
        tokens: &'a [&'a str],
    ) -> Row<'a> {
        Row {
            oid_hex: OID,
            subject,
            people,
            addresses,
            tokens,
        }
    }

    fn plain(subject: &str) -> Row<'_> {
        row(subject, &[], &[], &[])
    }

    fn hits(query: &str, r: &Row<'_>) -> bool {
        Query::new(query).is_some_and(|q| q.matches(r))
    }

    #[test]
    fn nothing_to_search_for() {
        assert!(Query::new("").is_none());
        assert!(Query::new(" ").is_none());
        assert!(Query::new("\t \n").is_none());
        assert!(Query::new("　").is_none(), "an ideographic space too");
    }

    #[test]
    fn the_subject_matches_anywhere_and_ignores_case() {
        let r = plain("Fix the PARSER");
        assert!(hits("fix", &r));
        assert!(hits("parser", &r));
        assert!(hits("PaRsEr", &r));
        assert!(!hits("reader", &r));
    }

    #[test]
    fn the_description_is_not_searched() {
        // The omission is the decision (module note); the words
        // below are what such a description holds.
        let r = plain("fix: harden the parser");
        assert!(!hits("the writer too", &r));
    }

    #[test]
    fn a_query_with_a_space_is_taken_literally() {
        let r = plain("fix the parser");
        assert!(hits("fix the", &r));
        assert!(!hits("the fix", &r), "the words are not reordered");
        assert!(!hits("fix parser", &r), "nor ANDed as terms");
    }

    #[test]
    fn spaces_are_kept_as_typed() {
        assert!(!hits("fix ", &plain("prefix")));
        assert!(hits("fix ", &plain("fix the parser")));
    }

    #[test]
    fn a_name_matches_anywhere_inside() {
        let r = row("", &["山田 太郎", "Ada Lovelace"], &[], &[]);
        assert!(hits("山田 太郎", &r), "a name holds a space");
        assert!(hits("lovelace", &r), "and is searched from anywhere");
    }

    #[test]
    fn an_address_matches_from_its_start() {
        let r = row("", &[], &["ada@example.com"], &[]);
        assert!(hits("ada", &r));
        assert!(hits("ada@example.com", &r), "however it was pasted");
        assert!(hits("ADA@Example.com", &r));
        assert!(!hits("example.com", &r));
        assert!(!hits("gmail", &row("", &[], &["someone@gmail.com"], &[])));
        // An address cannot hold whitespace either way.
        assert!(!hits("ada example", &r));
    }

    #[test]
    fn co_authors_are_searched_like_the_author() {
        let r = row(
            "feat: share the work",
            &["Ada Lovelace", "Grace Hopper"],
            &["ada@example.com", "grace@example.com"],
            &[],
        );
        assert!(hits("hopper", &r));
        assert!(hits("grace@example.com", &r));
    }

    #[test]
    fn ref_names_match_anywhere_inside() {
        let r = row("", &[], &[], &["feature/topic-a", "v0.3-local", "HEAD"]);
        assert!(hits("topic", &r));
        assert!(hits("v0.3", &r));
        assert!(hits("head", &r), "the HEAD marker is a name on the row");
        assert!(!hits("feature topic", &r), "a refname holds no whitespace");
    }

    #[test]
    fn a_remote_branch_is_found_by_its_remote() {
        // The name carries the namespace, so the remote is searchable
        // without a field of its own (session::RefLabel).
        let r = row("", &[], &[], &["origin/main"]);
        assert!(hits("origin", &r));
        assert!(hits("origin/main", &r));
    }

    #[test]
    fn a_stash_is_found_by_the_message_the_row_shows() {
        // Keeping the selector out is the caller's (`GraphModel::hits`)
        // and tested there.
        let r = row("On main: the login refactor", &[], &[], &[]);
        assert!(hits("login refactor", &r));
    }

    #[test]
    fn an_object_name_matches_from_its_start() {
        let r = plain("");
        assert!(hits("3f2a", &r));
        assert!(hits("3F2A1B0C", &r), "however it was pasted");
        assert!(hits(OID, &r), "the whole of it");
        assert!(!hits("2a1b", &r), "not from the middle");
    }

    #[test]
    fn short_hex_is_not_an_object_name() {
        assert!(!hits("3f2", &plain("")));
        assert!(hits("3f2", &plain("commit 3f2 was the one")), "as prose");
    }

    #[test]
    fn a_word_that_happens_to_be_hex_still_searches_the_message() {
        // "added" is all hex; reading it as an object name can only add rows.
        let r = plain("feat: added the thing");
        assert!(hits("added", &r));
    }

    #[test]
    fn a_row_that_is_not_a_commit_matches_nothing() {
        let wip = Row {
            oid_hex: "",
            subject: "Uncommitted changes",
            ..Row::default()
        };
        assert!(!hits("uncommitted", &wip));
        assert!(!hits("0000", &wip));
    }

    #[test]
    fn folding_leaves_other_scripts_alone() {
        let r = plain("docs: 利用案内の骨子を日本語で直す");
        assert!(hits("骨子", &r));
        assert!(hits("日本語", &r));
        assert!(!hits("骨組", &r));
    }

    #[test]
    fn a_needle_never_lands_mid_character() {
        // 直 is E7 9B B4: its second byte is a continuation byte.
        let r = plain("直す");
        assert!(hits("直", &r));
        assert!(!hits("す直", &r));
    }
}
