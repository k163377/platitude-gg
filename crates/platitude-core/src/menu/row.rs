//! The rows a right-click menu draws, the part each belongs to, and where
//! each stands.

/// Declares [`Row`] and [`Row::ALL`] from one list, so a row added to the
/// enum cannot be left out of the list the tables walk.
macro_rules! rows {
    ($($(#[$doc:meta])* $row:ident),* $(,)?) => {
        /// Every row of a right-click menu: one kind per operation,
        /// whichever entrance meets it, so an operation has one rule and one
        /// wording. Declared in drawing order, which [`Row::place`] follows
        /// (`rows_stand_in_the_order_of_their_places`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Row {
            $($(#[$doc])* $row),*
        }

        impl Row {
            /// Every row, in drawing order.
            pub const ALL: &'static [Row] = &[$(Row::$row),*];
        }
    };
}

rows! {
    /// `Create branch here…`: a new branch on the row's commit — the current
    /// branch's and a detached HEAD's too (デザイン規約 §ブランチ・コミットへの移動).
    BranchHere,
    /// `switch`, or `Open` and the worktree that has the branch out — or,
    /// on another worktree's uncommitted row, that worktree.
    Reach,
    /// `cherry-pick`: the commit copied onto HEAD. It only adds a commit, so
    /// it runs detached or not, on any commit.
    CherryPick,
    /// `revert`: the commit undone by a commit on HEAD, as
    /// [`Self::CherryPick`] adds one.
    Revert,
    /// Another worktree's uncommitted changes brought into this one
    /// (`Apply changes to this worktree`, デザイン規約 §別の worktree を読む).
    ApplyChanges,
    /// A stash's `apply`: its changes brought into the worktree, the entry
    /// kept.
    ApplyStash,
    /// A stash's `pop`: …and the entry dropped once they are in.
    PopStash,
    /// `squash` the commit into its parent.
    Squash,
    /// `drop` the commit from the branch's history.
    Drop,
    /// `rebase --interactive` from the commit up, opened as a plan: nothing
    /// runs until the plan's own button.
    RebaseInteractive,
    /// `merge` into the branch the tab moves.
    Merge,
    /// `rebase` the branch the tab moves onto it.
    Rebase,
    /// `reset --soft`: the branch the tab moves taken to the commit,
    /// everything kept staged.
    ResetSoft,
    /// `reset --mixed`: …everything kept unstaged.
    ResetMixed,
    /// `reset --hard`: …everything after it discarded.
    ResetHard,
    /// `pull`: the tab's branch from its upstream, or the row's own branch
    /// fast-forwarded to its upstream (デザイン規約 §取り込んで合流させる). A row
    /// on a branch's menus and never a toolbar button (user decision — ask
    /// before changing): the toolbar's fixed targets speak for the whole
    /// repository, while a pull moves one branch, whose row is the target.
    Pull,
    /// `push` of a branch the tab is not on.
    Push,
    /// `branch --delete` / `branch -D`.
    DeleteBranch,
    /// `push --delete`: the row's own remote branch, or the one a local
    /// branch reads, deleted without touching the local one.
    DeleteRemoteBranch,
    /// A local branch and the remote branch it reads, together.
    DeleteBoth,
    /// A stash's `drop`.
    DropStash,
    /// `Set upstream…`: which remote branch the local one is measured
    /// against. It writes configuration, not the branch: the branch the tab
    /// is on and one another worktree has out take it too.
    SetUpstream,
    /// `Unset upstream`: the upstream taken off, leaving the branch measured
    /// against nothing. Configuration as [`Self::SetUpstream`] is, and it
    /// needs no remote — an upstream may be a branch here.
    UnsetUpstream,
    /// `Create worktree here…`: a new branch on the row's commit, out in a
    /// new worktree. Nothing in this worktree is touched, so it is the one
    /// way to start on another branch while an operation is stopped here.
    WorktreeHere,
    /// `worktree add`: the row's own branch out in a new worktree — a remote
    /// branch's by a local one of its name made to follow it, as `switch`
    /// makes one ([`super::Subject::worktree_checkout`]). Both worktree rows
    /// give the new worktree a branch: no row makes a detached one
    /// (デザイン規約 §worktree を作る).
    WorktreeAdd,
    /// `worktree remove`.
    RemoveWorktree,
    /// `Create tag here…`.
    TagHere,
    /// A tag's `push` to the remote this repository pushes to. The working
    /// tree has no bearing, and drift does not grey it: a remote holding the
    /// name elsewhere turns the row into the leased overwrite
    /// (デザイン規約 §相手の履歴を置き換える).
    PushTag,
    /// `tag --delete`.
    DeleteTag,
    /// A tag's `push --delete`, leased to the commit the remote was last
    /// read to hold it on ([`crate::remote::delete_remote_tag`]) — so only
    /// where a remote was last heard to carry the name.
    DeleteRemoteTag,
    /// A tag here and over there, together.
    DeleteTagBoth,
    /// `Keep <ours>'s version`: the chosen conflicted files settled on the
    /// side HEAD brought (`git restore --ours`, then `git add`), named by
    /// branch (デザイン規約 §conflict の ours / theirs).
    KeepOurs,
    /// `Take <theirs>'s version`: …on the incoming side.
    TakeTheirs,
    /// `Edit in <tool>`: the chosen conflicted files handed to the
    /// configured merge tool by name; unset, the row is the door to the
    /// setting (デザイン規約 §conflict を外部ツールへ渡す).
    MergeTool,
    /// `stash` of the chosen files (`git stash push -- <paths>`), named out
    /// of the commit box as the toolbar's is (デザイン規約 §変更を退避する).
    StashFiles,
    /// `Discard`: the chosen changes undone back to HEAD — held, and copied
    /// to the discard record where it can (デザイン規約 §回数押し).
    Discard,
    /// A plan step's `pick`: kept as it is.
    StepPick,
    /// `reword`: kept, its message changed.
    StepReword,
    /// `edit`: kept, the run stopping there to amend it.
    StepEdit,
    /// `squash`: folded into the nearest kept step below, both messages
    /// kept.
    StepSquash,
    /// `fixup`: folded there, its own message dropped.
    StepFixup,
    /// `drop`: left out of the history.
    StepDrop,
    /// `Where <remote> is…`: the form that holds a remote's URL, the one
    /// `Add remote…` opens with the name filled in
    /// (デザイン規約 §リモートを書き留める).
    RemoteUrl,
    /// `Mark as origin`: `remote.pushDefault` and `checkout.defaultRemote`
    /// set to the remote together.
    MarkOrigin,
}

/// The kinds of element a subject is made of. Every row belongs to the one
/// it acts on ([`Row::part`]), and an element carries every row of its
/// kind: which rows a menu has is which parts its subject has
/// ([`super::Class`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    /// A commit the row stands on — or will, once its branch has one.
    Position,
    /// Somewhere this tab is not: a branch it is not on, a worktree it does
    /// not show.
    Elsewhere,
    /// A graph row's commit, whose change can be copied onto HEAD.
    Copied,
    /// A graph row's commit, whose change can be undone on HEAD — not in the
    /// discard log's display mode, where it would undo what the branch never
    /// had (`Seen::Graph::provisional`).
    Undone,
    /// Something to bring into the branch this tab moves.
    IntoHere,
    /// A graph row's commit, in the history the branch this tab moves is
    /// rewritten over — HEAD's commit included, the one most often meant.
    Rewritten,
    /// A graph row's commit, where the branch this tab moves can be taken.
    MovedTo,
    /// Another worktree's uncommitted changes.
    Carried,
    /// A branch moved along its upstream: the row's own, or the tab's from
    /// the remote branch it tracks. Only where the row names the one local
    /// branch that moves (user decision — ask before changing): the tab's
    /// branch on either end of the comparison the screen already shows as
    /// ↑↓, another local branch on its own row. Not on another remote
    /// branch's row (user decision — ask before changing): `git pull` moves
    /// the branch it runs on, so there it would move one the row does not
    /// name — bringing that remote branch into the current one is merge /
    /// rebase. Not on the upstream row of another local branch either, for
    /// the same reason, nor on tags and stashes.
    Pulled,
    /// A local branch the tab is not on, sent from its own row; the tab's
    /// own goes out through the toolbar, where its counts are
    /// (デザイン規約 §リモートへ送る).
    Sent,
    LocalBranch,
    /// A remote branch: the row's own, or the one a local branch reads.
    RemoteBranch,
    /// A local branch together with the remote branch it reads.
    BranchAndReading,
    /// A local branch's configured upstream.
    Upstream,
    /// A branch to take out in a new worktree.
    Checkout,
    /// A worktree other than the repository's own, which git never removes
    /// (`is a main working tree`).
    OtherWorktree,
    /// A tag this repository holds: a tag only a remote has is neither sent
    /// nor left to `tag --delete`.
    TagHere,
    /// A tag a remote was last heard to carry.
    TagFar,
    TagBoth,
    Stash,
    /// A conflicted file of the working tree pane's choice, settled on one
    /// side or in the merge tool.
    Conflicted,
    /// The changes of the files chosen there, set aside as they stand.
    Changes,
    /// A change git restores back to HEAD: any but a conflicted file's,
    /// whose own rows settle it — git restores no unmerged path.
    Restorable,
    /// A step of a rebase plan.
    Step,
    /// A step with another below it, which a fold lands in: the oldest has
    /// none.
    Fold,
    /// A remote of the repository, as its configuration names it.
    RemoteConfig,
}

/// The first level's groups, in drawing order — widening reach down the
/// menu, what is used rarely behind the cards (デザイン規約 §メニュー). A
/// divider stands between two groups that both have rows; a card's title
/// row stands in the group [`Card::seat`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    /// Where the reader goes next: a branch made here, or a move — one
    /// group at the head, since `Create branch here…` runs `switch
    /// --create` (user decision — ask before changing).
    Start,
    /// What brings a change here: a commit's copied or undone, another
    /// worktree's or a stash's applied.
    Copy,
    /// What rewrites the history of the branch the tab moves.
    Rewrite,
    /// What moves the branch the tab moves: a change brought into it, or
    /// the branch taken to the commit (the RESET card).
    IntoHere,
    /// What moves the named branch along its upstream: `pull` and the push,
    /// one group apart from [`Group::IntoHere`] (user decision — ask before
    /// changing) — merge / rebase bring the row into the branch the tab
    /// moves, while a pull moves the named branch itself.
    Sync,
    /// What takes the row's name away: the last group of the rows used
    /// often, under `pull` / `push` and above the cards (user decision —
    /// ask before changing).
    Delete,
    /// The cards, in this order (user decision — ask before changing).
    BranchCard,
    WorktreeCard,
    TagCard,
    /// What settles a conflicted file.
    Resolve,
    /// What takes the chosen changes out of the worktree: set aside, or
    /// undone.
    Clear,
    /// What keeps a plan's step in the history.
    Keep,
    /// What folds it into the step below.
    Fold,
    /// What leaves it out.
    Leave,
    /// Where a remote is and whether it is the origin, one group (user
    /// decision — ask before changing).
    Configure,
}

/// The cards the rows used rarely hang behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Card {
    /// `reset` and where it takes the branch, the command named once on the
    /// title (`reset main here`).
    Reset,
    /// The upstream set and taken off. The push and the delete table, used
    /// often, stand on the first level (user decision — ask before
    /// changing).
    Branch,
    /// A worktree made and taken away: its making rows live here (user
    /// decision — ask before changing).
    Worktree,
    /// A tag made, sent and deleted: `Create tag here…` lives here while
    /// `Create branch here…` stands on the first level — a tag is a mark
    /// left behind, a branch a place the reader moves on from (user
    /// decision — ask before changing).
    Tag,
}

/// A card's groups, in drawing order: a divider between two that both have
/// rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardGroup {
    Make,
    /// Where the branch is taken.
    Move,
    Send,
    Upstream,
    Delete,
}

/// Where a row stands: a group of the first level, or a group of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Place {
    Top(Group),
    In(Card, CardGroup),
}

impl Card {
    /// The first-level group the card's title row stands in: a group of its
    /// own, but the RESET card's, which moves the branch the tab moves as
    /// `merge` and `rebase` above it do.
    pub const fn seat(self) -> Group {
        match self {
            Self::Reset => Group::IntoHere,
            Self::Branch => Group::BranchCard,
            Self::Worktree => Group::WorktreeCard,
            Self::Tag => Group::TagCard,
        }
    }
}

impl Place {
    /// The first-level group, and the card's group inside it: what drawing
    /// order sorts by.
    pub const fn order(self) -> (Group, Option<CardGroup>) {
        match self {
            Self::Top(group) => (group, None),
            Self::In(card, group) => (card.seat(), Some(group)),
        }
    }

    /// The card the row hangs behind; `None` on the first level.
    pub const fn card(self) -> Option<Card> {
        match self {
            Self::Top(_) => None,
            Self::In(card, _) => Some(card),
        }
    }
}

impl Group {
    /// The word the menu's record carries it in: what a drawn menu parts
    /// its first level by.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Copy => "copy",
            Self::Rewrite => "rewrite",
            Self::IntoHere => "into-here",
            Self::Sync => "sync",
            Self::Delete => "delete",
            Self::BranchCard => "branch-card",
            Self::WorktreeCard => "worktree-card",
            Self::TagCard => "tag-card",
            Self::Resolve => "resolve",
            Self::Clear => "clear",
            Self::Keep => "keep",
            Self::Fold => "fold",
            Self::Leave => "leave",
            Self::Configure => "configure",
        }
    }
}

impl Card {
    /// The word the menu's record carries it in, which the QML card of the
    /// kind names itself by.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Reset => "reset",
            Self::Branch => "branch",
            Self::Worktree => "worktree",
            Self::Tag => "tag",
        }
    }
}

impl CardGroup {
    /// The word the menu's record carries it in: what a drawn card parts its
    /// rows by.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Make => "make",
            Self::Move => "move",
            Self::Send => "send",
            Self::Upstream => "upstream",
            Self::Delete => "delete",
        }
    }
}

impl Row {
    /// The word the menu's record carries it in, which the QML row of the
    /// kind looks itself up by.
    pub const fn word(self) -> &'static str {
        match self {
            Self::BranchHere => "branch-here",
            Self::Reach => "reach",
            Self::CherryPick => "cherry-pick",
            Self::Revert => "revert",
            Self::ApplyChanges => "apply-changes",
            Self::ApplyStash => "apply-stash",
            Self::PopStash => "pop-stash",
            Self::Squash => "squash",
            Self::Drop => "drop",
            Self::RebaseInteractive => "rebase-interactive",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::ResetSoft => "reset-soft",
            Self::ResetMixed => "reset-mixed",
            Self::ResetHard => "reset-hard",
            Self::Pull => "pull",
            Self::Push => "push",
            Self::DeleteBranch => "delete-branch",
            Self::DeleteRemoteBranch => "delete-remote-branch",
            Self::DeleteBoth => "delete-both",
            Self::DropStash => "drop-stash",
            Self::SetUpstream => "set-upstream",
            Self::UnsetUpstream => "unset-upstream",
            Self::WorktreeHere => "worktree-here",
            Self::WorktreeAdd => "worktree-add",
            Self::RemoveWorktree => "remove-worktree",
            Self::TagHere => "tag-here",
            Self::PushTag => "push-tag",
            Self::DeleteTag => "delete-tag",
            Self::DeleteRemoteTag => "delete-remote-tag",
            Self::DeleteTagBoth => "delete-tag-both",
            Self::KeepOurs => "keep-ours",
            Self::TakeTheirs => "take-theirs",
            Self::MergeTool => "merge-tool",
            Self::StashFiles => "stash-files",
            Self::Discard => "discard",
            Self::StepPick => "step-pick",
            Self::StepReword => "step-reword",
            Self::StepEdit => "step-edit",
            Self::StepSquash => "step-squash",
            Self::StepFixup => "step-fixup",
            Self::StepDrop => "step-drop",
            Self::RemoteUrl => "remote-url",
            Self::MarkOrigin => "mark-origin",
        }
    }

    /// It acts on the worktree the row's WORKTREE card stands on, whatever
    /// branch that worktree has out: what still applies on a worktree's row
    /// once the worktree has another branch out than the menu was raised on
    /// (`RefRowMenu.standOn`).
    pub const fn on_worktree(self) -> bool {
        matches!(self.part(), Part::OtherWorktree)
    }

    /// The part the row belongs to.
    pub const fn part(self) -> Part {
        match self {
            Self::BranchHere | Self::WorktreeHere | Self::TagHere => Part::Position,
            Self::Reach => Part::Elsewhere,
            Self::CherryPick => Part::Copied,
            Self::Revert => Part::Undone,
            Self::ApplyChanges => Part::Carried,
            Self::Squash | Self::Drop | Self::RebaseInteractive => Part::Rewritten,
            Self::Merge | Self::Rebase => Part::IntoHere,
            Self::ResetSoft | Self::ResetMixed | Self::ResetHard => Part::MovedTo,
            Self::Pull => Part::Pulled,
            Self::Push => Part::Sent,
            Self::DeleteBranch | Self::SetUpstream => Part::LocalBranch,
            Self::DeleteRemoteBranch => Part::RemoteBranch,
            Self::DeleteBoth => Part::BranchAndReading,
            Self::UnsetUpstream => Part::Upstream,
            Self::WorktreeAdd => Part::Checkout,
            Self::RemoveWorktree => Part::OtherWorktree,
            Self::PushTag | Self::DeleteTag => Part::TagHere,
            Self::DeleteRemoteTag => Part::TagFar,
            Self::DeleteTagBoth => Part::TagBoth,
            Self::ApplyStash | Self::PopStash | Self::DropStash => Part::Stash,
            Self::KeepOurs | Self::TakeTheirs | Self::MergeTool => Part::Conflicted,
            Self::StashFiles => Part::Changes,
            Self::Discard => Part::Restorable,
            Self::StepPick | Self::StepReword | Self::StepEdit | Self::StepDrop => Part::Step,
            Self::StepSquash | Self::StepFixup => Part::Fold,
            Self::RemoteUrl | Self::MarkOrigin => Part::RemoteConfig,
        }
    }

    /// Where the row stands.
    pub const fn place(self) -> Place {
        match self {
            Self::BranchHere | Self::Reach => Place::Top(Group::Start),
            Self::CherryPick
            | Self::Revert
            | Self::ApplyChanges
            | Self::ApplyStash
            | Self::PopStash => Place::Top(Group::Copy),
            Self::Squash | Self::Drop | Self::RebaseInteractive => Place::Top(Group::Rewrite),
            Self::Merge | Self::Rebase => Place::Top(Group::IntoHere),
            Self::ResetSoft | Self::ResetMixed | Self::ResetHard => {
                Place::In(Card::Reset, CardGroup::Move)
            }
            Self::Pull | Self::Push => Place::Top(Group::Sync),
            Self::DeleteBranch | Self::DeleteRemoteBranch | Self::DeleteBoth | Self::DropStash => {
                Place::Top(Group::Delete)
            }
            Self::SetUpstream | Self::UnsetUpstream => Place::In(Card::Branch, CardGroup::Upstream),
            Self::WorktreeHere | Self::WorktreeAdd => Place::In(Card::Worktree, CardGroup::Make),
            Self::RemoveWorktree => Place::In(Card::Worktree, CardGroup::Delete),
            Self::TagHere => Place::In(Card::Tag, CardGroup::Make),
            Self::PushTag => Place::In(Card::Tag, CardGroup::Send),
            Self::DeleteTag | Self::DeleteRemoteTag | Self::DeleteTagBoth => {
                Place::In(Card::Tag, CardGroup::Delete)
            }
            Self::KeepOurs | Self::TakeTheirs | Self::MergeTool => Place::Top(Group::Resolve),
            Self::StashFiles | Self::Discard => Place::Top(Group::Clear),
            Self::StepPick | Self::StepReword | Self::StepEdit => Place::Top(Group::Keep),
            Self::StepSquash | Self::StepFixup => Place::Top(Group::Fold),
            Self::StepDrop => Place::Top(Group::Leave),
            Self::RemoteUrl | Self::MarkOrigin => Place::Top(Group::Configure),
        }
    }
}
