//! The range the interactive-rebase screen composes over.
//!
//! A right-clicked commit asks what `git rebase --interactive` would offer
//! from there up: the commits of `from^..HEAD`, oldest first, with enough
//! about each to draw a row. Nothing here runs a rebase — the plan the
//! screen assembles replays through [`crate::sequencer`], and this read is
//! what it starts from. [`read_rows`] is the one parser both this preview
//! and the sequencer's own plans go through, so the two entry points
//! cannot come to read one repository differently.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};
use crate::publish;
use crate::sequencer;

/// One commit of the range, as the screen lists it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanRow {
    pub oid: String,
    pub subject: String,
    pub author_name: String,
    pub author_email: String,
}

/// What one pass over a range read: the rows oldest-first, and whether a
/// merge sits among them — taken off the same `%P` field, so the refusal
/// costs no second process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RangeRead {
    pub rows: Vec<PlanRow>,
    pub merges: bool,
}

/// What a preview came back with: the rows to compose over, oldest first
/// (git's todo order — the screen turns them round itself).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPreview {
    /// The commit the plan was asked from — the oldest one included.
    pub from: String,
    /// Its parent, which the plan replays onto; empty when `root`.
    pub upstream: String,
    /// The range reaches the very first commit, so a rebase needs
    /// `--root` and there is no upstream to name.
    pub root: bool,
    /// The very range the plan replays, spelled once here — the rewrite
    /// warning asks about this string, so it cannot drift from what the
    /// rebase touches.
    pub range: String,
    /// How many commits of that range a remote already has, as of this
    /// read — the `already pushed` count the run button wears. Read with
    /// the rows, so the screen opens with its warning on; the refs
    /// moving under an open plan is what asks again
    /// ([`crate::session::RepoSession::check_plan_published`]).
    pub published: u32,
    pub rows: Vec<PlanRow>,
    /// The commit the rows land on, for the screen's own `onto` row.
    /// `None` when `root` — there is nothing under the first commit.
    pub onto: Option<PlanRow>,
    /// A local branch standing exactly on `upstream`, where one does —
    /// the name the screen says first, with the id as the fallback.
    /// Empty when none stands there, or on `root`.
    pub onto_ref: String,
}

/// Why a preview was not made — this end's own answer, decided before a
/// rebase is ever spawned. The wording belongs to the screen
/// (app-ui.md「Rust に文言を置かない」), so what travels is the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRefusal {
    /// The range holds a merge commit, which a plain interactive rebase
    /// drops — carrying on would silently flatten the history
    /// (デザイン規約 §履歴を合流させる; `--rebase-merges` is a different
    /// operation and the UI does not offer it).
    AcrossMerge,
    /// The commit is not in the current branch's history: a rebase only
    /// ever rewrites the branch the tree is standing on.
    OffBranch,
    /// The commit below the range was never fetched — a shallow clone's
    /// edge, which git answers exactly as it answers the real first commit
    /// (`%P` empty, `<edge>~1` exits 1). Offered as `--root`, the replay
    /// rewrites the edge into a first commit and cuts the branch off from
    /// the history this clone does not hold: measured on a `--depth=3`
    /// clone, dropping the edge left the branch on one commit where the
    /// remote had six.
    UnfetchedBase,
}

/// A preview's two honest outcomes, apart from a read that failed. The
/// plan rides boxed: a refusal is one byte, and the rows make the other
/// variant hundreds (clippy `large_enum_variant`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanAnswer {
    Plan(Box<PlanPreview>),
    Refused(PlanRefusal),
}

/// Reads what a plan from `from` (a full commit id) would be made of.
///
/// Two serial process latencies in all: the range read carries the
/// merge answer in its own `%P` field, and the onto row, its branch name
/// and the count a remote already has of the range — all about the
/// already-resolved range — run side by side. Serial spawns are what a
/// 100ms interaction budget goes on (CLAUDE.md §性能予算,
/// ci/baseline/code-costs-windows-x64.md §git のプロセス代). More go out
/// only where git says there is nothing under `from`, to tell the
/// history's first commit from a clone that stops there
/// ([`sequencer::base_of`]).
pub async fn preview(
    executor: &GitExecutor,
    workdir: &Path,
    from: &str,
    cancel: &CancellationToken,
) -> Result<PlanAnswer, GitError> {
    // The base answers before the range is read: a clone that stops here
    // has nothing under `from` to replay onto, and refusing now also
    // spares it the walk over everything it *does* hold.
    let (upstream, root) = match sequencer::base_of(executor, workdir, from, cancel).await? {
        sequencer::Base::Commit(oid) => (oid, false),
        sequencer::Base::Root => (String::new(), true),
        sequencer::Base::Unfetched => {
            return Ok(PlanAnswer::Refused(PlanRefusal::UnfetchedBase));
        }
    };
    let range = sequencer::range_arg(&upstream, root);

    let read = read_rows(executor, workdir, &range, cancel).await?;
    // The merge answers first, as it does for the one-commit edits
    // (`sequencer::plan_edit`): the two refusals overlap on a mis-click
    // into another branch's history, and the two entry points should
    // answer such a click with the same word.
    if read.merges {
        return Ok(PlanAnswer::Refused(PlanRefusal::AcrossMerge));
    }
    // A linear `from^..HEAD` starts at `from` exactly when `from` is an
    // ancestor of HEAD; anything else was a click on some other branch's
    // row, and the range holds that branch's unrelated tail.
    if read.rows.first().is_none_or(|row| row.oid != from) {
        return Ok(PlanAnswer::Refused(PlanRefusal::OffBranch));
    }

    // What the remotes already have of the range, beside the base reads:
    // a warning that arrived a beat after the screen opened would be one
    // the reader had already pressed past. The count is the answer even
    // where it could not be read — no warning is what a plan over a
    // repository with no remote shows, and a read that failed is logged
    // where every read's failure is.
    let published = publish::state_of(executor, workdir, &range, cancel);
    let (onto, onto_ref, published) = if root {
        let published = published.await;
        (None, String::new(), published)
    } else {
        let just_the_base = format!("{upstream}^!");
        let (onto, named, published) = tokio::join!(
            read_rows(executor, workdir, &just_the_base, cancel),
            branch_at(executor, workdir, &upstream, cancel),
            published,
        );
        // The name is the decoration; the base itself is the answer. The
        // screen already writes the short id where no branch stands there
        // (デザイン規約 §フル interactive rebase: 指すブランチが無ければ sha),
        // so a `for-each-ref` that failed takes the same road and the
        // preview stands.
        let named = named.unwrap_or_else(|error| {
            tracing::debug!(%error, "no branch name for the plan's base; its id stands in");
            String::new()
        });
        (onto?.rows.pop(), named, published)
    };
    let published = match published {
        Ok(state) => state.published(),
        Err(error) if error.is_cancelled() => return Err(error),
        Err(error) => {
            tracing::warn!(%error, "could not count what a remote has of the plan's range");
            0
        }
    };
    Ok(PlanAnswer::Plan(Box::new(PlanPreview {
        from: from.to_string(),
        upstream,
        root,
        range,
        published,
        rows: read.rows,
        onto,
        onto_ref,
    })))
}

/// The first local branch standing exactly on `oid`, empty where none is.
///
/// "First" is the graph's own order, so the base wears in the header the
/// name its row would lead with: current branch, then kind, then name
/// (`session::model::LabelIndex::from_pairs`). Only the last of the three
/// can decide anything here — the query is local branches alone, and the
/// current branch cannot stand on the base, which the range `upstream..HEAD`
/// has already been shown to sit above. Sorting is named here, and git's
/// default happens to agree (measured 2.55).
async fn branch_at(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &str,
    cancel: &CancellationToken,
) -> Result<String, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["for-each-ref", "refs/heads", "--count=1"])
        .arg("--sort=refname:short")
        .arg(format!("--points-at={oid}"))
        .args(["--format=%(refname:short)"]);
    let out = executor.run(cmd, cancel).await?;
    Ok(out.stdout_utf8().lines().next().unwrap_or("").to_string())
}

/// The commits of `range`, oldest first, with the fields a screen row
/// shows and whether any of them is a merge. Five NUL-separated fields
/// per record — none of them can carry a NUL, so the flat split below
/// cannot tear one.
pub(crate) async fn read_rows(
    executor: &GitExecutor,
    workdir: &Path,
    range: &str,
    cancel: &CancellationToken,
) -> Result<RangeRead, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "log",
            "--reverse",
            "-z",
            "--format=%H%x00%P%x00%an%x00%ae%x00%s",
        ])
        .arg(range);
    let out = executor.run(cmd, cancel).await?;
    let fields: Vec<&[u8]> = out.stdout.split(|b| *b == 0).collect();
    let mut read = RangeRead::default();
    for record in fields.chunks(5) {
        let [oid, parents, name, email, subject] = record else {
            continue;
        };
        let oid = String::from_utf8_lossy(oid);
        let oid = oid.trim();
        if oid.is_empty() {
            continue;
        }
        // Two ids in `%P` is a merge: the answer rides the row read, so
        // refusing a range never costs a `rev-list` of its own.
        read.merges |= parents
            .split(|b| *b == b' ')
            .filter(|p| !p.is_empty())
            .count()
            > 1;
        read.rows.push(PlanRow {
            oid: oid.to_string(),
            author_name: String::from_utf8_lossy(name).into_owned(),
            author_email: String::from_utf8_lossy(email).into_owned(),
            subject: String::from_utf8_lossy(subject).trim_end().to_string(),
        });
    }
    Ok(read)
}
