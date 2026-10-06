//! Laying a walked graph out again with the synthetic rows as the
//! readings have them now — this window's uncommitted row and every
//! other worktree's.
//!
//! Whether the uncommitted row stands is the status's to say and the
//! walk's to draw, and the two are separate processes that land in
//! either order; published off the walk alone, the graph says something
//! about the tree the tree never said.
//!
//! Off the rows alone: the lanes need only each row's id and parents
//! (`LogRow::parents`) and whether it is a stash (`stash_ref`), so
//! laying out again costs a pass over the window and no git
//! (`GraphBuilder::push_ids`).

use super::rows::CarriedRows;
use super::*;

/// What the synthetic rows were laid from, compared so a pass lays out
/// again only when the answer moved under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Standing {
    /// The sides this window's uncommitted row leashes, or `None` where
    /// it does not stand at all (`RepoSession::pending_commit`).
    pub(super) pending: Option<Vec<Oid>>,
    /// The session's own pointer, so comparing two readings short-cuts on
    /// it while the pass is still on the one it started with.
    pub(super) carried: std::sync::Arc<Vec<Carried>>,
}

impl RepoSession {
    /// What the synthetic rows stand on right now.
    ///
    /// A worktree the listing has already moved past draws nothing, and its
    /// reading is asked for again on the spot: the listing learns a worktree
    /// committed long before that worktree's `status` reading does, and
    /// drawn from the older reading the row would stand on a commit the
    /// worktree has left. The pass does not wait for that read — this
    /// window's graph would be paced by another worktree
    /// (デザイン規約 §未コミット行が名乗るもの).
    pub(super) fn standing_rows(self: &Arc<Self>) -> Standing {
        Standing {
            pending: self.pending_commit(),
            carried: self.carried_current(),
        }
    }

    /// Where the worktrees stood when a listing last named them, and which
    /// worktrees there are to read (`carried::OtherWorktrees`, `session::pace`).
    ///
    /// Every listing writes it, the worktree pass's own included
    /// (`carried::pass_over_worktrees`): a record older than a reading would
    /// drop the row of a worktree whose reading is the fresher of the two.
    pub(super) fn note_worktree_heads(
        &self,
        worktrees: &[crate::worktrees::WorktreeEntry],
        workdir: &Path,
    ) {
        let here = super::joins::same_path_key(&workdir.to_string_lossy());
        let heads: std::collections::HashMap<String, Oid> = worktrees
            .iter()
            .filter(|w| !w.bare && super::joins::same_path_key(&w.path) != here)
            .filter_map(|w| {
                let oid = Oid::from_hex_str(w.head_hex.as_deref()?.trim()).ok()?;
                Some((super::joins::same_path_key(&w.path), oid))
            })
            .collect();
        *relock(&self.worktree_heads) = Arc::new(heads);
        let listed = super::carried::listed(worktrees, workdir);
        let keys: Vec<String> = listed.iter().map(|c| c.key.clone()).collect();
        self.other_worktrees.list(listed);
        self.pacing.change(|rules, at| rules.list(&keys, at));
    }

    /// The readings the rows may be drawn from — see [`Self::standing_rows`]
    /// for why one can be left out. The walk asks this too, so a pass and
    /// the laying that follows it draw the same worktrees.
    pub(super) fn carried_current(self: &Arc<Self>) -> Arc<Vec<Carried>> {
        let listed = Arc::clone(&relock(&self.worktree_heads));
        let readings = self.carried();
        // Before the first listing nothing can be behind.
        if listed.is_empty() {
            readings
        } else {
            let current = still_where_the_listing_says(&readings, &listed);
            if current.len() != readings.len() {
                self.read_worktrees_behind(&readings, &current);
            }
            Arc::new(current)
        }
    }

    /// Asks again for the readings the listing has moved past: due at once
    /// where the page paces its worktrees, else a pass of their own — a
    /// second ask while one runs is dropped, so asking costs nothing
    /// (`RepoSession::refresh_carried`).
    fn read_worktrees_behind(self: &Arc<Self>, readings: &[Carried], current: &[Carried]) {
        let behind: Vec<String> = readings
            .iter()
            .filter(|r| !current.iter().any(|c| c.path == r.path))
            .map(|r| super::joins::same_path_key(&r.path))
            .collect();
        let paced = self.pacing.change(|rules, at| {
            if rules.active() {
                for key in &behind {
                    rules.worktree_stale(key, at);
                }
            }
            rules.active()
        });
        if !paced {
            drop(self.refresh_carried());
        }
    }

    /// What is published is laid from the answer standing now (the
    /// module doc says why), which comes back for a laying after this one
    /// (`session::leaving`).
    pub(super) fn lay_again_if_moved(
        self: &Arc<Self>,
        rows: &mut Vec<LogRow>,
        builder: &mut GraphBuilder,
        laid_from: &Standing,
    ) -> Standing {
        let standing = self.standing_rows();
        if standing != *laid_from {
            let (relaid, laid) = self.relay_rows(std::mem::take(rows), &standing);
            *rows = relaid;
            *builder = laid;
        }
        standing
    }

    /// The rows again, laid from `standing`: commit rows keep everything
    /// but their lanes, synthetic rows are made fresh from the readings.
    ///
    /// The builder comes back too — the chips' next diff and the
    /// truncation footer read its end state, so it must be the one that
    /// laid these rows.
    pub(super) fn relay_rows(
        &self,
        rows: Vec<LogRow>,
        standing: &Standing,
    ) -> (Vec<LogRow>, GraphBuilder) {
        // HEAD unknown and HEAD naming no commit both stand the row alone.
        lay(rows, standing, self.known_head_tip().flatten())
    }
}

/// [`RepoSession::relay_rows`] with the repository's answers handed in,
/// so the laying can be tested without one.
pub(super) fn lay(
    rows: Vec<LogRow>,
    standing: &Standing,
    head_tip: Option<Oid>,
) -> (Vec<LogRow>, GraphBuilder) {
    lay_without(rows, standing, head_tip, &std::collections::HashSet::new())
}

/// [`lay`], leaving out the commits in `gone` — what a delete that is out
/// takes off the graph (`session::leaving`).
pub(super) fn lay_without(
    rows: Vec<LogRow>,
    standing: &Standing,
    head_tip: Option<Oid>,
    gone: &std::collections::HashSet<Oid>,
) -> (Vec<LogRow>, GraphBuilder) {
    let mut laying = Laying::new(standing, head_tip);
    let mut out: Vec<LogRow> = Vec::with_capacity(rows.len() + 1);
    out.extend(laying.top());
    for row in rows {
        if is_synthetic(&row) {
            continue;
        }
        let Ok(oid) = Oid::from_hex_str(&row.oid_hex) else {
            // Kept un-laned: it anchors nothing, and losing the row would
            // be worse.
            out.push(row);
            continue;
        };
        if gone.contains(&oid) {
            continue;
        }
        let (made, g) = laying.commit(
            &oid,
            &row.parents,
            !row.stash_ref.is_empty(),
            row.provisional,
        );
        out.extend(made);
        out.push(LogRow {
            row: g.row,
            node_lane: g.node_lane,
            node_color: g.node_color,
            width: g.width,
            segments: g.segments,
            ..row
        });
    }
    (out, laying.builder)
}

/// One laying-out in progress: the builder, and the synthetic rows the
/// readings still owe it. Stepped through by [`lay_without`] over a pass's
/// rows and by the stand-in over the walk's record
/// (`session::leaving::lay_walked`), so the two cannot lay a graph two
/// ways.
pub(super) struct Laying {
    pub(super) builder: GraphBuilder,
    carried: CarriedRows,
    top: Option<LogRow>,
}

impl Laying {
    pub(super) fn new(standing: &Standing, head_tip: Option<Oid>) -> Self {
        let mut builder = GraphBuilder::new();
        // First, where the walk puts it, so HEAD's chain keeps lane 0
        // either way (`session::walk`).
        let top = match (&standing.pending, head_tip) {
            (Some(incoming), Some(head)) => Some(super::rows::wip_row(
                &head,
                incoming.as_slice(),
                &mut builder,
            )),
            // Unborn: the row stands where the first commit will.
            (Some(_), None) => Some(super::rows::wip_root_row(&mut builder)),
            (None, _) => None,
        };
        Self {
            builder,
            carried: CarriedRows::new(&standing.carried),
            top,
        }
    }

    /// This window's uncommitted row, where it stands: first, and once.
    pub(super) fn top(&mut self) -> Option<LogRow> {
        self.top.take()
    }

    /// Lays one commit: the worktrees' rows owed above it, and its lanes —
    /// dashed for a stash and for a provisional commit, as the walk drew
    /// them (`rows::StreamItem::row`).
    pub(super) fn commit(
        &mut self,
        oid: &Oid,
        parents: &[Oid],
        stash: bool,
        provisional: bool,
    ) -> (Vec<LogRow>, GraphRow) {
        // A worktree standing here draws above the commit, and a stash asks
        // with the commit it was taken on (`CarriedRows`).
        let anchor = if stash {
            parents.first().copied()
        } else {
            Some(*oid)
        };
        let made = match anchor {
            Some(anchor) => self.carried.take_at(&anchor, &mut self.builder),
            None => Vec::new(),
        };
        (
            made,
            self.builder.push_ids(oid, parents, stash || provisional),
        )
    }
}

/// Every worktree's uncommitted row carries the all-zero id, which no
/// commit can.
fn is_synthetic(row: &LogRow) -> bool {
    crate::oid::Oid::hex_is_zero(&row.oid_hex)
}

/// The readings still describing where the listing says each worktree is.
///
/// A worktree the listing does not name is kept: one taken since the last
/// listing has a reading and no entry yet.
pub(super) fn still_where_the_listing_says(
    readings: &[Carried],
    listed: &std::collections::HashMap<String, Oid>,
) -> Vec<Carried> {
    readings
        .iter()
        .filter(|c| {
            listed
                .get(&super::joins::same_path_key(&c.path))
                .is_none_or(|head| *head == c.head)
        })
        .cloned()
        .collect()
}
