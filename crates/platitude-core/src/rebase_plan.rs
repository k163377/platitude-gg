//! The range the interactive-rebase screen composes over: the commits of
//! `from^..HEAD`, oldest first, with enough about each to draw a row.
//! Nothing here runs a rebase — the plan replays through
//! [`crate::sequencer`]. [`read_rows`] is the one parser this preview and
//! the sequencer's plans share, so the two cannot read one repository
//! differently.

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

/// The rows oldest-first, and whether a merge sits among them — off the
/// same `%P` field, so the refusal costs no second process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RangeRead {
    pub rows: Vec<PlanRow>,
    pub merges: bool,
}

/// Rows are oldest first (git's todo order; the screen turns them round).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPreview {
    /// The commit the plan was asked from — the oldest one included.
    pub from: String,
    /// Its parent, which the plan replays onto; empty when `root`.
    pub upstream: String,
    /// The range reaches the very first commit, so a rebase needs
    /// `--root` and there is no upstream to name.
    pub root: bool,
    /// The range the plan replays, spelled once — the rewrite warning asks
    /// about this string, so it cannot drift from what the rebase touches.
    pub range: String,
    /// How many commits of `range` a remote already has, as of this read
    /// (the run button's `already pushed`). Refs moving under an open plan
    /// ask again ([`crate::session::RepoSession::check_plan_published`]).
    pub published: u32,
    pub rows: Vec<PlanRow>,
    /// The commit the rows land on, for the screen's own `onto` row.
    /// `None` when `root`.
    pub onto: Option<PlanRow>,
    /// A local branch standing exactly on `upstream`, the name the screen
    /// prefers to the id. Empty when none stands there, or on `root`.
    pub onto_ref: String,
}

/// Why a preview was not made, decided before any rebase spawns. The
/// wording is the screen's (rules-refs/app-ui.md「Rust に文言を置かない」).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRefusal {
    /// The range holds a merge, which a plain interactive rebase silently
    /// flattens (`--rebase-merges` is not offered; デザイン規約
    /// §履歴を合流させる).
    AcrossMerge,
    /// The commit is not in the current branch's history: a rebase only
    /// ever rewrites the branch the tree is standing on.
    OffBranch,
    /// The commit below the range was never fetched — a shallow clone's
    /// edge, which git answers like the real first commit (`%P` empty,
    /// `<edge>~1` exits 1). Replayed as `--root`, it would cut the branch
    /// off from the history this clone does not hold.
    UnfetchedBase,
}

/// A preview's two outcomes besides a failed read. Boxed for clippy
/// `large_enum_variant`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanAnswer {
    Plan(Box<PlanPreview>),
    Refused(PlanRefusal),
}

/// Reads what a plan from `from` (a full commit id) would be made of.
///
/// Serial spawns are what the interaction budget goes on (CLAUDE.md
/// §性能予算): the merge answer rides the range read's `%P`, and the onto
/// row, its branch name and the published count run side by side.
pub async fn preview(
    executor: &GitExecutor,
    workdir: &Path,
    from: &str,
    cancel: &CancellationToken,
) -> Result<PlanAnswer, GitError> {
    // The base first: refusing an unfetched base spares the walk over
    // everything the clone does hold.
    let (upstream, root) = match sequencer::base_of(executor, workdir, from, cancel).await? {
        sequencer::Base::Commit(oid) => (oid, false),
        sequencer::Base::Root => (String::new(), true),
        sequencer::Base::Unfetched => {
            return Ok(PlanAnswer::Refused(PlanRefusal::UnfetchedBase));
        }
    };
    let range = sequencer::range_arg(&upstream, root);

    let read = read_rows(executor, workdir, &range, cancel).await?;
    // The merge first, as in `sequencer::plan_edit`: both refusals can fit
    // a mis-click into another branch, and both entry points should give
    // it the same one.
    if read.merges {
        return Ok(PlanAnswer::Refused(PlanRefusal::AcrossMerge));
    }
    // A linear `from^..HEAD` starts at `from` exactly when `from` is an
    // ancestor of HEAD.
    if read.rows.first().is_none_or(|row| row.oid != from) {
        return Ok(PlanAnswer::Refused(PlanRefusal::OffBranch));
    }

    // Read with the rows: a warning arriving after the screen opened would
    // already have been pressed past. A failed count reads as 0 (logged),
    // as for a repository with no remote.
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
        // The name is decoration: a failed `for-each-ref` falls back to the
        // id, as when no branch stands there
        // (デザイン規約 §フル interactive rebase).
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
/// "First" is the name the base's graph row leads with
/// (`session::model::LabelIndex::from_pairs`): with local branches only and
/// HEAD above the base, that is name order.
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

/// The commits of `range`, oldest first, and whether any is a merge. None
/// of the five fields can carry a NUL, so the flat split cannot tear a
/// record.
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
