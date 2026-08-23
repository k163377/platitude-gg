//! The push marks: where a push goes when configuration, rather than the
//! command line, decides it — reading them, setting the repository's, and
//! clearing it.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

use super::list::config_value;

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

/// Reads `branch.<branch>.pushRemote` — the branch's own answer to where
/// its pushes go, and the one that beats every other (git-config(5); the
/// order is [`super::plan_current_push`]'s, 実測 2.55).
///
/// The one reader of the key. [`super::plan_current_push`] runs it before
/// a send, and the session snapshot runs it so the toolbar can name the
/// same destination — a second, separate reading of it is how the label
/// comes to name a remote the push never goes to.
pub async fn branch_push_remote(
    executor: &GitExecutor,
    workdir: &Path,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    config_value(
        executor,
        workdir,
        &format!("branch.{branch}.pushRemote"),
        cancel,
    )
    .await
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
}
