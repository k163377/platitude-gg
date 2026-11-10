//! Where the worktree stands, as every rule here reads it.

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
    /// (`WorktreeModel.upstream`).
    pub upstream: String,
    /// The branch and its upstream have both moved
    /// (`WorktreeModel.pullBlocked`).
    pub diverged: bool,
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

    /// Whether `name` is the branch HEAD is on. While HEAD is on none, no
    /// name is.
    pub fn is_current(&self, name: &str) -> bool {
        !self.branch.is_empty() && name == self.branch
    }

    /// Whether `oid_hex` is the commit HEAD is on. Before the first commit,
    /// none is.
    pub fn is_head(&self, oid_hex: &str) -> bool {
        !self.head_oid.is_empty() && oid_hex == self.head_oid
    }
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
    fn the_badge_is_the_operation() {
        assert!(!TreeStanding::on("main").op_standing());
        assert!(TreeStanding::on("main").standing("MERGING").op_standing());
        assert!(
            TreeStanding::detached().standing("BISECTING").op_standing(),
            "bisect counts"
        );
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
