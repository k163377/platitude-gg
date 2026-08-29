//! The one line-ending setting this app offers: `core.autocrlf`, read out
//! of one of git's own files and written back into it.
//!
//! **This is the settings screen's, and only the settings screen's.** The
//! warning that reads the same key (`attrs::normalises`) has no way in here
//! and is never getting one — a notice that offers to fix itself is where
//! every other GUI's line-ending accident starts (デザイン規約 §改行コード
//! の警告). What a reader changes here they came here to change.
//!
//! Nothing here converts a file. git does that, or does not, according to
//! what is written; all this does is write it.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

// The module that owns configuration owns the word for which of git's
// files (`config::ConfigScope`), and it is not public on its own — so
// every module that takes one keeps a door to it, the way `identity` does.
pub use crate::config::ConfigScope;

/// The key, spelled once.
const KEY: &str = "core.autocrlf";

/// Just it, for a `--get-regexp` pattern.
const PATTERN: &str = r"^core\.autocrlf$";

/// What the read is called when it fails, in front of a reader.
const READ_NAMED: &str = "git config --get-regexp core.autocrlf";

/// The three answers git takes for `core.autocrlf`.
///
/// Not a `bool` with a third state bolted on: `input` is neither of the
/// other two — it converts on the way into the index and leaves the
/// working tree alone — and a caller that only wants to know whether git
/// converts asks [`AutoCrlf::normalises`] rather than reading the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoCrlf {
    /// Store LF, check out CRLF.
    True,
    /// Store LF, and check out what was stored.
    Input,
    /// Store the file's own endings and check them out unchanged.
    False,
}

impl AutoCrlf {
    /// The word git stores, which is also the word the screen hands back.
    pub fn spelled(self) -> &'static str {
        match self {
            Self::True => "true",
            Self::Input => "input",
            Self::False => "false",
        }
    }

    /// A value back from whoever was shown [`AutoCrlf::spelled`]; empty
    /// asks for the key to be taken out, and so does anything unknown —
    /// the caller is the screen, and the screen can only offer these.
    pub fn spoken(word: &str) -> Option<Self> {
        match word.trim() {
            "true" => Some(Self::True),
            "input" => Some(Self::Input),
            "false" => Some(Self::False),
            _ => None,
        }
    }

    /// Whether git converts on the way into the index under this setting.
    ///
    /// `input` counts: it converts going in and not coming out, which is
    /// still git deciding what gets stored, so there is nothing for a
    /// notice to warn about either way.
    pub fn normalises(self) -> bool {
        matches!(self, Self::True | Self::Input)
    }

    /// What git makes of one `core.autocrlf` record, `None` where git is
    /// given nothing it can use.
    ///
    /// That covers two cases the screen cannot tell apart and does not
    /// need to: a key that is simply not there, and a key holding a word
    /// git itself refuses (実測 2.55: `INPUT` is accepted, `banana` is
    /// `fatal: bad boolean config value`). The second is a configuration
    /// git will not run on at all, so showing it as a fourth value the
    /// picker could return to would be offering to restore a broken file.
    ///
    /// Everything else is git's own boolean vocabulary, matched the way
    /// git matches it — case-insensitively, `input` first, and a valueless
    /// key as true (実測 2.55: `yes` / `on` / `1` / any non-zero number are
    /// true, `no` / `off` / `0` / an empty value are false).
    fn of_record(value: Option<&str>) -> Option<Self> {
        // A key written with no `=` at all. git reads it as true, the same
        // shape `identity` reads for `commit.gpgsign`.
        let Some(value) = value else {
            return Some(Self::True);
        };
        let value = value.trim();
        if value.eq_ignore_ascii_case("input") {
            return Some(Self::Input);
        }
        if ["true", "yes", "on"]
            .iter()
            .any(|word| value.eq_ignore_ascii_case(word))
        {
            return Some(Self::True);
        }
        if value.is_empty()
            || ["false", "no", "off"]
                .iter()
                .any(|word| value.eq_ignore_ascii_case(word))
        {
            return Some(Self::False);
        }
        match value.parse::<i64>() {
            Ok(0) => Some(Self::False),
            Ok(_) => Some(Self::True),
            Err(_) => None,
        }
    }

    /// The effective value of a `-z` read that may hold a record per level.
    ///
    /// **The last record wins**, which is the whole reason the read is
    /// never deduplicated (`config::parse_z_records`).
    fn of_records(out: &[u8]) -> Option<Self> {
        let mut held = None;
        for record in config::parse_z_records(out) {
            if record.key().trim() != KEY {
                continue;
            }
            held = Self::of_record(record.value());
        }
        held
    }
}

/// What one of git's own files sets, and nothing else.
///
/// Not [`effective`], which answers with what git would use here: a value
/// that is only inherited arrives there spelled exactly like one this file
/// wrote down, and telling those two apart is what the screen's empty row
/// means (the same distinction `identity::load_local` exists for).
pub async fn held(
    executor: &GitExecutor,
    workdir: &Path,
    scope: ConfigScope,
    cancel: &CancellationToken,
) -> Result<Option<AutoCrlf>, GitError> {
    let out = config::get_regexp_at(executor, workdir, scope, PATTERN, READ_NAMED, cancel).await?;
    Ok(AutoCrlf::of_records(&out))
}

/// What git would use in `workdir`, whichever of its files that came out
/// of — including the one the reader cannot write from here.
///
/// On Windows that last part is the common case rather than an edge: the
/// Git for Windows installer writes `core.autocrlf=true` into the system
/// configuration, so a global level that sets nothing still converts
/// (実測, and the trap `attrs::normalises` documents).
pub async fn effective(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<AutoCrlf>, GitError> {
    let out = config::get_regexp(executor, workdir, PATTERN, READ_NAMED, cancel).await?;
    Ok(AutoCrlf::of_records(&out))
}

/// What a [`set`] left behind.
///
/// Read back from git rather than echoed, for the reason the identity's
/// write reads itself back: the answer on screen has to be the file's, not
/// the screen's own idea of what it asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoCrlfWrite {
    /// What that file holds now.
    pub held: Option<AutoCrlf>,
    /// git reports what was asked for.
    pub saved: bool,
    /// git's own message when the write failed; empty when it did not.
    pub message: String,
}

/// Records `core.autocrlf` in one of git's files, where **`None` asks for
/// the key to be taken out** rather than set to nothing.
///
/// What comes back is what that file holds afterwards ([`held`]), not what
/// git would use: the question the screen asked was what this level sets,
/// and the answer to that cannot be read at the effective level.
///
/// **What the file already says is read first**, and a file that already
/// says it is left alone. Not an optimisation: `git config --unset` fails
/// when there was nothing to unset, and it fails with the *same* exit code
/// as its refusal to touch a key written more than once (実測 2.55: both
/// are 5). Asking first is what keeps those two apart — after it, a failed
/// unset is a real one and is reported as such.
///
/// One key, so unlike the identity there is no second pass: a lost lock is
/// a failure git names, and there is no half-written state for a retry to
/// settle (規約 core.md).
pub async fn set(
    executor: &GitExecutor,
    workdir: &Path,
    scope: ConfigScope,
    wanted: Option<AutoCrlf>,
    cancel: &CancellationToken,
) -> Result<AutoCrlfWrite, GitError> {
    let mut message = String::new();
    if held(executor, workdir, scope, cancel).await? != wanted
        && let Err(e) = write_key(executor, workdir, scope, wanted, cancel).await
    {
        // Shutting down is not a write that failed; it is no write.
        if e.is_cancelled() {
            return Err(e);
        }
        message = e.to_string();
    }
    let held = held(executor, workdir, scope, cancel).await?;
    Ok(AutoCrlfWrite {
        saved: held == wanted,
        held,
        message,
    })
}

/// The key, written or taken out.
///
/// The scope is spelled out on both, though git writes locally by default
/// (実測 2.55: a bare `--unset` of a key held only in the user's own file
/// exits 5 and leaves that file alone). The level a value is read back
/// from and the level it is written at are then named by the same word,
/// which is what stops the two from drifting apart later.
async fn write_key(
    executor: &GitExecutor,
    workdir: &Path,
    scope: ConfigScope,
    wanted: Option<AutoCrlf>,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir);
    let cmd = match wanted {
        // No `--` separator: `git config <key> -- <value>` stores "--" as
        // the value (the trap `identity::set_identity` documents).
        Some(value) => cmd.args(["config", scope.flag(), KEY, value.spelled()]),
        None => cmd.args(["config", scope.flag(), "--unset", KEY]),
    };
    executor.run(cmd, cancel).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(value: Option<&str>) -> Option<AutoCrlf> {
        AutoCrlf::of_record(value)
    }

    #[test]
    fn a_valueless_key_is_true() {
        assert_eq!(read(None), Some(AutoCrlf::True));
    }

    #[test]
    fn gits_own_boolean_spellings_are_matched_the_way_git_matches_them() {
        for word in ["true", "TRUE", "True", "yes", "on", "1", "42"] {
            assert_eq!(read(Some(word)), Some(AutoCrlf::True), "{word}");
        }
        for word in ["false", "False", "no", "off", "0", ""] {
            assert_eq!(read(Some(word)), Some(AutoCrlf::False), "{word}");
        }
        for word in ["input", "INPUT", "Input"] {
            assert_eq!(read(Some(word)), Some(AutoCrlf::Input), "{word}");
        }
    }

    /// A word git will not run on reads as nothing rather than as a fourth
    /// value: the picker can only offer what git accepts.
    #[test]
    fn a_word_git_refuses_is_no_answer() {
        assert_eq!(read(Some("banana")), None);
        assert_eq!(read(Some("1.5")), None);
    }

    #[test]
    fn the_last_record_is_the_effective_one() {
        let out = config::z(&["core.autocrlf\ntrue", "core.autocrlf\nfalse"]);
        assert_eq!(
            AutoCrlf::of_records(&out),
            Some(AutoCrlf::False),
            "git prints the lowest level first"
        );
        assert_eq!(AutoCrlf::of_records(&config::z(&[])), None);
    }

    /// Only `input` and `true` mean git decides what gets stored.
    #[test]
    fn normalising_is_what_converts_on_the_way_in() {
        assert!(AutoCrlf::True.normalises());
        assert!(AutoCrlf::Input.normalises());
        assert!(!AutoCrlf::False.normalises());
    }

    /// The screen hands back exactly what it was shown, and an empty
    /// answer is the row that takes the key out.
    #[test]
    fn a_spelled_value_comes_back_as_itself() {
        for value in [AutoCrlf::True, AutoCrlf::Input, AutoCrlf::False] {
            assert_eq!(AutoCrlf::spoken(value.spelled()), Some(value));
        }
        assert_eq!(AutoCrlf::spoken(""), None);
        assert_eq!(AutoCrlf::spoken("banana"), None);
    }
}
