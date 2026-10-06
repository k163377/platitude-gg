//! What a delete takes off the graph before git has answered for it
//! (デザイン規約 §消す操作は先に画面から消す): the commits only the names
//! leaving hold, gone the moment the delete is accepted and back the
//! moment it is refused.
//!
//! Laid out again off the walk's record of the rows on screen ([`Walked`]),
//! as a pass's own rows are (`relay::Laying`) — no git, no wait. While a
//! delete is out, every pass that lands is laid the same way
//! (`RepoSession::run_swap_pass`), so a walk begun before git answered
//! cannot bring the rows back.

use std::collections::HashSet;

use super::relay::{Laying, Standing};
use super::*;

/// One delete that is out.
#[derive(Debug, Clone)]
pub(super) struct Leaving {
    /// The write taking the names away.
    write: OperationId,
    /// Their full refnames: a commit another name stands on stays.
    refs: Vec<String>,
    /// The commits they stood on when the delete was accepted — kept,
    /// since the refs read behind a delete that landed no longer names
    /// them.
    tips: Vec<Oid>,
    /// Landed, and the walk behind it did not: git has let the names go,
    /// but the record (`Shared::walked`) still holds what only they held.
    /// Stands until a walk lands ([`RepoSession::leaving_walked`]), or the
    /// next laying-out would draw those commits back.
    unwalked: bool,
}

/// A name a delete takes away, as the menu offering it has it.
#[derive(Debug, Clone, Copy)]
pub(super) enum LeavingRef<'a> {
    Branch(&'a str),
    /// A remote-tracking branch, by its two halves.
    Remote(&'a str, &'a str),
    Tag(&'a str),
}

/// The names of one delete, resolved against the snapshot on screen and
/// waiting for the id the queue accepts the write under.
pub(super) struct Pending {
    refs: Vec<String>,
    tips: Vec<Oid>,
}

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
struct WalkedRow {
    oid: Oid,
    stash: bool,
    /// Drawn for the discard log's picked entry ([`LogRow::provisional`]).
    provisional: bool,
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

    /// Writes down a pass's commit rows, in the order they were walked.
    pub(super) fn extend(&mut self, rows: &[LogRow]) {
        for row in rows {
            if Oid::hex_is_zero(&row.oid_hex) {
                continue;
            }
            let Ok(oid) = Oid::from_hex_str(&row.oid_hex) else {
                continue;
            };
            self.parents.extend(row.parents.iter().copied());
            self.rows.push(WalkedRow {
                oid,
                stash: !row.stash_ref.is_empty(),
                provisional: row.provisional,
                content: RowPrint::content_of(row),
                parents_end: self.parents.len() as u32,
            });
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&WalkedRow, &[Oid])> {
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

/// The rows nothing but `tips` holds, asked children first as the walk
/// gave them, so every child has had its say by the time its parent is
/// asked (`--date-order` never puts a parent above a child).
///
/// A row goes only where every child that reaches it went, and nothing
/// stands on the row itself (`held`: another name, HEAD, a stash, a
/// worktree).
/// A row the tips do not reach stays, and so does all it reaches: what no
/// name is known to hold is kept, not guessed away.
pub(super) fn doomed(
    walked: &Walked,
    tips: &[Oid],
    held: impl Fn(&Oid, bool) -> bool,
) -> HashSet<Oid> {
    let mut reached: HashSet<Oid> = tips.iter().copied().collect();
    let mut kept: HashSet<Oid> = HashSet::new();
    let mut gone = HashSet::new();
    for (row, parents) in walked.iter() {
        // Everything the tips reached has been asked.
        if reached.is_empty() {
            break;
        }
        let goes =
            reached.remove(&row.oid) && !kept.contains(&row.oid) && !held(&row.oid, row.stash);
        if goes {
            gone.insert(row.oid);
            reached.extend(parents.iter().copied());
        } else {
            kept.extend(parents.iter().copied());
        }
    }
    gone
}

/// The footer a walk without `gone` would report: fewer commits walked
/// where the window held them all. A full window stays full — the walk
/// behind the delete fills it from below, which only it can.
pub(super) fn footer_without(footer: Footer, gone: &HashSet<Oid>) -> Footer {
    Footer {
        walked: if footer.truncated {
            footer.walked
        } else {
            footer.walked.saturating_sub(gone.len() as u32)
        },
        ..footer
    }
}

/// What says whether something other than the leaving names stands on a
/// commit, read before the graph's lock is taken (each is a lock of its
/// own).
pub(super) struct Holders {
    pub(super) snapshot: Option<Arc<RefsSnapshot>>,
    pub(super) head_tip: Option<Oid>,
    pub(super) incoming: Vec<Oid>,
    pub(super) detached: Vec<Oid>,
    /// The discard log's picked entry's tips (`session::shown_discard`).
    pub(super) shown_tips: Vec<Oid>,
    /// Whether the graph walks tags: hidden, a tag starts no walk.
    pub(super) tags: bool,
}

impl Holders {
    /// Whether anything but `leaving` stands on `oid`: a name the walk
    /// starts from, HEAD, a stash, a merge's other side or a worktree on no
    /// branch — the walk's own starting points (`walk::walk_command`).
    pub(super) fn hold(
        &self,
        oid: &Oid,
        stash: bool,
        labels: &LabelIndex,
        leaving: &HashSet<&str>,
    ) -> bool {
        stash
            || self.head_tip.as_ref() == Some(oid)
            || self.incoming.contains(oid)
            || self.detached.contains(oid)
            || self.shown_tips.contains(oid)
            || labels
                .labels_of(oid, self.tags)
                .iter()
                .any(|label| self.label_holds(label, leaving))
    }

    fn label_holds(&self, label: &RefLabel, leaving: &HashSet<&str>) -> bool {
        let stays = |full: String| !leaving.contains(full.as_str());
        match label.kind {
            LabelKind::Head | LabelKind::Worktree => true,
            // A remote branch on the same commit rides on the local chip
            // (`RemoteBranches::folded_into_local`), and holds it too.
            LabelKind::LocalBranch => {
                stays(format!("refs/heads/{}", label.text))
                    || self.folded_remote(&label.text).is_some_and(stays)
            }
            LabelKind::RemoteBranch => stays(format!("refs/remotes/{}", label.text)),
            // A tag only a remote has is no starting point of the walk.
            LabelKind::Tag => label.here && stays(format!("refs/tags/{}", label.text)),
        }
    }

    /// The remote branch folded into this local branch's chip, by full
    /// name — its upstream standing on the same commit. `None` with no
    /// snapshot to ask: the chip's own name decides then.
    fn folded_remote(&self, local: &str) -> Option<String> {
        let snapshot = self.snapshot.as_ref()?;
        let item = snapshot.local_named(local)?;
        (!item.upstream.is_empty() && !item.upstream_drifted)
            .then(|| format!("refs/remotes/{}", item.upstream))
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

impl RepoSession {
    /// The names a delete takes away, resolved against the snapshot on
    /// screen; `None` where none of them stands on anything the graph
    /// draws — a tag while the graph walks no tags, or a name the snapshot
    /// no longer lists.
    ///
    /// Binary searches only: this runs on the press.
    pub(super) fn resolve_leaving(&self, names: &[LeavingRef<'_>]) -> Option<Pending> {
        let snapshot = self.published_snapshot()?;
        let tags = self.tags_shown();
        let mut pending = Pending {
            refs: Vec::new(),
            tips: Vec::new(),
        };
        for name in names {
            let (full, tip) = match *name {
                LeavingRef::Branch(branch) => (
                    format!("refs/heads/{branch}"),
                    snapshot.local_named(branch).map(|item| item.oid),
                ),
                LeavingRef::Remote(remote, branch) => (
                    format!("refs/remotes/{remote}/{branch}"),
                    snapshot
                        .remote_named(&format!("{remote}/{branch}"))
                        .map(|item| item.oid),
                ),
                LeavingRef::Tag(tag) => (
                    format!("refs/tags/{tag}"),
                    snapshot
                        .tag_named(tag)
                        .filter(|item| tags && item.here)
                        .map(|item| item.oid),
                ),
            };
            if let Some(tip) = tip {
                pending.refs.push(full);
                pending.tips.push(tip);
            }
        }
        (!pending.tips.is_empty()).then_some(pending)
    }

    /// Writes a delete down under the id its write was accepted with —
    /// from inside the acceptance, so no answer can arrive for a delete
    /// not yet written down (`RepoSession::write_taking`).
    pub(super) fn leave(&self, write: OperationId, pending: Pending) {
        relock(&self.leaving).push(Leaving {
            write,
            refs: pending.refs,
            tips: pending.tips,
            unwalked: false,
        });
    }

    /// The delete accepted under `write` was refused: what it took comes
    /// back now, so the refusal is about rows on screen.
    pub(super) fn leaving_refused(self: &Arc<Self>, write: OperationId) {
        if self.leaving_done(write) {
            self.relay_leaving();
        }
    }

    /// The delete under `write` landed and the walk behind it did not: it
    /// stands until one does ([`Leaving::unwalked`]).
    pub(super) fn leaving_unwalked(&self, write: OperationId) {
        for delete in relock(&self.leaving)
            .iter_mut()
            .filter(|delete| delete.write == write)
        {
            delete.unwalked = true;
        }
    }

    /// A walk landed, begun after every delete that landed unwalked: the
    /// record it leaves holds none of their commits.
    pub(super) fn leaving_walked(&self) {
        relock(&self.leaving).retain(|delete| !delete.unwalked);
    }

    /// The write under `write` is over. Where it landed, the walk behind it
    /// has already drawn the graph without the names, so nothing is laid
    /// again here — unless that walk did not land ([`Leaving::unwalked`]).
    /// Says whether a delete was put down.
    pub(super) fn leaving_done(&self, write: OperationId) -> bool {
        let mut leaving = relock(&self.leaving);
        let before = leaving.len();
        leaving.retain(|delete| delete.write != write || delete.unwalked);
        leaving.len() != before
    }

    /// Whether any delete is out.
    pub(super) fn deletes_out(&self) -> bool {
        !relock(&self.leaving).is_empty()
    }

    /// What says who else stands on a commit ([`Holders`]).
    pub(super) fn holders(&self) -> Holders {
        Holders {
            snapshot: self.published_snapshot(),
            head_tip: self.known_head_tip().flatten(),
            incoming: self.standing.merge_incoming(),
            detached: self
                .worktree_holders()
                .detached
                .iter()
                .map(|worktree| worktree.oid)
                .collect(),
            shown_tips: self.shown_tips(),
            tags: self.tags_shown(),
        }
    }

    /// The commits the deletes out now take off `walked`. Under the
    /// graph's lock, which is where the list of deletes is read: one
    /// written down after a pass read it would never be laid. The list's
    /// own lock is let go before the pass over the window — a press takes
    /// it on the UI thread (`write_taking`).
    pub(super) fn doomed_under_lock(
        &self,
        walked: &Walked,
        labels: &LabelIndex,
        holders: &Holders,
    ) -> HashSet<Oid> {
        let (refs, tips): (Vec<String>, Vec<Oid>) = {
            let leaving = relock(&self.leaving);
            if leaving.is_empty() {
                return HashSet::new();
            }
            (
                leaving
                    .iter()
                    .flat_map(|delete| delete.refs.iter().cloned())
                    .collect(),
                leaving
                    .iter()
                    .flat_map(|delete| delete.tips.iter().copied())
                    .collect(),
            )
        };
        let refs: HashSet<&str> = refs.iter().map(String::as_str).collect();
        doomed(walked, &tips, |oid, stash| {
            holders.hold(oid, stash, labels, &refs)
        })
    }

    /// A pass's rows, laid again without what the deletes out now take
    /// away; `record` is them as walked. Under the graph's lock, where
    /// [`Self::doomed_under_lock`] reads the deletes. Answers with what
    /// went, which the footer is told less of ([`footer_without`]).
    pub(super) fn lay_leaving(
        &self,
        rows: &mut Vec<LogRow>,
        builder: &mut GraphBuilder,
        record: &Walked,
        standing: &Standing,
        holders: &Holders,
        labels: &LabelIndex,
    ) -> HashSet<Oid> {
        let gone = self.doomed_under_lock(record, labels, holders);
        if !gone.is_empty() {
            let (laid, laid_by) =
                super::relay::lay_without(std::mem::take(rows), standing, holders.head_tip, &gone);
            *rows = laid;
            *builder = laid_by;
        }
        gone
    }

    /// Lays the graph on screen out again for the deletes out now — taking
    /// away what only they hold, or putting back what a refused one took —
    /// and sends it where it moved ([`SessionEvent::LogRelaid`]).
    ///
    /// Numbered under the graph's lock like every pass
    /// (`run_direct_pass` / `run_swap_pass`), so a consumer never sees
    /// generations out of the order they were installed in.
    pub(super) fn relay_leaving(self: &Arc<Self>) {
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
