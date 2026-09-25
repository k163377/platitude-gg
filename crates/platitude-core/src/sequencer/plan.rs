//! Composing the range a rebase will replay: the one-commit edits, and
//! reading what sits under the oldest commit a plan takes in.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::RebaseOptions;
use crate::process::GitExecutor;
use crate::rebase_plan::RangeRead;
use crate::report;

use super::todo::{RebaseStep, TodoAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPlan {
    /// Revision the rebase treats as upstream; empty when `root` is set.
    pub upstream: String,
    /// The plan reaches the first commit, so the rebase needs `--root`.
    pub root: bool,
    pub steps: Vec<RebaseStep>,
    /// The tip `steps` was read against — what the replay checks HEAD
    /// against before it spawns ([`crate::session`]'s `Replay`). Taken from
    /// the last row (the range ends at HEAD), so pinning it costs no process.
    pub tip: String,
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
    /// How far back from the target (itself = 1) the plan starts: `squash`
    /// folds into the line above, so it needs the parent too. Reaching any
    /// further pulls what lies below into the range, and a merge there
    /// refuses the whole edit (`a_merge_under_the_dropped_commit_is_left_alone`).
    fn depth(&self) -> u32 {
        match self {
            Edit::SquashIntoParent => 2,
            Edit::Reword(_) | Edit::Drop => 1,
        }
    }
}

/// Builds the plan that applies `edit` to `oid`.
///
/// Refuses a range containing a merge, which a plain interactive rebase
/// would flatten (`--rebase-merges` is not offered here).
pub async fn plan_edit(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &str,
    edit: Edit,
    cancel: &CancellationToken,
) -> Result<EditPlan, GitError> {
    // The oldest commit the plan takes in; what sits under it is the
    // upstream.
    let bottom = match edit.depth() {
        1 => oid.to_string(),
        depth => format!("{oid}~{}", depth - 1),
    };
    let base = base_of(executor, workdir, &bottom, cancel).await?;
    // `decide_edit` checks this again, but refusing here spares the range
    // read.
    let Some((upstream, root)) = base.upstream() else {
        return Err(report::rewrite_unfetched_base());
    };

    // One read answers both the rows and whether a merge sits among them
    // (`%P`).
    let read =
        crate::rebase_plan::read_rows(executor, workdir, &range_arg(&upstream, root), cancel)
            .await?;
    decide_edit(&base, &read, oid, edit)
}

/// What those two reads mean: the plan, or the refusal. Kept free of git so
/// the four refusals × three edits are unit-tested without shaped
/// repositories. The order matters: the range is read before the commit is
/// looked for, so a merge answers before off-branch.
fn decide_edit(base: &Base, read: &RangeRead, oid: &str, edit: Edit) -> Result<EditPlan, GitError> {
    let Some((upstream, root)) = base.upstream() else {
        return Err(report::rewrite_unfetched_base());
    };
    if read.merges {
        return Err(report::rewrite_across_merge());
    }
    let mut steps: Vec<RebaseStep> = read
        .rows
        .iter()
        .map(|row| RebaseStep::pick(row.oid.clone(), row.subject.clone()))
        .collect();
    let Some(index) = steps.iter().position(|s| s.oid == oid) else {
        return Err(report::rewrite_off_branch(short(oid)));
    };
    // The position above proves the list non-empty; an empty tip would
    // refuse the replay (`Replay::tip_still_stands`).
    let tip = steps
        .last()
        .map(|step| step.oid.clone())
        .unwrap_or_default();
    match edit {
        Edit::SquashIntoParent => {
            if index == 0 {
                return Err(report::fold_first_commit(short(oid)));
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
        tip,
    })
}

fn short(oid: &str) -> &str {
    oid.get(..8).unwrap_or(oid)
}

/// What a plan starting at `bottom` would replay onto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Base {
    /// The commit under `bottom`, which the rebase names as upstream.
    Commit(String),
    /// Nothing under it: `bottom` is the first commit, so the rebase needs
    /// `--root`. Also the answer when `bottom` itself does not exist (the
    /// caller reached past the first commit; its own scan says so better).
    Root,
    /// Something under it that this clone never fetched: it cannot be
    /// named and `--root` would be wrong, so it must never read as
    /// [`Base::Root`].
    Unfetched,
}

impl Base {
    /// The revision a rebase would name as upstream and whether it needs
    /// `--root` — `None` where there is nothing to replay onto at all.
    /// One copy, so the range read and the range composed are the same.
    fn upstream(&self) -> Option<(String, bool)> {
        match self {
            Base::Commit(oid) => Some((oid.clone(), false)),
            Base::Root => Some((String::new(), true)),
            Base::Unfetched => None,
        }
    }
}

/// Reads what sits under `bottom`, the oldest commit a plan takes in. The
/// extra reads run only when git says nothing is under it.
pub(crate) async fn base_of(
    executor: &GitExecutor,
    workdir: &Path,
    bottom: &str,
    cancel: &CancellationToken,
) -> Result<Base, GitError> {
    if let Some(oid) = resolve(executor, workdir, &format!("{bottom}~1"), cancel).await? {
        return Ok(Base::Commit(oid));
    }
    // `bottom` may itself not exist (`Base::Root`).
    let Some(bottom) = resolve(executor, workdir, bottom, cancel).await? else {
        return Ok(Base::Root);
    };
    if parent_is_unfetched(executor, workdir, &bottom, cancel).await? {
        return Ok(Base::Unfetched);
    }
    Ok(Base::Root)
}

/// Resolves a revision, returning `None` when git does not know it.
async fn resolve(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    let cmd = crate::process::GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
        .arg(format!("{rev}^{{commit}}"))
        // Exit 1 = "no such commit", the answer at the first commit's
        // parent (.claude/rules/core.md).
        .answers_by_code(1);
    let described = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => {}
        // Only 1 is "not found" (a missing name, an unknown id, a
        // non-commit). A 128 is git failing to read; taking it as "no such
        // commit" would rebase a broken repository from the root.
        1 => return Ok(None),
        code => {
            return Err(GitError::Failed {
                command: described,
                code,
                stderr: out.failure_message(),
            });
        }
    }
    let text = out.stdout_utf8().trim().to_string();
    Ok((!text.is_empty()).then_some(text))
}

/// Whether `rev` has a parent this clone never fetched, as against being
/// the first commit. At a shallow edge `%P` is empty and `<edge>~1` exits
/// 1, as at the real first commit; only the raw object still differs — the
/// edge keeps its `parent` header.
async fn parent_is_unfetched(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = crate::process::GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "commit", "--end-of-options"])
        .arg(rev);
    let out = executor.run(cmd, cancel).await?;
    Ok(has_parent_header(&out.stdout))
}

/// Whether a raw commit object carries a `parent` header. Only the headers
/// (up to the first empty line) are scanned, so a message cannot pass for
/// one. A CR is stripped although git writes LF: mistaking an edge for the
/// root is the costly direction.
pub(crate) fn has_parent_header(object: &[u8]) -> bool {
    for line in object.split(|b| *b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            return false;
        }
        if line.starts_with(b"parent ") {
            return true;
        }
    }
    false
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

/// The range a plan replays, spelled once for the sequencer's reads, the
/// preview and the rewrite warning.
pub(crate) fn range_arg(upstream: &str, root: bool) -> String {
    if root {
        "HEAD".to_string()
    } else {
        format!("{upstream}..HEAD")
    }
}

#[cfg(test)]
mod tests {
    use super::{Base, Edit, RangeRead, TodoAction, decide_edit};
    use crate::rebase_plan::PlanRow;
    use crate::report::ReportKind;

    fn row(oid: &str) -> PlanRow {
        PlanRow {
            oid: oid.to_string(),
            subject: format!("{oid} subject"),
            author_name: "Ada".to_string(),
            author_email: "ada@example.com".to_string(),
        }
    }

    /// A range read over three commits, oldest first, with no merge in it.
    fn three() -> RangeRead {
        RangeRead {
            rows: vec![row("aaa"), row("bbb"), row("ccc")],
            merges: false,
        }
    }

    /// Compared by kind: the error's display is the sentence alone
    /// (`#[error("{message}")]`).
    fn refusal(edit: Edit, base: &Base, read: &RangeRead, oid: &str) -> ReportKind {
        let err = decide_edit(base, read, oid, edit).expect_err("refused");
        err.report().expect("a refusal carries its report").kind
    }

    #[test]
    fn a_base_that_was_never_fetched_is_refused() {
        assert_eq!(
            refusal(Edit::Drop, &Base::Unfetched, &three(), "bbb"),
            ReportKind::RewriteUnfetchedBase
        );
    }

    #[test]
    fn a_range_holding_a_merge_is_refused() {
        let read = RangeRead {
            merges: true,
            ..three()
        };
        assert_eq!(
            refusal(Edit::Drop, &Base::Root, &read, "bbb"),
            ReportKind::RewriteAcrossMerge
        );
    }

    /// Pins the order `decide_edit` names: the merge answers first.
    #[test]
    fn the_merge_answers_before_the_commit_is_looked_for() {
        let read = RangeRead {
            merges: true,
            ..three()
        };
        assert_eq!(
            refusal(Edit::Drop, &Base::Root, &read, "nowhere"),
            ReportKind::RewriteAcrossMerge
        );
        assert_eq!(
            refusal(Edit::SquashIntoParent, &Base::Root, &read, "aaa"),
            ReportKind::RewriteAcrossMerge,
            "and before the fold of a first commit, for the same reason"
        );
    }

    #[test]
    fn a_commit_the_range_does_not_hold_is_refused() {
        assert_eq!(
            refusal(Edit::Reword("nope".into()), &Base::Root, &three(), "zzz"),
            ReportKind::RewriteOffBranch
        );
        assert_eq!(
            refusal(Edit::Drop, &Base::Root, &RangeRead::default(), "aaa"),
            ReportKind::RewriteOffBranch,
            "an empty range holds nothing, so it holds this commit least of all"
        );
    }

    #[test]
    fn folding_the_oldest_commit_in_the_range_is_refused_and_the_others_are_not() {
        assert_eq!(
            refusal(Edit::SquashIntoParent, &Base::Root, &three(), "aaa"),
            ReportKind::FoldFirstCommit
        );
        for edit in [Edit::Drop, Edit::Reword("new".into())] {
            decide_edit(&Base::Root, &three(), "aaa", edit).expect("reaches only itself");
        }
    }

    /// The tip comes off the last row whatever the edit.
    #[test]
    fn a_plan_picks_the_whole_range_and_marks_the_one_commit() {
        let plan = decide_edit(&Base::Root, &three(), "bbb", Edit::SquashIntoParent)
            .expect("the middle commit folds into the one before it");
        assert!(plan.root, "a root base needs --root and names no upstream");
        assert_eq!(plan.upstream, "");
        assert_eq!(plan.tip, "ccc");
        assert_eq!(
            plan.steps.iter().map(|s| s.action).collect::<Vec<_>>(),
            vec![TodoAction::Pick, TodoAction::Squash, TodoAction::Pick]
        );
        assert!(
            plan.steps.iter().all(|s| s.message.is_none()),
            "only a reword carries one"
        );

        let plan = decide_edit(&Base::Commit("aaa".into()), &three(), "ccc", Edit::Drop)
            .expect("the tip is dropped");
        assert!(!plan.root);
        assert_eq!(plan.upstream, "aaa", "named as the rebase's upstream");
        assert_eq!(plan.tip, "ccc", "the tip is what was read, dropped or not");
        assert_eq!(plan.steps[2].action, TodoAction::Drop);

        let plan = decide_edit(&Base::Root, &three(), "ccc", Edit::Reword("new".into()))
            .expect("the tip is reworded");
        assert_eq!(plan.steps[2].action, TodoAction::Reword);
        assert_eq!(plan.steps[2].message.as_deref(), Some("new"));
    }
}
