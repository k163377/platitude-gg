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
    /// against before it spawns ([`crate::session`]'s `Replay`).
    ///
    /// **Taken out of the rows.** The range always ends at HEAD
    /// (`range_arg`) and `read_rows` reverses git's own order, so the
    /// last step *is* the tip this plan was composed against: pinning it
    /// costs no process at all, which is what kept it unpinned while the
    /// pin was thought to need one on the response path of the three edits
    /// people click most (CLAUDE.md §性能予算).
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
    /// How many commits before the target the plan has to start at.
    ///
    /// `squash` folds into the line above it, so the parent must be in the
    /// plan as well; a reword and a drop only need the commit itself.
    ///
    /// Reaching one further than that costs: whatever sits below the
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
    // The oldest commit the plan takes in; what sits under *that* is the
    // upstream. History shorter than the plan needs means the range starts
    // at the very first commit, which has no parent to name as upstream.
    let bottom = match edit.depth() {
        1 => oid.to_string(),
        depth => format!("{oid}~{}", depth - 1),
    };
    let base = base_of(executor, workdir, &bottom, cancel).await?;
    // Read here for the range to spell, and read again inside the
    // decision — a base nothing can be replayed onto is the whole answer,
    // so the range is never spelled and the second process is never
    // spawned to reach it.
    let Some((upstream, root)) = base.upstream() else {
        return Err(report::rewrite_unfetched_base());
    };

    // One read answers both questions: the rows, and whether a merge sits
    // among them (`%P` rides along — crate::rebase_plan::read_rows, the
    // same parser the full plan's preview goes through).
    let read =
        crate::rebase_plan::read_rows(executor, workdir, &range_arg(&upstream, root), cancel)
            .await?;
    decide_edit(&base, &read, oid, edit)
}

/// What those two reads mean: the plan, or the refusal.
///
/// **The deciding half, with no git in it.** Four refusals and three
/// edits over one range is a table, and every row of it costs a
/// repository shaped to produce that answer — a merge under the target, a
/// shallow clone, a commit on another branch. The order the four are
/// decided in is the part that has ever been wrong (`plan_edit` reaches
/// the range before it looks for the commit, so a merge answers before
/// off-branch), and an order is exactly what a table can be read for.
///
/// The refusals are this application's own, decided before a rebase is
/// ever spawned, and the screen states each of them in its own words:
/// they are `report`'s to word, so the sentence the reader gets and the
/// sentence the log keeps stay one decision
/// (規約 §git が言ったことを読む場所).
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
    // The last step is the tip: the range ends at HEAD either way
    // (`range_arg`) and `read_rows` hands git's newest-first order back
    // reversed. The position above says the list is not empty, and an
    // empty tip would refuse the replay
    // (`Replay::tip_still_stands`).
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
    /// There is nothing under it: `bottom` is the history's first commit,
    /// so the rebase needs `--root`. Also the answer where `bottom` is
    /// itself not a commit this repository holds — the caller reaching one
    /// further down than the history goes, which its own scan says better.
    Root,
    /// There is something under it that this clone never fetched. Naming
    /// it is impossible and `--root` would be a lie — the three-way answer
    /// exists so neither entry point can take this for [`Base::Root`].
    Unfetched,
}

impl Base {
    /// The revision a rebase would name as upstream and whether it needs
    /// `--root` — `None` where there is nothing to replay onto at all.
    /// One copy, because the range a plan is read over and the range it
    /// is composed against have to be the same one.
    fn upstream(&self) -> Option<(String, bool)> {
        match self {
            Base::Commit(oid) => Some((oid.clone(), false)),
            Base::Root => Some((String::new(), true)),
            Base::Unfetched => None,
        }
    }
}

/// Reads what sits under `bottom`, the oldest commit a plan takes in.
///
/// The extra reads go out only where git says there is nothing under it —
/// the rarest answer, and the one worth asking about. Whatever the
/// walk costs from there dwarfs them.
pub(crate) async fn base_of(
    executor: &GitExecutor,
    workdir: &Path,
    bottom: &str,
    cancel: &CancellationToken,
) -> Result<Base, GitError> {
    if let Some(oid) = resolve(executor, workdir, &format!("{bottom}~1"), cancel).await? {
        return Ok(Base::Commit(oid));
    }
    // Nothing under it — but `bottom` may be the thing that is not there:
    // a squash asked for at the very first commit names a range starting
    // one below where the history goes. That is not this function's to
    // answer; the caller's own scan has the better word for it.
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
        // "there is no such commit" is the answer here: a plan that
        // reaches the very first commit asks for its parent and is
        // told there is none. Left unmarked it counts as a failed command
        // and the command log throws its panel open over a perfectly good
        // squash or drop near the root (.claude/rules/core.md).
        .answers_by_code(1);
    let described = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => {}
        // Only the code the command named is the answer, exactly the line
        // `answers_by_code` draws for the log: everything git can read the
        // repository for and still not find exits 1 — a name that is not
        // there, an id of the right shape that is no object, a tree asked
        // for as a commit (measured 2.55). A 128 is git failing to read at
        // all, and reading that as "there is no such commit" would turn a
        // broken repository into a plan that rebases from the root.
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
/// the history's own first commit.
///
/// A shallow clone answers the two the same way: at its edge `%P` comes
/// back empty and `<edge>~1` exits 1, exactly as at the real first commit
/// (measured 2.55). git grafts the edge as it parses and leaves the stored
/// object alone, so the raw commit is the one place the two still differ —
/// the edge keeps its `parent` header, the first commit never had one.
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

/// Whether a raw commit object carries a `parent` header.
///
/// The headers run to the first empty line, so a message that opens with
/// the word cannot be taken for one. git writes the object with LF even on
/// Windows (measured), and a CR is stripped anyway: mistaking an edge for
/// the root is the costly direction of this answer.
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

    /// Which report a refusal carries, which is what the screen writes
    /// both of its lines from — the sentence alone reads the same whatever
    /// kind went back (`#[error("{message}")]`).
    fn refusal(edit: Edit, base: &Base, read: &RangeRead, oid: &str) -> ReportKind {
        let err = decide_edit(base, read, oid, edit).expect_err("refused");
        err.report().expect("a refusal carries its report").kind
    }

    /// A base this clone never fetched cannot be named and is not the
    /// root, so nothing can be composed over it.
    #[test]
    fn a_base_that_was_never_fetched_is_refused() {
        assert_eq!(
            refusal(Edit::Drop, &Base::Unfetched, &three(), "bbb"),
            ReportKind::RewriteUnfetchedBase
        );
    }

    /// A plain interactive rebase drops merge commits, so a range holding
    /// one is refused rather than silently flattened.
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

    /// **And it is refused first.** The range is read before the commit is
    /// looked for, so a click on a commit the branch cannot see, over a
    /// history with a merge in it, is answered by the merge — the order
    /// the screen's sentences come out in, and the one thing here that
    /// has ever been wrong.
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

    /// A commit the range does not hold is one this branch cannot see.
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

    /// The oldest commit in the range has nothing above it to fold into.
    /// The other two edits reach only the commit itself, so they are fine
    /// there.
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

    /// What a plan comes out as: every commit in the range picked, the one
    /// asked for carrying the edit, and the tip taken off the last row —
    /// which is what the replay checks HEAD against before it spawns.
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
