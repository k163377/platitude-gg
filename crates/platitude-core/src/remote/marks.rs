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
    /// The remote it names. **git does not check that one exists**: a name
    /// no remote holds is taken for a URL, and the push fails at the
    /// connection instead (measured 2.55: `'nope' does not appear to be a git
    /// repository`).
    pub remote: String,
    /// Whether this repository's own config is what says so.
    ///
    /// A value set anywhere else cannot be cleared from here. git has no
    /// local spelling for "not set" — an empty local value means "no
    /// destination at all", and a plain `git push` then fails with
    /// `No configured push destination.` (measured). Marking another remote is
    /// the only move a repository has against a global value.
    pub local: bool,
}

impl PushDefault {
    fn at(scope: &str, remote: &str) -> Self {
        Self {
            remote: remote.to_string(),
            // Every other level — global, system, worktree, a `-c` on the
            // command line — is one this repository cannot unset.
            local: scope == "local",
        }
    }
}

/// The keys `Mark as origin` writes ([`mark_origin`]), as this repository
/// reads them.
///
/// Two, because the role answers two questions in git: where a push goes
/// when no branch says otherwise (`remote.pushDefault`), and which remote
/// `git switch <name>` takes `<name>` from when more than one carries it
/// (`checkout.defaultRemote` — unset, git refuses with `'topic' matched
/// multiple (2) remote tracking branches`, measured 2.55). **Read as two
/// because git keeps them as two**: `git remote rename` moves
/// `remote.pushDefault` to the new name and `remove` unsets it, and both
/// leave `checkout.defaultRemote` naming the old remote (measured 2.55),
/// so a remote can hold one without the other.
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
/// The record shape and the level order are [`push_marks`]'s; neither
/// key set is exit 1, which is an answer.
///
/// A key written without a remote name behind it (`remote.pushDefault=`)
/// answers `None`: no remote is called that, so there is nothing here to
/// point at. What git does with it is refuse the push outright, and only
/// git can say that (measured).
pub async fn origin_marks(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<OriginMarks, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // Unset is the usual state and it is an answer.
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
/// per record, every level in precedence order, lowest first — so a key
/// seen again overwrites what an earlier record said, and the last one is
/// the effective value.
///
/// A value that is empty or blank comes through as `None`, as does a key
/// written bare, whose record has no newline in it (measured 2.55: a bare
/// `pushDefault` line arrives as `local\0remote.pushdefault\0`). Both still
/// override a level below.
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
    /// order is [`super::plan_current_push`]'s, measured 2.55). `None` where
    /// the branch does not mark one.
    pub push_remote: Option<String>,
    /// The repository's mark, with the level that set it.
    pub push_default: Option<PushDefault>,
}

/// Reads both marks with one `--get-regexp` over the two exact keys.
///
/// `-z --show-scope --get-regexp` writes `<scope>\0<key>\n<value>\0` per
/// record, every level in precedence order, so the last record per key is
/// the effective value (measured 2.55:
/// `global\0remote.pushdefault\nfork\0local\0remote.pushdefault\nhome\0`
/// — `home` wins). Keys arrive with section and variable lower-cased and
/// the branch name spelled as it was written; the pattern embeds the
/// branch name escaped, because the pattern is a regex and the name may
/// hold `.` or `+` (`config::regexp_literal` — measured: unescaped,
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
    each_scoped_record(bytes, |scope, key, value| {
        if key == branch_key {
            marks.push_remote = value.map(str::to_string);
        } else if key == "remote.pushdefault" {
            marks.push_default = value.map(|remote| PushDefault::at(scope, remote));
        }
    });
    marks
}

/// The [`OriginMarks`] keys in the order they are written and cleared: the
/// push's first, because it is the one the remote's row draws its badge
/// from. Either half left behind by a second call that fails reads as a
/// mark not wholly set, so the row still offers it — marking finishes it,
/// and marking then clearing takes the rest off.
const ORIGIN_KEYS: [&str; 2] = ["remote.pushDefault", "checkout.defaultRemote"];

/// Marks `name` as this repository's origin: `git config
/// remote.pushDefault <name>`, then `git config checkout.defaultRemote
/// <name>`.
///
/// The old spelling on purpose (規約 git最低バージョン整合: `git config
/// set` is 2.46), which sets one key per process. The key ends the
/// options: git stops looking for them after it, so a remote actually
/// named `-x` — which `remote add` will make — is taken as the value
/// (measured, 2.55).
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
/// A key not being set is the state the caller asked for, and git says so
/// with exit 5 (measured). This repository's own config is all it reaches
/// ([`PushDefault::local`]).
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

    /// Recorded from git 2.55: `config -z --show-scope --get-regexp` over
    /// the two keys, `<scope>\0<key>\n<value>\0` per record.
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

    /// Each key stands alone: a remote renamed in a terminal takes the push
    /// mark along and leaves the checkout one behind (measured 2.55).
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

    /// Levels arrive lowest first, so the last record per key is the
    /// effective value.
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

    /// A remote may be named `-x` (`remote add --end-of-options` makes one),
    /// and the value comes back as it was written.
    #[test]
    fn a_dashed_remote_name_survives_the_read() {
        let read = parse_origin_marks(b"local\0remote.pushdefault\n-x\0");
        assert_eq!(read.push_default.expect("a value").remote, "-x");
    }

    /// The key written with nothing behind it. No remote is called that, so
    /// there is nothing to mark — git refuses the push, and only git can
    /// say so.
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
    /// effective value (measured, local `home` prints after global `fork`).
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
    /// arrives as a record with no newline in it (measured, 2.55). Both
    /// override a level below.
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
    /// `Topic`'s mark is not `topic`'s (measured, 2.55).
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
    /// the escaping lives in the pattern, and the parse compares as text.
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
    /// (measured, 2.55: `local\0remote.pushdefault\nfork\nx\0`).
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
