//! Repository discovery and validation.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Hash algorithm of a repository's object database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFormat {
    Sha1,
    Sha256,
}

/// A validated repository location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoInfo {
    /// Absolute path of the work tree root.
    pub workdir: PathBuf,
    /// Absolute path of the `.git` directory (may live elsewhere for
    /// worktrees).
    pub git_dir: PathBuf,
    /// Absolute path of the repository's own config file — the one
    /// `git remote add` writes. A linked worktree shares the common
    /// repository's, which is why this is asked for rather than joined
    /// onto `git_dir` (実測 2.55: `--git-path config` in a worktree
    /// answers the common `.git/config`).
    pub config_path: PathBuf,
    pub object_format: ObjectFormat,
}

/// Validates that `path` is inside a git work tree and resolves its root.
///
/// This is the "open repository" entry point: any user-selected folder goes
/// through here first.
pub async fn open(
    executor: &GitExecutor,
    path: &Path,
    cancel: &CancellationToken,
) -> Result<RepoInfo, GitError> {
    if !path.is_dir() {
        return Err(GitError::NotARepository {
            path: path.to_path_buf(),
            stderr: String::new(),
            bare: false,
        });
    }

    let cmd = GitCommand::new()
        .cwd(path)
        .args([
            "rev-parse",
            "--show-toplevel",
            "--absolute-git-dir",
            "--show-object-format",
            // Said before the path below and after everything else: it
            // decides how paths come out, and the two above are absolute
            // already. Without it `--git-path` answers relative to the
            // working directory, which is the folder the user picked and
            // not necessarily the repository root.
            "--path-format=absolute",
            "--git-path",
            "config",
        ])
        .timeout(Duration::from_secs(10));
    let described = "git rev-parse --show-toplevel --absolute-git-dir --show-object-format \
         --path-format=absolute --git-path config";

    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        let stderr = out.stderr_utf8().trim().to_string();
        // Covers both "not a git repository" and bare repositories ("this
        // operation must be run in a work tree"): neither can be opened.
        // Which of the two it was takes a second question, asked only on
        // the way out — the two folders look nothing alike to the person
        // who picked one, so the screen has to be able to tell them apart.
        return Err(GitError::NotARepository {
            path: path.to_path_buf(),
            stderr,
            bare: is_bare(executor, path, cancel).await,
        });
    }

    let text = out.stdout_utf8();
    let mut lines = text.lines();
    let (Some(toplevel), Some(git_dir), Some(format), Some(config_path)) =
        (lines.next(), lines.next(), lines.next(), lines.next())
    else {
        return Err(GitError::UnexpectedOutput {
            command: described.to_string(),
            message: text.trim().to_string(),
        });
    };

    let object_format = match format.trim() {
        "sha1" => ObjectFormat::Sha1,
        "sha256" => ObjectFormat::Sha256,
        other => {
            return Err(GitError::UnexpectedOutput {
                command: described.to_string(),
                message: format!("unknown object format `{other}`"),
            });
        }
    };

    Ok(RepoInfo {
        workdir: PathBuf::from(toplevel.trim_end()),
        git_dir: PathBuf::from(git_dir.trim_end()),
        config_path: PathBuf::from(config_path.trim_end()),
        object_format,
    })
}

/// Whether `path` is a repository git will not give a work tree for.
///
/// Only asked once [`open`] has already failed, so the cost lands on the
/// folder nobody could open (実測 33ms) rather than on every one that
/// works. `false` for a folder that is no repository at all, and for a
/// question git could not answer — the screen falls back to git's own
/// wording either way.
async fn is_bare(executor: &GitExecutor, path: &Path, cancel: &CancellationToken) -> bool {
    let cmd = GitCommand::new()
        .cwd(path)
        .args(["rev-parse", "--is-bare-repository"])
        .timeout(Duration::from_secs(10))
        // Outside a repository this exits 128, which is the answer here
        // and not a failure worth raising the command log over.
        .answers_by_code();
    match executor.run_unchecked(cmd, cancel).await {
        Ok(out) => out.code == 0 && out.stdout_utf8().trim() == "true",
        Err(_) => false,
    }
}

/// The key two paths are judged to be the same folder by — what decides
/// whether a repository is already open.
///
/// Unlike [`crate::settings::repo_key`] this one is never written down,
/// so it is free to ask the filesystem, and asking is the whole point:
/// one directory reaches the application spelled several ways depending
/// on which way in was taken. A folder picker returns what the shell
/// handed it, while `git worktree list` prints forward slashes and the
/// long name — on Windows that is `C:\Users\WRONGW~1\…\repo` against
/// `C:/Users/wrongwrong/…/repo` for one folder (measured). Resolving
/// folds all of it together: separators, a trailing one, the letter case
/// Windows keeps but does not distinguish, the 8.3 short name, and links.
///
/// A path the filesystem will not resolve — removed, or never there —
/// falls back to `repo_key`, which keeps two absent paths apart rather
/// than folding them into one answer.
pub fn open_key(path: &str) -> PathBuf {
    let path = path.trim();
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(crate::settings::repo_key(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::create_dir(&path).expect("create folder");
        path
    }

    #[test]
    fn one_folder_spelled_several_ways_is_one_key() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let here = folder(&dir, "repo");
        let plain = here.to_string_lossy().into_owned();

        assert_eq!(open_key(&plain), open_key(&plain.replace('\\', "/")));
        assert_eq!(
            open_key(&plain),
            open_key(&format!("{plain}{}", std::path::MAIN_SEPARATOR))
        );
        assert_eq!(open_key(&plain), open_key(&format!("  {plain}  ")));
    }

    #[test]
    fn two_folders_keep_their_own_keys() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let one = folder(&dir, "one");
        let two = folder(&dir, "two");
        assert_ne!(
            open_key(&one.to_string_lossy()),
            open_key(&two.to_string_lossy())
        );
    }

    /// A path the filesystem will not resolve still answers, and two of
    /// them stay apart: folding every unresolvable path into one answer
    /// would make a repository that has been moved away stand for all
    /// the others.
    #[test]
    fn paths_that_are_not_there_stay_apart() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let one = dir.path().join("gone-one").to_string_lossy().into_owned();
        let two = dir.path().join("gone-two").to_string_lossy().into_owned();
        assert_ne!(open_key(&one), open_key(&two));
        assert_eq!(open_key(&one), open_key(&one.replace('\\', "/")));
    }

    /// Windows keeps the letter case a name was written in but does not
    /// tell names apart by it. Resolving through the filesystem is what
    /// folds that away — the same step that folds the 8.3 short name a
    /// temp directory is handed out under (`WRONGW~1` for `wrongwrong`),
    /// which is how `git worktree list` and a folder picker name one
    /// directory with two strings that are not equal.
    #[cfg(windows)]
    #[test]
    fn windows_letter_case_is_not_part_of_the_name() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let here = folder(&dir, "repo");
        assert_eq!(
            open_key(&here.to_string_lossy()),
            open_key(&here.with_file_name("REPO").to_string_lossy())
        );
    }
}
