//! The one line-ending setting this app offers: `core.autocrlf`, read out
//! of one of git's own files and written back into it.
//!
//! Only the settings screen writes it; the warning that reads the same key
//! (`attrs::normalises`) never offers a fix (デザイン規約 §改行コードの警告).
//! Nothing here converts a file — git does, according to what is written.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

// `config` is not public, so every module that takes a scope re-exports it
// (as `identity` does).
pub use crate::config::ConfigScope;

/// The key, spelled once.
const KEY: &str = "core.autocrlf";

/// Just it, for a `--get-regexp` pattern.
const PATTERN: &str = r"^core\.autocrlf$";

/// What the read is called when it fails, in front of a reader.
const READ_NAMED: &str = "git config --get-regexp core.autocrlf";

/// The three answers git takes for `core.autocrlf`. Whether git converts
/// at all is [`AutoCrlf::normalises`].
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
    /// `input` counts: git still decides what gets stored.
    pub fn normalises(self) -> bool {
        matches!(self, Self::True | Self::Input)
    }

    /// What git makes of one `core.autocrlf` record, in git's own boolean
    /// vocabulary (rules-refs/core.md「`core.autocrlf` の綴りは git の bool 語彙」).
    ///
    /// `None` for a word git refuses: git will not run on that file, so it
    /// reads like an unset key rather than a fourth value the picker could
    /// restore.
    fn of_record(value: Option<&str>) -> Option<Self> {
        // A key written with no `=` at all: git reads it as true.
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

    /// The effective value of a `-z` read that may hold a record per level:
    /// the last record wins (`config::parse_z_records`).
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
/// Not [`effective`]: there an inherited value reads exactly like one this
/// file holds, and the screen's empty row is that difference.
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
/// On Windows that last part is the common case: the Git for Windows
/// installer writes `core.autocrlf=true` into the system configuration,
/// so a global level that sets nothing still converts.
pub async fn effective(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<AutoCrlf>, GitError> {
    let out = config::get_regexp(executor, workdir, PATTERN, READ_NAMED, cancel).await?;
    Ok(AutoCrlf::of_records(&out))
}

/// What a [`set`] left behind, read back from git: the answer on screen
/// has to be the file's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoCrlfWrite {
    /// What that file holds now.
    pub held: Option<AutoCrlf>,
    /// git reports what was asked for.
    pub saved: bool,
    /// git's own message when the write failed; empty when it did not.
    pub message: String,
}

/// Records `core.autocrlf` in one of git's files; `None` takes the key out.
///
/// Returns what that file holds afterwards ([`held`]), not the effective
/// value: the screen asked what this level sets.
///
/// The file is read first and left alone when it already says it — not an
/// optimisation: `--unset` of an absent key exits like its refusal of a
/// multi-valued key. One key, so no retry pass: a lost lock leaves nothing
/// half-written (rules-refs/core.md「`core.autocrlf` の書きは 1 キーなので 1 発」).
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

/// The key, written or taken out, with the scope spelled out on both
/// though git defaults to local: the level read back and the level written
/// are then named by the same word.
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
        // the value.
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

    #[test]
    fn normalising_is_what_converts_on_the_way_in() {
        assert!(AutoCrlf::True.normalises());
        assert!(AutoCrlf::Input.normalises());
        assert!(!AutoCrlf::False.normalises());
    }

    #[test]
    fn a_spelled_value_comes_back_as_itself() {
        for value in [AutoCrlf::True, AutoCrlf::Input, AutoCrlf::False] {
            assert_eq!(AutoCrlf::spoken(value.spelled()), Some(value));
        }
        assert_eq!(AutoCrlf::spoken(""), None);
        assert_eq!(AutoCrlf::spoken("banana"), None);
    }
}
