//! The statuses the session tells: the one a read brought, and the one a
//! commit leaves — worked out at git's answer from the status told before
//! it and told at once, ahead of the status read behind the write.
//!
//! **A commit shows what it left when git answers** (user decision — ask
//! before changing): the staged rows go, and where nothing is left the
//! worktree row with them, without waiting for a status — the longest read
//! there is (ci/baseline/code-costs-windows-x64.md §git のプロセス代).
//! Whoever committed with changes left over stages the next ones over rows
//! that are already the commit's.
//!
//! **The rule** is [`WorktreeStatus::left_by_commit`]: a commit records
//! the index and writes neither it nor the working tree, so every staged
//! mark goes and the rest stands. It holds wherever `git commit` took the
//! index whole — a plain commit, an amend, and HEAD's message saved, which
//! is an amend that takes what is staged along.
//!
//! **The write says it committed the index**
//! ([`RepoSession::note_index_committed`]), from inside its own task; its
//! kind does not. A reword is that command only where the commit is HEAD,
//! found out under the queue — further down it is a replay, which no row
//! foretells — and the two answer under one kind, as the app reads them.
//!
//! **What is not worked out.** The branch's commit, its counts against the
//! upstream and against where its push goes, and what Git LFS asks of the
//! pending files are the ones told before the commit, until the read behind
//! the write tells git's own. The line-ending marks lose the index side's,
//! and the rest stand until that read: it reads them again whatever the
//! status key says (`RepoSession::status_key`, left where the last read
//! put it), the write having marked them stale — a commit can record a
//! `.gitattributes` (`write::forget_what_it_reached`).
//!
//! **Where it is wrong** the read behind the write puts it right: a hook
//! that touched the index or the tree, a file saved or staged outside
//! since the status it was worked out from. Where that turns the worktree
//! row, the history is walked a second time (`write::settle_after`).
//!
//! **Told under the write's fence.** The status goes out under the two
//! numbers the write's answer carries ([`Fence`]): stamped with the write's
//! own end, it is the first status that looked after the write, and it
//! names the report of HEAD the write is owed — so a consumer waiting on
//! that write, by either number, takes it for the write's own, and the
//! read behind the write, stamped above it, replaces it.
//!
//! **One order for the statuses told** ([`StatusTold`]'s lock): a read
//! tells its status only while no write has ended behind it, asked under
//! the lock the status a write left is told under. A read that looked
//! before a commit ended can therefore not be told after the status that
//! commit left — it would draw the committed rows back until the read
//! behind the write landed.

use super::standing::Fence;
use super::*;

/// One status as the consumer is told it: the fields of
/// [`SessionEvent::StatusLoaded`].
#[derive(Clone)]
pub(super) struct Told {
    pub(super) status: Arc<WorktreeStatus>,
    pub(super) head_seq: u64,
    pub(super) looked: u64,
    pub(super) op_state: OpState,
    pub(super) progress: Option<conflict::Progress>,
    pub(super) sides: conflict::Sides,
    pub(super) op_message: String,
    pub(super) merge_tool: String,
    pub(super) push: remote::StatusPush,
    pub(super) eol_marks: Arc<Vec<EolMark>>,
    pub(super) lfs_needed: usize,
    pub(super) not_copied: Arc<Vec<String>>,
    pub(super) stop: integrate::RebaseStop,
}

impl Told {
    fn event(self) -> SessionEvent {
        let Self {
            status,
            head_seq,
            looked,
            op_state,
            progress,
            sides,
            op_message,
            merge_tool,
            push,
            eol_marks,
            lfs_needed,
            not_copied,
            stop,
        } = self;
        SessionEvent::StatusLoaded {
            status,
            head_seq,
            looked,
            op_state,
            progress,
            sides,
            op_message,
            merge_tool,
            push,
            eol_marks,
            lfs_needed,
            not_copied,
            stop,
        }
    }

    /// The status a commit git has just taken leaves behind this one, told
    /// under that write's `fence` — `None` where none follows from it:
    ///
    /// * a conflict stands in it ([`WorktreeStatus::left_by_commit`]);
    /// * an operation stands in it, any `OpState::any` names — a bisect
    ///   too: the commit that ends a merge, or the one a stopped replay
    ///   goes on from, takes the operation along, which no row says — and
    ///   the worktree row stands on the operation as much as on the rows
    ///   (`graph::wip_row_stands`).
    fn left_by_commit(&self, fence: &Fence) -> Option<Self> {
        if self.op_state.any() {
            return None;
        }
        let status = self.status.left_by_commit()?;
        // A mark on the index side went into the commit with its change.
        let eol_marks = self
            .eol_marks
            .iter()
            .filter(|mark| !mark.staged)
            .cloned()
            .collect();
        Some(Self {
            status: Arc::new(status),
            head_seq: fence.head_seq,
            looked: fence.reads_from,
            eol_marks: Arc::new(eol_marks),
            ..self.clone()
        })
    }
}

/// What the session keeps of the statuses it tells
/// (`RepoSession::status_told`), under the lock every one of them is told
/// under (module doc).
///
/// The statuses go to the sink with the lock held
/// ([`RepoSession::tell_status`], [`RepoSession::tell_what_it_left`]), and so
/// does what a read records beside one (`RepoSession::record_status`): the
/// sink is to return at once. One that waits there — a test's hook or park
/// on `StatusLoaded` — holds every status read and the answer of every
/// write that committed the index behind it.
#[derive(Default)]
pub(super) struct StatusTold {
    /// The status told last, for the next commit's to be worked out from.
    ///
    /// Kept, not read at the commit: the commit's answer is the one moment
    /// no status can be waited for. Its rows are the allocation the event
    /// carries (`SessionEvent::StatusLoaded::status`), so keeping them
    /// copies nothing; they scale with the pending files, not with the
    /// history, and the rest is a few words. Held through a close, as the
    /// line-ending marks of the same files are: the close gives back what
    /// the page drew, and takes no lock a status is told under.
    last: Option<Told>,
    /// The end of the last write that can have moved a status's rows: a
    /// status that looked before it does not say what that write left
    /// staged, and nothing follows from it. This session's own count at
    /// their end (`Fence::reads_from`); one another session landed where
    /// this one reads counts from when this one hears of it
    /// ([`StatusTold::landed_elsewhere`]). Only such writes count — a fetch
    /// that brought nothing reads no tree behind itself, and held against it
    /// the status told last would be behind until the next tick came round.
    rows_from: u64,
    /// The write that said git took a commit of the index under it
    /// ([`RepoSession::note_index_committed`]), until its answer.
    committed: Option<OperationId>,
}

impl StatusTold {
    /// A write ended at `fence`: the status told last is behind it where
    /// it can have moved a status's rows (`moved_rows`), and where it is a
    /// `commit` git took, the status it left is answered, and kept as the
    /// one told last. `None` — nothing is told ahead — where:
    ///
    /// * it is no commit git took, or the session is closed (`commit`,
    ///   [`RepoSession::tell_what_it_left`]);
    /// * no status has been told yet;
    /// * the status told last looked before an earlier write that can have
    ///   moved its rows (`rows_from`);
    /// * none follows from it ([`Told::left_by_commit`]): an operation
    ///   stands, a bisect among them, or a conflict is listed.
    fn answered(&mut self, commit: bool, moved_rows: bool, fence: &Fence) -> Option<Told> {
        // Before this write's own end goes in: its status is the one being
        // worked out.
        let rows_from = self.rows_from;
        if moved_rows {
            self.rows_from = fence.reads_from;
        }
        if !commit {
            return None;
        }
        let left = self
            .last
            .as_ref()
            .filter(|last| last.looked >= rows_from)?
            .left_by_commit(fence)?;
        self.last = Some(left.clone());
        Some(left)
    }

    /// A write another session ran has landed in a tree this session reads,
    /// told to this one at stamp `at` — after the write ended: it fences
    /// none of this session's reads, so the status told last can be from
    /// before it, and is behind from here on.
    fn landed_elsewhere(&mut self, at: u64) {
        self.rows_from = self.rows_from.max(at);
    }

    /// What the kept status weighs, and how many rows it holds
    /// (`RepoSession::heap_report`).
    pub(super) fn weight(&self) -> (usize, usize) {
        use crate::mem::Footprint as _;
        self.last.as_ref().map_or((0, 0), |told| {
            (
                told.status.heap_bytes()
                    + told.op_message.heap_bytes()
                    + told.merge_tool.heap_bytes()
                    + told.push.heap_bytes(),
                told.status.items.len(),
            )
        })
    }
}

impl RepoSession {
    /// git took a commit of the index — `git commit`, amending or not, with
    /// no path named — under the write the queue is serving: said from
    /// inside that write's own task, as its stop is
    /// ([`Self::note_landing`]), once the command has answered. The status
    /// it left is told at the write's answer ([`Self::tell_what_it_left`]).
    pub(super) fn note_index_committed(&self) {
        match self.running_write() {
            Some(write) => relock(&self.status_told).committed = Some(write.id),
            None => tracing::warn!("a commit was noted with no write being served"),
        }
    }

    /// A write another session ran has landed in a tree this session reads
    /// (`RepoSession::read_again`): the status told last is behind it
    /// ([`StatusTold::landed_elsewhere`]), stamped now.
    pub(super) fn rows_moved_elsewhere(&self) {
        let at = self.standing.stamp();
        relock(&self.status_told).landed_elsewhere(at);
    }

    /// Tells the status a read brought, and keeps it — unless a write has
    /// ended behind the read since it last asked (`Standing::current`):
    /// then nothing is told, and `false` says the read is to be made again.
    ///
    /// Asked here, under the lock, and not only where the read came back:
    /// between the two a status pass reads the marks, which can take as
    /// long as the diffs they are read off, and a commit answering
    /// meanwhile has told what it left (module doc).
    pub(super) fn tell_status(&self, told: Told) -> bool {
        let mut kept = relock(&self.status_told);
        if !self.standing.current(told.looked) {
            return false;
        }
        kept.last = Some(told.clone());
        self.sink.event(told.event());
        true
    }

    /// git has answered the write under `id`, which ended at `fence`,
    /// `landed` or refused.
    ///
    /// Where the write can have moved a status's rows (`moved_rows` — the
    /// writes whose reads take the tree whatever else they find), the
    /// status told last is behind it from here on.
    ///
    /// Where it landed having committed the index
    /// ([`Self::note_index_committed`]), the status it left is told now, and
    /// `true` answered: worked out from the status told last
    /// ([`Told::left_by_commit`]) where that one looked after every earlier
    /// write that can have moved its rows — the queue reads the tree behind
    /// each of this session's own before a commit starts, and one another
    /// session landed here asks this one for a read at once
    /// ([`Self::rows_moved_elsewhere`]); so a read that failed, or one still
    /// out when the commit answers, leaves it behind. The worktree row
    /// follows what is told (`Standing::set_wip_dirty`), so the walk behind
    /// the commit lays the graph from it and the status read that finds the
    /// same asks for no second one; the amend in view is asked about the
    /// index and HEAD as told ([`Self::authorship_status`]).
    ///
    /// Nothing is told by a closed session: its page is gone and its sink
    /// retired, as for the reads behind its writes (`write::settle`).
    pub(super) fn tell_what_it_left(
        self: &Arc<Self>,
        id: OperationId,
        landed: bool,
        moved_rows: bool,
        fence: &Fence,
    ) -> bool {
        let open = !self.root_cancel.is_cancelled();
        let left = {
            let mut kept = relock(&self.status_told);
            // Taken whatever the answer: the note is this write's alone.
            let committed = kept.committed.take() == Some(id);
            let Some(left) = kept.answered(landed && committed && open, moved_rows, fence) else {
                return false;
            };
            self.standing
                .set_wip_dirty(crate::graph::wip_row_stands(&left.status, &left.op_state));
            let status = Arc::clone(&left.status);
            self.sink.event(left.event());
            status
        };
        self.authorship_status(&left);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::StatusItem;

    fn row(staged: char, unstaged: char, path: &str) -> StatusItem {
        StatusItem::Tracked {
            staged,
            unstaged,
            path: path.to_string(),
            orig_path: None,
        }
    }

    fn mark(path: &str, staged: bool) -> EolMark {
        EolMark {
            path: path.to_string(),
            notice: crate::eol::Notice::Flipped {
                from: crate::eol::Eol::Crlf,
                to: crate::eol::Eol::Lf,
            },
            staged,
        }
    }

    /// A status told at stamp 3 beside report 2: one row staged alone, one
    /// on both sides, and a line-ending mark on each side of the second.
    fn told() -> Told {
        Told {
            status: Arc::new(WorktreeStatus {
                upstream_counts: Some(crate::AheadBehind {
                    ahead: 1,
                    behind: 0,
                }),
                items: vec![row('M', '.', "staged.txt"), row('M', 'M', "both.txt")],
                ..WorktreeStatus::default()
            }),
            head_seq: 2,
            looked: 3,
            op_state: OpState::default(),
            progress: None,
            sides: conflict::Sides::default(),
            op_message: String::new(),
            merge_tool: "meld".to_string(),
            push: remote::StatusPush {
                push_remote: Some("fork".to_string()),
                push_default: None,
                upstream_here: true,
                track: None,
            },
            eol_marks: Arc::new(vec![mark("both.txt", true), mark("both.txt", false)]),
            lfs_needed: 4,
            not_copied: Arc::new(vec!["big.bin".to_string()]),
            stop: integrate::RebaseStop::default(),
        }
    }

    const FENCE: Fence = Fence {
        head_seq: 9,
        reads_from: 12,
    };

    /// The status a commit leaves goes out under the write's two numbers,
    /// with the rows and the marks the commit took gone and everything no
    /// row decides as it was told before.
    #[test]
    fn the_status_a_commit_leaves_is_told_under_the_writes_fence() {
        let before = told();
        let left = before
            .left_by_commit(&FENCE)
            .expect("a status follows the commit");
        assert_eq!(
            (left.head_seq, left.looked),
            (FENCE.head_seq, FENCE.reads_from)
        );
        assert_eq!(left.status.items, [row('.', 'M', "both.txt")]);
        assert_eq!(*left.eol_marks, [mark("both.txt", false)]);
        assert_eq!(
            left.status.upstream_counts.map(|counts| counts.ahead),
            Some(1),
            "the counts are not worked out"
        );
        assert_eq!(
            (
                left.merge_tool.as_str(),
                left.push.push_remote.as_deref(),
                left.push.upstream_here,
                left.lfs_needed,
                left.not_copied.as_slice()
            ),
            (
                "meld",
                Some("fork"),
                true,
                4,
                ["big.bin".to_string()].as_slice()
            )
        );
    }

    /// The commit that ends a merge takes the merge along, which no row
    /// says: nothing follows from a status an operation stands in.
    #[test]
    fn nothing_follows_from_a_status_an_operation_stands_in() {
        let merging = Told {
            op_state: OpState {
                merging: true,
                ..OpState::default()
            },
            ..told()
        };
        assert!(merging.left_by_commit(&FENCE).is_none());
    }

    fn fence(reads_from: u64) -> Fence {
        Fence {
            head_seq: 9,
            reads_from,
        }
    }

    /// The status told at stamp 3 kept ([`told`]).
    fn keeping() -> StatusTold {
        StatusTold {
            last: Some(told()),
            ..StatusTold::default()
        }
    }

    /// A commit's status is worked out from the one told last, and kept in
    /// its place: the next commit's follows from it.
    #[test]
    fn a_commit_is_told_from_the_status_told_last_and_kept_in_its_place() {
        let mut kept = keeping();
        let left = kept
            .answered(true, true, &fence(5))
            .expect("a status follows the commit");
        assert_eq!(left.looked, 5);
        assert_eq!(left.status.items, [row('.', 'M', "both.txt")]);

        let again = kept
            .answered(true, true, &fence(8))
            .expect("a second commit follows from what the first left");
        assert_eq!(again.looked, 8);
        assert_eq!(again.status.items, [row('.', 'M', "both.txt")]);
    }

    /// Only a commit git took tells one; any other answer tells nothing,
    /// and before the first status there is none to work from.
    #[test]
    fn nothing_is_told_but_behind_a_commit_with_a_status_to_work_from() {
        assert!(keeping().answered(false, true, &fence(5)).is_none());
        assert!(
            StatusTold::default()
                .answered(true, true, &fence(5))
                .is_none()
        );
    }

    /// A stage answered at 5 and no status read behind it landed: the one
    /// told at 3 does not say what is staged now, so the commit behind tells
    /// nothing. A status that looked from the stage's end on does.
    #[test]
    fn a_status_behind_a_write_that_moved_its_rows_tells_nothing() {
        let mut kept = keeping();
        assert!(kept.answered(false, true, &fence(5)).is_none());
        assert!(
            kept.answered(true, true, &fence(7)).is_none(),
            "the status told last looked before the stage ended"
        );

        kept.last = Some(Told {
            looked: 8,
            ..told()
        });
        assert!(
            kept.answered(true, true, &fence(9)).is_some(),
            "read after every write that moved the rows"
        );
    }

    /// A write another session landed here, heard of at 5, fences none of
    /// this session's reads: the status told at 3 is behind it, one that
    /// looked at 6 is not.
    #[test]
    fn a_status_from_before_a_write_another_session_landed_tells_nothing() {
        let mut kept = keeping();
        kept.landed_elsewhere(5);
        assert!(kept.answered(true, true, &fence(7)).is_none());

        let mut kept = StatusTold {
            last: Some(Told {
                looked: 6,
                ..told()
            }),
            ..StatusTold::default()
        };
        kept.landed_elsewhere(5);
        assert!(kept.answered(true, true, &fence(7)).is_some());
    }

    /// A fetch reads no tree behind itself where it brought nothing: its
    /// end is no reason to hold the status told before it for stale.
    #[test]
    fn a_write_that_cannot_move_the_rows_leaves_the_status_standing() {
        let mut kept = keeping();
        assert!(kept.answered(false, false, &fence(5)).is_none());
        assert!(kept.answered(true, true, &fence(7)).is_some());
    }

    /// Nor from one holding a conflict, which git takes no commit over.
    #[test]
    fn nothing_follows_from_a_status_holding_a_conflict() {
        let mut status = WorktreeStatus::clone(&told().status);
        status.items.push(StatusItem::Unmerged {
            ours: 'U',
            theirs: 'U',
            path: "clash.txt".to_string(),
        });
        let conflicted = Told {
            status: Arc::new(status),
            ..told()
        };
        assert!(conflicted.left_by_commit(&FENCE).is_none());
    }
}
