//! Where the repository stands, as one record every reader shares.
//!
//! The reads (refs listing, status, walk) each see the repository at
//! their own moment; their reports land here — HEAD, whether anything
//! else holds its tip, whether a remote has its commit, and what the last
//! status saw of the standing operation — and every reader of those
//! answers reads them here (デザイン規約 §行が読む答えはどこから来るか).
//!
//! **Newer wins, and a write is a clock.** Two reads can land in the
//! wrong order (a poll's status begun before a commit, landing after the
//! commit's own read). Every read takes a stamp before it spawns git and
//! offers its report under it; a write takes a stamp on its way out
//! ([`Standing::fence`]) and a report stamped before that is refused. So
//! HEAD never moves backwards past a write, and no consumer orders the
//! reads itself.
//!
//! **The report numbers run across sessions.** A consumer arms a landing
//! on "the first report after this write" by the number the write's
//! answer carries ([`Standing::fence`]); the feed outlives the session,
//! so a count restarting at 1 would leave a landing armed by a closed
//! session waiting on a number the new one takes a lifetime to reach.

use super::*;

/// Numbers every report of HEAD this process sends, in acceptance order —
/// process-wide (module doc).
static NEXT_HEAD_SEQ: AtomicU64 = AtomicU64::new(0);

/// What the refs listing last saw of the branch tip, which is everything
/// the reachability walk needs to start (see [`crate::reachable`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HeadHold {
    /// The commit HEAD is on.
    pub(super) tip: Oid,
    /// Short name of the branch HEAD is on; empty when detached.
    pub(super) branch: String,
    /// Some other ref sits exactly on `tip` — answered without a walk.
    pub(super) on_a_ref: bool,
}

/// What became of a report of HEAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeadOffer {
    /// The read looked before a write ended, or before a read that has
    /// already reported; changes nothing.
    Stale,
    /// The same HEAD as the one standing, and nothing was waiting to hear
    /// it again.
    Same,
    /// The same HEAD, reported by the first read to land after a write —
    /// what a consumer waiting on that write is owed ([`Standing::fence`]).
    Settled { seq: u64 },
    /// HEAD is somewhere else now.
    Moved { seq: u64 },
}

/// Whether a remote already has the commit HEAD is on, and which commit
/// that answer is about — `None` for an unborn branch, which has no
/// commit to have been sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HeadPublished {
    pub(super) oid: Option<Oid>,
    pub(super) published: bool,
}

pub(super) struct Standing {
    /// Hands out the stamps the reads and the fence are ordered by.
    stamps: AtomicU64,
    inner: Mutex<Inner>,
}

/// What a write's end hands its answer: the two numbers a consumer can
/// wait on that write by.
///
/// Both mean "at or above looked after the write", but they order
/// different things and cannot be compared with each other. The consumer
/// is answered by the first report or listing at or above its number, in
/// whichever order that and the write's answer reach it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fence {
    /// The smallest number the first report of HEAD after this write can
    /// carry ([`NEXT_HEAD_SEQ`]). Reads nobody asked for move it too.
    pub head_seq: u64,
    /// The smallest stamp a read that looked after this write can have
    /// ([`Standing::stamp`]) — what a consumer waiting for a listing
    /// measures against. Counting arrivals cannot tell a listing in flight
    /// when the write ended from one begun after: they travel separate
    /// feeds in no fixed order.
    pub reads_from: u64,
}

#[derive(Default)]
struct Inner {
    /// HEAD as the newest report left it. `None` until one has, which the
    /// walk must tell from "no commit yet" (`head.oid` is `None`): the two
    /// take opposite actions.
    head: Option<HeadState>,
    /// The stamp of the last write's end; a read that looked before it is
    /// refused, HEAD and listing alike.
    fence: u64,
    /// The stamp of the report `head` came from. A report stamped
    /// earlier is older news about HEAD alone: the listing or the status
    /// it came with is still the newest of its kind, and still goes out.
    head_at: u64,
    /// A write ended and no read has reported since — the next accepted
    /// report is owed to whoever waits on that write, moved or not.
    fenced: bool,
    /// The number of the last report sent ([`NEXT_HEAD_SEQ`]) — a move,
    /// or the first read after a write. A read that found HEAD unmoved
    /// takes no number, so the number a status is read under (`head_seq`)
    /// names the report the consumer holds.
    seq: u64,
    /// The (commit, refs listing) the off-window question was last put to
    /// git for (`RepoSession::settle_head_published`); only the refs
    /// moving can change the answer.
    published_ask: Option<(Oid, u64)>,
    hold: Option<HeadHold>,
    /// Whether anything besides the branch HEAD is on reaches its tip, as
    /// last answered; `None` until it has been.
    reach: Option<bool>,
    /// Whether a remote already has the commit HEAD is on, as the walk
    /// last read it off its rows (`session::published`); `None` until a
    /// walk has answered.
    published: Option<HeadPublished>,
    /// Dirty working tree — one of the two halves that put a synthetic WIP
    /// row in front of the log stream (the other is below).
    wip_dirty: bool,
    /// What a standing merge is bringing in (`MERGE_HEAD`), else empty;
    /// the WIP row leashes these as well as HEAD. With `wip_dirty` it
    /// decides that row: either makes it, either moving rebuilds the graph.
    merge_incoming: Vec<Oid>,
    /// The merge tool as the last status read that asked for it saw it,
    /// repeated by the reads that did not ask.
    merge_tool: String,
    /// The push destination's counts as the last read that could tell
    /// left them, and the branch they are about
    /// (`RepoSession::read_push_track`): one failed read would drop a
    /// diverged `push -f` to the plain shape for a tick, and a hold under
    /// way with it.
    push_track: Option<(String, crate::remote::PushTrack)>,
}

impl Default for Standing {
    fn default() -> Self {
        Self {
            stamps: AtomicU64::new(0),
            inner: Mutex::new(Inner::default()),
        }
    }
}

impl Standing {
    /// A stamp for a read about to start: taken before git is spawned, so
    /// it orders when the repository was looked at.
    pub(super) fn stamp(&self) -> u64 {
        self.stamps.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// A write has ended: only reads begun after it speak for the
    /// repository, and the next to report is [`HeadOffer::Settled`].
    pub(super) fn fence(&self) -> Fence {
        let at = self.stamp();
        let mut inner = self.lock();
        inner.fence = at;
        inner.fenced = true;
        Fence {
            head_seq: NEXT_HEAD_SEQ.load(Ordering::SeqCst) + 1,
            reads_from: at,
        }
    }

    /// Whether a read stamped `at` may still speak for the repository —
    /// false once a write has ended behind it. Two reads of one tick both
    /// may: [`Self::offer_head`] orders their HEADs, and what each listed
    /// is still the newest of its kind.
    pub(super) fn current(&self, at: u64) -> bool {
        at >= self.lock().fence
    }

    /// Takes a read's report of HEAD, stamped at its spawn; only a report
    /// the consumer is sent takes a number.
    pub(super) fn offer_head(&self, at: u64, head: &HeadState) -> HeadOffer {
        let mut inner = self.lock();
        if at < inner.fence || at < inner.head_at {
            return HeadOffer::Stale;
        }
        inner.head_at = at;
        let settling = std::mem::take(&mut inner.fenced);
        if inner.head.as_ref() != Some(head) {
            inner.head = Some(head.clone());
            // The walk's answer was about the commit HEAD just left; the
            // next one goes out even where it reads the same.
            inner.published = None;
            let seq = NEXT_HEAD_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
            inner.seq = seq;
            return HeadOffer::Moved { seq };
        }
        if settling {
            let seq = NEXT_HEAD_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
            inner.seq = seq;
            HeadOffer::Settled { seq }
        } else {
            HeadOffer::Same
        }
    }

    /// The number of the last report sent — what a status read under it
    /// carries, so the consumer knows which report its counts stand beside.
    pub(super) fn head_seq(&self) -> u64 {
        self.lock().seq
    }

    /// HEAD as it stands; `None` until a read has reported one.
    #[cfg(test)]
    pub(super) fn head(&self) -> Option<HeadState> {
        self.lock().head.clone()
    }

    /// The commit HEAD is on, for the walk: `None` until a read has
    /// landed, `Some(None)` for a branch with no commits yet.
    pub(super) fn head_tip(&self) -> Option<Option<Oid>> {
        self.lock().head.as_ref().map(|head| head.oid)
    }

    pub(super) fn set_hold(&self, hold: Option<HeadHold>) {
        self.lock().hold = hold;
    }

    pub(super) fn hold(&self) -> Option<HeadHold> {
        self.lock().hold.clone()
    }

    /// Records whether anything else reaches the tip, answering whether
    /// that moved — the only time the consumer is told.
    pub(super) fn offer_reach(&self, reached_elsewhere: bool) -> bool {
        self.lock().reach.replace(reached_elsewhere) != Some(reached_elsewhere)
    }

    /// Records what the walk read off HEAD's row, answering whether that
    /// moved.
    pub(super) fn offer_published(&self, answer: HeadPublished) -> bool {
        self.lock().published.replace(answer) != Some(answer)
    }

    #[cfg(test)]
    pub(super) fn published(&self) -> Option<HeadPublished> {
        self.lock().published
    }

    /// Claims the off-window question for `oid` under the refs listing
    /// `refs`, answering whether it is a new one (`Inner::published_ask`).
    pub(super) fn claim_published_ask(&self, oid: Oid, refs: u64) -> bool {
        self.lock().published_ask.replace((oid, refs)) != Some((oid, refs))
    }

    /// Records whether the tree is dirty, answering whether that flipped.
    pub(super) fn set_wip_dirty(&self, dirty: bool) -> bool {
        std::mem::replace(&mut self.lock().wip_dirty, dirty) != dirty
    }

    pub(super) fn wip_dirty(&self) -> bool {
        self.lock().wip_dirty
    }

    /// Records the sides a standing merge brings in, answering whether
    /// they moved.
    pub(super) fn set_merge_incoming(&self, incoming: Vec<Oid>) -> bool {
        let mut inner = self.lock();
        let moved = inner.merge_incoming != incoming;
        inner.merge_incoming = incoming;
        moved
    }

    pub(super) fn merge_incoming(&self) -> Vec<Oid> {
        self.lock().merge_incoming.clone()
    }

    pub(super) fn merge_tool(&self) -> String {
        self.lock().merge_tool.clone()
    }

    pub(super) fn set_merge_tool(&self, tool: String) {
        self.lock().merge_tool = tool;
    }

    /// The kept push counts where they are about `branch`; nothing where
    /// the last read was about another.
    pub(super) fn push_track_of(&self, branch: &str) -> crate::remote::PushTrack {
        match &self.lock().push_track {
            Some((kept, track)) if kept == branch => track.clone(),
            _ => crate::remote::PushTrack::default(),
        }
    }

    pub(super) fn set_push_track(&self, branch: &str, track: crate::remote::PushTrack) {
        self.lock().push_track = Some((branch.to_string(), track));
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        relock(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(branch: &str, oid: u8) -> HeadState {
        HeadState {
            branch: Some(branch.to_string()),
            oid: Some(Oid::from_hex_str(&format!("{oid:02x}").repeat(20)).expect("test oid")),
            detached: false,
        }
    }

    fn number(offer: HeadOffer) -> u64 {
        match offer {
            HeadOffer::Moved { seq } | HeadOffer::Settled { seq } => seq,
            other => panic!("{other:?} carries no number"),
        }
    }

    /// What a failed read repeats is only ever the same branch's answer.
    #[test]
    fn the_kept_push_counts_are_the_branch_they_were_read_for() {
        let standing = Standing::default();
        let track = crate::remote::PushTrack {
            tracking: "fork/main".to_string(),
            ahead: 1,
            behind: 2,
        };
        assert_eq!(
            standing.push_track_of("main"),
            crate::remote::PushTrack::default()
        );
        standing.set_push_track("main", track.clone());
        assert_eq!(standing.push_track_of("main"), track);
        assert_eq!(
            standing.push_track_of("topic"),
            crate::remote::PushTrack::default(),
            "main's destination is not topic's"
        );
    }

    #[test]
    fn the_first_report_moves_head_and_the_same_one_again_says_nothing() {
        let standing = Standing::default();
        assert_eq!(standing.head(), None);
        let first = standing.stamp();
        assert!(matches!(
            standing.offer_head(first, &on("main", 1)),
            HeadOffer::Moved { .. }
        ));
        let again = standing.stamp();
        assert_eq!(standing.offer_head(again, &on("main", 1)), HeadOffer::Same);
        assert_eq!(standing.head(), Some(on("main", 1)));
    }

    /// A read that looked earlier cannot overwrite one that looked later,
    /// whichever lands first — but only its HEAD is stale: both still speak
    /// for what they listed.
    #[test]
    fn a_read_that_looked_earlier_is_stale_once_a_later_one_has_reported() {
        let standing = Standing::default();
        let earlier = standing.stamp();
        let later = standing.stamp();
        assert!(matches!(
            standing.offer_head(later, &on("main", 2)),
            HeadOffer::Moved { .. }
        ));
        assert_eq!(
            standing.offer_head(earlier, &on("main", 1)),
            HeadOffer::Stale
        );
        assert_eq!(
            standing.head(),
            Some(on("main", 2)),
            "the later look stands"
        );
        assert!(standing.current(earlier), "no write ended behind it");
        assert!(standing.current(later));
    }

    /// A poll begun before a write ended is stale however late it lands,
    /// and the first read after it is `Settled` even where HEAD did not
    /// move, numbered at or above what the fence named.
    #[test]
    fn a_write_fences_off_the_reads_that_began_before_it_ended() {
        let standing = Standing::default();
        let before = standing.stamp();
        let first = number(standing.offer_head(before, &on("main", 1)));
        let polling = standing.stamp();
        let owed = standing.fence().head_seq;
        assert!(
            owed > first,
            "the report the write is owed is numbered above every one before it"
        );
        assert!(!standing.current(polling), "began before the write ended");
        assert_eq!(
            standing.offer_head(polling, &on("main", 9)),
            HeadOffer::Stale
        );
        assert_eq!(
            standing.head(),
            Some(on("main", 1)),
            "a stale look moves nothing"
        );

        let settling = standing.stamp();
        assert!(standing.current(settling));
        let settled = standing.offer_head(settling, &on("main", 1));
        assert!(
            matches!(settled, HeadOffer::Settled { .. }),
            "the first report after the fence is owed to whoever waits on the write"
        );
        assert!(
            number(settled) >= owed,
            "and it carries the number the write's answer named, or one above it"
        );
        assert_eq!(
            standing.head_seq(),
            number(settled),
            "a status read now stands beside that report"
        );
        let quiet = standing.stamp();
        assert_eq!(standing.offer_head(quiet, &on("main", 1)), HeadOffer::Same);
        assert_eq!(
            standing.head_seq(),
            number(settled),
            "a read that found HEAD where it was numbers nothing"
        );
    }

    /// The listing half of the fence ([`Fence::reads_from`]).
    #[test]
    fn a_listing_that_looked_before_the_write_ended_is_below_the_fence() {
        let standing = Standing::default();
        let in_flight = standing.stamp();
        let fence = standing.fence();
        let afterwards = standing.stamp();
        assert!(
            in_flight < fence.reads_from,
            "a read that began before the write ended cannot speak for what it left"
        );
        assert!(
            afterwards >= fence.reads_from,
            "and one that began after it can"
        );
    }

    #[test]
    fn a_report_after_a_write_that_moved_head_is_a_move_not_a_settling() {
        let standing = Standing::default();
        let at = standing.stamp();
        let first = number(standing.offer_head(at, &on("main", 1)));
        let owed = standing.fence().head_seq;
        let at = standing.stamp();
        let moved = standing.offer_head(at, &on("main", 2));
        assert!(matches!(moved, HeadOffer::Moved { .. }));
        assert!(number(moved) >= owed && number(moved) > first);
        let at = standing.stamp();
        assert_eq!(standing.offer_head(at, &on("main", 2)), HeadOffer::Same);
    }

    /// A report of one record is never numbered below a fence of another,
    /// so a landing armed by a closed session is answered by its
    /// replacement's first report.
    #[test]
    fn the_numbers_run_across_records() {
        let one = Standing::default();
        let two = Standing::default();
        let a = number(one.offer_head(one.stamp(), &on("main", 1)));
        let b = number(two.offer_head(two.stamp(), &on("main", 1)));
        assert!(b > a);
        let owed = one.fence().head_seq;
        let c = number(two.offer_head(two.stamp(), &on("main", 2)));
        assert!(
            c >= owed,
            "a report of the other record after the fence is numbered above it too"
        );
    }

    #[test]
    fn a_move_takes_the_published_answer_with_it() {
        let standing = Standing::default();
        standing.offer_head(standing.stamp(), &on("main", 1));
        let sent = HeadPublished {
            oid: on("main", 1).oid,
            published: true,
        };
        assert!(standing.offer_published(sent));
        assert!(!standing.offer_published(sent));
        standing.offer_head(standing.stamp(), &on("main", 2));
        assert_eq!(standing.published(), None);
        let next = HeadPublished {
            oid: on("main", 2).oid,
            published: true,
        };
        assert!(
            standing.offer_published(next),
            "the answer for the new commit is news even where it reads the same"
        );
    }

    #[test]
    fn the_off_window_question_is_claimed_once_per_listing() {
        let standing = Standing::default();
        let oid = on("main", 1).oid.expect("a test oid");
        assert!(standing.claim_published_ask(oid, 7));
        assert!(!standing.claim_published_ask(oid, 7));
        assert!(standing.claim_published_ask(oid, 8), "the refs moved");
        assert!(
            standing.claim_published_ask(oid, 7),
            "and moved again — a different listing than the one last asked under"
        );
    }

    #[test]
    fn the_derived_answers_say_whether_they_moved() {
        let standing = Standing::default();
        assert!(standing.offer_reach(false), "the first answer is news");
        assert!(!standing.offer_reach(false));
        assert!(standing.offer_reach(true));
        let sent = HeadPublished {
            oid: on("main", 1).oid,
            published: true,
        };
        assert!(standing.offer_published(sent));
        assert!(!standing.offer_published(sent));
        assert!(standing.set_wip_dirty(true));
        assert!(!standing.set_wip_dirty(true));
        let side = on("x", 3).oid.expect("a test oid");
        assert!(standing.set_merge_incoming(vec![side]));
        assert!(!standing.set_merge_incoming(vec![side]));
        assert!(standing.set_merge_incoming(Vec::new()));
    }
}
