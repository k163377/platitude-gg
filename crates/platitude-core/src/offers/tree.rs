//! Where the worktree stands, as every rule here reads it.

use crate::worktrees::{Keeper, KeptBranch};

/// HEAD, the operation standing on it, and what its branch is measured
/// against: what the menus, the message boxes and a press on a ref ask
/// about the tree. One value off `WorktreeModel` (`standing`, or the
/// menus' `reading`), handed on whole, so no caller lines its parts up by
/// position.
///
/// Whether the tab is open is not the tree's: each rule that asks it takes
/// it as an argument of its own (`open`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeStanding {
    /// The branch HEAD is on; empty detached, and before HEAD is read.
    pub branch: String,
    pub detached: bool,
    /// The commit HEAD is on, full hex — detached too; empty before the
    /// first commit.
    pub head_oid: String,
    /// The operation badge, empty exactly when nothing stands (bisect
    /// counts).
    pub op_text: String,
    /// The standing rebase stopped on purpose at an `edit` step
    /// ([`crate::integrate::RebaseStop`]).
    pub op_editing: bool,
    /// A move has to clear the way first ([`super::moves_blocked`],
    /// `WorktreeModel.movesBlocked`): what a press on a ref raises the
    /// question on, read off the last status. Not [`Self::op_standing`]: the
    /// badge a replay's faster tick puts up can stand ahead of that status.
    pub moves_blocked: bool,
    /// What the branch tracks, empty where it tracks nothing
    /// (`WorktreeModel.upstream`): the far end of the comparison the window
    /// shows as ↑↓, which only a branch HEAD is on has.
    pub upstream: String,
    /// git counted the branch against `upstream` in the last status (its
    /// `branch.ab`): the upstream's ref is there to count against. False
    /// where the branch tracks nothing, where the ref is gone (git's
    /// `[gone]`), and while the branch has no commit to count with.
    pub upstream_counted: bool,
    /// The branch and its upstream have both moved
    /// (`WorktreeModel.pullBlocked`).
    pub diverged: bool,
    /// While HEAD is detached, the branch a stopped rebase or bisect keeps
    /// to the tree, and which of them keeps it — read with HEAD
    /// (`refs::HeadState::kept`), ahead of the badge's status; `None`
    /// otherwise.
    pub kept: Option<KeptBranch>,
}

/// The branch the tab moves ([`TreeStanding::moved`]), and how the tab holds
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovedBranch<'a> {
    /// HEAD is on it.
    Head(&'a str),
    /// A stopped rebase or bisect keeps it while HEAD is on none: git moves
    /// it once the operation is finished, and counts it as out here until
    /// then.
    Kept { name: &'a str, by: Keeper },
}

impl<'a> MovedBranch<'a> {
    /// The branch, however it is held.
    pub fn name(self) -> &'a str {
        match self {
            Self::Head(name) | Self::Kept { name, .. } => name,
        }
    }
}

impl TreeStanding {
    /// HEAD is on a branch — one a merge, a rebase or a reset can move.
    pub fn on_branch(&self) -> bool {
        !self.detached && !self.branch.is_empty()
    }

    /// An operation stands: one at a time, so whatever would start or
    /// rewrite history waits until it is finished or put down.
    pub fn op_standing(&self) -> bool {
        !self.op_text.is_empty()
    }

    /// HEAD is on a branch with no commits yet: there is nothing on it to
    /// undo a commit on, to rebase or to rewrite — `git revert` exits 1 and
    /// `git rebase` 128 (`Could not resolve HEAD to a commit`), while
    /// `cherry-pick`, `merge` and `reset` take the branch to a commit
    /// (measured on 2.55) — `merge` as a fast-forward, which
    /// `merge.ff=false` turns down into an empty head.
    pub fn head_unborn(&self) -> bool {
        self.on_branch() && self.head_oid.is_empty()
    }

    /// The branch `merge`, `rebase`, the rewrites and `reset` move: the one
    /// HEAD is on, or — while a stopped rebase or bisect keeps HEAD detached
    /// — the one it keeps, which they move once it is finished. `None` while
    /// the tab moves no branch.
    pub fn moved(&self) -> Option<MovedBranch<'_>> {
        if self.on_branch() {
            return Some(MovedBranch::Head(&self.branch));
        }
        self.kept.as_ref().map(|kept| MovedBranch::Kept {
            name: &kept.branch,
            by: kept.by,
        })
    }

    /// Whether `name` is the branch HEAD is on ([`is_current_branch`]).
    pub fn is_current(&self, name: &str) -> bool {
        is_current_branch(&self.branch, name)
    }

    /// Whether `oid_hex` is the commit HEAD is on. Before the first commit,
    /// none is.
    pub fn is_head(&self, oid_hex: &str) -> bool {
        !self.head_oid.is_empty() && oid_hex == self.head_oid
    }
}

/// Whether `name` is `head`, the branch HEAD is on — empty while HEAD is on
/// none, when no name is. The one rule for a reader that holds HEAD's
/// branch alone (the sidebar's rows) and for [`TreeStanding::is_current`].
pub fn is_current_branch(head: &str, name: &str) -> bool {
    !head.is_empty() && name == head
}

/// The trees the cases bend one field of at a time.
#[cfg(test)]
impl TreeStanding {
    /// HEAD on `branch` at `abc123`, nothing standing, tracking nothing.
    pub(crate) fn on(branch: &str) -> Self {
        Self {
            branch: branch.into(),
            head_oid: "abc123".into(),
            ..Self::default()
        }
    }

    /// HEAD on no branch, at `abc123`.
    pub(crate) fn detached() -> Self {
        Self {
            detached: true,
            ..Self::on("")
        }
    }

    /// The same tree with `op_text` standing, holding moves back as an
    /// operation does.
    pub(crate) fn standing(self, op_text: &str) -> Self {
        Self {
            op_text: op_text.into(),
            moves_blocked: true,
            ..self
        }
    }

    /// The same tree with `branch` kept by a rebase or bisect stopped in it,
    /// as HEAD's reading names it — the badge read apart.
    pub(crate) fn keeping(self, branch: &str, by: Keeper) -> Self {
        Self {
            kept: Some(KeptBranch {
                branch: branch.into(),
                by,
            }),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_branch_is_on_one_and_detached_is_not() {
        assert!(TreeStanding::on("main").on_branch());
        assert!(!TreeStanding::detached().on_branch());
        // An unborn branch has a name and no commit: still a branch.
        let unborn = TreeStanding {
            head_oid: String::new(),
            ..TreeStanding::on("main")
        };
        assert!(unborn.on_branch());
        assert!(!TreeStanding::default().on_branch(), "before HEAD is read");
    }

    #[test]
    fn only_a_branch_with_no_commits_yet_is_unborn() {
        let unborn = TreeStanding {
            head_oid: String::new(),
            ..TreeStanding::on("main")
        };
        assert!(unborn.head_unborn());
        assert!(!TreeStanding::on("main").head_unborn());
        assert!(!TreeStanding::detached().head_unborn());
        assert!(
            !TreeStanding::default().head_unborn(),
            "before HEAD is read"
        );
    }

    #[test]
    fn the_badge_is_the_operation() {
        assert!(!TreeStanding::on("main").op_standing());
        assert!(TreeStanding::on("main").standing("MERGING").op_standing());
        assert!(
            TreeStanding::detached().standing("BISECTING").op_standing(),
            "bisect counts"
        );
    }

    #[test]
    fn the_branch_moved_is_heads_or_the_one_a_stopped_operation_keeps() {
        assert_eq!(
            TreeStanding::on("main").moved(),
            Some(MovedBranch::Head("main"))
        );
        let kept = TreeStanding::detached().keeping("main", Keeper::Bisect);
        assert_eq!(
            kept.moved(),
            Some(MovedBranch::Kept {
                name: "main",
                by: Keeper::Bisect
            })
        );
        assert_eq!(kept.moved().map(MovedBranch::name), Some("main"));
        assert_eq!(TreeStanding::detached().moved(), None);
        assert_eq!(TreeStanding::default().moved(), None, "before HEAD is read");
    }

    #[test]
    fn only_the_branch_head_is_on_is_current() {
        let tree = TreeStanding::on("main");
        assert!(tree.is_current("main"));
        assert!(!tree.is_current("feat"));
        assert!(!tree.is_current(""));
        assert!(
            !TreeStanding::detached().is_current(""),
            "detached, no name is current — not even none"
        );
    }

    #[test]
    fn only_the_commit_head_is_on_is_head() {
        let tree = TreeStanding::on("main");
        assert!(tree.is_head("abc123"));
        assert!(!tree.is_head("def456"));
        assert!(TreeStanding::detached().is_head("abc123"), "detached too");
        let unborn = TreeStanding {
            head_oid: String::new(),
            ..TreeStanding::on("main")
        };
        assert!(!unborn.is_head(""), "before the first commit, none is");
    }
}
