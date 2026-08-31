//! Composing the range a rebase will replay: the one-commit edits, and
//! reading what sits under the oldest commit a plan takes in.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::RebaseOptions;
use crate::process::GitExecutor;

use super::todo::{RebaseStep, TodoAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPlan {
    /// Revision the rebase treats as upstream; empty when `root` is set.
    pub upstream: String,
    /// The plan reaches the first commit, so the rebase needs `--root`.
    pub root: bool,
    pub steps: Vec<RebaseStep>,
}

impl EditPlan {
    /// Rebase options that replay exactly this plan's range.
    pub fn options(&self) -> RebaseOptions {
        RebaseOptions {
            root: self.root,
            ..Default::default()
        }
    }
}

/// A single-commit change to an existing history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Fold the commit into the one before it, combining the messages.
    SquashIntoParent,
    /// Replace the commit's message.
    Reword(String),
    /// Leave the commit out of the history entirely.
    Drop,
}

impl Edit {
    /// How many commits before the target the plan has to start at.
    ///
    /// `squash` folds into the line above it, so the parent must be in the
    /// plan as well; a reword and a drop only need the commit itself.
    ///
    /// Reaching one further than that is not free: whatever sits below the
    /// commit joins the range, and a merge down there is enough to refuse
    /// the whole edit even though the replay would never have touched it
    /// (measured — `a_merge_under_the_dropped_commit_is_left_alone`).
    fn depth(&self) -> u32 {
        match self {
            Edit::SquashIntoParent => 2,
            Edit::Reword(_) | Edit::Drop => 1,
        }
    }
}

/// Builds the plan that applies `edit` to `oid`.
///
/// Refuses a range containing a merge: a plain interactive rebase drops
/// merge commits, so carrying on would silently flatten the history the
/// user is looking at. `--rebase-merges` is a different operation, and the
/// UI does not offer it here.
pub async fn plan_edit(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &str,
    edit: Edit,
    cancel: &CancellationToken,
) -> Result<EditPlan, GitError> {
    // These refusals are this application's own, decided before a rebase
    // is ever spawned. Naming a command as having said something
    // unexpected would put a command the person never ran in front of
    // them; `Rejected` is shown as it stands (規約 §git が言ったことを読む場所).
    let fail = |message: String| GitError::Rejected { message };

    // History shorter than the plan needs means the range starts at the
    // very first commit, which has no parent to name as upstream.
    let start = format!("{oid}~{}", edit.depth());
    let upstream: String = resolve(executor, workdir, &start, cancel)
        .await?
        .unwrap_or_default();
    let root = upstream.is_empty();

    // One read answers both questions: the rows, and whether a merge sits
    // among them (`%P` rides along — crate::rebase_plan::read_rows, the
    // same parser the full plan's preview goes through).
    let read =
        crate::rebase_plan::read_rows(executor, workdir, &range_arg(&upstream, root), cancel)
            .await?;
    if read.merges {
        return Err(fail(
            "this range contains a merge commit, which a rebase would drop".to_string(),
        ));
    }

    let mut steps: Vec<RebaseStep> = read
        .rows
        .into_iter()
        .map(|row| RebaseStep::pick(row.oid, row.subject))
        .collect();
    let Some(index) = steps.iter().position(|s| s.oid == oid) else {
        return Err(fail(format!(
            "{} is not in the history of the current branch",
            short(oid)
        )));
    };
    match edit {
        Edit::SquashIntoParent => {
            if index == 0 {
                return Err(fail(format!(
                    "{} is the first commit, so it has nothing to fold into",
                    short(oid)
                )));
            }
            steps[index].action = TodoAction::Squash;
        }
        Edit::Reword(message) => {
            steps[index].action = TodoAction::Reword;
            steps[index].message = Some(message);
        }
        Edit::Drop => steps[index].action = TodoAction::Drop,
    }
    Ok(EditPlan {
        upstream,
        root,
        steps,
    })
}

fn short(oid: &str) -> &str {
    oid.get(..8).unwrap_or(oid)
}

/// Resolves a revision, returning `None` when git does not know it.
pub(crate) async fn resolve(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    let cmd = crate::process::GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
        .arg(format!("{rev}^{{commit}}"))
        // "there is no such commit" is the answer, not a failure: a plan
        // that reaches the very first commit asks for its parent and is
        // told there is none. Left unmarked it counts as a failed command
        // and the command log throws its panel open over a perfectly good
        // squash or drop near the root (.claude/rules/core.md).
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Ok(None);
    }
    let text = out.stdout_utf8().trim().to_string();
    Ok((!text.is_empty()).then_some(text))
}

/// The commits `git rebase -i <upstream>` would offer, oldest first.
pub async fn plan_for(
    executor: &GitExecutor,
    workdir: &Path,
    upstream: &str,
    cancel: &CancellationToken,
) -> Result<Vec<RebaseStep>, GitError> {
    let read =
        crate::rebase_plan::read_rows(executor, workdir, &range_arg(upstream, false), cancel)
            .await?;
    Ok(read
        .rows
        .into_iter()
        .map(|row| RebaseStep::pick(row.oid, row.subject))
        .collect())
}

/// The range a plan replays, spelled in one place for everyone who says
/// it — the sequencer's own reads, the preview, and the rewrite warning
/// the screen asks about (three sayers is past the tolerated two).
pub(crate) fn range_arg(upstream: &str, root: bool) -> String {
    if root {
        "HEAD".to_string()
    } else {
        format!("{upstream}..HEAD")
    }
}
