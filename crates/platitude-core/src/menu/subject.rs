//! The subject a ref's or a graph row's menu is handed (`GitFacts.menuOn`):
//! the row it opens on, parted into what is there and its state.

use super::class::{Seen, Shape, UpstreamShape, WorktreeShape};
use super::line::{Facts, History, Placing, State};
use crate::Oid;
use crate::offers::{
    FarSide, Leads, MovedBranch, RefKind, TagDeleteHeld, Tracks, TreeStanding, WorktreeCheckout,
};
use crate::remote::PushStanding;

/// What a menu is handed about the row it opens on: which row it is, the
/// tree, where the row leads, what its card reads over there, where a new
/// worktree would go, and what a push of its branch can do.
#[derive(Debug, Clone)]
pub struct Subject<'a> {
    pub seen: Seen,
    /// The ref the row names; `None` on a graph row that names none.
    pub kind: Option<RefKind>,
    pub name: &'a str,
    /// The commit the row points at; `None` on a branch with no commits
    /// yet.
    pub commit: Option<Oid>,
    /// How the branch HEAD is on has that commit — whether HEAD's commit
    /// reaches it, and down a straight line or past a merge —, off the rows
    /// the graph has drawn (`GraphModel.historyOf`), whichever row asks;
    /// `None` where the window cannot say, which git answers at the press.
    pub history: Option<History>,
    pub open: bool,
    pub tree: &'a TreeStanding,
    pub leads: Leads,
    /// What the row's card reads over there.
    pub far: FarSide,
    /// A remote row's local namesake is there.
    pub local_listed: bool,
    pub worktree: Option<RowWorktree>,
    pub placing: Placing,
    pub push: PushStanding,
}

impl Subject<'_> {
    /// Whether `switch` asks first: an operation or unmerged files to clear —
    /// the `!` in the row's mark seat —, read off the answer the press itself
    /// reads ([`TreeStanding::moves_blocked`]). A row leading to another
    /// worktree asks nothing and wears no mark: the press opens that worktree
    /// (デザイン規約 §進行中の操作から出る).
    pub fn switch_asks(&self) -> bool {
        self.leads == Leads::Nowhere && self.tree.moves_blocked
    }

    /// How `worktree add` takes the row's branch out: a local branch as it
    /// is, a remote one by a local one of its name made to follow it, as
    /// `switch` makes one. `None` on a remote row whose local branch is there
    /// or out — that branch's own row takes it out, and the row greys for it
    /// ([`State::namesake_there`]) —, on a row with no branch, and in a tab
    /// not open yet.
    pub fn worktree_checkout(&self) -> Option<WorktreeCheckout> {
        match self.kind {
            Some(RefKind::Branch) => Some(WorktreeCheckout::Branch),
            Some(RefKind::Remote) if !Facts::of_subject(self).state.namesake_there() => {
                Some(WorktreeCheckout::Track)
            }
            _ => None,
        }
        .filter(|_| self.open)
    }
}

/// What the worktree a menu was raised on has out now, as the worktree
/// listing reads it ([`worktree_left`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeOut<'a> {
    /// Gone from the listing.
    Gone,
    /// On no branch.
    Detached,
    /// On the branch of this name, with commits or none yet.
    Branch(&'a str),
}

/// Whether the worktree a WORKTREES row raised a menu on no longer has the
/// branch the menu stands on (`branch`) out: gone from the listing, on no
/// branch, or on a branch of another name. The menu was raised on the
/// worktree, so its branch's rows no longer act on what was clicked, and
/// only the rows on the worktree itself stay ([`super::Row::on_worktree`];
/// `RefRowMenu.standOn`).
///
/// A rename outside reaches the two listings one at a time, and a name read
/// on one alone is a rename half read or a switch to a new branch — nothing
/// tells them apart until the other lands. Either leaves the branch: the
/// rows grey until both listings name the menu's branch again (the menu
/// follows a rename on the branches' — `RefRowMenu.standAgain`), so a
/// press meant for the branch the menu went up on never lands on another.
pub fn worktree_left(branch: &str, out: WorktreeOut<'_>) -> bool {
    !matches!(out, WorktreeOut::Branch(name) if name == branch)
}

/// The worktree a row's WORKTREE card stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowWorktree {
    /// The repository's own.
    pub main: bool,
    /// The tab shows it.
    pub here: bool,
    pub locked: bool,
}

impl<'a> Facts<'a> {
    /// The facts of `row`: what is there off the row, the tree and what the
    /// far side's listing holds; everything else its state.
    pub fn of_subject(row: &Subject<'a>) -> Self {
        let far = row
            .kind
            .map_or(FarSide::Unread, |kind| row.far.clone().on(kind));
        let (branch, tag) = match &far {
            FarSide::Branch(branch) => (Some(*branch), None),
            FarSide::Tag(tag) => (None, Some(tag)),
            FarSide::Unread => (None, None),
        };
        let tree = row.tree;
        // The branch HEAD is on has its upstream as the tree reads it — the
        // far end of the window's ↑↓, which the remote branch's row reads
        // too (`Shape::compared`); another branch, as the listing says.
        // Where that upstream stands is the listing's where it has a row
        // for it. Where it has none — not read yet, or fetched outside the
        // remote branches, which it never holds —, git's own count on the
        // tree says whether the ref is there; a branch with no commit yet
        // is counted against nothing, and has no row of the listing either.
        let heads_own = row.kind == Some(RefKind::Branch)
            && matches!(tree.moved(), Some(MovedBranch::Head(name)) if name == row.name);
        let upstream = branch.and_then(|branch| {
            let listed = UpstreamShape::listed(branch.tracks);
            if !heads_own {
                return listed;
            }
            if tree.upstream.is_empty() {
                return None;
            }
            match listed {
                Some(UpstreamShape::Read | UpstreamShape::Here) => listed,
                _ if tree.upstream_counted || tree.head_unborn() => Some(UpstreamShape::Counted),
                _ => Some(UpstreamShape::Gone),
            }
        });
        Self {
            shape: Shape {
                seen: row.seen,
                kind: row.kind,
                name: row.name,
                moved: tree.moved(),
                tab_upstream: &tree.upstream,
                upstream,
                tag_sides: tag.and_then(|tag| tag.sides),
                worktree: row.worktree.map(|worktree| WorktreeShape {
                    main: worktree.main,
                    here: worktree.here,
                }),
            },
            state: State {
                commit: row.commit,
                history: row.history,
                tree,
                held: row.leads == Leads::Holder,
                local_listed: row.local_listed,
                drifted: branch.is_some_and(|branch| branch.tracks == Tracks::Drifted),
                ahead: branch.map_or(0, |branch| branch.ahead),
                has_remote: branch.is_some_and(|branch| branch.has_remote),
                tag_remote: tag.is_some_and(|tag| tag.has_remote()),
                tag_held: tag.map_or(TagDeleteHeld::No, |tag| tag.held_back),
                push: row.push,
                locked: row.worktree.is_some_and(|worktree| worktree.locked),
                placing: row.placing,
            },
        }
    }
}
