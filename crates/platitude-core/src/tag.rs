//! Tag names.
//!
//! git has no rename for a tag, so this module builds one out of the two
//! commands it does have: point a new name at what the old one pointed at,
//! then drop the old name. The object itself is never rewritten, so an
//! annotated or signed tag keeps its message, its tagger and its signature
//! — the new name points at that same tag object. What it cannot change is
//! the name recorded *inside* an annotated tag; that is git's own limit,
//! and re-creating the tag to fix it would throw away everything else the
//! object holds.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Points `to` at whatever `from` names, then deletes `from`.
///
/// Left to git to refuse when `to` already exists: overwriting a tag
/// silently is how a release mark ends up somewhere else.
///
/// The first step is the one that can fail on its own (bad name, name
/// taken); if it does, the old tag is still there and nothing was lost.
///
/// A rename that changes only letter case is refused outright. On a
/// case-insensitive filesystem (Windows, macOS) with the tag packed —
/// the normal state after a clone — the create step sees the new name as
/// free and writes a loose ref whose *file* collides with the old name;
/// the delete step then removes both, and every command involved exits 0
/// (measured, `tag V1.0 v1.0` + `tag -d v1.0` on packed refs
/// leaves no tag at all). Refused everywhere, since the same repository
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
    // **The two halves fail differently, and say so differently**
    // (デザイン規約 §答えの要らない報せ): nothing has moved while the new name is
    // being made, so a refusal there is one the box the name was typed
    // into can still answer — most often the name is taken. Once it is
    // made, a failure to take the old one off leaves both standing, which
    // is the same half-done rename a stash's has.
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
            // **A session closing is a cancellation.** That is how a write
            // is stopped on the way out, and it is told apart from a
            // failure one layer up (`session::write`); dressed as a report
            // it would raise a bar over a window that is going away.
            if error.is_cancelled() || error.report().is_some() {
                return error;
            }
            crate::report::half_renamed(from, error)
        })
}

/// Puts `name` on `commit` — a lightweight tag, which is what `git tag`
/// makes when nothing asks for more.
///
/// Plain, for [`rename`]'s reason: git refuses when the name is taken,
/// and a release mark that moves without anybody saying so is the
/// accident that refusal exists to stop.
///
/// `commit` is anything git resolves — the row's oid is what the menus
/// hand over — and an empty one leaves the tag on HEAD, which is what git
/// does with the argument left off.
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

/// Deletes a tag. Only the name goes: whatever it marked is still in the
/// repository, reachable or not.
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

/// Whether a name is one git will take for a branch or a tag.
///
/// The rules are `git check-ref-format`'s, applied to `refs/heads/<name>`
/// (which is also what `refs/tags/<name>` allows). Checked here as a
/// pure function of the string: this is the answer an input box needs
/// on every character it is given.
/// `tests/it/rename_integration.rs` holds it to what real git says.
pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // Whole-name rules first: these say nothing about where they matched,
    // so no component walk can see them.
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

    /// Every shape an argv can carry is pinned against real git in
    /// `tests/it/rename_integration.rs`
    /// (`the_name_rules_are_the_ones_git_applies` — including `@` and a
    /// leading dash, which git takes and this must too). Only what cannot
    /// reach `check-ref-format` through a command line stays here.
    #[test]
    fn control_characters_are_refused_without_asking_git() {
        assert!(!is_valid_name("new\nline"));
        assert!(!is_valid_name("bell\u{7}"));
        assert!(!is_valid_name("del\u{7f}"));
    }
}
