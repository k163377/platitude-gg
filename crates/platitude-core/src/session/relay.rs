//! Laying a walked graph out again with the synthetic rows as the
//! readings have them now — this window's uncommitted row and every
//! other working copy's.
//!
//! **Off the rows alone.** What the lanes are made of is a commit's id
//! and its parents', and a row that has been drawn holds both
//! (`LogRow::parents`); a stash row says so itself (`stash_ref`), which
//! is the only other thing the builder asks. So a pass that walked with
//! one answer about the synthetic rows can be laid out again with
//! another, for the price of a pass over the window
//! (`GraphBuilder::push_ids`).
//!
//! **Why a pass's answer goes stale at all.** Whether the uncommitted
//! row stands is the status's to say and the walk's to draw, and the two
//! are read by different processes: the opening starts the walk before
//! the first status has answered, and either can land first. Read off
//! the walk alone, the graph then says something about the working tree
//! that the tree never said — for about half of every opening, until the
//! status asks for another walk and the row arrives a pass later
//! (`PageSettled`). Laying out again closes that: what is published is
//! always laid from the answer standing at the moment it is published.

use super::rows::CarriedRows;
use super::*;

/// What the synthetic rows were laid from, kept so a pass can tell
/// whether the answer moved under it.
///
/// **Compared.** A reading that came back the same is no reason to lay
/// anything out again, and the readings are a handful of ids either way
/// — where the uncommitted row stands and what each other copy is
/// carrying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Standing {
    /// The sides this window's uncommitted row leashes, or `None` where
    /// it does not stand at all (`RepoSession::pending_commit`).
    pub(super) pending: Option<Vec<Oid>>,
    /// Held by the pointer the session hands out: the rows are read from
    /// it and never written through it, and comparing two readings is a
    /// pointer comparison wherever the pass is still on the one it
    /// started with.
    pub(super) carried: std::sync::Arc<Vec<Carried>>,
}

impl RepoSession {
    /// What the synthetic rows stand on right now.
    ///
    /// **A copy the listing has already moved past draws nothing**, and
    /// its reading is asked for again on the spot. The listing names
    /// every copy's HEAD for twenty-odd milliseconds on the page's tick
    /// and the readings cost a `status` each on a tick of their own, so
    /// this window learns that a neighbour has committed long before it
    /// learns what the neighbour is now carrying. Drawn from the older
    /// reading, the row stands on a commit that copy has left, with
    /// tallies half a minute old; left out, it is missing for as long as
    /// one `status` takes and comes back where it belongs.
    ///
    /// **The pass goes on.** Holding for that read would keep the row
    /// from ever being missing, but this window's own graph would then
    /// be paced by somebody else's working tree, and the press that is
    /// waiting for it is ours (デザイン規約 §未コミット行が名乗るもの).
    pub(super) fn standing_rows(self: &Arc<Self>) -> Standing {
        Standing {
            pending: self.pending_commit(),
            carried: self.carried_current(),
        }
    }

    /// Where the copies stood when a listing last named them, off any
    /// listing this session makes.
    ///
    /// **Every listing writes it.** The pass over
    /// the copies takes a listing of its own before it reads them
    /// (`carried::pass_over_copies`), so it can learn that a copy has
    /// committed before the page's tick does — and a reading newer than
    /// this record would then be taken for one left behind, and the row
    /// dropped for a copy whose reading is in fact the fresher of the
    /// two. Written from both, this is the newest listing anyone made,
    /// which is the only reading of it that can order the pair.
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
    /// for why one can be left out. **The walk asks this too**, so a pass
    /// and the laying that follows it draw the same copies.
    pub(super) fn carried_current(self: &Arc<Self>) -> Arc<Vec<Carried>> {
        let listed = Arc::clone(&relock(&self.copy_heads));
        let readings = self.carried();
        // Before the first listing there is nothing to be behind, and
        // the rows are the readings' own — an empty listing is its own
        // case.
        if listed.is_empty() {
            readings
        } else {
            let current = still_where_the_listing_says(&readings, &listed);
            if current.len() != readings.len() {
                // Asked for here: the pass over the copies drops a tick
                // it finds one already running, so a second ask inside
                // one costs nothing
                // (`RepoSession::refresh_carried`).
                drop(self.refresh_carried());
            }
            Arc::new(current)
        }
    }

    /// **What is published is laid from the answer standing
    /// now.**
    ///
    /// The status that says whether the uncommitted row stands, and the
    /// readings that say what each other copy is carrying, are read by
    /// processes of their own and can land at any point during a walk —
    /// so a graph read off its own start says something about a working
    /// tree that no reading ever said. Laying out again costs a pass
    /// over the window and no git at all.
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

    /// The rows again, laid from `standing`.
    ///
    /// The commit rows keep everything but their place in the graph —
    /// their words, their chips, their published mark — because none of
    /// that is what moved; only the lanes are drawn again. The synthetic
    /// rows are made fresh, since what they say *is* the reading.
    ///
    /// The builder comes back with them: it is the end state the chips'
    /// next diff and the truncation footer are read against, so a graph
    /// laid out again has to hand over the one that laid it.
    pub(super) fn relay_rows(
        &self,
        rows: Vec<LogRow>,
        standing: &Standing,
    ) -> (Vec<LogRow>, GraphBuilder) {
        // `None` twice over: nobody has reported HEAD, and a HEAD that
        // was reported as naming no commit. The row stands alone in both
        // — there is nothing for it to reach down to either way.
        lay(rows, standing, self.known_head_tip().flatten())
    }
}

/// [`RepoSession::relay_rows`] with the repository's two answers handed
/// in, which is the whole of what it reads — so the laying can be tested
/// without one.
pub(super) fn lay(
    rows: Vec<LogRow>,
    standing: &Standing,
    head_tip: Option<Oid>,
) -> (Vec<LogRow>, GraphBuilder) {
    let mut builder = GraphBuilder::new();
    let mut out: Vec<LogRow> = Vec::with_capacity(rows.len() + 1);
    // First, before any commit, exactly where the walk puts it: the chain
    // HEAD is on keeps lane 0 either way (`session::walk`).
    match (&standing.pending, head_tip) {
        (Some(incoming), Some(head)) => {
            out.push(super::rows::wip_row(
                &head,
                incoming.as_slice(),
                &mut builder,
            ));
        }
        // A branch with no commits yet: nothing to reach down to, and the
        // row stands where the first commit will.
        (Some(_), None) => out.push(super::rows::wip_root_row(&mut builder)),
        (None, _) => {}
    }

    let mut carried = CarriedRows::new(&standing.carried);
    for row in rows {
        if is_synthetic(&row) {
            // Laid again from the reading below: a row that says what
            // a working copy holds is the reading itself, made fresh
            // each time.
            continue;
        }
        let Ok(oid) = Oid::from_hex_str(&row.oid_hex) else {
            // No id the builder can use. Kept where it is and
            // un-laned, since it anchors nothing and a lost row
            // would be worse.
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

/// Whether this row is one a reading put there: every working copy's
/// uncommitted row carries git's all-zero id — "there is no object
/// here" — which no commit can.
fn is_synthetic(row: &LogRow) -> bool {
    crate::oid::Oid::hex_is_zero(&row.oid_hex)
}

/// The readings still describing where the listing says each copy is.
///
/// **A copy the listing does not name at all is kept.** The two are read
/// by different passes and a copy taken since the last listing has a
/// reading and no entry yet; dropping it would take a row off the screen
/// for a copy that is there.
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
