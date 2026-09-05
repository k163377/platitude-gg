//! Where the repository stands, as one record every reader shares.
//!
//! The reads — the refs listing, the status, the walk — each see a piece
//! of the same repository at their own moment, and this is the one place
//! their reports land: HEAD, whether anything else holds its tip, whether
//! a remote already has the commit it is on, and what the last status saw
//! of the standing operation. Everything that wants one of those answers
//! reads it from here rather than asking git again, and everything that
//! reports one goes through here rather than keeping a copy
//! (CLAUDE.md 性能予算; デザイン規約 §行が読む答えはどこから来るか).
//!
//! **Newer wins, and a write is a clock.** Two reads can land in the
//! wrong order — a poll's status that began before a commit and landed
//! after the commit's own read — and the stale one must not overwrite what
//! the fresh one saw. Every read takes a stamp before it spawns git and
//! offers its report under that stamp; a write takes a stamp on its way
//! out ([`Standing::fence`]), and a report stamped before that is refused.
//! So HEAD here never moves backwards past a write, whichever read lands
//! first, and no consumer has to order the reads for itself.
//!
//! **Every report has a number, and the numbers run across sessions.**
//! A consumer arms a landing on "the first report after this write" and
//! reads that number off the write's own answer ([`Standing::fence`]);
//! the feed carrying both outlives the session that filled it, and a
//! count starting again at 1 would leave a landing armed by a closed
//! session waiting on a number the new one takes a lifetime to reach.

use super::*;

/// Numbers every report of HEAD this process sends, in the order the
/// reports were accepted — one counter for every session, for the reason
/// the details read's is ([`super::details_read`]).
static NEXT_HEAD_SEQ: AtomicU64 = AtomicU64::new(0);

/// What the refs listing last saw of the branch tip, which is everything
/// the reachability walk needs to start (see [`crate::reachable`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HeadHold {
    /// The commit HEAD is on.
    pub(super) tip: Oid,
    /// Short name of the branch HEAD is on; empty when detached.
    pub(super) branch: String,
    /// Some other ref already sits exactly on `tip`, which the listing
    /// answers on its own — no walk needed.
    pub(super) on_a_ref: bool,
}

/// What became of a report of HEAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeadOffer {
    /// The read observed the repository before a write ended, or before a
    /// read that has already reported — its HEAD is older than the one
    /// standing and changes nothing.
    Stale,
    /// The same HEAD as the one standing, and nothing was waiting to hear
    /// it again.
    Same,
    /// The same HEAD, reported by the first read to land after a write —
    /// what a consumer waiting on "the repository as the write left it"
    /// is waiting for, moved or not ([`Standing::fence`]).
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

#[derive(Default)]
struct Inner {
    /// HEAD as the newest read that reported it left it. `None` until one
    /// has — which the walk tells from "looked, and there is no commit
    /// yet" (`head.oid` is `None` then), because the two take opposite
    /// actions.
    head: Option<HeadState>,
    /// The stamp of the last write's end. A read that looked before it
    /// may not speak for the repository after it — not its HEAD, and not
    /// its listing either.
    fence: u64,
    /// The stamp of the report `head` came from. A report stamped
    /// earlier is older news about HEAD alone: the listing or the status
    /// it came with is still the newest of its kind, and still goes out.
    head_at: u64,
    /// A write ended and no read has reported since — the next accepted
    /// report is the one a consumer waiting on that write is owed, moved
    /// or not.
    fenced: bool,
    /// The number of the last report sent ([`NEXT_HEAD_SEQ`]) — a move,
    /// or the first read after a write. A read that found HEAD where the
    /// last report left it sends nothing and takes no number, so the
    /// number a status is read under (`head_seq`) names the report the
    /// consumer holds.
    seq: u64,
    /// Which commit, under which refs listing, the off-window question
    /// was last put to git for (`RepoSession::settle_head_published`):
    /// the same pair is the same answer, and the refs moving is the one
    /// thing that can change it.
    published_ask: Option<(Oid, u64)>,
    /// What the refs listing saw of the tip — the half of the reach
    /// question the listing answers by itself.
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
    /// What a standing merge is bringing in (`MERGE_HEAD`), empty the rest
    /// of the time: the WIP row leashes these as well as HEAD, so it draws
    /// the fork the merge commit is about to have. Beside `wip_dirty`
    /// because the two decide the same row — either one makes it, and
    /// either moving is a graph to rebuild.
    merge_incoming: Vec<Oid>,
    /// Why the standing rebase stopped, as the last read that managed to
    /// tell left it. A read of git's markers can fail transiently, and a
    /// single tick answering "not an `edit` stop" would turn the exit
    /// card's `--skip` from a hold back into a click (`offers::skip_is_free`).
    /// Cleared by the first read that finds no rebase standing.
    rebase_stop: integrate::RebaseStop,
    /// The merge tool as the last status read that asked for it saw it,
    /// repeated by the reads that did not ask.
    merge_tool: String,
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
    /// A stamp for a read about to start: taken **before** git is
    /// spawned, so what it orders is when the repository was looked at,
    /// not when the answer came back.
    pub(super) fn stamp(&self) -> u64 {
        self.stamps.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// A write has ended. Nothing observed before this moment may stand
    /// for the repository after it, and the next read that reports is the
    /// one a consumer waiting on this write is owed ([`HeadOffer::Settled`]).
    ///
    /// Answers the smallest number that report can carry: every report
    /// accepted after this moment is numbered above every one accepted
    /// before it, so a consumer holding a report numbered at or above the
    /// answer holds one that looked after the write — whichever order the
    /// write's answer and that report reach it in.
    pub(super) fn fence(&self) -> u64 {
        let at = self.stamp();
        let mut inner = self.lock();
        inner.fence = at;
        inner.fenced = true;
        NEXT_HEAD_SEQ.load(Ordering::SeqCst) + 1
    }

    /// Whether a read stamped `at` may still speak for the repository —
    /// false once a write has ended behind it. Two reads of one tick both
    /// may: which of them speaks for HEAD is [`Self::offer_head`]'s to
    /// order, and what each read listed is still the newest of its kind.
    pub(super) fn current(&self, at: u64) -> bool {
        at >= self.lock().fence
    }

    /// Takes a read's report of HEAD, stamped at its spawn. Only a report
    /// the consumer is sent takes a number — a read that found HEAD where
    /// the last report left it changes nothing and numbers nothing.
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
            // next walk answers for where it is now, and that answer goes
            // out even where it reads the same.
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
    /// carries, so the consumer can tell which report its counts stand
    /// beside.
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
    /// `refs` names, answering whether it is a new one: the same commit
    /// under the same listing was asked already, and asking git again
    /// would read the same answer.
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
    /// they moved — a merge that started, finished or was aborted redraws
    /// the WIP row's leashes.
    pub(super) fn set_merge_incoming(&self, incoming: Vec<Oid>) -> bool {
        let mut inner = self.lock();
        let moved = inner.merge_incoming != incoming;
        inner.merge_incoming = incoming;
        moved
    }

    pub(super) fn merge_incoming(&self) -> Vec<Oid> {
        self.lock().merge_incoming.clone()
    }

    pub(super) fn rebase_stop(&self) -> integrate::RebaseStop {
        self.lock().rebase_stop.clone()
    }

    pub(super) fn set_rebase_stop(&self, stop: integrate::RebaseStop) {
        self.lock().rebase_stop = stop;
    }

    pub(super) fn merge_tool(&self) -> String {
        self.lock().merge_tool.clone()
    }

    pub(super) fn set_merge_tool(&self, tool: String) {
        self.lock().merge_tool = tool;
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

    /// The number a report went out under; a report that went nowhere
    /// has none.
    fn number(offer: HeadOffer) -> u64 {
        match offer {
            HeadOffer::Moved { seq } | HeadOffer::Settled { seq } => seq,
            other => panic!("{other:?} carries no number"),
        }
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

    /// The order the reads are stamped in is the order they looked, and a
    /// read that looked earlier cannot overwrite one that looked later —
    /// whichever of the two lands first. Its HEAD is the stale part and
    /// nothing more: the two reads of one tick both still speak for what
    /// they listed.
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

    /// A write is a clock: a poll that began before it ended cannot speak
    /// for the repository after it, however late it lands, and the write's
    /// own read is answered `Settled` even where HEAD did not move — that
    /// is the report a consumer waiting on the write is owed, under the
    /// number the write's answer named or one above it.
    #[test]
    fn a_write_fences_off_the_reads_that_began_before_it_ended() {
        let standing = Standing::default();
        let before = standing.stamp();
        let first = number(standing.offer_head(before, &on("main", 1)));
        let polling = standing.stamp();
        let owed = standing.fence();
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

    #[test]
    fn a_report_after_a_write_that_moved_head_is_a_move_not_a_settling() {
        let standing = Standing::default();
        let at = standing.stamp();
        let first = number(standing.offer_head(at, &on("main", 1)));
        let owed = standing.fence();
        let at = standing.stamp();
        let moved = standing.offer_head(at, &on("main", 2));
        assert!(matches!(moved, HeadOffer::Moved { .. }));
        assert!(number(moved) >= owed && number(moved) > first);
        let at = standing.stamp();
        assert_eq!(standing.offer_head(at, &on("main", 2)), HeadOffer::Same);
    }

    /// The numbers run across records: a report of one is never numbered
    /// below a fence of another, so a landing armed by a session that has
    /// since closed is answered by the first report of the one that
    /// replaced it rather than left waiting.
    #[test]
    fn the_numbers_run_across_records() {
        let one = Standing::default();
        let two = Standing::default();
        let a = number(one.offer_head(one.stamp(), &on("main", 1)));
        let b = number(two.offer_head(two.stamp(), &on("main", 1)));
        assert!(b > a);
        let owed = one.fence();
        let c = number(two.offer_head(two.stamp(), &on("main", 2)));
        assert!(
            c >= owed,
            "a report of the other record after the fence is numbered above it too"
        );
    }

    /// A moved HEAD takes the walk's answer with it: the next walk answers
    /// for where HEAD is, and that answer goes out even where it reads the
    /// same as the one about the commit HEAD left.
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

    /// The off-window question goes to git once per commit per refs
    /// listing: the same pair again would read the same answer, and the
    /// listing moving is the one thing that could have changed it.
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
