//! Conflict resolution support: which files are conflicted, how far a
//! stepped operation has got, what its two sides are called, and taking
//! one of them wholesale.
//!
//! Handing the merge itself over to an editor is the other half, and it
//! is about reaching that tool rather than about the conflict
//! ([`tool`]). **Not `mergetool`** — [`mergetool`] is the launch itself,
//! and a module of that name would shadow it.

pub(crate) mod tool;

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::InProgress;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::status::{StatusItem, WorkTreeStatus};

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

    /// True when a merge tool has three usable sides to work with. The
    /// delete/delete and add/add-with-no-base cases are decided by picking
    /// a side, not by editing.
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
/// **The two swap over during a rebase**: the commits being replayed are
/// `theirs`, and `ours` is the upstream they are landing on. Reading the
/// names from the operation is what keeps a UI from having to explain
/// that — each side is called what it actually is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sides {
    /// The side already in place (`--ours`).
    pub ours: String,
    /// The side being brought in (`--theirs`).
    pub theirs: String,
}

/// Reads what the stopped operation's two sides are called.
///
/// Where each name comes from (measured —
/// `what_a_stopped_operation_says_about_its_two_sides`):
///
/// | operation | ours | theirs |
/// |---|---|---|
/// | rebase | `rebase-*/onto`, named | `rebase-*/head-name`, a full ref |
/// | merge | the current branch | `MERGE_HEAD`, named |
/// | cherry-pick | the current branch | `CHERRY_PICK_HEAD`, named |
/// | revert | the current branch | `REVERT_HEAD`, named |
pub async fn sides(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    cancel: &CancellationToken,
) -> Result<Sides, GitError> {
    if op == InProgress::Rebase {
        // Both backends keep the same two files under their own
        // directory; the merge backend is the default and the only one
        // this app starts, but a rebase begun at the command line may be
        // the other. One `rev-parse` resolves all four paths (the shape
        // `opstate::detect` uses) instead of a process per file.
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

    let head = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--abbrev-ref", "HEAD"]);
    let ours = executor
        .run(head, cancel)
        .await?
        .stdout_utf8()
        .trim()
        .into();
    // A detached HEAD names itself "HEAD", which says nothing about a
    // side; better to leave it to the caller's own wording.
    let ours = if ours == "HEAD" { String::new() } else { ours };

    let pseudo_ref = match op {
        InProgress::Merge => "MERGE_HEAD",
        InProgress::CherryPick => "CHERRY_PICK_HEAD",
        InProgress::Revert => "REVERT_HEAD",
        // Answered above. Named here rather than left to a panic: the
        // match stays exhaustive, and an empty rev names nothing.
        InProgress::Rebase => "",
    };
    Ok(Sides {
        ours,
        theirs: name_of(executor, workdir, pseudo_ref, cancel).await,
    })
}

/// Reads a file in the git directory, empty when it is not there. The
/// path is asked for rather than assumed: a worktree's git directory is
/// not `<workdir>/.git`.
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

/// The branch name a commit is reachable by, empty when git cannot name
/// one. An older commit comes back as `main~3`, which still says which
/// line of history it belongs to.
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

/// `git restore --ours|--theirs` followed by `git add`, which is what
/// "take this side" means to git (restore writes the chosen stage into
/// the working tree and leaves the path unmerged; the add resolves it —
/// measured, identical to the older `checkout --ours` spelling).
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
