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
/// The author fields are the mailmap spellings (`%aN` / `%aE`): one
/// person under two addresses is one person, and `.mailmap` already says
/// so. Anything that writes an identity back (the author an amend carries
/// over) reads the raw fields — a display mapping is not to be recorded
/// in a commit.
///
/// The `Co-authored-by` trailers ride along at no extra process; git
/// decides what counts as a trailer, as in [`crate::details`]. No mailmap
/// applies to them: a trailer is message text.
///
/// The body rides along for the row's hover, instead of a `git show` per
/// hover. `%b` still contains the trailers (it is `%B` minus the
/// subject), so the credited lines are taken back out here.
pub const LOG_FORMAT_ARG: &str = "--format=%H%x00%P%x00%aN%x00%aE%x00%at%x00\
     %(trailers:key=Co-authored-by,valueonly,unfold,separator=%x1F)%x00%b%x00%s";

/// Number of NUL-terminated tokens per record.
pub const LOG_FIELDS: usize = 8;

/// Fatal parse error: with NUL both separating fields and terminating
/// records there is no resynchronizing, so the caller must abort the stream.
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
        // Tolerate a stray trailing newline after the last NUL.
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
                // Folded by the one function that decides the key, so a
                // picture filed from the settings and a log row agree.
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
                // Only the trailers git returned in field 5 come out; a
                // `Co-authored-by:` in the prose stays.
                let body = String::from_utf8_lossy(token);
                let kept: Vec<&str> = body
                    .lines()
                    .filter(|line| {
                        let l = line.trim();
                        // Compared as bytes: slicing the `str` panics on a
                        // character straddling the key's end, and on the
                        // walk's worker that stops the graph.
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
                // Pooled like the author: the same few names repeat.
                self.cur_mates.clear();
                for mate in crate::details::parse_co_authors(&String::from_utf8_lossy(token)) {
                    let name = self.pool.intern(&mate.name);
                    let email = self.pool.intern(&mate.email);
                    self.cur_mates.push((name, email));
                }
            }
            _ => {
                let subject = String::from_utf8_lossy(token).into_owned().into_boxed_str();
                // Field 0 always ran first; the error is a fallback.
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
#[path = "log_tests.rs"]
mod tests;
