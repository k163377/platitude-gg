//! Tag names.
//!
//! git has no tag rename. [`rename`] never rewrites the object, so an
//! annotated or signed tag keeps its message, tagger and signature; the
//! name recorded *inside* it stays the old one (re-creating the tag would
//! lose the rest).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Points `to` at whatever `from` names, then deletes `from`.
///
/// Left to git to refuse when `to` exists: overwriting a tag silently is
/// how a release mark ends up somewhere else. If that first step fails,
/// the old tag is untouched.
///
/// A case-only rename is refused outright: on a case-insensitive disk
/// with the tag packed (normal after a clone) the new loose ref's file
/// collides with the old name, and the delete then removes both with
/// every command exiting 0. Refused everywhere, since the same repository
/// may be opened from either kind of filesystem.
pub async fn rename(
    executor: &GitExecutor,
    workdir: &Path,
    from: &str,
    to: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if from != to && from.to_lowercase() == to.to_lowercase() {
        return Err(GitError::Rejected {
            message: format!(
                "renaming {from} to {to} changes only letter case, which \
                 deletes both names on a case-insensitive disk"
            ),
        });
    }
    // The two halves report differently (デザイン規約 §答えの要らない報せ):
    // a refused create has moved nothing, so the name box can still answer
    // it; a failed delete leaves both names, a half-done rename.
    let create = GitCommand::new()
        .cwd(workdir)
        .args(["tag", "--end-of-options", to, from]);
    let command = create.describe();
    let out = executor.run_unchecked(create, cancel).await?;
    if out.code != 0 {
        return Err(crate::report::rename_refused(from, command, &out));
    }
    delete(executor, workdir, from, cancel)
        .await
        .map_err(|error| {
            // A closing session cancels, and `session::write` tells that
            // apart; as a report it would raise a bar over a closing window.
            if error.is_cancelled() || error.report().is_some() {
                return error;
            }
            crate::report::half_renamed(from, error)
        })
}

/// Puts a lightweight tag `name` on `commit` (anything git resolves;
/// empty means HEAD). No `--force`, for [`rename`]'s reason.
pub async fn create(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    commit: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["tag", "--end-of-options", name]);
    if !commit.is_empty() {
        cmd = cmd.arg(commit);
    }
    executor.run(cmd, cancel).await.map(drop)
}

/// Deletes a tag name; what it marked stays in the repository.
pub async fn delete(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["tag", "--delete", "--end-of-options", name]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Whether git will take a name for a branch or a tag: the rules of
/// `git check-ref-format` for `refs/heads/<name>` (the same for tags), as
/// a pure function because an input box asks on every keystroke.
/// `tests/it/rename_integration.rs` holds it to real git.
pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // Whole-name rules: a per-component walk cannot see these.
    if name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with('.')
        || name.contains("//")
        || name.contains("..")
        || name.contains("@{")
    {
        return false;
    }
    if name.chars().any(|c| {
        c.is_ascii_control()
            || c == '\u{7f}'
            || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return false;
    }
    name.split('/')
        .all(|part| !part.is_empty() && !part.starts_with('.') && !part.ends_with(".lock"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;

    /// The executor cannot run anything and the repository is not there,
    /// so a rename that reached for git would fail on the spawn rather
    /// than come back as this refusal.
    #[tokio::test]
    async fn a_case_only_rename_is_refused_without_asking_git() {
        let (exec, asked) = refusing::git();

        let err = rename(
            &exec,
            &refusing::nowhere(),
            "v1.0",
            "V1.0",
            &CancellationToken::new(),
        )
        .await
        .expect_err("a case-only rename is refused");

        assert!(err.to_string().contains("letter case"), "{err}");
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }

    /// Only what cannot reach `check-ref-format` through an argv; the rest
    /// is pinned against real git in
    /// `rename_integration::the_name_rules_are_the_ones_git_applies`.
    #[test]
    fn control_characters_are_refused_without_asking_git() {
        assert!(!is_valid_name("new\nline"));
        assert!(!is_valid_name("bell\u{7}"));
        assert!(!is_valid_name("del\u{7f}"));
    }
}
