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
        });
    }

    let cmd = GitCommand::new()
        .cwd(path)
        .args([
            "rev-parse",
            "--show-toplevel",
            "--absolute-git-dir",
            "--show-object-format",
        ])
        .timeout(Duration::from_secs(10));
    let described = "git rev-parse --show-toplevel --absolute-git-dir --show-object-format";

    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        let stderr = out.stderr_utf8().trim().to_string();
        // Covers both "not a git repository" and bare repositories ("this
        // operation must be run in a work tree"): neither can be opened.
        return Err(GitError::NotARepository {
            path: path.to_path_buf(),
            stderr,
        });
    }

    let text = out.stdout_utf8();
    let mut lines = text.lines();
    let (Some(toplevel), Some(git_dir), Some(format)) = (lines.next(), lines.next(), lines.next())
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
        object_format,
    })
}
