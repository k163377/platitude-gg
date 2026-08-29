//! Streaming parser for `git log` output in our NUL-delimited format.
//!
//! The command is expected to run with `-z` (records terminated by NUL) and
//! [`LOG_FORMAT_ARG`] (fields separated by NUL), so the byte stream is a flat
//! sequence of NUL-terminated tokens with a fixed arity per record. The
//! parser accepts arbitrary chunk boundaries, including mid-token.

use crate::model::{CommitMeta, StrPool};
use crate::oid::Oid;

/// `--format=` argument matching [`LOG_FIELDS`]; keep the two in sync.
/// Fields: full id, parent ids, author name, author address, author time,
/// co-author trailers, body, subject.
///
/// The two author fields are the **mailmap** spellings (`%aN` / `%aE`, not
/// `%an` / `%ae`): one person committing under a laptop address and an
/// office one is one person, and git already has the file that says so.
/// Reimplementing that here would mean a second, worse answer to a
/// question `.mailmap` answers for every other tool on the machine.
///
/// Anything that writes an identity back — the author an amend carries
/// over — keeps reading the raw fields. A mapping made for display is not
/// a thing to record in a commit.
/// The `Co-authored-by` trailers ride along in the same record: the graph
/// draws the first of them on the node and the row's hover names the
/// rest, and asking git for them here costs no extra process. git decides
/// what counts as a trailer — see [`crate::details`], which reads the
/// same field for one commit at a time.
///
/// **No mailmap applies to these.** `%aN` folds the author, but a trailer
/// is message text, so the same person can appear under two spellings.
/// The body rides along too, for the row's hover. The window is 2,000
/// rows, so carrying it costs about a megabyte and saves a `git show`
/// per hover.
///
/// **`%b` still contains the trailers** (measured: it is `%B` minus the
/// subject, nothing else), so the credited lines are taken back out
/// here — the hover shows the writing, and the credits have their own
/// place in the card.
pub const LOG_FORMAT_ARG: &str = "--format=%H%x00%P%x00%aN%x00%aE%x00%at%x00\
     %(trailers:key=Co-authored-by,valueonly,unfold,separator=%x1F)%x00%b%x00%s";

/// Number of NUL-terminated tokens per record.
pub const LOG_FIELDS: usize = 8;

/// Fatal parse error: the stream no longer matches the expected shape.
/// (With NUL both separating fields and terminating records there is no way
/// to resynchronize, so the caller must abort the stream.)
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LogParseError {
    #[error("record {record}: invalid commit id field")]
    InvalidOid { record: usize },
    #[error("record {record}: invalid parent id field")]
    InvalidParent { record: usize },
    #[error("record {record}: invalid author timestamp")]
    InvalidTimestamp { record: usize },
    #[error("log output ended mid-record")]
    Truncated,
}

/// Incremental parser; feed raw stdout chunks, collect [`CommitMeta`].
#[derive(Debug, Default)]
pub struct LogParser {
    pool: StrPool,
    /// Unterminated token bytes carried across chunk boundaries.
    tail: Vec<u8>,
    /// Index of the next field within the current record (0..LOG_FIELDS).
    field: usize,
    /// Number of complete records parsed so far.
    records: usize,
    cur_oid: Option<Oid>,
    cur_parents: Vec<Oid>,
    cur_author: u32,
    cur_author_email: u32,
    /// Pooled (name, address) of each `Co-authored-by` on this record.
    cur_mates: Vec<(u32, u32)>,
    /// Message body with those trailer lines taken out.
    cur_body: String,
    cur_time: i64,
}

/// What a `Co-authored-by` line starts with, for taking them back out of
/// the body git hands over.
const CO_AUTHOR_KEY: &str = "co-authored-by:";

impl LogParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Author-name pool built up while parsing.
    pub fn pool(&self) -> &StrPool {
        &self.pool
    }

    /// Number of complete records parsed so far.
    pub fn records(&self) -> usize {
        self.records
    }

    /// Parses one stdout chunk, appending completed commits to `out`.
    pub fn feed(&mut self, chunk: &[u8], out: &mut Vec<CommitMeta>) -> Result<(), LogParseError> {
        let mut rest = chunk;
        while let Some(pos) = rest.iter().position(|b| *b == 0) {
            let part = &rest[..pos];
            if self.tail.is_empty() {
                self.take_field(part, out)?;
            } else {
                self.tail.extend_from_slice(part);
                let token = std::mem::take(&mut self.tail);
                self.take_field(&token, out)?;
            }
            rest = &rest[pos + 1..];
        }
        self.tail.extend_from_slice(rest);
        Ok(())
    }

    /// Must be called at end of stream; fails if a record was cut short.
    pub fn finish(&mut self) -> Result<(), LogParseError> {
        // git terminates the last record with NUL under `-z`; tolerate a
        // stray trailing newline just in case.
        let tail_is_noise = self.tail.iter().all(|b| b.is_ascii_whitespace());
        if self.field != 0 || !tail_is_noise {
            return Err(LogParseError::Truncated);
        }
        self.tail.clear();
        Ok(())
    }

    fn take_field(&mut self, token: &[u8], out: &mut Vec<CommitMeta>) -> Result<(), LogParseError> {
        let record = self.records;
        match self.field {
            0 => {
                self.cur_oid =
                    Some(Oid::from_hex(token).map_err(|_| LogParseError::InvalidOid { record })?);
            }
            1 => {
                self.cur_parents.clear();
                for hex in token.split(|b| *b == b' ').filter(|s| !s.is_empty()) {
                    self.cur_parents.push(
                        Oid::from_hex(hex).map_err(|_| LogParseError::InvalidParent { record })?,
                    );
                }
            }
            2 => {
                let name = String::from_utf8_lossy(token);
                self.cur_author = self.pool.intern(&name);
            }
            3 => {
                // Folded on the way in by the one function that decides
                // what counts as the same person, so a picture filed from
                // the settings list and a row read out of the log cannot
                // disagree about the key.
                let email = crate::avatar::key(&String::from_utf8_lossy(token));
                self.cur_author_email = self.pool.intern(&email);
            }
            4 => {
                let text = std::str::from_utf8(token)
                    .map_err(|_| LogParseError::InvalidTimestamp { record })?;
                self.cur_time = text
                    .trim()
                    .parse()
                    .map_err(|_| LogParseError::InvalidTimestamp { record })?;
            }
            6 => {
                // Only the lines git named as co-author trailers come
                // out — a `Co-authored-by:` written in the middle of the
                // prose is not one, and git already said so by not
                // returning it in field 5.
                let body = String::from_utf8_lossy(token);
                let kept: Vec<&str> = body
                    .lines()
                    .filter(|line| {
                        let l = line.trim();
                        // Compared as bytes: the key is ASCII, and a line
                        // of prose can have a character straddling the
                        // byte the key ends on (an em dash, a CJK word).
                        // Slicing the `str` there panics, and the panic
                        // is on the walk's own worker — the graph would
                        // stop at whatever it had streamed so far.
                        !(l.as_bytes().get(..CO_AUTHOR_KEY.len()).is_some_and(|head| {
                            head.eq_ignore_ascii_case(CO_AUTHOR_KEY.as_bytes())
                        }) && self
                            .cur_mates
                            .iter()
                            .any(|(name, _)| l.contains(self.pool.get(*name))))
                    })
                    .collect();
                self.cur_body = kept.join("\n").trim().to_string();
            }
            5 => {
                // Pool ids like the author's: the same few names repeat
                // down the whole history, so each costs one entry.
                self.cur_mates.clear();
                for mate in crate::details::parse_co_authors(&String::from_utf8_lossy(token)) {
                    let name = self.pool.intern(&mate.name);
                    let email = self.pool.intern(&mate.email);
                    self.cur_mates.push((name, email));
                }
            }
            _ => {
                let subject = String::from_utf8_lossy(token).into_owned().into_boxed_str();
                // Field 0 always ran before we get here, so cur_oid is set;
                // fall back to a parse error instead of unwrapping.
                let oid = self
                    .cur_oid
                    .take()
                    .ok_or(LogParseError::InvalidOid { record })?;
                out.push(CommitMeta {
                    oid,
                    parents: self.cur_parents.drain(..).collect(),
                    author: self.cur_author,
                    author_email: self.cur_author_email,
                    co_authors: self.cur_mates.drain(..).collect(),
                    body: std::mem::take(&mut self.cur_body).into_boxed_str(),
                    time: self.cur_time,
                    subject,
                });
                self.records += 1;
                self.field = 0;
                return Ok(());
            }
        }
        self.field += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const C: &str = "cccccccccccccccccccccccccccccccccccccccc";

    /// One record with an address derived from the name, for the tests
    /// that are not about the address.
    fn record(oid: &str, parents: &str, author: &str, time: &str, subject: &str) -> Vec<u8> {
        let email = format!("{}@example.com", author.to_lowercase());
        record_with_email(oid, parents, author, &email, time, subject)
    }

    fn record_with_email(
        oid: &str,
        parents: &str,
        author: &str,
        email: &str,
        time: &str,
        subject: &str,
    ) -> Vec<u8> {
        record_with_mates(oid, parents, author, email, time, "", "", subject)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one parameter per field of the record it builds; naming \
                  them at each call site is what makes the fixtures read"
    )]
    fn record_with_mates(
        oid: &str,
        parents: &str,
        author: &str,
        email: &str,
        time: &str,
        mates: &str,
        body: &str,
        subject: &str,
    ) -> Vec<u8> {
        let mut v = Vec::new();
        for field in [oid, parents, author, email, time, mates, body, subject] {
            v.extend_from_slice(field.as_bytes());
            v.push(0);
        }
        v
    }

    fn parse_all(bytes: &[u8], chunk_size: usize) -> (Vec<CommitMeta>, LogParser) {
        let mut parser = LogParser::new();
        let mut out = Vec::new();
        for chunk in bytes.chunks(chunk_size.max(1)) {
            parser.feed(chunk, &mut out).unwrap();
        }
        parser.finish().unwrap();
        (out, parser)
    }

    #[test]
    fn co_author_trailers_ride_along_in_the_record() {
        let bytes = record_with_mates(
            A,
            "",
            "Alice",
            "alice@example.com",
            "1700000000",
            "Claude Opus 5 <noreply@anthropic.com>\u{1f}Nameless",
            "Why it was done.\n\nCo-Authored-By: Claude Opus 5 <noreply@anthropic.com>\n\
             Co-authored-by: Nameless",
            "pair on it",
        );
        let (commits, parser) = parse_all(&bytes, bytes.len());
        assert_eq!(commits.len(), 1);
        let mates = &commits[0].co_authors;
        assert_eq!(mates.len(), 2);
        assert_eq!(parser.pool().get(mates[0].0), "Claude Opus 5");
        assert_eq!(parser.pool().get(mates[0].1), "noreply@anthropic.com");
        assert_eq!(parser.pool().get(mates[1].0), "Nameless");
        assert_eq!(parser.pool().get(mates[1].1), "");
        // git hands the trailers back inside %b as well; the body the
        // hover reads is the writing without them.
        assert_eq!(&*commits[0].body, "Why it was done.");
    }

    #[test]
    fn a_co_authored_by_in_the_prose_stays_in_the_body() {
        // git did not call it a trailer (field 5 is empty), so neither
        // does the filter — it is a sentence somebody wrote.
        let bytes = record_with_mates(
            A,
            "",
            "Alice",
            "alice@example.com",
            "1700000000",
            "",
            "I wrote Co-authored-by: nobody <n@e.com> in the body.",
            "alone",
        );
        let (commits, _) = parse_all(&bytes, bytes.len());
        assert!(commits[0].body.contains("Co-authored-by: nobody"));
    }

    #[test]
    fn a_body_line_that_is_not_ascii_where_the_key_ends_is_kept() {
        // The filter reads as many bytes as the key is long, so a line
        // whose character *straddles* that byte is the one that matters
        // — an em dash or a CJK character starting one or two bytes
        // short of the end. Both are everyday writing (this repository's
        // own history is full of the first), and slicing a `str` there
        // took the whole walk down with it: the graph never left its
        // loading ring (observed on platitude-gg itself).
        // The guard below is what keeps these honest: hand-counted bytes
        // stop reproducing the moment somebody rewords them.
        let bodies = [
            "The reason is—said plainly",
            "The reason is 版で書いてある",
            "co-authored-b—not the key",
        ];
        for body in bodies {
            assert!(
                !body.is_char_boundary(CO_AUTHOR_KEY.len()),
                "{body} does not reproduce: byte {} is a boundary",
                CO_AUTHOR_KEY.len()
            );
            let bytes = record_with_mates(
                A,
                "",
                "Alice",
                "alice@example.com",
                "1700000000",
                "",
                body,
                "wrote prose",
            );
            let (commits, _) = parse_all(&bytes, bytes.len());
            assert_eq!(commits.len(), 1, "{body}");
            assert_eq!(&*commits[0].body, body);
        }
    }

    #[test]
    fn a_commit_with_no_trailer_credits_nobody() {
        let bytes = record(A, "", "Alice", "1700000000", "alone");
        let (commits, _) = parse_all(&bytes, bytes.len());
        assert!(commits[0].co_authors.is_empty());
    }

    #[test]
    fn parses_multiple_records() {
        let mut bytes = record(A, &format!("{B} {C}"), "Alice", "1700000000", "merge two");
        bytes.extend(record(B, C, "Bob", "1699999940", "second"));
        bytes.extend(record(C, "", "Alice", "1699999880", "root"));

        let (commits, parser) = parse_all(&bytes, bytes.len());
        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0].oid.to_hex(), A);
        assert_eq!(commits[0].parents.len(), 2);
        assert_eq!(parser.pool().get(commits[0].author), "Alice");
        assert_eq!(commits[0].time, 1_700_000_000);
        assert_eq!(&*commits[0].subject, "merge two");
        assert_eq!(commits[2].parents.len(), 0);
        assert_eq!(commits[0].author, commits[2].author, "author interned");
    }

    #[test]
    fn the_address_is_lowercased_and_interned() {
        // The same person, shouting on one commit and not the other. The
        // key a picture is filed under has to come out the same either
        // way, which is why case is dropped on the way in.
        let mut bytes = record_with_email(A, "", "Alice", "Alice@Example.COM", "1", "one");
        bytes.extend(record_with_email(
            B,
            A,
            "Alice",
            "alice@example.com",
            "2",
            "two",
        ));
        let (commits, parser) = parse_all(&bytes, 3);
        assert_eq!(
            parser.pool().get(commits[0].author_email),
            "alice@example.com"
        );
        assert_eq!(
            commits[0].author_email, commits[1].author_email,
            "one address, one pool entry"
        );
    }

    #[test]
    fn an_empty_address_is_kept_as_one() {
        // git writes `<>` for an author with no address, and `%aE` comes
        // back empty. Nothing is a valid answer; it just matches no
        // picture.
        let bytes = record_with_email(A, "", "Nobody", "", "1", "one");
        let (commits, parser) = parse_all(&bytes, 7);
        assert_eq!(parser.pool().get(commits[0].author_email), "");
    }

    #[test]
    fn arbitrary_chunk_boundaries_do_not_matter() {
        let mut bytes = record(A, B, "Alice", "1", "one");
        bytes.extend(record(B, "", "日本語 太郎", "2", "日本語サブジェクト 🎌"));
        for chunk_size in [1, 2, 3, 7, 16, 64] {
            let (commits, parser) = parse_all(&bytes, chunk_size);
            assert_eq!(commits.len(), 2, "chunk size {chunk_size}");
            assert_eq!(parser.pool().get(commits[1].author), "日本語 太郎");
            assert_eq!(&*commits[1].subject, "日本語サブジェクト 🎌");
        }
    }

    #[test]
    fn empty_subject_is_preserved() {
        let bytes = record(A, "", "Alice", "1", "");
        let (commits, _) = parse_all(&bytes, 5);
        assert_eq!(&*commits[0].subject, "");
    }

    #[test]
    fn truncated_stream_is_an_error() {
        let bytes = record(A, "", "Alice", "1", "subject");
        let cut = &bytes[..bytes.len() - 3];
        let mut parser = LogParser::new();
        let mut out = Vec::new();
        parser.feed(cut, &mut out).unwrap();
        assert_eq!(parser.finish(), Err(LogParseError::Truncated));
    }

    #[test]
    fn trailing_newline_is_tolerated() {
        let mut bytes = record(A, "", "Alice", "1", "subject");
        bytes.push(b'\n');
        let mut parser = LogParser::new();
        let mut out = Vec::new();
        parser.feed(&bytes, &mut out).unwrap();
        parser.finish().unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn invalid_oid_is_fatal() {
        let bytes = record("zzzz", "", "Alice", "1", "subject");
        let mut parser = LogParser::new();
        let mut out = Vec::new();
        assert!(matches!(
            parser.feed(&bytes, &mut out),
            Err(LogParseError::InvalidOid { record: 0 })
        ));
    }

    #[test]
    fn invalid_timestamp_is_fatal() {
        let bytes = record(A, "", "Alice", "not-a-number", "subject");
        let mut parser = LogParser::new();
        let mut out = Vec::new();
        assert!(matches!(
            parser.feed(&bytes, &mut out),
            Err(LogParseError::InvalidTimestamp { record: 0 })
        ));
    }

    #[test]
    fn negative_timestamp_parses() {
        // Commits before the epoch exist in the wild.
        let bytes = record(A, "", "Alice", "-100", "old");
        let (commits, _) = parse_all(&bytes, 4);
        assert_eq!(commits[0].time, -100);
    }
}
