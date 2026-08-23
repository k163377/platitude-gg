//! The push marks: where a push goes when configuration, rather than the
//! command line, decides it — reading them, setting the repository's, and
//! clearing it.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// The remote a push goes to when no branch says otherwise
/// (`remote.pushDefault`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PushDefault {
    /// The remote it names. **git does not check that one exists**: a name
    /// no remote holds is taken for a URL, and the push fails at the
    /// connection instead (実測 2.55: `'nope' does not appear to be a git
    /// repository`).
    pub remote: String,
    /// Whether this repository's own config is what says so.
    ///
    /// A value set anywhere else cannot be cleared from here. git has no
    /// local spelling for "not set" — an empty local value is not "unset"
    /// but "no destination at all", and a plain `git push` then fails with
    /// `No configured push destination.` (実測). Marking another remote is
    /// the only move a repository has against a global value.
    pub local: bool,
}

/// Reads `remote.pushDefault`, and which level of configuration set it.
///
/// `--get` answers with the effective value alone, so the pair that comes
/// back names the level that decided it — the pair, because `-z` writes
/// `<scope>\0<value>\0` (実測 2.55: `local\0origin\0`; unset is exit 1 with
/// nothing on stdout, which is an answer).
///
/// A key written without a remote name behind it (`remote.pushDefault=`)
/// answers `None`: no remote is called that, so there is nothing here to
/// point at. What git does with it is refuse the push outright, and only
/// git can say that (実測).
pub async fn push_default(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<PushDefault>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // Unset is the usual state and it is an answer, not a failure.
        .answers_by_code(1)
        .args([
            "config",
            "-z",
            "--show-scope",
            "--get",
            "remote.pushDefault",
        ]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(parse_push_default(&out.stdout)),
        1 => Ok(None),
        code => Err(GitError::Failed {
            command: "git config --get remote.pushDefault".to_string(),
            code,
            stderr: out.failure_message(),
        }),
    }
}

fn parse_push_default(bytes: &[u8]) -> Option<PushDefault> {
    let mut fields = bytes
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty())
        .map(String::from_utf8_lossy);
    let scope = fields.next()?;
    let remote = fields.next()?.trim().to_string();
    (!remote.is_empty()).then(|| PushDefault {
        remote,
        // Every other level — global, system, worktree, a `-c` on the
        // command line — is one this repository cannot unset.
        local: scope == "local",
    })
}

/// Both marks at once: the branch's own `branch.<branch>.pushRemote`, and
/// the repository's [`PushDefault`] beside it.
///
/// The one reader of the branch key, and one process for the pair.
/// [`super::plan_current_push`] runs it before a send, and the status tick
/// runs it so the toolbar names the same destination and the refs snapshot
/// hears about a mark moved from a terminal — in **any** scope, which is
/// what the remotes cache cannot see on its own (its invalidation stats
/// the repository's config file alone, and `git config --global
/// remote.pushDefault` writes a different one). A second, separate
/// spelling of either key is how the label comes to name a remote the
/// push never goes to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushMarks {
    /// The branch's own mark, which beats every other (git-config(5); the
    /// order is [`super::plan_current_push`]'s, 実測 2.55). `None` where
    /// the branch does not mark one.
    pub push_remote: Option<String>,
    /// The repository's mark, with the level that set it.
    pub push_default: Option<PushDefault>,
}

/// Reads both marks with one `--get-regexp` over the two exact keys.
///
/// `-z --show-scope --get-regexp` writes `<scope>\0<key>\n<value>\0` per
/// record, every level in precedence order, so the last record per key is
/// the effective value (実測 2.55:
/// `global\0remote.pushdefault\nfork\0local\0remote.pushdefault\nhome\0`
/// — `home` wins). Keys arrive with section and variable lower-cased and
/// the branch name spelled as it was written; the pattern embeds the
/// branch name escaped, because the pattern is a regex and the name may
/// hold `.` or `+` (`config::regexp_literal` — 実測: unescaped,
/// `wip.v2+x` answers with `wipAv22x`'s mark). Neither key set is exit 1,
/// which is an answer.
pub async fn push_marks(
    executor: &GitExecutor,
    workdir: &Path,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<PushMarks, GitError> {
    let pattern = format!(
        r"^(branch\.{}\.pushremote|remote\.pushdefault)$",
        config::regexp_literal(branch)
    );
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["config", "-z", "--show-scope", "--get-regexp"])
        .arg(pattern);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(parse_push_marks(branch, &out.stdout)),
        1 => Ok(PushMarks::default()),
        code => Err(GitError::Failed {
            command: format!(
                "git config --get-regexp branch.{branch}.pushRemote remote.pushDefault"
            ),
            code,
            stderr: out.failure_message(),
        }),
    }
}

fn parse_push_marks(branch: &str, bytes: &[u8]) -> PushMarks {
    let branch_key = format!("branch.{branch}.pushremote");
    let mut marks = PushMarks::default();
    let mut fields = bytes
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty())
        .map(String::from_utf8_lossy);
    while let (Some(scope), Some(record)) = (fields.next(), fields.next()) {
        // The separating newline is inside the record; a key written with
        // no value at all has no newline in its record (実測 2.55: a bare
        // `pushDefault` line arrives as `local\0remote.pushdefault\0`).
        let (key, value) = match record.as_ref().split_once('\n') {
            Some((key, value)) => (key, Some(value)),
            None => (record.as_ref(), None),
        };
        let value = value.map(str::trim).filter(|value| !value.is_empty());
        // Later records overwrite earlier ones on purpose: levels arrive
        // lowest first, and the last one per key is the effective value.
        if key == branch_key {
            marks.push_remote = value.map(str::to_string);
        } else if key == "remote.pushdefault" {
            marks.push_default = value.map(|remote| PushDefault {
                remote: remote.to_string(),
                // Every other level — global, system, worktree, a `-c` on
                // the command line — is one this repository cannot unset.
                local: scope == "local",
            });
        }
    }
    marks
}

/// `git config remote.pushDefault <name>` — marks where pushes go.
///
/// The old spelling on purpose (規約 git最低バージョン整合: `git config
/// set` is 2.46). No `--end-of-options`: git stops looking for options
/// after the key, so a remote actually named `-x` — which `remote add`
/// will make — is taken as the value (実測 2.55).
pub async fn set_push_default(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["config", "remote.pushDefault", name]);
    executor.run(cmd, cancel).await?;
    Ok(())
}

/// `git config --unset remote.pushDefault`.
///
/// The key not being set is the state the caller asked for, and git says so
/// with exit 5 (実測) rather than a failure.
pub async fn clear_push_default(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir).answers_by_code(5).args([
        "config",
        "--unset",
        "remote.pushDefault",
    ]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 | 5 => Ok(()),
        code => Err(GitError::Failed {
            command: "git config --unset remote.pushDefault".to_string(),
            code,
            stderr: out.failure_message(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded from git 2.55: `config -z --show-scope --get` writes the
    /// level and the value as two NUL-terminated fields.
    #[test]
    fn a_push_default_names_its_remote_and_its_level() {
        let read = parse_push_default(b"local\0fork\0").expect("a value");
        assert_eq!(read.remote, "fork");
        assert!(read.local, "the repository's own config can be unset here");
    }

    #[test]
    fn a_push_default_from_anywhere_else_is_not_local() {
        for scope in ["global", "system", "worktree", "command"] {
            let bytes = format!("{scope}\0fork\0").into_bytes();
            let read = parse_push_default(&bytes).expect("a value");
            assert!(!read.local, "{scope} is not this repository's config");
        }
    }

    /// A remote may be named `-x` (`remote add --end-of-options` makes one),
    /// and the value comes back as it was written.
    #[test]
    fn a_dashed_remote_name_survives_the_read() {
        let read = parse_push_default(b"local\0-x\0").expect("a value");
        assert_eq!(read.remote, "-x");
    }

    /// The key written with nothing behind it. No remote is called that, so
    /// there is nothing to mark — git refuses the push, and only git can
    /// say so.
    #[test]
    fn a_push_default_without_a_remote_names_nothing() {
        assert!(parse_push_default(b"local\0\0").is_none());
        assert!(parse_push_default(b"local\0").is_none());
        assert!(parse_push_default(b"").is_none());
    }

    /// Recorded from git 2.55: `config -z --show-scope --get-regexp`
    /// writes `<scope>\0<key>\n<value>\0` per record, section and
    /// variable lower-cased, the branch name as written.
    #[test]
    fn the_marks_read_takes_both_keys_from_one_answer() {
        let marks = parse_push_marks(
            "main",
            b"global\0remote.pushdefault\nfork\0local\0branch.main.pushremote\norigin\0",
        );
        assert_eq!(marks.push_remote.as_deref(), Some("origin"));
        let marked = marks.push_default.expect("a mark");
        assert_eq!(marked.remote, "fork");
        assert!(!marked.local, "the global level set it");
    }

    /// Levels arrive lowest first, so the last record per key is the
    /// effective value (実測: local `home` prints after global `fork`).
    #[test]
    fn the_last_record_per_key_is_the_effective_one() {
        let marks = parse_push_marks(
            "main",
            b"global\0remote.pushdefault\nfork\0local\0branch.main.pushremote\norigin\0\
              local\0remote.pushdefault\nhome\0",
        );
        let marked = marks.push_default.expect("a mark");
        assert_eq!(marked.remote, "home");
        assert!(marked.local);
    }

    /// The two spellings of "nothing here": an empty local value (the
    /// "no destination at all" state) and a key written bare, which
    /// arrives as a record with no newline in it (実測 2.55). Both must
    /// override a level below rather than fall back to it.
    #[test]
    fn an_empty_or_bare_key_names_nothing_and_still_overrides() {
        let empty = parse_push_marks(
            "main",
            b"global\0remote.pushdefault\nfork\0local\0remote.pushdefault\n\0",
        );
        assert!(empty.push_default.is_none());
        let bare = parse_push_marks(
            "main",
            b"global\0remote.pushdefault\nfork\0local\0remote.pushdefault\0",
        );
        assert!(bare.push_default.is_none());
        assert!(parse_push_marks("main", b"").push_remote.is_none());
    }

    /// The branch name keeps its case in the key, and the match is exact:
    /// `Topic`'s mark is not `topic`'s (実測 2.55).
    #[test]
    fn a_branch_keeps_its_case_and_another_case_is_another_branch() {
        let bytes: &[u8] = b"local\0branch.Topic.pushremote\nfork\0";
        assert_eq!(
            parse_push_marks("Topic", bytes).push_remote.as_deref(),
            Some("fork")
        );
        assert!(parse_push_marks("topic", bytes).push_remote.is_none());
    }

    /// A branch name holding regex metacharacters is compared literally —
    /// the escaping lives in the pattern, the parse must not re-interpret.
    #[test]
    fn a_metacharacter_branch_name_is_matched_literally() {
        let bytes: &[u8] = b"local\0branch.wip.v2+x.pushremote\nfork\0";
        assert_eq!(
            parse_push_marks("wip.v2+x", bytes).push_remote.as_deref(),
            Some("fork")
        );
        assert!(parse_push_marks("wip", bytes).push_remote.is_none());
    }

    /// The separating newline is inside the record, so a value holding
    /// newlines of its own cannot shift the scope/record pairing
    /// (実測 2.55: `local\0remote.pushdefault\nfork\nx\0`).
    #[test]
    fn a_value_holding_a_newline_does_not_shift_the_pairing() {
        let marks = parse_push_marks(
            "main",
            b"local\0remote.pushdefault\nfork\nx\0local\0branch.main.pushremote\norigin\0",
        );
        assert_eq!(marks.push_default.expect("a mark").remote, "fork\nx");
        assert_eq!(marks.push_remote.as_deref(), Some("origin"));
    }
}
