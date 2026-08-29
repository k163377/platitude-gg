//! The one shape of configuration read this app makes:
//! `git config -z --get-regexp <pattern>`, and the records it answers with.
//!
//! Seven callers ask git for a group of keys this way — the identity and
//! its signing keys, the remotes, what a branch tracks, `core.autocrlf`,
//! the merge tools someone wrote a command for, the identity one
//! repository sets for itself, and the line-ending setting one file sets
//! for itself. What they do with the answer differs; how it is asked for
//! and how it arrives does not.
//!
//! The word for **which of git's files** is here as well ([`ConfigScope`]),
//! because a read narrowed to one of them and a write aimed at one of them
//! are the same question asked twice — and two enums for it would be two
//! spellings of `--global` to keep in step.

use std::borrow::Cow;
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Reads every key matching `pattern` from the configuration as git would
/// resolve it here, for [`parse_z_records`].
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
    read(executor, workdir, None, pattern, named, cancel).await
}

/// The same read, narrowed to one of git's configuration files.
///
/// A door of its own rather than a flag on [`get_regexp`], because the two
/// answer different questions and every caller knows which one it came
/// for. [`get_regexp`] cannot answer this one at all: a key set in two
/// places arrives twice with no word for which file either record came out
/// of, so a value that is only inherited reads there exactly like one the
/// named file wrote down.
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
/// There is deliberately no `System` — nothing this app writes belongs to
/// the machine rather than to the person, and a level nobody can write is
/// a level the screen cannot offer to give back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// This repository only.
    Local,
    /// The user's global configuration — the right default for a first-run
    /// prompt, since the answer is about the person, not the project.
    Global,
}

impl ConfigScope {
    /// The flag that names this file to `git config`.
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

/// `name` escaped to match itself inside a `--get-regexp` pattern.
///
/// The pattern is a POSIX ERE (the remotes read relies on `(url|pushurl)`
/// grouping), and a branch name may hold characters it gives meaning to —
/// `.` in any dotted name, `+`, braces. measured 2.55: asking for
/// `branch.wip.v2+x.pushremote` unescaped answers with
/// `branch.wipAv22x.pushremote`, another branch's mark.
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

    /// Every ERE metacharacter a ref name can carry, spelled literally.
    /// Names that carry none pass through untouched.
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
