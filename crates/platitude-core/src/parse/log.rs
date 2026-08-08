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
/// subject.
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
pub const LOG_FORMAT_ARG: &str = "--format=%H%x00%P%x00%aN%x00%aE%x00%at%x00%s";

/// Number of NUL-terminated tokens per record.
pub const LOG_FIELDS: usize = 6;

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
    cur_time: i64,
}

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
        let mut v = Vec::new();
        for field in [oid, parents, author, email, time, subject] {
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
