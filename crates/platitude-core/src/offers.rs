//! Pure rules for what the standing repository state lets the UI offer:
//! where a press on a ref lands and what leaving a stopped operation costs
//! ([`moves`]), and what the details pane's message boxes do. Every
//! refusal encoded here is git's own (the doc on each item). A menu is the
//! type's ([`crate::menu`]), which reads the words here: the row's kind and
//! lead, the tree ([`TreeStanding`]) and what is over there ([`FarSide`]).
//!
//! Nothing here runs git. A git command still running is no input: the
//! menus stand on the tree as it was asked (`RepoPage.treeAtAsk`).
//!
//! **These are the rules, and the app holds no copy.** It asks by value
//! through `GitFacts` (`switchMove` for `RepoPage.switchToRef`,
//! `messageEdit` for the boxes), and what follows from the standing state
//! arrives as `WorktreeModel` properties derived at drain (`movesBlocked` /
//! `leaveUndoes` / `leaveCode` / `opSkipFree`). QML keeps only the lookups
//! that feed them and the `qsTr` wording.

mod far;
mod message;
mod moves;
mod pull;
mod tree;

pub use far::{BranchFar, FarSide, TagDeleteHeld, TagFar, TagSides, Tracks};
pub use message::{MessageEdit, message_edit};
pub use moves::{
    SwitchAction, SwitchTarget, leave_code, leaving_undoes, move_landed, moves_blocked,
    skip_is_free, switch_action,
};
pub use pull::PullRuns;
pub use tree::{MovedBranch, TreeStanding, is_current_branch};

/// The kind of ref a menu row stands on, in the words the chip records
/// carry (`branch` / `remote` / `tag` / `stash`), and `worktree` for a
/// worktree on no branch (its WORKTREES row, or its chip on the graph,
/// named by its folder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Branch,
    Remote,
    Tag,
    Stash,
    /// Another worktree, named by its path. What the row leads to is
    /// that worktree ([`Leads::Itself`]) while it is not the one the tab
    /// stands in.
    Worktree,
}

impl RefKind {
    /// The kind a menu names itself by; `None` for anything else — the
    /// HEAD marker, or a row still loading.
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "branch" => Some(Self::Branch),
            "remote" => Some(Self::Remote),
            "tag" => Some(Self::Tag),
            "stash" => Some(Self::Stash),
            "worktree" => Some(Self::Worktree),
            _ => None,
        }
    }
}

/// Where a row leads besides the tree the tab stands in: the other
/// worktree a press on it opens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Leads {
    /// Nowhere else: no other worktree has the row's branch out, the tab
    /// stands in the row's worktree, or the row names neither.
    #[default]
    Nowhere,
    /// Another worktree has the row's branch out — a remote row's through
    /// the local branch of its name. git refuses switching to it and
    /// deleting it there (`is already used by worktree at`).
    Holder,
    /// The row is a worktree on no branch, and not the one the tab stands
    /// in.
    Itself,
}

impl Leads {
    /// The word the worktrees' record carries it in (`NavSectionModel`'s
    /// `leadOf`); empty for [`Self::Nowhere`].
    pub fn word(self) -> &'static str {
        match self {
            Self::Nowhere => "",
            Self::Holder => "holder",
            Self::Itself => "itself",
        }
    }

    /// The lead [`Self::word`] wrote; `None` for any other word.
    pub fn from_word(word: &str) -> Option<Self> {
        [Self::Nowhere, Self::Holder, Self::Itself]
            .into_iter()
            .find(|leads| leads.word() == word)
    }
}

/// Where the upstream a local branch is configured with stands, which is
/// the colour its name wears on the BRANCH card (デザイン規約 §ref の種別
/// 「名前の色 = 在処」).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UpstreamPlace {
    /// No upstream.
    #[default]
    None,
    /// A remote branch, listed or gone.
    Remote,
    /// A branch here other than the one HEAD is on, listed or gone.
    Local,
    /// The branch HEAD is on.
    Current,
}

impl UpstreamPlace {
    /// The word the menu's record carries it in; empty for [`Self::None`].
    pub fn word(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Remote => "remote",
            Self::Local => "local",
            Self::Current => "current",
        }
    }
}

/// Where `upstream` stands: `here` is whether it is a branch of this
/// repository (`branch.<n>.remote = .`), listed or gone —
/// `session::BranchItem::upstream_here`; a name alone cannot say so.
pub fn upstream_place(upstream: &str, here: bool, tree: &TreeStanding) -> UpstreamPlace {
    if upstream.is_empty() {
        UpstreamPlace::None
    } else if !here {
        UpstreamPlace::Remote
    } else if tree.is_current(upstream) {
        UpstreamPlace::Current
    } else {
        UpstreamPlace::Local
    }
}

/// What the spaced second click does on a worktree with no branch out, whose
/// name is its folder (デザイン規約 §グラフ行のダブルクリック「箱のプレースホルダ」).
/// A worktree with a branch out renames the branch, and asks nothing here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeRename {
    /// The name box opens on the folder's name (`Worktree name`).
    Box,
    /// Nothing: git never moves the repository's own worktree, as it never
    /// removes it ([`crate::menu::Part::OtherWorktree`]).
    Never,
    /// No box; the reason stands where the box would have, as a refused
    /// name's does: a lock, which git keeps the folder for —
    Locked,
    /// — or a folder that is gone, of which git has nothing to move.
    Gone,
}

impl WorktreeRename {
    /// The answer as a word, the shape `GitFacts.worktreeRename` answers with:
    /// `box`, `locked`, `gone`, or empty for nothing.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Box => "box",
            Self::Never => "",
            Self::Locked => "locked",
            Self::Gone => "gone",
        }
    }
}

/// The rule. `main` is the repository's own worktree; a lock wins over a gone
/// folder, as on the row (the one the reader can undo).
#[must_use]
pub fn worktree_rename(main: bool, locked: bool, gone: bool) -> WorktreeRename {
    if main {
        WorktreeRename::Never
    } else if locked {
        WorktreeRename::Locked
    } else if gone {
        WorktreeRename::Gone
    } else {
        WorktreeRename::Box
    }
}

/// How the row's branch goes out in the new worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeCheckout {
    /// A local branch no worktree has out: checked out as it is.
    Branch,
    /// A remote branch with no local one of its name: a local one is made
    /// to follow it, as `switch` makes one.
    Track,
}

impl WorktreeCheckout {
    /// The word the menu's answer carries it in
    /// (`encode::MenuAnswer.worktree_add`).
    pub fn word(self) -> &'static str {
        match self {
            Self::Branch => "branch",
            Self::Track => "track",
        }
    }
}

#[cfg(test)]
mod tests;
