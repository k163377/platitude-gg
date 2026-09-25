//! The one shape of configuration read this app makes:
//! `git config -z --get-regexp <pattern>`, and the records it answers with.
//!
//! [`ConfigScope`] lives here too: a narrowed read and a targeted write name
//! the same files, and one enum keeps the flags in step.

use std::borrow::Cow;
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Reads every key matching `pattern` from the configuration as git would
/// resolve it here, for [`parse_z_records`].
///
/// Nothing matching (exit 1) is an answer: empty output.
///
/// `named` is what a failure is called in the error the user sees, so each
/// caller names the keys it was after.
pub async fn get_regexp(
    executor: &GitExecutor,
    workdir: &Path,
    pattern: &str,
    named: &str,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
    read(executor, workdir, None, pattern, named, cancel).await
}

/// The same read, narrowed to one of git's configuration files — the only
/// way to tell a value that file wrote from an inherited one
/// ([`get_regexp`] does not say which file a record came from).
pub async fn get_regexp_at(
    executor: &GitExecutor,
    workdir: &Path,
    scope: ConfigScope,
    pattern: &str,
    named: &str,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
    read(executor, workdir, Some(scope), pattern, named, cancel).await
}

/// Which of git's configuration files a value is written to, and which one
/// a narrowed read is answered from.
///
/// `Local` and `Global` only — the screen offers only levels the person
/// can write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// This repository only.
    Local,
    /// The user's global configuration (the first-run prompt's default).
    Global,
}

impl ConfigScope {
    pub(crate) fn flag(self) -> &'static str {
        match self {
            Self::Local => "--local",
            Self::Global => "--global",
        }
    }
}

/// `None` reads every level at once, the way git resolves it here.
async fn read(
    executor: &GitExecutor,
    workdir: &Path,
    scope: Option<ConfigScope>,
    pattern: &str,
    named: &str,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["config", "-z"]);
    if let Some(scope) = scope {
        cmd = cmd.arg(scope.flag());
    }
    let cmd = cmd.args(["--get-regexp", pattern]);
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

/// `name` escaped to match itself inside a `--get-regexp` pattern (a POSIX
/// ERE): unescaped, `branch.wip.v2+x.pushremote` also matches another
/// branch's `branch.wipAv22x.pushremote`.
pub(crate) fn regexp_literal(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if matches!(
            c,
            '\\' | '^' | '$' | '.' | '[' | ']' | '|' | '(' | ')' | '*' | '+' | '?' | '{' | '}'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The records of a `-z` read, in the order git printed them: lowest level
/// first, so for a key set in several places the *last* record is the
/// effective one. Nothing here deduplicates or stops at a first match
/// (`eol::setting::effective` says what reading the first does on Windows).
pub fn parse_z_records(bytes: &[u8]) -> impl Iterator<Item = ConfigRecord<'_>> {
    bytes
        .split(|b| *b == 0)
        .filter(|record| !record.is_empty())
        .map(|record| ConfigRecord {
            text: String::from_utf8_lossy(record),
        })
}

/// One `key\nvalue` record; only the first newline separates.
pub struct ConfigRecord<'a> {
    text: Cow<'a, str>,
}

impl ConfigRecord<'_> {
    /// The key, lower-cased by `--get-regexp` (`tag.gpgSign` arrives as
    /// `tag.gpgsign`).
    pub fn key(&self) -> &str {
        match self.text.split_once('\n') {
            Some((key, _)) => key,
            None => &self.text,
        }
    }

    /// The value, or `None` for a key written without one (a bare `gpgsign`
    /// under `[commit]`; git's boolean keys read it as true — the caller
    /// decides).
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

    #[test]
    fn a_regexp_literal_spells_a_name_as_itself() {
        assert_eq!(regexp_literal("wip.v2+x"), r"wip\.v2\+x");
        assert_eq!(regexp_literal("a(b)|c{d}$e"), r"a\(b\)\|c\{d\}\$e");
        assert_eq!(
            regexp_literal("feature/plain-name_1"),
            "feature/plain-name_1"
        );
    }
}
