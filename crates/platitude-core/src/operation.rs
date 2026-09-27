//! What one write is, as the queue was asked for it: the id that follows
//! it from acceptance to the end of the reads it invalidated, which kind
//! of write it is, and which lane serves it.
//!
//! **One id from the press to the last publish.** The queue hands the id
//! back when it accepts the request (`RepoSession::write` and every entry
//! point on it; none where it accepts none — an id nobody will answer is
//! worse than a refusal), and everything said about that write carries
//! it: its start, a stop, `SessionEvent::WriteFinished`,
//! `SessionEvent::WriteSettled`, and every git command spawned under it
//! (`SessionEvent::CommandStarted::operation`). A consumer waiting on its
//! own write matches the id, never the order or the kind: two stash
//! operations answer under the same label.
//!
//! The graph's and the details' generations and `head_seq` order other
//! things (unasked reads move them) and are not this id.

use std::sync::atomic::{AtomicU64, Ordering};

/// Process-wide, not per session: a feed outlives the session that
/// filled it, and a count restarting at 1 would let the next session's
/// write answer a closed session's id.
static NEXT_OPERATION: AtomicU64 = AtomicU64::new(1);

/// Names one write from the moment the queue accepted it.
///
/// Compared by equality and nothing else: a reader asks for its own
/// answer by this id, and whatever answered in between is somebody
/// else's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

impl OperationId {
    /// The next id, taken at acceptance. Always positive, so an integer
    /// bridge can use zero for "nothing was accepted".
    pub(crate) fn next() -> Self {
        Self(NEXT_OPERATION.fetch_add(1, Ordering::Relaxed))
    }

    /// The number itself, for an integer bridge.
    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Which write it is.
///
/// One type, not a label read as a string, for the queue's lane, the
/// poll's gate under a replay and the application's classification: the
/// matches are exhaustive, so a new kind has to be placed in each.
///
/// As coarse as the application reads them ([`Self::label`]); which press
/// an answer belongs to is the id's to say. The two compound deletes are
/// their own kinds because their lane cannot be read off their label.
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
    /// `git worktree remove`: another working copy taken off the disk.
    Worktree,
    /// A branch deleted here and on its remote as one write: answers as
    /// a branch write, runs on the remote lane (the second half is a push).
    DeleteBranchEverywhere,
    /// The same pair for a tag.
    DeleteTagEverywhere,
    /// The fetch the interval fires.
    AutoFetch,
    /// The fetch an opening fires. Apart from the interval's because only
    /// this one leaves the command log where it was, so a tab opened
    /// offline opens quietly (デザイン規約 §リモートから取り込む).
    OpenFetch,
}

impl OperationKind {
    /// The word the write answers under: what the application classifies
    /// the answer by and the log names the write with. Coarser than the
    /// kind where the application reads two kinds alike.
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
            Self::Worktree => "worktree",
            Self::AutoFetch => "auto-fetch",
            Self::OpenFetch => "open-fetch",
        }
    }

    /// Whether a person asked for a write of this kind, rather than the
    /// application firing a fetch on its own. Read off the lane
    /// ([`Lane::UnaskedFetch`]) so the two cannot disagree.
    #[must_use]
    pub fn asked_for(self) -> bool {
        self.lane() != Lane::UnaskedFetch
    }

    /// Which lane serves a write of this kind.
    ///
    /// **The one place the sets are written**: the budget, the token and
    /// the executor all follow from it (`session::write`). Exhaustive on
    /// purpose: a kind that reaches the network in *any* half goes on the
    /// remote lane, and the local lane — waited out, uncancellable,
    /// holding the quit gate — is no default to fall into unnamed.
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
            | Self::Identity
            | Self::Worktree => Lane::Local,
        }
    }

    /// Whether a write of this kind changes what this copy holds (its
    /// index, its own refs, a standing operation), so the working tree's
    /// order serves it in turn (`session::write_order`).
    ///
    /// **Not read off [`Self::lane`]**, which says how a write is
    /// supervised: a pull and the composite deletes are paced by the
    /// network but move a ref here, so they stay behind a rename accepted
    /// before them. `false` only for a push and the three fetches, whose
    /// effect is all at the other end — ordering them would hold a commit
    /// behind a round trip for nothing. Exhaustive for the lane's reason.
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
            | Self::Identity
            | Self::Worktree => true,
        }
    }

    /// Whether a write of this kind replays history a commit at a time:
    /// it pays per commit (ci/baseline/code-costs-windows-x64.md), and git
    /// leaves a standing operation on disk while it runs. So the poll is
    /// let through under these to count the steps
    /// (`RepoSession::refresh_poll`), and the screen holds its write doors
    /// down while one is out. A pull is in because it is a rebase or a
    /// merge underneath (`pull.rebase`); the three one-commit edits
    /// because each is a rebase (`sequencer::plan_edit` + `run_plan`).
    ///
    /// **The one place the set is written**, as with [`Self::lane`].
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
/// ([`OperationKind::lane`]), carried on the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    /// Paced by the repository's own size: slowness is not a hang, so the
    /// queue waits it out — no stock budget, no cancellation, even through
    /// a close. A kill mid-write loses what the user asked for (a killed
    /// commit is gone); a wedged hook shows as busy, with the command in
    /// the log.
    Local,
    /// Paced by the far end of a network connection, whose one real way to
    /// hang is a remote that stopped answering. Keeps the stock time
    /// budget and dies with the session.
    Remote,
    /// The remote lane on a handle of its own, for the fetches nobody
    /// asked for: one that lands leaves no command-log row, and one git
    /// said no to leaves its own (`process::Kept::UnaskedUnlessItFails`).
    UnaskedFetch,
}

#[cfg(test)]
mod tests {
    use super::{Lane, OperationKind};

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
            OperationKind::Worktree,
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
            OperationKind::Worktree,
            OperationKind::Remote,
            OperationKind::Config,
        ] {
            assert_eq!(kind.lane(), Lane::Local, "{kind:?} is local and waited out");
        }
    }

    #[test]
    fn a_compound_delete_answers_as_its_local_half_and_runs_as_its_remote_half() {
        assert_eq!(OperationKind::DeleteBranchEverywhere.label(), "branch");
        assert_eq!(OperationKind::DeleteTagEverywhere.label(), "tag");
        assert_eq!(OperationKind::Branch.lane(), Lane::Local);
        assert_eq!(OperationKind::DeleteBranchEverywhere.lane(), Lane::Remote);
    }

    #[test]
    fn a_pull_runs_on_the_remote_lane_and_takes_its_place_in_the_tree() {
        assert_eq!(OperationKind::Pull.lane(), Lane::Remote);
        assert!(OperationKind::Pull.writes_here());
        assert!(!OperationKind::Fetch.writes_here());
        assert_eq!(OperationKind::Merge.lane(), Lane::Local);
    }

    #[test]
    fn ids_are_distinct_and_never_zero() {
        let first = super::OperationId::next();
        let second = super::OperationId::next();
        assert_ne!(first, second);
        assert!(first.as_u64() > 0 && second.as_u64() > first.as_u64());
    }
}
