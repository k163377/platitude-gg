//! What one typed line matches, over the commits already on screen.
//!
//! The search runs no git: it reads the rows the graph window already
//! holds, so the answer arrives inside a keystroke and the walk is left
//! alone. What that buys, and what it costs, is written down where the
//! walk is (`session::DEFAULT_LOG_LIMIT`): nothing outside the loaded
//! window can be found here, and the bar says so rather than pretending.
//!
//! **One line, taken literally.** Not split into terms, not a pattern:
//! `fix the parser` looks for exactly that run of characters. Splitting on
//! spaces would have to decide whether the parts are ANDed across fields
//! ("this author AND that word"), and the answer differs per pair; naming
//! the field is what the advanced search is for.
//!
//! **The rule is per field, not per query.** A single blank cannot be a
//! refname or an address because neither can hold one, and four hex
//! characters are read as the start of an object name rather than as
//! prose that happens to be hex. Each field below says which shape of
//! query it can answer, so a query that answers none of them lights
//! nothing instead of lighting everything.

/// One typed line, ready to be asked of a row.
///
/// Built once per keystroke; the folding and the shape tests are done
/// here so that the per-row work is comparisons only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The line as typed, ASCII-folded. Whitespace is kept exactly:
    /// `fix ` is a search for `fix ` (the trailing space is how somebody
    /// asks for the word and not the prefix).
    needle: String,
    /// The line could be the start of an object name: hex only, and long
    /// enough that git itself would resolve it (`--abbrev` floors at 4).
    /// Shorter than that, one in sixteen rows would light up per
    /// character and the light would mean nothing.
    oid_prefix: bool,
}

/// The fields of one row a search can look at — all of them already in
/// memory, none of them needing git.
///
/// The three groups are the three rules: prose is searched anywhere
/// inside, tokens the same way but they cannot hold whitespace so a query
/// with one never lands there, and the object name only from its start.
#[derive(Debug, Default, Clone, Copy)]
pub struct Row<'a> {
    /// Full object name in lowercase hex. Empty for a row that is not a
    /// commit (the working-tree row), which matches nothing at all.
    pub oid_hex: &'a str,
    /// The message: first line, then the rest.
    pub subject: &'a str,
    pub body: &'a str,
    /// Everyone the row names — the author, then whoever the message
    /// credits. Prose: a name holds spaces, and people search for them.
    pub people: &'a [&'a str],
    /// Their addresses. Tokens.
    pub addresses: &'a [&'a str],
    /// The names standing on this row: branches, tags, the HEAD marker,
    /// and `stash@{n}` where the row is a stash. Tokens — git refuses a
    /// refname with whitespace in it (`check-ref-format`).
    pub tokens: &'a [&'a str],
}

impl Query {
    /// Reads a typed line. `None` when there is nothing to search for:
    /// empty, or whitespace only.
    ///
    /// Whitespace alone is not a search. Taken literally it would match
    /// the space in nearly every subject — every row lit, which is the
    /// same picture as no search at all but with the count claiming
    /// otherwise.
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
        // A row that is not a commit has none of these fields and must
        // not be caught by the object name either: the working-tree row
        // carries all-zero id, which `0000` would otherwise light.
        if row.oid_hex.is_empty() {
            return false;
        }
        self.at_start_of(row.oid_hex)
            || self.inside(row.subject)
            || self.inside(row.body)
            || row.people.iter().any(|p| self.inside(p))
            || row.addresses.iter().any(|a| self.inside(a))
            || row.tokens.iter().any(|t| self.inside(t))
    }

    /// Anywhere inside, ignoring case.
    ///
    /// Folding is ASCII only, which is the whole of it: the scripts that
    /// have no case (the CJK a subject is as likely to be written in)
    /// fold to themselves, and the ones that do case beyond ASCII are not
    /// worth a Unicode table on a path that runs on every keystroke.
    fn inside(&self, hay: &str) -> bool {
        contains_folded(hay, &self.needle)
    }

    /// From the start only, and only when the line could be an object
    /// name at all. Anywhere-inside would be meaningless: forty random
    /// hex characters contain every short run of them, so `abc` would
    /// light a fifth of any history for no reason a reader could see.
    /// Git resolves abbreviations the same way — from the front.
    fn at_start_of(&self, oid_hex: &str) -> bool {
        self.oid_prefix && oid_hex.starts_with(&self.needle)
    }
}

/// Shortest abbreviation git will resolve (`--abbrev` floors here), and
/// so the shortest run of hex this reads as an object name.
const MIN_OID_PREFIX: usize = 4;

/// Case-folded substring test over bytes.
///
/// Bytes rather than characters: a needle in valid UTF-8 can never start
/// at a continuation byte (those are 0x80..=0xBF and no leading byte is),
/// so a byte-level hit is always a real substring hit.
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
        body: &'a str,
        people: &'a [&'a str],
        addresses: &'a [&'a str],
        tokens: &'a [&'a str],
    ) -> Row<'a> {
        Row {
            oid_hex: OID,
            subject,
            body,
            people,
            addresses,
            tokens,
        }
    }

    fn plain(subject: &str) -> Row<'_> {
        row(subject, "", &[], &[], &[])
    }

    fn hits(query: &str, r: &Row<'_>) -> bool {
        Query::new(query).is_some_and(|q| q.matches(r))
    }

    #[test]
    fn nothing_to_search_for() {
        assert!(Query::new("").is_none());
        assert!(Query::new(" ").is_none());
        assert!(Query::new("\t \n").is_none());
        // A blank alone would otherwise match the space in almost every
        // subject there is.
        assert!(Query::new("　").is_none(), "an ideographic space too");
    }

    #[test]
    fn the_message_matches_anywhere_and_ignores_case() {
        let r = row("Fix the PARSER", "and the writer too", &[], &[], &[]);
        assert!(hits("fix", &r));
        assert!(hits("parser", &r));
        assert!(hits("PaRsEr", &r));
        assert!(hits("writer", &r), "the body counts as the message");
        assert!(!hits("reader", &r));
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
    fn people_are_prose_and_addresses_are_tokens() {
        let r = row(
            "",
            "",
            &["山田 太郎", "Ada Lovelace"],
            &["ada@example.com"],
            &[],
        );
        assert!(hits("山田 太郎", &r), "a name holds a space");
        assert!(hits("lovelace", &r));
        assert!(hits("ada@", &r));
        assert!(hits("example.com", &r));
        // An address cannot hold whitespace, so a query with one can
        // never be answered by this field — no gate needed for that, the
        // characters are simply not there.
        assert!(!hits("ada example", &r));
    }

    #[test]
    fn co_authors_are_searched_like_the_author() {
        let r = row(
            "feat: share the work",
            "",
            &["Ada Lovelace", "Grace Hopper"],
            &["ada@example.com", "grace@example.com"],
            &[],
        );
        assert!(hits("hopper", &r));
        assert!(hits("grace@example.com", &r));
    }

    #[test]
    fn ref_names_match_anywhere_inside() {
        let r = row("", "", &[], &[], &["feature/topic-a", "v0.3-local", "HEAD"]);
        assert!(hits("topic", &r));
        assert!(hits("v0.3", &r));
        assert!(hits("head", &r), "the HEAD marker is a name on the row");
        assert!(!hits("feature topic", &r), "a refname holds no whitespace");
    }

    #[test]
    fn a_stash_is_found_by_its_selector() {
        let r = row("WIP on main", "", &[], &[], &["stash@{0}"]);
        assert!(hits("stash@{0}", &r));
        assert!(hits("stash@", &r));
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
        // Three characters would light one row in every 4096 for no
        // reason the reader could see; git will not resolve them either.
        assert!(!hits("3f2", &plain("")));
        assert!(hits("3f2", &plain("commit 3f2 was the one")), "as prose");
    }

    #[test]
    fn a_word_that_happens_to_be_hex_still_searches_the_message() {
        // "added" is all hex digits. Reading it as an object name as well
        // can only add rows, never take one away.
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
        // The second byte of 直 (E7 9B B4) is 0x9B; nothing whose first
        // byte is a continuation byte can be a needle, so no hit can
        // start inside a character.
        let r = plain("直す");
        assert!(hits("直", &r));
        assert!(!hits("す直", &r));
    }
}
