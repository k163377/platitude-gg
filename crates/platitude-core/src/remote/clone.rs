//! `git clone`.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// `git clone <url> <into>`, where `into` is the folder the working
/// copy becomes.
///
/// **The destination is always named.** Left to itself git works one out
/// of the URL and prints it, and the caller would have to read that line
/// back to know where the repository landed — a message written for
/// people, which nothing here parses (規約 §git が言ったことを読む場所).
/// Naming it makes the answer the caller's own: the folder it asked for
/// is the folder to open.
///
/// The parent is the working directory, so git's own refusals name the
/// destination the way the dialog spells it. git makes the leading
/// folders it is short of, and refuses a destination that already holds
/// anything — that refusal is git's to make and comes back untouched in
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
    // Only where there is one to stand in: a destination at a root has
    // none, and a working directory that is not there fails the spawn
    // before git can say anything about the clone.
    let cmd = match into.parent().filter(|parent| parent.is_dir()) {
        Some(parent) => cmd.cwd(parent),
        None => cmd,
    };
    executor.run(cmd, cancel).await.map(|_| ())
}

/// What to call the folder a clone of `url` would go into — the name the
/// dialog offers before anybody types one.
///
/// **A suggestion for the box.** git has a rule of its own for the
/// destination it picks when none is given (`guess_dir_name`), and this
/// one stands apart from it: the clone always names its destination
/// ([`clone`]), so what this returns is only what stands in
/// the box until it is edited. The shapes it has to read are the ones
/// people paste — `https://host/you/repo.git`, `git@host:you/repo.git`,
/// `file:///srv/repo`, a plain path — and the answer is the last segment
/// with one `.git` taken off the end.
///
/// `""` where nothing in the URL reads as a name, which leaves the box
/// empty and the accept button refusing: a folder has to be named.
pub fn folder_name_for(url: &str) -> String {
    // A pasted URL often carries the line's own whitespace, and a URL
    // typed with a trailing separator names the same repository.
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

    /// The box stays empty: that is what refuses the accept button, and
    /// a name guessed out of a URL that carries none would be one
    /// somebody has to notice and delete.
    #[test]
    fn a_url_with_no_name_in_it_offers_none() {
        for url in ["", "   ", ".git", "/", "  /  "] {
            assert_eq!(folder_name_for(url), "", "{url}");
        }
    }
}
