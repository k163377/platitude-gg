//! The one shape of configuration read this app makes:
//! `git config -z --get-regexp <pattern>`, and the records it answers with.
//!
//! Five callers ask git for a group of keys this way — the identity and its
//! signing keys, the remotes, what a branch tracks, `core.autocrlf`, the
//! merge tools someone wrote a command for. What they do with the answer
//! differs; how it is asked for and how it arrives does not.

use std::borrow::Cow;
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Reads every key matching `pattern`, for [`parse_z_records`].
///
/// **Nothing matching is an answer, not a failure**: git exits 1 for it,
/// every caller here asks about keys that are usually unset, and the empty
/// output comes back as such. The command says so as well
/// (`GitCommand::answers_by_code`), or the command log raises itself over
/// every one of those answers (規約 core.md §終了コードで答える問い合わせ).
///
/// `named` is what the read is called if it does fail — that string is the
/// one a UI error puts in front of the user, so each caller names the keys
/// it was after rather than the regex it got there with.
pub async fn get_regexp(
    executor: &GitExecutor,
    workdir: &Path,
    pattern: &str,
    named: &str,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "config",
        "-z",
        "--get-regexp",
        pattern,
    ]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(out.stdout),
        1 => Ok(Vec::new()),
        code => Err(GitError::Failed {
            command: named.to_string(),
            code,
            stderr: out.failure_message(),
        }),
    }
}

/// The records of a `-z` read, in the order git printed them.
///
/// **That order is part of the answer.** git prints what every
/// configuration level has to say, lowest level first, so a key set in more
/// than one place arrives more than once and the *last* record is the
/// effective value — which is why nothing here deduplicates or stops at a
/// first match (`eol::attrs::normalises` documents what reading the first
/// one does to Windows).
pub fn parse_z_records(bytes: &[u8]) -> impl Iterator<Item = ConfigRecord<'_>> {
    bytes
        .split(|b| *b == 0)
        .filter(|record| !record.is_empty())
        .map(|record| ConfigRecord {
            text: String::from_utf8_lossy(record),
        })
}

/// One `key\nvalue` record.
///
/// `-z` puts the separating newline *inside* the record, so a value holding
/// newlines of its own cannot be mistaken for the next key.
pub struct ConfigRecord<'a> {
    text: Cow<'a, str>,
}

impl ConfigRecord<'_> {
    /// The key, as git spells it — `--get-regexp` lower-cases it, so
    /// `tag.gpgSign` arrives as `tag.gpgsign`.
    pub fn key(&self) -> &str {
        match self.text.split_once('\n') {
            Some((key, _)) => key,
            None => &self.text,
        }
    }

    /// The value, or `None` for a key written without one (a bare
    /// `gpgsign` under `[commit]`), which git spells as a record with no
    /// newline in it at all.
    ///
    /// What that absence means belongs to the caller: to git's boolean keys
    /// it reads as true, and to a key that wanted a string there is nothing
    /// there to take.
    pub fn value(&self) -> Option<&str> {
        self.text.split_once('\n').map(|(_, value)| value)
    }
}

/// Builds `-z` output from its records, for the parsing tests.
#[cfg(test)]
pub(crate) fn z(records: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for r in records {
        v.extend_from_slice(r.as_bytes());
        v.push(0);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(bytes: &[u8]) -> Vec<(String, Option<String>)> {
        parse_z_records(bytes)
            .map(|r| (r.key().to_string(), r.value().map(str::to_string)))
            .collect()
    }

    #[test]
    fn a_record_is_a_key_and_a_value() {
        assert_eq!(
            split(&z(&["user.name\nAda Lovelace"])),
            vec![("user.name".to_string(), Some("Ada Lovelace".to_string()))]
        );
    }

    #[test]
    fn a_key_written_without_a_value_has_none() {
        assert_eq!(
            split(&z(&["commit.gpgsign"])),
            vec![("commit.gpgsign".to_string(), None)],
            "which is not the same record as one whose value is empty"
        );
        assert_eq!(
            split(&z(&["commit.gpgsign\n"])),
            vec![("commit.gpgsign".to_string(), Some(String::new()))]
        );
    }

    #[test]
    fn a_value_may_hold_newlines() {
        assert_eq!(
            split(&z(&["alias.lg\nlog \\\n--oneline"])),
            vec![(
                "alias.lg".to_string(),
                Some("log \\\n--oneline".to_string())
            )],
            "only the first newline separates; the rest is value"
        );
    }

    #[test]
    fn every_level_is_kept_in_the_order_git_printed_it() {
        assert_eq!(
            split(&z(&["core.autocrlf\ntrue", "core.autocrlf\nfalse"])),
            vec![
                ("core.autocrlf".to_string(), Some("true".to_string())),
                ("core.autocrlf".to_string(), Some("false".to_string())),
            ]
        );
    }

    #[test]
    fn an_empty_read_has_no_records() {
        assert!(split(b"").is_empty());
    }
}
