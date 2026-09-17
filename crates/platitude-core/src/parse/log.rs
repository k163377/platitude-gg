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
                // fall back to a parse error.
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
