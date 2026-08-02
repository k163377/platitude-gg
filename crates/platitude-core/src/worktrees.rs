//! Worktree listing: `git worktree list --porcelain -z`.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// One entry of `git worktree list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// Absolute path of the worktree root (as git prints it).
    pub path: String,
    /// Checked-out branch (short name); `None` when detached or bare.
    pub branch: Option<String>,
    /// HEAD commit id (hex); `None` for a bare entry.
    pub head_hex: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed worktree list output")]
pub struct WorktreeParseError;

/// Parses `--porcelain -z` output: NUL-terminated attribute lines, an
/// empty line between entries. Unknown attributes are ignored (forward
/// compatibility).
pub fn parse_worktrees(bytes: &[u8]) -> Result<Vec<WorktreeEntry>, WorktreeParseError> {
    let mut out = Vec::new();
    let mut cur: Option<WorktreeEntry> = None;
    for token in bytes.split(|b| *b == 0) {
        if token.is_empty() {
            if let Some(entry) = cur.take() {
                out.push(entry);
            }
            continue;
        }
        let line = String::from_utf8_lossy(token);
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(entry) = cur.take() {
                out.push(entry);
            }
            cur = Some(WorktreeEntry {
                path: path.to_string(),
                branch: None,
                head_hex: None,
                bare: false,
                detached: false,
                locked: false,
            });
            continue;
        }
        let Some(entry) = cur.as_mut() else {
            return Err(WorktreeParseError);
        };
        if let Some(head) = line.strip_prefix("HEAD ") {
            entry.head_hex = Some(head.trim().to_string());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            let branch = branch.trim();
            entry.branch = Some(
                branch
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch)
                    .to_string(),
            );
        } else if line.as_ref() == "bare" {
            entry.bare = true;
        } else if line.as_ref() == "detached" {
            entry.detached = true;
        } else if line.starts_with("locked") {
            entry.locked = true;
        }
    }
    if let Some(entry) = cur.take() {
        out.push(entry);
    }
    Ok(out)
}

/// Loads the worktree list.
pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<WorktreeEntry>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["worktree", "list", "--porcelain", "-z"]);
    let out = executor.run(cmd, cancel).await?;
    parse_worktrees(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git worktree list".to_string(),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(lines: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for l in lines {
            v.extend_from_slice(l.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_main_linked_and_detached() {
        let bytes = z(&[
            "worktree C:/repo",
            "HEAD 1111111111111111111111111111111111111111",
            "branch refs/heads/main",
            "",
            "worktree C:/repo/.claude/worktrees/wt-1",
            "HEAD 2222222222222222222222222222222222222222",
            "detached",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, "C:/repo");
        assert_eq!(list[0].branch.as_deref(), Some("main"));
        assert!(!list[0].detached);
        assert!(list[1].detached);
        assert_eq!(list[1].branch, None);
    }

    #[test]
    fn parses_bare_and_locked() {
        let bytes = z(&[
            "worktree /srv/repo.git",
            "bare",
            "",
            "worktree /srv/wt",
            "HEAD 3333333333333333333333333333333333333333",
            "branch refs/heads/dev",
            "locked reason text",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert!(list[0].bare);
        assert!(list[1].locked);
        assert_eq!(list[1].branch.as_deref(), Some("dev"));
    }

    #[test]
    fn empty_output_is_empty() {
        assert!(parse_worktrees(b"").unwrap().is_empty());
    }

    #[test]
    fn attribute_before_worktree_is_an_error() {
        assert!(parse_worktrees(&z(&["branch refs/heads/x"])).is_err());
    }
}
