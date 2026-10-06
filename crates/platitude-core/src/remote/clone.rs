//! `git clone`.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// `git clone <url> <into>`, where `into` is the folder the worktree
/// becomes.
///
/// The destination is always named, or the caller would have to parse
/// git's human-facing `Cloning into '…'`
/// (rules-refs/core.md「`remote::clone` は行き先を必ず名指しする」).
///
/// The parent is the working directory, so git's refusals name the
/// destination as the dialog spells it. git makes missing leading folders
/// and refuses a non-empty destination; that comes back untouched in
/// [`GitError::Failed`].
pub async fn clone(
    executor: &GitExecutor,
    url: &str,
    into: &Path,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .args(["clone", "--"])
        .arg(url)
        .arg(into)
        .timeout(timeout)
        .paced_elsewhere();
    // Only an existing parent: a missing working directory fails the
    // spawn before git can say anything.
    let cmd = match into.parent().filter(|parent| parent.is_dir()) {
        Some(parent) => cmd.cwd(parent),
        None => cmd,
    };
    executor.run(cmd, cancel).await.map(|_| ())
}

/// What to call the folder a clone of `url` would go into — the name the
/// dialog offers before anybody types one.
///
/// Only a prefill, not a second `guess_dir_name`: [`clone`] always names
/// its destination. The answer is the last segment with one `.git` taken
/// off. `""` where nothing reads as a name, which leaves the box empty
/// and the accept button refusing.
pub fn folder_name_for(url: &str) -> String {
    // A trailing separator names the same repository.
    let trimmed = url.trim().trim_end_matches(['/', '\\']);
    // `:` ends the host of the scp-like form (`git@host:repo.git`), where
    // no slash separates host from path at all.
    let last = trimmed
        .rsplit(['/', '\\', ':'])
        .next()
        .unwrap_or(trimmed)
        .trim();
    // One suffix only: a repository really called `repo.git.git`
    // is served from `repo.git.git.git`.
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_offered_is_the_last_segment_without_its_git_suffix() {
        for (url, name) in [
            ("https://github.com/you/repo.git", "repo"),
            ("https://github.com/you/repo", "repo"),
            ("https://github.com/you/repo.git/", "repo"),
            ("git@github.com:you/repo.git", "repo"),
            ("git@github.com:repo.git", "repo"),
            ("ssh://git@host:2222/srv/repo.git", "repo"),
            ("file:///C:/dev/repo", "repo"),
            (r"C:\dev\repo.git", "repo"),
            ("/srv/git/repo.git", "repo"),
            ("  https://host/you/repo.git  ", "repo"),
            ("https://host/you/repo.git.git", "repo.git"),
        ] {
            assert_eq!(folder_name_for(url), name, "{url}");
        }
    }

    /// The empty box is what refuses accept; a guessed name would be one
    /// somebody has to notice and delete.
    #[test]
    fn a_url_with_no_name_in_it_offers_none() {
        for url in ["", "   ", ".git", "/", "  /  "] {
            assert_eq!(folder_name_for(url), "", "{url}");
        }
    }
}
