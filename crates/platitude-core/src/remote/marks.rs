//! The marks: where a plain `git push` goes when configuration decides
//! it, and which remote this repository calls origin — reading them,
//! setting the repository's, and clearing it.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// The remote a push goes to when no branch says otherwise
/// (`remote.pushDefault`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PushDefault {
    /// The remote it names. git does not check that one exists: an unknown
    /// name is taken for a URL and the push fails at the connection.
    pub remote: String,
    /// Whether this repository's own config says so.
    ///
    /// A value from any other level cannot be cleared from here: git has no
    /// local "unset" (an empty local value means "no destination" and a
    /// plain push fails), so marking another remote is the only move.
    pub local: bool,
}

impl PushDefault {
    fn at(scope: &str, remote: &str) -> Self {
        Self {
            remote: remote.to_string(),
            local: scope == "local",
        }
    }
}

/// The keys `Mark as origin` writes ([`mark_origin`]), as this repository
/// reads them.
///
/// Two because the role answers two questions in git: where a push goes
/// (`remote.pushDefault`) and which remote `git switch <name>` takes a
/// name carried by several from (`checkout.defaultRemote`). Read as two
/// because `git remote rename` / `remove` update only
/// `remote.pushDefault`, so a remote can hold one without the other.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginMarks {
    /// The push's, with the level that set it.
    pub push_default: Option<PushDefault>,
    /// The remote `checkout.defaultRemote` names. Its level is not kept:
    /// all it decides here is whether marking that remote still has
    /// anything to write.
    pub checkout_default: Option<String>,
}

/// Reads both [`OriginMarks`] keys with one `--get-regexp`, and which
/// level set the push's.
///
/// A key with no remote name behind it (`remote.pushDefault=`) answers
/// `None`: there is nothing to point at, and git refuses the push itself.
pub async fn origin_marks(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<OriginMarks, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // Exit 1: neither key is set.
        .answers_by_code(1)
        .args([
            "config",
            "-z",
            "--show-scope",
            "--get-regexp",
            r"^(remote\.pushdefault|checkout\.defaultremote)$",
        ]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(parse_origin_marks(&out.stdout)),
        1 => Ok(OriginMarks::default()),
        code => Err(GitError::Failed {
            command: "git config --get-regexp remote.pushDefault checkout.defaultRemote"
                .to_string(),
            code,
            stderr: out.failure_message(),
        }),
    }
}

fn parse_origin_marks(bytes: &[u8]) -> OriginMarks {
    let mut marks = OriginMarks::default();
    each_scoped_record(bytes, |scope, key, value| match key {
        "remote.pushdefault" => {
            marks.push_default = value.map(|remote| PushDefault::at(scope, remote));
        }
        "checkout.defaultremote" => marks.checkout_default = value.map(str::to_string),
        _ => {}
    });
    marks
}

/// Walks a `-z --show-scope --get-regexp` answer: `<scope>\0<key>\n<value>\0`
/// per record, levels lowest first, so the last record per key is the
/// effective value.
///
/// An empty or blank value, and a bare key (a record with no newline),
/// come through as `None` and still override a level below.
fn each_scoped_record(bytes: &[u8], mut each: impl FnMut(&str, &str, Option<&str>)) {
    let mut fields = bytes
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty())
        .map(String::from_utf8_lossy);
    while let (Some(scope), Some(record)) = (fields.next(), fields.next()) {
        // The separating newline is inside the record.
        let (key, value) = match record.as_ref().split_once('\n') {
            Some((key, value)) => (key, Some(value)),
            None => (record.as_ref(), None),
        };
        each(
            &scope,
            key,
            value.map(str::trim).filter(|value| !value.is_empty()),
        );
    }
}

/// Every key a push of the branch is resolved from: the branch's own
/// `branch.<branch>.pushRemote`, the repository's [`PushDefault`] beside
/// it, and what the branch tracks (`branch.<branch>.remote` / `.merge`).
///
/// The one reader of the branch keys, shared by
/// [`super::plan_current_push`] and the status tick: a second spelling lets
/// the label name a remote the push never goes to
/// (rules-refs/core.md「push の印の読みは `remote::push_marks` 1 本だけ」).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushMarks {
    /// The branch's own mark, which beats every other (order:
    /// [`super::plan_current_push`]). `None` where unset.
    pub push_remote: Option<String>,
    /// The repository's mark, with the level that set it.
    pub push_default: Option<PushDefault>,
    /// `branch.<branch>.remote`: the remote the branch tracks.
    pub tracks: Option<String>,
    /// `branch.<branch>.merge`: the branch it tracks there, as a full ref.
    pub merge: Option<String>,
}

/// Reads every [`PushMarks`] key with one `--get-regexp` over the exact
/// keys, at the moment of asking — a terminal can move any of them
/// between two of this application's reads.
///
/// Keys arrive with section and variable lower-cased and the branch name
/// as written. The branch name is escaped into the regex
/// (`config::regexp_literal`): unescaped, `wip.v2+x` picks up
/// `wipAv22x`'s keys. A key set at several levels answers with its last
/// record, as `--get` does.
pub async fn push_marks(
    executor: &GitExecutor,
    workdir: &Path,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<PushMarks, GitError> {
    let pattern = format!(
        r"^(branch\.{}\.(pushremote|remote|merge)|remote\.pushdefault)$",
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
        // Only 1 is "unset"; anything else (an unreadable config) is a
        // failure: a push planned on a misread "unset" rewrites upstreams.
        code => Err(GitError::Failed {
            command: format!(
                "git config --get-regexp branch.{branch}.pushRemote/remote/merge remote.pushDefault"
            ),
            code,
            stderr: out.failure_message(),
        }),
    }
}

fn parse_push_marks(branch: &str, bytes: &[u8]) -> PushMarks {
    let mut marks = PushMarks::default();
    each_scoped_record(bytes, |scope, key, value| {
        if key == "remote.pushdefault" {
            marks.push_default = value.map(|remote| PushDefault::at(scope, remote));
            return;
        }
        let Some(variable) = key
            .strip_prefix("branch.")
            .and_then(|rest| rest.strip_prefix(branch))
            .and_then(|rest| rest.strip_prefix('.'))
        else {
            return;
        };
        let slot = match variable {
            "pushremote" => &mut marks.push_remote,
            "remote" => &mut marks.tracks,
            "merge" => &mut marks.merge,
            _ => return,
        };
        *slot = value.map(str::to_string);
    });
    marks
}

/// The [`OriginMarks`] keys in write and clear order: the push's first,
/// since the remote's row draws its badge from it. A half left by a failed
/// second call reads as a mark not wholly set, so the row still offers
/// marking, which finishes it.
const ORIGIN_KEYS: [&str; 2] = ["remote.pushDefault", "checkout.defaultRemote"];

/// Marks `name` as this repository's origin: `git config
/// remote.pushDefault <name>`, then `git config checkout.defaultRemote
/// <name>`.
///
/// The old spelling on purpose (git最低バージョン整合.md: `git config set`
/// is 2.46), one key per process. The key ends the options, so a remote
/// named `-x` is taken as the value.
pub async fn mark_origin(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    for key in ORIGIN_KEYS {
        let cmd = GitCommand::new().cwd(workdir).args(["config", key, name]);
        executor.run(cmd, cancel).await?;
    }
    Ok(())
}

/// `git config --unset` on both [`OriginMarks`] keys.
///
/// Exit 5 (already unset) is the state asked for. Reaches only this
/// repository's config ([`PushDefault::local`]).
pub async fn clear_origin(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    for key in ORIGIN_KEYS {
        let cmd = GitCommand::new()
            .cwd(workdir)
            .answers_by_code(5)
            .args(["config", "--unset", key]);
        let out = executor.run_unchecked(cmd, cancel).await?;
        match out.code {
            0 | 5 => {}
            code => {
                return Err(GitError::Failed {
                    command: format!("git config --unset {key}"),
                    code,
                    stderr: out.failure_message(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture recorded from git 2.55.
    #[test]
    fn the_origin_marks_name_their_remotes_and_the_push_level() {
        let read = parse_origin_marks(
            b"local\0remote.pushdefault\nfork\0local\0checkout.defaultremote\nfork\0",
        );
        let pushes = read.push_default.expect("a push mark");
        assert_eq!(pushes.remote, "fork");
        assert!(
            pushes.local,
            "the repository's own config can be unset here"
        );
        assert_eq!(read.checkout_default.as_deref(), Some("fork"));
    }

    #[test]
    fn a_push_default_from_anywhere_else_is_not_local() {
        for scope in ["global", "system", "worktree", "command"] {
            let bytes = format!("{scope}\0remote.pushdefault\nfork\0").into_bytes();
            let read = parse_origin_marks(&bytes).push_default.expect("a value");
            assert!(!read.local, "{scope} is not this repository's config");
        }
    }

    /// A `remote rename` in a terminal moves the push mark and leaves the
    /// checkout one behind.
    #[test]
    fn the_two_keys_may_name_different_remotes() {
        let read = parse_origin_marks(
            b"local\0remote.pushdefault\nhome\0local\0checkout.defaultremote\nfork\0",
        );
        assert_eq!(read.push_default.expect("a push mark").remote, "home");
        assert_eq!(read.checkout_default.as_deref(), Some("fork"));
        let checkout_only = parse_origin_marks(b"global\0checkout.defaultremote\norigin\0");
        assert!(checkout_only.push_default.is_none());
        assert_eq!(checkout_only.checkout_default.as_deref(), Some("origin"));
    }

    #[test]
    fn the_last_origin_record_per_key_is_the_effective_one() {
        let read = parse_origin_marks(
            b"global\0checkout.defaultremote\norigin\0global\0remote.pushdefault\norigin\0\
              local\0checkout.defaultremote\nfork\0local\0remote.pushdefault\nfork\0",
        );
        let pushes = read.push_default.expect("a push mark");
        assert_eq!(pushes.remote, "fork");
        assert!(pushes.local);
        assert_eq!(read.checkout_default.as_deref(), Some("fork"));
    }

    /// `remote add --end-of-options` can make a remote named `-x`.
    #[test]
    fn a_dashed_remote_name_survives_the_read() {
        let read = parse_origin_marks(b"local\0remote.pushdefault\n-x\0");
        assert_eq!(read.push_default.expect("a value").remote, "-x");
    }

    #[test]
    fn an_origin_mark_without_a_remote_names_nothing() {
        let empty =
            parse_origin_marks(b"local\0remote.pushdefault\n\0local\0checkout.defaultremote\n\0");
        assert_eq!(empty, OriginMarks::default());
        let bare =
            parse_origin_marks(b"local\0remote.pushdefault\0local\0checkout.defaultremote\0");
        assert_eq!(bare, OriginMarks::default());
        assert_eq!(parse_origin_marks(b""), OriginMarks::default());
    }

    /// Fixture recorded from git 2.55.
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

    /// git keeps the branch name's case in the key (2.55).
    #[test]
    fn a_branch_keeps_its_case_and_another_case_is_another_branch() {
        let bytes: &[u8] = b"local\0branch.Topic.pushremote\nfork\0";
        assert_eq!(
            parse_push_marks("Topic", bytes).push_remote.as_deref(),
            Some("fork")
        );
        assert!(parse_push_marks("topic", bytes).push_remote.is_none());
    }

    /// The escaping lives in the pattern; the parse compares as text.
    #[test]
    fn a_metacharacter_branch_name_is_matched_literally() {
        let bytes: &[u8] = b"local\0branch.wip.v2+x.pushremote\nfork\0";
        assert_eq!(
            parse_push_marks("wip.v2+x", bytes).push_remote.as_deref(),
            Some("fork")
        );
        assert!(parse_push_marks("wip", bytes).push_remote.is_none());
    }

    /// The separating newline is inside the record, so a value's own
    /// newlines cannot shift the scope/record pairing.
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
