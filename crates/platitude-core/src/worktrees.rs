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
    /// What `git worktree lock --reason` was given, as git prints it
    /// after the word. Empty both when the entry is not locked and when
    /// it was locked without one, so `locked` is the flag and this is
    /// only ever the words beside it.
    pub lock_reason: String,
    /// `git worktree prune` would drop this entry — its directory is
    /// gone from where the administrative file says it is. The entry is
    /// still listed, and the path it names leads nowhere.
    pub prunable: bool,
    /// Why git would drop it, in git's own words (`gitdir file points to
    /// non-existent location`). Empty when `prunable` is false.
    pub prune_reason: String,
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
                lock_reason: String::new(),
                prunable: false,
                prune_reason: String::new(),
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
        } else if let Some(reason) = annotation(&line, "locked") {
            entry.locked = true;
            entry.lock_reason = reason.to_string();
        } else if let Some(reason) = annotation(&line, "prunable") {
            entry.prunable = true;
            entry.prune_reason = reason.to_string();
        }
    }
    if let Some(entry) = cur.take() {
        out.push(entry);
    }
    Ok(out)
}

/// A flag line that may carry words after it (`locked`, `prunable`):
/// `Some("")` for the bare word, `Some(reason)` for the word and its
/// reason, `None` for anything else. **The word has to end there** — a
/// future `lockedsomething` is another attribute, and git adds them to
/// this listing (the parse ignores the ones it does not know precisely
/// so that it can).
fn annotation<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    if line == word {
        return Some("");
    }
    line.strip_prefix(word)
        .and_then(|rest| rest.strip_prefix(' '))
        .map(str::trim)
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
        assert_eq!(list[1].lock_reason, "reason text");
        assert_eq!(list[1].branch.as_deref(), Some("dev"));
    }

    /// Every annotation git puts on one of these entries, copied off a
    /// real listing (git 2.55.0.windows.3): a lock with a reason and a
    /// lock without one read apart, and the entry whose folder is gone
    /// carries git's own words for why.
    #[test]
    fn parses_every_state_of_a_real_listing() {
        let bytes = z(&[
            "worktree C:/tmp/wtprobe/main",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/master",
            "",
            "worktree C:/tmp/wtprobe/wt-det",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "detached",
            "",
            "worktree C:/tmp/wtprobe/wt-gone",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/gone",
            "prunable gitdir file points to non-existent location",
            "",
            "worktree C:/tmp/wtprobe/wt-lock",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/locked",
            "locked seat held by claude",
            "",
            "worktree C:/tmp/wtprobe/wt-lock2",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/locked2",
            "locked",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert_eq!(list.len(), 5);
        // The everyday entry carries none of them.
        assert!(!list[0].locked && !list[0].prunable && !list[0].detached);
        assert!(list[1].detached);
        assert!(list[2].prunable);
        assert_eq!(
            list[2].prune_reason,
            "gitdir file points to non-existent location"
        );
        assert!(!list[2].locked);
        assert!(list[3].locked);
        assert_eq!(list[3].lock_reason, "seat held by claude");
        // Locked without a reason: the flag is on and there are no words
        // to show beside it. The two cannot collapse — a row saying
        // `Locked —` with nothing after it is what that would draw.
        assert!(list[4].locked);
        assert_eq!(list[4].lock_reason, "");
    }

    /// git adds attributes to this listing, so the parse ignores what it
    /// does not know — and a word that only starts like a flag it does
    /// know stays one of those.
    #[test]
    fn an_attribute_that_only_starts_like_a_flag_is_ignored() {
        let bytes = z(&[
            "worktree /srv/wt",
            "HEAD 3333333333333333333333333333333333333333",
            "lockedness whatever",
            "prunableness whatever",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert!(!list[0].locked);
        assert!(!list[0].prunable);
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
