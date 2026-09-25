//! Laying a walked graph out again with the synthetic rows as the
//! readings have them now — this window's uncommitted row and every
//! other working copy's.
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
    /// A copy the listing has already moved past draws nothing, and its
    /// reading is asked for again on the spot: the listing learns a copy
    /// committed long before that copy's `status` reading does, and drawn
    /// from the older reading the row would stand on a commit the copy
    /// has left. The pass does not wait for that read — this window's
    /// graph would be paced by another working tree
    /// (デザイン規約 §未コミット行が名乗るもの).
    pub(super) fn standing_rows(self: &Arc<Self>) -> Standing {
        Standing {
            pending: self.pending_commit(),
            carried: self.carried_current(),
        }
    }

    /// Where the copies stood when a listing last named them.
    ///
    /// Every listing writes it, the copies pass's own included
    /// (`carried::pass_over_copies`): a record older than a reading would
    /// drop the row of a copy whose reading is the fresher of the two.
    pub(super) fn note_copy_heads(
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
        *relock(&self.copy_heads) = Arc::new(heads);
    }

    /// The readings the rows may be drawn from — see [`Self::standing_rows`]
    /// for why one can be left out. The walk asks this too, so a pass and
    /// the laying that follows it draw the same copies.
    pub(super) fn carried_current(self: &Arc<Self>) -> Arc<Vec<Carried>> {
        let listed = Arc::clone(&relock(&self.copy_heads));
        let readings = self.carried();
        // Before the first listing nothing can be behind.
        if listed.is_empty() {
            readings
        } else {
            let current = still_where_the_listing_says(&readings, &listed);
            if current.len() != readings.len() {
                // A second ask while a pass runs is dropped, so this costs
                // nothing (`RepoSession::refresh_carried`).
                drop(self.refresh_carried());
            }
            Arc::new(current)
        }
    }

    /// What is published is laid from the answer standing now (the
    /// module doc says why).
    pub(super) fn lay_again_if_moved(
        self: &Arc<Self>,
        rows: &mut Vec<LogRow>,
        builder: &mut GraphBuilder,
        laid_from: &Standing,
    ) {
        let standing = self.standing_rows();
        if standing == *laid_from {
            return;
        }
        let (relaid, laid) = self.relay_rows(std::mem::take(rows), &standing);
        *rows = relaid;
        *builder = laid;
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
    let mut builder = GraphBuilder::new();
    let mut out: Vec<LogRow> = Vec::with_capacity(rows.len() + 1);
    // First, where the walk puts it, so HEAD's chain keeps lane 0 either
    // way (`session::walk`).
    match (&standing.pending, head_tip) {
        (Some(incoming), Some(head)) => {
            out.push(super::rows::wip_row(
                &head,
                incoming.as_slice(),
                &mut builder,
            ));
        }
        // Unborn: the row stands where the first commit will.
        (Some(_), None) => out.push(super::rows::wip_root_row(&mut builder)),
        (None, _) => {}
    }

    let mut carried = CarriedRows::new(&standing.carried);
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
        // A copy standing here draws above the commit, and a stash asks
        // with the commit it was taken on (`CarriedRows`).
        let dashed = !row.stash_ref.is_empty();
        let anchor = if dashed {
            row.parents.first().copied()
        } else {
            Some(oid)
        };
        if let Some(anchor) = anchor {
            out.extend(carried.take_at(&anchor, &mut builder));
        }
        let g = builder.push_ids(&oid, &row.parents, dashed);
        out.push(LogRow {
            row: g.row,
            node_lane: g.node_lane,
            node_color: g.node_color,
            width: g.width,
            segments: g.segments,
            ..row
        });
    }
    (out, builder)
}

/// Every working copy's uncommitted row carries the all-zero id, which no
/// commit can.
fn is_synthetic(row: &LogRow) -> bool {
    crate::oid::Oid::hex_is_zero(&row.oid_hex)
}

/// The readings still describing where the listing says each copy is.
///
/// A copy the listing does not name is kept: one taken since the last
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
