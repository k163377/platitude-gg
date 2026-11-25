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
//! (`GraphBuilder::push_ids`). A synthetic row coming or going moves no
//! lane or colour of the history below (`graph::tests::leash` pins it by
//! equality), so laying out again moves only the synthetic rows and the
//! rows their leash crosses. Not dropped at paint time instead: the rows
//! below own that column's segments, and edges would be left leading
//! nowhere.
//!
//! The same laying, off the walk's record, puts what moved between passes on
//! screen (`RepoSession::relay_screen`): another worktree's reading going
//! with the listing, or landing on a commit the history on screen holds.
//! Neither moved the history, so neither is walked for; a reading landing on
//! a commit the record does not hold is
//! (`RepoSession::history_holds_the_readings`).

use std::collections::HashSet;

use super::leaving::footer_without;
use super::rows::CarriedRows;
use super::*;

/// The commit rows of the graph on screen as the walk gave them, before
/// any delete took some away: what the stand-in lays out again.
///
/// Beside the prints (`Shared::sent_rows`) and nearly as lean: an id, the
/// parents, and a hash of everything else the row says
/// ([`RowPrint::content_of`]). The synthetic rows are left out — they are
/// the readings', made fresh at every laying.
#[derive(Debug, Default)]
pub(super) struct Walked {
    rows: Vec<WalkedRow>,
    /// Every row's parents in one run, as the app keeps them (`marks.rs`):
    /// two allocations for the window rather than one per row.
    parents: Vec<Oid>,
    /// What the walk reported under them; `None` while a stream is still
    /// arriving.
    pub(super) footer: Option<Footer>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct WalkedRow {
    pub(super) oid: Oid,
    pub(super) stash: bool,
    /// Drawn for the discard log's picked entry ([`LogRow::provisional`]).
    pub(super) provisional: bool,
    content: u64,
    /// One past this row's last parent in [`Walked::parents`].
    parents_end: u32,
}

impl Walked {
    pub(super) fn of(rows: &[LogRow]) -> Self {
        let mut walked = Self::default();
        walked.extend(rows);
        walked
    }

    pub(super) fn clear(&mut self) {
        self.rows.clear();
        self.parents.clear();
        self.footer = None;
    }

    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the walk gave `oid` a row of its own.
    pub(super) fn holds(&self, oid: &Oid) -> bool {
        self.rows.iter().any(|row| row.oid == *oid)
    }

    /// Writes down a pass's commit rows, in the order they were walked.
    pub(super) fn extend(&mut self, rows: &[LogRow]) {
        for row in rows {
            let Some(commit) = row.commit() else {
                continue;
            };
            self.parents.extend(commit.parents.iter().copied());
            self.rows.push(WalkedRow {
                oid: commit.oid,
                stash: !commit.stash_ref.is_empty(),
                provisional: commit.provisional,
                content: RowPrint::content_of(row),
                parents_end: self.parents.len() as u32,
            });
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&WalkedRow, &[Oid])> {
        let mut start = 0usize;
        self.rows.iter().map(move |row| {
            let end = row.parents_end as usize;
            let parents = self.parents.get(start..end).unwrap_or_default();
            start = end;
            (row, parents)
        })
    }
}

impl crate::mem::Footprint for Walked {
    fn heap_bytes(&self) -> usize {
        self.rows.capacity() * size_of::<WalkedRow>() + self.parents.capacity() * size_of::<Oid>()
    }
}

/// A graph laid out off the walk's record: what goes to the consumer, and
/// the record of it the session keeps.
#[derive(Default)]
pub(super) struct Laid {
    pub(super) rows: Vec<RelaidRow>,
    pub(super) prints: Vec<RowPrint>,
    pub(super) applied: HashMap<u32, Vec<RefLabel>>,
    pub(super) builder: GraphBuilder,
}

impl Laid {
    /// A synthetic row, sent whole.
    fn made(&mut self, row: LogRow) {
        self.prints.push(RowPrint::of(&row));
        self.rows.push(RelaidRow::Made(Box::new(row)));
    }
}

/// The walk's record laid out again without `gone`, as
/// `relay::lay_without` lays a pass's own rows, each commit wearing the
/// chips the label map gives it.
pub(super) fn lay_walked(
    walked: &Walked,
    standing: &Standing,
    head_tip: Option<Oid>,
    gone: &HashSet<Oid>,
    labels: &LabelIndex,
    tags: bool,
) -> Laid {
    let mut laying = Laying::new(standing, head_tip);
    let mut laid = Laid::default();
    if let Some(top) = laying.top() {
        laid.made(top);
    }
    for (row, parents) in walked.iter() {
        if gone.contains(&row.oid) {
            continue;
        }
        let (worktree_rows, lanes) = laying.commit(&row.oid, parents, row.stash, row.provisional);
        for worktree_row in worktree_rows {
            laid.made(worktree_row);
        }
        let chips = labels.labels_of(&row.oid, tags).to_vec();
        laid.prints
            .push(RowPrint::laid(row.content, &lanes, &chips));
        if !chips.is_empty() {
            laid.applied.insert(lanes.row, chips.clone());
        }
        laid.rows.push(RelaidRow::Moved {
            oid: row.oid,
            lanes,
            labels: chips,
        });
    }
    laid.builder = laying.builder;
    laid
}

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
    /// window's graph would be paced by another worktree.
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
    ///
    /// A listing that no longer names a worktree — removed, pruned or moved
    /// (its row is keyed by where it is) — takes that worktree's reading
    /// with it, and the graph on screen is laid again at once
    /// ([`Self::relay_screen`]): no history moved, so nothing is walked, and
    /// no read of that worktree is coming to take its row down.
    pub(super) fn note_worktree_heads(
        self: &Arc<Self>,
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
        let reading_went = self.other_worktrees.list(listed);
        self.pacing.change(|rules, at| rules.list(&keys, at));
        if reading_went {
            self.relay_screen();
        }
    }

    /// Whether the history on screen holds the commit every reading drawn
    /// stands on — all a laying again needs ([`Self::relay_screen`]). One it
    /// does not hold is a worktree that has committed, seen by a listing
    /// before this window's refs were read, or one standing below the
    /// window: only a walk draws those.
    pub(super) fn history_holds_the_readings(self: &Arc<Self>) -> bool {
        let readings = self.carried_current();
        let shared = self.lock_shared();
        readings
            .iter()
            .all(|reading| shared.walked.holds(&reading.head))
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
    /// where the page paces its worktrees, else a pass of their own — one
    /// asked while a read of them runs is owed to it and runs after it
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

    /// Lays the graph on screen out again for what stands now, and sends it
    /// where it moved ([`SessionEvent::LogRelaid`]): the deletes out —
    /// taking away what only they hold, or putting back what a refused one
    /// took — and the synthetic rows as the readings have them
    /// ([`RepoSession::standing_rows`]). Asked for wherever either moves
    /// with no history to walk: a delete accepted or refused, another
    /// worktree's reading landed or taken off with the listing
    /// (`session::relay`), and a stream that ended with either moved under
    /// it.
    ///
    /// Numbered under the graph's lock like every pass
    /// (`run_direct_pass` / `run_swap_pass`), so a consumer never sees
    /// generations out of the order they were installed in.
    pub(super) fn relay_screen(self: &Arc<Self>) {
        let standing = self.standing_rows();
        let holders = self.holders();
        let Some(mut guard) = self.store_shared() else {
            return;
        };
        let shared = &mut *guard;
        let gone = self.doomed_under_lock(&shared.walked, &shared.label_map, &holders);
        // A stream still arriving is laid when it ends (`run_direct_pass`).
        let Some(footer) = shared.walked.footer.map(|f| footer_without(f, &gone)) else {
            return;
        };
        let laid = lay_walked(
            &shared.walked,
            &standing,
            holders.head_tip,
            &gone,
            &shared.label_map,
            holders.tags,
        );
        // Both, as a pass compares them (`run_swap_pass`): the walk behind
        // a landed delete then finds this picture its own.
        if laid.prints == shared.sent_rows && shared.sent_footer == Some(footer) {
            return;
        }
        let from = shared.generation;
        let generation = self.log_gen.fetch_add(1, Ordering::SeqCst) + 1;
        shared.builder = laid.builder;
        shared.applied = laid.applied;
        shared.sent_rows = laid.prints;
        shared.sent_footer = Some(footer);
        shared.generation = generation;
        self.sink.event(SessionEvent::LogRelaid {
            generation,
            from,
            rows: laid.rows,
            walked: footer.walked,
            truncated: footer.truncated,
        });
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
        // Uncommitted rows are made fresh from the readings, above.
        let Some(commit) = row.commit() else {
            continue;
        };
        if gone.contains(&commit.oid) {
            continue;
        }
        let (made, g) = laying.commit(
            &commit.oid,
            &commit.parents,
            !commit.stash_ref.is_empty(),
            commit.provisional,
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
/// ([`lay_walked`]), so the two cannot lay a graph two
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
