//! Repository discovery and validation.

use std::path::{Path, PathBuf};

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
    /// Absolute path of the repository's own config file (the one
    /// `git remote add` writes). A linked worktree's is the common
    /// repository's, so it is asked of `--git-path`, not built from `git_dir`.
    pub config_path: PathBuf,
    pub object_format: ObjectFormat,
}

/// Validates that `path` is inside a git work tree and resolves its root.
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

    let cmd = GitCommand::new().cwd(path).args([
        "rev-parse",
        "--show-toplevel",
        "--absolute-git-dir",
        "--show-object-format",
        // Applies to what follows: without it `--git-path` answers relative
        // to the picked folder, not necessarily the repository root.
        "--path-format=absolute",
        "--git-path",
        "config",
    ]);
    let described = "git rev-parse --show-toplevel --absolute-git-dir --show-object-format \
         --path-format=absolute --git-path config";

    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        let stderr = out.stderr_utf8().trim().to_string();
        // Both "not a git repository" and a bare repository land here; the
        // screen tells them apart, so a second question is asked only here.
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

/// Whether `path` is a bare repository. Only asked once [`open`] has
/// failed. `false` for no repository at all and for a question git could
/// not answer — the screen then falls back to git's own wording.
async fn is_bare(executor: &GitExecutor, path: &Path, cancel: &CancellationToken) -> bool {
    // Answers on stdout, never by code (measured on 2.55): exit 0 with
    // `true` / `false` in any repository, 128 outside one. 128 stays a failed row in the command
    // log: it is also every other failure (rules-refs/core.md).
    let cmd = GitCommand::new()
        .cwd(path)
        .args(["rev-parse", "--is-bare-repository"]);
    match executor.run(cmd, cancel).await {
        Ok(out) => out.stdout_utf8().trim() == "true",
        Err(_) => false,
    }
}

/// Where a folder opens: the working copy it is in, and the repository
/// that copy belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// What git answered about the folder itself; `workdir` is the root of
    /// the working copy it sits in.
    pub info: RepoInfo,
    /// The repository's own working copy — what a tab is named after.
    /// Every copy of one repository answers the same path
    /// ([`crate::worktrees::WorktreeEntry::main`]). A bare repository puts
    /// its bare directory here, the only name it has.
    pub repo: PathBuf,
}

/// [`open`], and the repository the working copy it found belongs to —
/// what tells linked copies of one repository from two repositories
/// (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」).
///
/// A listing git will not give does not stop the opening: the copy then
/// stands for itself.
pub async fn place(
    executor: &GitExecutor,
    path: &Path,
    cancel: &CancellationToken,
) -> Result<Place, GitError> {
    let info = open(executor, path, cancel).await?;
    let repo = match crate::worktrees::load(executor, &info.workdir, cancel).await {
        Ok(entries) => entries
            .into_iter()
            .find(|entry| entry.main)
            .map_or_else(|| info.workdir.clone(), |entry| PathBuf::from(entry.path)),
        Err(error) => {
            tracing::warn!(%error, path = %info.workdir.display(), "no worktree listing: the copy stands for itself");
            info.workdir.clone()
        }
    };
    Ok(Place { info, repo })
}

/// The key two paths are judged to be the same folder by — what decides
/// whether a repository is already open.
///
/// Unlike [`crate::settings::repo_key`] this is never written down, so it
/// may resolve through the filesystem: one folder arrives spelled
/// differently from a picker and from `git worktree list` (separators, a
/// trailing one, Windows letter case, the 8.3 short name, links). A path
/// that will not resolve falls back to `repo_key`, which keeps two absent
/// paths apart.
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

    /// Folding every unresolvable path into one key would make one
    /// moved-away repository stand for all the others.
    #[test]
    fn paths_that_are_not_there_stay_apart() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let one = dir.path().join("gone-one").to_string_lossy().into_owned();
        let two = dir.path().join("gone-two").to_string_lossy().into_owned();
        assert_ne!(open_key(&one), open_key(&two));
        assert_eq!(open_key(&one), open_key(&one.replace('\\', "/")));
    }

    /// Resolving through the filesystem folds case away, the same step that
    /// folds the 8.3 short name.
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
