//! Conflict resolution support: which files are conflicted, how far a
//! stepped operation has got, and handing resolution to `git mergetool`.
//!
//! Resolving conflicts is explicitly out of scope for this application
//! (要望.md「内蔵conflictエディタ」): the built-in editor belongs to
//! whatever tool the user already configured.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::status::{StatusItem, WorkTreeStatus};

/// How a conflicted file conflicts, derived from its two stage letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both sides changed the content (`UU`).
    BothModified,
    /// Both sides added the path (`AA`).
    BothAdded,
    /// Deleted here, modified there (`DU`).
    DeletedByUs,
    /// Modified here, deleted there (`UD`).
    DeletedByThem,
    /// Added here only (`AU`).
    AddedByUs,
    /// Added there only (`UA`).
    AddedByThem,
    /// Both sides deleted it (`DD`).
    BothDeleted,
    /// Any other stage combination git may report.
    Other,
}

impl ConflictKind {
    fn from_stages(ours: char, theirs: char) -> Self {
        match (ours, theirs) {
            ('U', 'U') => ConflictKind::BothModified,
            ('A', 'A') => ConflictKind::BothAdded,
            ('D', 'U') => ConflictKind::DeletedByUs,
            ('U', 'D') => ConflictKind::DeletedByThem,
            ('A', 'U') => ConflictKind::AddedByUs,
            ('U', 'A') => ConflictKind::AddedByThem,
            ('D', 'D') => ConflictKind::BothDeleted,
            _ => ConflictKind::Other,
        }
    }

    /// True when a merge tool has three usable sides to work with. The
    /// delete/delete and add/add-with-no-base cases are decided by picking
    /// a side, not by editing.
    pub fn is_content_conflict(self) -> bool {
        matches!(self, ConflictKind::BothModified | ConflictKind::BothAdded)
    }
}

/// One conflicted path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictedFile {
    pub path: String,
    pub kind: ConflictKind,
}

/// Conflicted entries of a status snapshot, in git's order.
pub fn conflicted(status: &WorkTreeStatus) -> Vec<ConflictedFile> {
    status
        .conflicted()
        .filter_map(|item| match item {
            StatusItem::Unmerged { ours, theirs, path } => Some(ConflictedFile {
                path: path.clone(),
                kind: ConflictKind::from_stages(*ours, *theirs),
            }),
            _ => None,
        })
        .collect()
}

/// How far a stepped operation has got ("commit 3 of 7").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub current: u32,
    pub total: u32,
}

/// Progress of an in-progress rebase, or `None` when none is running.
///
/// Both rebase backends are read: the merge backend (the default, and what
/// `--interactive` always uses) counts in `rebase-merge/msgnum`, while the
/// apply backend counts in `rebase-apply/next`.
pub async fn rebase_progress(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<Progress>, GitError> {
    const FILES: [(&str, &str); 2] = [
        ("rebase-merge/msgnum", "rebase-merge/end"),
        ("rebase-apply/next", "rebase-apply/last"),
    ];
    let mut cmd = GitCommand::new().cwd(workdir).arg("rev-parse");
    for (current, total) in FILES {
        cmd = cmd.args(["--git-path", current]);
        cmd = cmd.args(["--git-path", total]);
    }
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let paths: Vec<&str> = text.lines().map(str::trim_end).collect();

    for pair in paths.chunks_exact(2) {
        // `--git-path` prints paths relative to the cwd or absolute ones;
        // joining handles both.
        let current = read_count(&workdir.join(pair[0]));
        let total = read_count(&workdir.join(pair[1]));
        if let (Some(current), Some(total)) = (current, total)
            && total > 0
        {
            return Ok(Some(Progress { current, total }));
        }
    }
    Ok(None)
}

fn read_count(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Launches `git mergetool` for `paths` (all conflicted files when empty).
///
/// No time limit: the tool runs for as long as the user takes, and
/// cancelling the session is what stops it. On Windows the subprocess gets
/// no console (CREATE_NO_WINDOW), so a terminal-based tool such as vimdiff
/// cannot be used — a GUI tool must be configured.
pub async fn mergetool(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // `--no-prompt` keeps git from asking on stdin, which is closed.
        .args(["mergetool", "--no-prompt"])
        .no_timeout();
    let cmd = if paths.is_empty() {
        cmd
    } else {
        cmd.arg("--")
            .args(paths.iter().map(|p| literal_pathspec(p)))
    };
    executor.run(cmd, cancel).await.map(drop)
}

/// The merge tool git would launch (`merge.guitool`, else `merge.tool`).
/// `None` means none is configured and `mergetool` would fail.
pub async fn configured_tool(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    for key in ["merge.guitool", "merge.tool"] {
        let cmd = GitCommand::new()
            .cwd(workdir)
            .args(["config", "--get", key]);
        let out = executor.run_unchecked(cmd, cancel).await?;
        if out.code == 0 {
            let value = out.stdout_utf8().trim().to_string();
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
    }
    Ok(None)
}

/// Resolves a conflict by taking one side wholesale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The branch being merged into (`--ours`).
    Ours,
    /// The branch being merged in (`--theirs`).
    Theirs,
}

/// `git checkout --ours|--theirs` followed by `git add`, which is what
/// "take this side" means to git.
///
/// Note the reversal during a rebase: the commits being replayed are
/// "theirs", so `Ours` is the upstream side the caller is landing on.
pub async fn take_side(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    side: Side,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let flag = match side {
        Side::Ours => "--ours",
        Side::Theirs => "--theirs",
    };
    let specs: Vec<String> = paths.iter().map(|p| literal_pathspec(p)).collect();
    let checkout = GitCommand::new()
        .cwd(workdir)
        .args(["checkout", flag, "--"])
        .args(specs.iter().map(String::as_str));
    executor.run(checkout, cancel).await?;
    let add = GitCommand::new()
        .cwd(workdir)
        .args(["add", "--"])
        .args(specs.iter().map(String::as_str));
    executor.run(add, cancel).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_stage_letters() {
        assert_eq!(
            ConflictKind::from_stages('U', 'U'),
            ConflictKind::BothModified
        );
        assert_eq!(
            ConflictKind::from_stages('D', 'U'),
            ConflictKind::DeletedByUs
        );
        assert_eq!(ConflictKind::from_stages('X', 'Y'), ConflictKind::Other);
    }

    #[test]
    fn only_content_conflicts_are_worth_a_merge_tool() {
        assert!(ConflictKind::BothModified.is_content_conflict());
        assert!(!ConflictKind::BothDeleted.is_content_conflict());
        assert!(!ConflictKind::DeletedByThem.is_content_conflict());
    }

    #[test]
    fn extracts_conflicted_entries_only() {
        let status = WorkTreeStatus {
            items: vec![
                StatusItem::Unmerged {
                    ours: 'U',
                    theirs: 'U',
                    path: "both.txt".into(),
                },
                StatusItem::Untracked {
                    path: "new.txt".into(),
                },
            ],
            ..Default::default()
        };
        let files = conflicted(&status);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "both.txt");
        assert_eq!(files[0].kind, ConflictKind::BothModified);
    }
}
