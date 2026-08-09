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
/// Never forced: git refuses when `to` already exists, and overwriting a
/// tag silently is how a release mark ends up somewhere else.
///
/// The first step is the one that can fail on its own (bad name, name
/// taken); if it does, the old tag is still there and nothing was lost.
///
/// A rename that changes only letter case is refused outright. On a
/// case-insensitive filesystem (Windows, macOS) with the tag packed —
/// the normal state after a clone — the create step sees the new name as
/// free and writes a loose ref whose *file* collides with the old name;
/// the delete step then removes both, and every command involved exits 0
/// (実測 2026-08-07: `tag V1.0 v1.0` + `tag -d v1.0` on packed refs
/// leaves no tag at all). Refused everywhere, not just where it breaks:
/// the same repository may be opened from either kind of filesystem.
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
    let create = GitCommand::new()
        .cwd(workdir)
        .args(["tag", "--end-of-options", to, from]);
    executor.run(create, cancel).await?;
    delete(executor, workdir, from, cancel).await
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
/// (which is also what `refs/tags/<name>` allows). Checked here rather
/// than by running git per keystroke: this is the answer an input box
/// needs on every character, and it is a pure function of the string.
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

    #[test]
    fn plain_names_pass() {
        assert!(is_valid_name("v1.0.0"));
        assert!(is_valid_name("release/2026-08"));
        assert!(is_valid_name("feature/login"));
        assert!(is_valid_name("a"));
    }

    #[test]
    fn the_shapes_git_refuses() {
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("with space"));
        assert!(!is_valid_name("tilde~1"));
        assert!(!is_valid_name("caret^"));
        assert!(!is_valid_name("colon:name"));
        assert!(!is_valid_name("question?"));
        assert!(!is_valid_name("star*"));
        assert!(!is_valid_name("bracket["));
        assert!(!is_valid_name("back\\slash"));
        assert!(!is_valid_name("new\nline"));
        assert!(!is_valid_name("a..b"));
        assert!(!is_valid_name("at@{0}"));
        assert!(!is_valid_name("/leading"));
        assert!(!is_valid_name("trailing/"));
        assert!(!is_valid_name("double//slash"));
        assert!(!is_valid_name("ends."));
        assert!(!is_valid_name(".hidden"));
        assert!(!is_valid_name("dir/.hidden"));
        assert!(!is_valid_name("name.lock"));
        assert!(!is_valid_name("dir/name.lock"));
    }

    #[test]
    fn the_shapes_only_git_would_defend() {
        // Nothing here is stricter than `check-ref-format`: a name git
        // takes, this takes. `@` reads as HEAD everywhere else, and a
        // leading dash reads as an option — but our commands name refs
        // after `--end-of-options`, and inventing a rule of our own is
        // how a GUI ends up refusing a branch the terminal just made.
        assert!(is_valid_name("@"));
        assert!(is_valid_name("-dash"));
    }
}
