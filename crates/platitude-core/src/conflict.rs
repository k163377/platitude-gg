//! Conflict resolution support: which files are conflicted, how far a
//! stepped operation has got, what its two sides are called, and taking
//! one of them wholesale. Handing the merge to an external tool is [`tool`]
//! (not `mergetool`, which would shadow the [`mergetool`] fn).

pub(crate) mod tool;

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::InProgress;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::status::{StatusItem, WorkTreeStatus};

/// The pre-merge tests' mock of [`available_tools`] (mry builds it into
/// debug builds only).
#[cfg(debug_assertions)]
pub use tool::mock_available_tools;
pub use tool::{available_tools, configured_tool, mergetool, set_merge_tool, user_defined_tools};

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
    pub(crate) fn from_stages(ours: char, theirs: char) -> Self {
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

    /// True when a merge tool has content on both sides to work with
    /// (`UU` / `AA`); the rest are decided by picking a side.
    pub fn is_content_conflict(self) -> bool {
        matches!(self, ConflictKind::BothModified | ConflictKind::BothAdded)
    }
}

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

/// What to call the two sides of a conflict, for whatever operation is
/// stopped. Either may be empty when git left nothing to name it by.
///
/// During a rebase the two swap over: `ours` is the upstream being landed
/// on, `theirs` the commits being replayed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sides {
    /// The side already in place (`--ours`).
    pub ours: String,
    /// The side being brought in (`--theirs`).
    pub theirs: String,
}

/// Reads what the stopped operation's two sides are called.
///
/// Where each name comes from (pinned by
/// `what_a_stopped_operation_says_about_its_two_sides`):
///
/// | operation | ours | theirs |
/// |---|---|---|
/// | rebase | `rebase-*/onto`, named | `rebase-*/head-name`, a full ref |
/// | merge | the current branch | `MERGE_HEAD`, named |
/// | cherry-pick | the current branch | `CHERRY_PICK_HEAD`, named |
/// | revert | the current branch | `REVERT_HEAD`, named |
///
/// `branch` is the current branch as the caller's status read it (`None`
/// detached: no name), handed in so each tick does not read HEAD again.
pub async fn sides(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    branch: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Sides, GitError> {
    if op == InProgress::Rebase {
        // Either backend may hold the rebase (one begun at the command line
        // may be `apply`); one `rev-parse` resolves all four paths.
        let mut cmd = GitCommand::new().cwd(workdir).arg("rev-parse");
        for rel in [
            "rebase-merge/head-name",
            "rebase-merge/onto",
            "rebase-apply/head-name",
            "rebase-apply/onto",
        ] {
            cmd = cmd.args(["--git-path", rel]);
        }
        let out = executor.run(cmd, cancel).await?;
        let mut values = [const { String::new() }; 4];
        for (slot, line) in values.iter_mut().zip(out.stdout_utf8().lines()) {
            *slot = std::fs::read_to_string(workdir.join(line.trim_end()))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
        }
        let [merge_head, merge_onto, apply_head, apply_onto] = values;
        let head_name = if merge_head.is_empty() {
            apply_head
        } else {
            merge_head
        };
        let onto = if merge_onto.is_empty() {
            apply_onto
        } else {
            merge_onto
        };
        return Ok(Sides {
            ours: name_of(executor, workdir, &onto, cancel).await,
            theirs: short_ref(&head_name),
        });
    }

    let ours = branch.unwrap_or_default().to_string();
    let pseudo_ref = match op {
        InProgress::Merge => "MERGE_HEAD",
        InProgress::CherryPick => "CHERRY_PICK_HEAD",
        InProgress::Revert => "REVERT_HEAD",
        // Answered above; an empty rev names nothing.
        InProgress::Rebase => "",
    };
    Ok(Sides {
        ours,
        theirs: name_of(executor, workdir, pseudo_ref, cancel).await,
    })
}

/// Reads a file in the git directory, empty when it is not there. The
/// path is asked for: a worktree's git directory is not
/// `<workdir>/.git`.
pub(crate) async fn git_file(
    executor: &GitExecutor,
    workdir: &Path,
    rel: &str,
    cancel: &CancellationToken,
) -> String {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--git-path", rel]);
    let Ok(out) = executor.run(cmd, cancel).await else {
        return String::new();
    };
    let path = out.stdout_utf8().trim().to_string();
    std::fs::read_to_string(workdir.join(path))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// The branch name a commit is reachable by (`main~3` for an older one),
/// empty when git cannot name one.
async fn name_of(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> String {
    if rev.is_empty() {
        return String::new();
    }
    let cmd = GitCommand::new().cwd(workdir).args([
        "name-rev",
        "--name-only",
        "--refs=refs/heads/*",
        rev,
    ]);
    let Ok(out) = executor.run(cmd, cancel).await else {
        return String::new();
    };
    let name = out.stdout_utf8().trim().to_string();
    // git says "undefined" for a commit no branch reaches.
    if name == "undefined" {
        String::new()
    } else {
        name
    }
}

fn short_ref(full: &str) -> String {
    full.strip_prefix("refs/heads/").unwrap_or(full).to_string()
}

/// Resolves a conflict by taking one side wholesale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The branch being merged into (`--ours`).
    Ours,
    /// The branch being merged in (`--theirs`).
    Theirs,
}

/// `git restore --ours|--theirs` then `git add`: restore writes the chosen
/// stage into the working tree but leaves the path unmerged; the add
/// resolves it. `Ours` is the upstream during a rebase ([`Sides`]).
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
    let restore = GitCommand::new()
        .cwd(workdir)
        .args(["restore", flag, "--"])
        .args(specs.iter().map(String::as_str));
    executor.run(restore, cancel).await?;
    let add = GitCommand::new()
        .cwd(workdir)
        .args(["add", "--"])
        .args(specs.iter().map(String::as_str));
    executor.run(add, cancel).await.map(drop)
}
