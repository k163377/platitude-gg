//! Composing the range a rebase will replay: the one-commit edits, and
//! reading what sits under the oldest commit a plan takes in.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::integrate::RebaseOptions;
use crate::process::GitExecutor;
use crate::report;

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
    // The refusals below are this application's own, decided before a
    // rebase is ever spawned, and the screen states each of them in its
    // own words: they are `report`'s to word, so that the sentence the
    // reader gets and the sentence the log keeps stay one decision
    // (規約 §git が言ったことを読む場所).

    // The oldest commit the plan takes in; what sits under *that* is the
    // upstream. History shorter than the plan needs means the range starts
    // at the very first commit, which has no parent to name as upstream.
    let bottom = match edit.depth() {
        1 => oid.to_string(),
        depth => format!("{oid}~{}", depth - 1),
    };
    let (upstream, root) = match base_of(executor, workdir, &bottom, cancel).await? {
        Base::Commit(oid) => (oid, false),
        Base::Root => (String::new(), true),
        Base::Unfetched => return Err(report::rewrite_unfetched_base()),
    };

    // One read answers both questions: the rows, and whether a merge sits
    // among them (`%P` rides along — crate::rebase_plan::read_rows, the
    // same parser the full plan's preview goes through).
    let read =
        crate::rebase_plan::read_rows(executor, workdir, &range_arg(&upstream, root), cancel)
            .await?;
    if read.merges {
        return Err(report::rewrite_across_merge());
    }

    let mut steps: Vec<RebaseStep> = read
        .rows
        .into_iter()
        .map(|row| RebaseStep::pick(row.oid, row.subject))
        .collect();
    let Some(index) = steps.iter().position(|s| s.oid == oid) else {
        return Err(report::rewrite_off_branch(short(oid)));
    };
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

/// Reads what sits under `bottom`, the oldest commit a plan takes in.
///
/// The extra reads go out only where git says there is nothing under it —
/// the rarest answer, and the one that must not be guessed. Whatever the
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
        // "there is no such commit" is the answer, not a failure: a plan
        // that reaches the very first commit asks for its parent and is
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
