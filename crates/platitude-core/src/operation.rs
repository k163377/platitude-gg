//! What one write is, as the queue was asked for it: the id that follows
//! it from acceptance to the end of the reads it invalidated, which kind
//! of write it is, and which lane serves it.
//!
//! **One id from the press to the last publish.** The queue hands the id
//! back the moment it accepts the request (`RepoSession::write` and every
//! entry point built on it — and nothing at all where it accepts none,
//! since an id nobody will ever answer is worse than a refusal the
//! caller can read), and everything said about that write
//! afterwards carries it: its start, a stop git left it on, git's own
//! answer (`SessionEvent::WriteFinished`), the end of the reads that
//! followed (`SessionEvent::WriteSettled`), and every git command spawned
//! under it (`SessionEvent::CommandStarted::operation` — a compound write
//! is several commands under one id). A consumer waiting on its own write
//! matches the id and reads nothing into the order answers arrive in or
//! into what kind they say they are: two stash operations answer under
//! the same label, and the one behind answers the moment the one in front
//! does.
//!
//! **An ordering of its own.** The graph's generation, the
//! details read's, and the numbered reports of HEAD (`head_seq`) each
//! order something else — reads nobody asked for move them — and none of
//! them is this id. A write's answer still names the `head_seq` a landing
//! waits for, beside the id.

use std::sync::atomic::{AtomicU64, Ordering};

/// Numbers every write this process accepts. Process-wide for the
/// reason the reports of HEAD are (`session::standing`):
/// a feed outlives the session that filled it, and a count starting again
/// at 1 would let a closed session's id be answered by the next session's
/// write.
static NEXT_OPERATION: AtomicU64 = AtomicU64::new(1);

/// Names one write from the moment the queue accepted it.
///
/// Compared by equality and nothing else. The queue runs writes in the
/// order it accepted them, but a reader that needs its own answer
/// asks for it by this id: what answered in between is somebody
/// else's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

impl OperationId {
    /// The next id, taken at acceptance. Always positive, so a bridge
    /// carries integers can use zero for "nothing was accepted".
    pub(crate) fn next() -> Self {
        Self(NEXT_OPERATION.fetch_add(1, Ordering::Relaxed))
    }

    /// The number itself, for a bridge that carries integers.
    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Which write it is.
///
/// One type for the three things that used to read the write's label as
/// a string — the queue's lane, the poll's gate under a replay, and the
/// application's classification of the answer — so a kind added here has
/// to be placed in each of them: the matches are exhaustive, and there is
/// no default lane to fall into by omission.
///
/// The kinds are as coarse as the application reads them ([`Self::label`]):
/// every stash operation is `Stash`, every branch write `Branch`. Which
/// press an answer belongs to is the id's to say. The
/// two compound deletes are their own kinds because the lane cannot be
/// read off the label they answer under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationKind {
    Stage,
    Unstage,
    Discard,
    Commit,
    Checkout,
    Reset,
    Stash,
    Branch,
    Tag,
    Push,
    Fetch,
    /// `git pull`: one command that reaches the network and then moves
    /// this copy's own branch, so it is supervised as a remote write and
    /// ordered as a local one.
    Pull,
    Remote,
    Merge,
    Rebase,
    Squash,
    Drop,
    Reword,
    CherryPick,
    Revert,
    Resolve,
    Mergetool,
    Config,
    Identity,
    /// A branch deleted here and on its remote as one write: answers as
    /// a branch write, runs on the remote lane, since the second half is
    /// a push.
    DeleteBranchEverywhere,
    /// The same pair for a tag.
    DeleteTagEverywhere,
    /// The fetch the interval fires.
    AutoFetch,
    /// The fetch an opening fires. Told apart from the interval's because
    /// the application answers it differently: the button turns for both,
    /// and only this one leaves the command log where it was — a tab
    /// opened on a machine that is offline opens quietly
    /// (デザイン規約 §リモートから取り込む).
    OpenFetch,
}

impl OperationKind {
    /// The word the write answers under — what the application classifies
    /// the answer by and the log names the write with. Coarser than the
    /// kind where the application reads two kinds alike: a branch deleted
    /// here and on its remote answers as a branch write.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Stage => "stage",
            Self::Unstage => "unstage",
            Self::Discard => "discard",
            Self::Commit => "commit",
            Self::Checkout => "checkout",
            Self::Reset => "reset",
            Self::Stash => "stash",
            Self::Branch | Self::DeleteBranchEverywhere => "branch",
            Self::Tag | Self::DeleteTagEverywhere => "tag",
            Self::Push => "push",
            Self::Fetch => "fetch",
            Self::Pull => "pull",
            Self::Remote => "remote",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::Squash => "squash",
            Self::Drop => "drop",
            Self::Reword => "reword",
            Self::CherryPick => "cherry-pick",
            Self::Revert => "revert",
            Self::Resolve => "resolve",
            Self::Mergetool => "mergetool",
            Self::Config => "config",
            Self::Identity => "identity",
            Self::AutoFetch => "auto-fetch",
            Self::OpenFetch => "open-fetch",
        }
    }

    /// Whether a person asked for a write of this kind, or it is one of
    /// the fetches this application makes on its own — the interval's and
    /// the one an opening fires.
    ///
    /// **Read off the lane**: the lane already names that set
    /// ([`Lane::UnaskedFetch`]), and a second spelling would let the
    /// two disagree about which writes have somebody waiting on
    /// them.
    #[must_use]
    pub fn asked_for(self) -> bool {
        self.lane() != Lane::UnaskedFetch
    }

    /// Which lane serves a write of this kind.
    ///
    /// **The one place the sets are written.** The budget, the token and
    /// the executor all follow from this answer (`session::write`), and a
    /// second spelling of it would let them disagree about what is
    /// running. Exhaustive on purpose: a kind whose task reaches the
    /// network in *any* half goes on the remote lane, and the local lane
    /// — waited out, uncancellable, holding the quit gate — is not a
    /// default a new kind can fall into unnamed.
    #[must_use]
    pub fn lane(self) -> Lane {
        match self {
            Self::Push
            | Self::Fetch
            | Self::Pull
            | Self::DeleteBranchEverywhere
            | Self::DeleteTagEverywhere => Lane::Remote,
            Self::AutoFetch | Self::OpenFetch => Lane::UnaskedFetch,
            Self::Stage
            | Self::Unstage
            | Self::Discard
            | Self::Commit
            | Self::Checkout
            | Self::Reset
            | Self::Stash
            | Self::Branch
            | Self::Tag
            | Self::Remote
            | Self::Merge
            | Self::Rebase
            | Self::Squash
            | Self::Drop
            | Self::Reword
            | Self::CherryPick
            | Self::Revert
            | Self::Resolve
            | Self::Mergetool
            | Self::Config
            | Self::Identity => Lane::Local,
        }
    }

    /// Whether a write of this kind changes what this copy of the
    /// repository holds — its index, its own refs, the operation left
    /// standing in it.
    ///
    /// **A second question from [`Self::lane`], and not read off it.**
    /// The lane says how a write is supervised: what budget it runs
    /// under, whose token can stop it, whether a close waits it out.
    /// This says whether the working tree's order has to serve it in
    /// turn (`session::write_order`). The composite deletes are where
    /// the two part company — their far half is paced by the network, so
    /// they are supervised as remote writes, and their near half deletes
    /// a ref here, so they stay behind a rename accepted before
    /// them.
    ///
    /// `false` only where the whole effect is at the other end and on
    /// the remote-tracking refs that mirror it: a push, and the three
    /// fetches. Putting those in the order would leave a commit waiting
    /// behind somebody else's round trip for nothing.
    ///
    /// Exhaustive for the same reason the lane is: a new kind has to be
    /// placed by hand, and neither answer is a default to fall into.
    #[must_use]
    pub fn writes_here(self) -> bool {
        match self {
            Self::Push | Self::Fetch | Self::AutoFetch | Self::OpenFetch => false,
            Self::Pull
            | Self::DeleteBranchEverywhere
            | Self::DeleteTagEverywhere
            | Self::Stage
            | Self::Unstage
            | Self::Discard
            | Self::Commit
            | Self::Checkout
            | Self::Reset
            | Self::Stash
            | Self::Branch
            | Self::Tag
            | Self::Remote
            | Self::Merge
            | Self::Rebase
            | Self::Squash
            | Self::Drop
            | Self::Reword
            | Self::CherryPick
            | Self::Revert
            | Self::Resolve
            | Self::Mergetool
            | Self::Config
            | Self::Identity => true,
        }
    }

    /// Whether a write of this kind replays history a commit at
    /// a time.
    ///
    /// These are the writes that can stand for tens of seconds — a replay
    /// pays per commit (ci/baseline/code-costs-windows-x64.md), so a
    /// range of a few hundred is seconds and the graph's own window is
    /// far longer — and they are the ones git leaves a standing operation
    /// on disk for while they run. Both halves of that matter: the poll
    /// is let through under these so the badge can count the steps out
    /// (`RepoSession::refresh_poll`), and the screen holds its write
    /// doors down for as long as one is out. A pull is in the set because
    /// it brings the far side in by one of the two already here —
    /// `pull.rebase` makes it a rebase and anything else a merge — and
    /// stops where they stop. The three one-commit edits
    /// are in the set because each is a rebase underneath
    /// (`sequencer::plan_edit` + `run_plan`), so a squash near the root
    /// replays everything above it — the same wait under a shorter name.
    ///
    /// **The one place the set is written**, for the reason
    /// [`Self::lane`] gives.
    #[must_use]
    pub fn replays_history(self) -> bool {
        matches!(
            self,
            Self::Pull
                | Self::Merge
                | Self::Rebase
                | Self::Squash
                | Self::Drop
                | Self::Reword
                | Self::CherryPick
                | Self::Revert
                | Self::Resolve
        )
    }
}

/// How the queue supervises a write: which executor runs it, what budget
/// binds it, and what a close does to it. A pure function of the kind
/// ([`OperationKind::lane`]), carried on the request so the queue reads
/// the answer once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    /// A write that costs what the repository's own size makes it cost:
    /// slowness is not a hang, so the queue waits it out to the end, with
    /// no stock budget and no cancellation, even through a close. A kill
    /// mid-write is the one way the queue can lose what the user asked
    /// for (measured: a killed commit is simply gone), and a user's own
    /// wedged hook is the user's to deal with — the screen shows busy
    /// and the command log shows what is running.
    Local,
    /// Paced by the far end of a network connection — the fetches and
    /// pushes, whose one real way to hang is a remote that stopped
    /// answering. Keeps the stock time budget and dies with the session.
    Remote,
    /// The remote lane on a handle of its own, for the fetches nobody
    /// asked for — the interval's and the one an opening fires: one that
    /// lands leaves no row in the command log, and one git said no to
    /// leaves its own (`process::Kept::UnaskedUnlessItFails`). Neither
    /// raises the panel by being a row, so an offline laptop still gets
    /// the one telling its first failure is entitled to.
    UnaskedFetch,
}

#[cfg(test)]
mod tests {
    use super::{Lane, OperationKind};

    /// The set is the writes that hand a range to git one commit at a
    /// time; everything else touches the index once and comes back, and
    /// the poll stays out under it the way it always has.
    #[test]
    fn the_writes_that_replay_are_the_ones_a_range_can_make_long() {
        for kind in [
            OperationKind::Pull,
            OperationKind::Merge,
            OperationKind::Rebase,
            OperationKind::Squash,
            OperationKind::Drop,
            OperationKind::Reword,
            OperationKind::CherryPick,
            OperationKind::Revert,
            OperationKind::Resolve,
        ] {
            assert!(kind.replays_history(), "{kind:?} hands a range to git");
        }
        for kind in [
            OperationKind::Commit,
            OperationKind::Stage,
            OperationKind::Unstage,
            OperationKind::Discard,
            OperationKind::Stash,
            OperationKind::Checkout,
            OperationKind::Reset,
            OperationKind::Branch,
            OperationKind::Tag,
            OperationKind::Push,
            OperationKind::Fetch,
            OperationKind::Config,
            OperationKind::Mergetool,
            OperationKind::Identity,
            OperationKind::Remote,
            OperationKind::DeleteBranchEverywhere,
            OperationKind::DeleteTagEverywhere,
            OperationKind::AutoFetch,
            OperationKind::OpenFetch,
        ] {
            assert!(
                !kind.replays_history(),
                "{kind:?} is one pass, not a replay"
            );
        }
    }

    /// The remote lane is the fetches and pushes and the two compound
    /// deletes whose second half is a push; the unasked fetches have a
    /// lane of their own; every other kind is local and waited out.
    #[test]
    fn the_lane_follows_the_kind_and_nothing_else() {
        for kind in [
            OperationKind::Push,
            OperationKind::Fetch,
            OperationKind::Pull,
            OperationKind::DeleteBranchEverywhere,
            OperationKind::DeleteTagEverywhere,
        ] {
            assert_eq!(
                kind.lane(),
                Lane::Remote,
                "{kind:?} is paced by the far end"
            );
        }
        for kind in [OperationKind::AutoFetch, OperationKind::OpenFetch] {
            assert_eq!(
                kind.lane(),
                Lane::UnaskedFetch,
                "{kind:?} was asked for by nobody"
            );
        }
        for kind in [
            OperationKind::Stage,
            OperationKind::Unstage,
            OperationKind::Discard,
            OperationKind::Commit,
            OperationKind::Checkout,
            OperationKind::Reset,
            OperationKind::Branch,
            OperationKind::Tag,
            OperationKind::Stash,
            OperationKind::Merge,
            OperationKind::Rebase,
            OperationKind::Squash,
            OperationKind::Drop,
            OperationKind::Reword,
            OperationKind::CherryPick,
            OperationKind::Revert,
            OperationKind::Resolve,
            OperationKind::Mergetool,
            OperationKind::Identity,
            OperationKind::Remote,
            OperationKind::Config,
        ] {
            assert_eq!(kind.lane(), Lane::Local, "{kind:?} is local and waited out");
        }
    }

    /// The compound deletes answer under the label of the local write
    /// they contain — what the application reads — while their lane is
    /// the push's.
    #[test]
    fn a_compound_delete_answers_as_its_local_half_and_runs_as_its_remote_half() {
        assert_eq!(OperationKind::DeleteBranchEverywhere.label(), "branch");
        assert_eq!(OperationKind::DeleteTagEverywhere.label(), "tag");
        assert_eq!(OperationKind::Branch.lane(), Lane::Local);
        assert_eq!(OperationKind::DeleteBranchEverywhere.lane(), Lane::Remote);
    }

    /// A pull is supervised by the far end and ordered by the working
    /// tree: the one command whose two halves are on opposite sides of
    /// that line, where the fetches are wholly on one and the merge
    /// wholly on the other.
    #[test]
    fn a_pull_runs_on_the_remote_lane_and_takes_its_place_in_the_tree() {
        assert_eq!(OperationKind::Pull.lane(), Lane::Remote);
        assert!(OperationKind::Pull.writes_here());
        assert!(!OperationKind::Fetch.writes_here());
        assert_eq!(OperationKind::Merge.lane(), Lane::Local);
    }

    /// Ids are handed out in acceptance order and never repeat within a
    /// process, and none of them is zero.
    #[test]
    fn ids_are_distinct_and_never_zero() {
        let first = super::OperationId::next();
        let second = super::OperationId::next();
        assert_ne!(first, second);
        assert!(first.as_u64() > 0 && second.as_u64() > first.as_u64());
    }
}
