//! Pure rules for what the standing repository state lets the UI offer:
//! the ref and commit menus' rows, and — in [`moves`] — where a press on
//! a ref lands and what leaving a stopped operation costs. Every refusal
//! encoded here is git's own (the doc on each item).
//!
//! Nothing here runs git; the menus freeze the answers as they open
//! (rules/app-ui.md「メニューは `AppMenu` 系で書く」).

mod message;
mod moves;

pub use message::{MessageEdit, message_edit};
pub use moves::{
    SwitchAction, leave_code, leaving_undoes, move_landed, moves_blocked, skip_is_free,
    switch_action,
};

/// The kind of ref a menu row stands on, in the words the chip records
/// carry (`branch` / `remote` / `tag` / `stash`), and `worktree` for a
/// working copy on no branch (its WORKTREES row, or its folder's chip).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Branch,
    Remote,
    Tag,
    Stash,
    /// Another working copy, named by its path. What the row leads to is
    /// that copy: `held_by_worktree` in [`ref_menu`] is its own path while
    /// it is not the one the tab stands in.
    Worktree,
}

impl RefKind {
    /// The kind a menu names itself by; `None` for anything else — the
    /// HEAD marker, or a row still loading — which offers nothing.
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

/// Which sides of a tag exist, in the words the menu asks with.
///
/// The rows differ by side (`tag --delete` needs a local tag,
/// `push --delete` a remote one, sending a local one), and the sidebar lists
/// a name held on both sides once, so the row cannot tell on its own
/// (`NavSectionModel.tagSides`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagSides {
    /// Made here and no remote is known to carry the name.
    Here,
    /// A remote carries it and this repository does not.
    Remote,
    Both,
}

impl TagSides {
    /// Decoded from the word `GitFacts.refMenuOffers` is handed; `None`
    /// for every row that is not a tag, and for a tag whose section has
    /// not been read yet.
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "here" => Some(Self::Here),
            "remote" => Some(Self::Remote),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// Whether this repository holds the tag.
    fn here(self) -> bool {
        matches!(self, Self::Here | Self::Both)
    }

    /// Whether a remote was last heard to carry the name.
    fn on_remote(self) -> bool {
        matches!(self, Self::Remote | Self::Both)
    }
}

/// What the right-click menu on a ref may offer, decided as it opens and
/// frozen while it stands (`RefRowMenu.offerOn`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefMenuOffers {
    /// `switch` lands somewhere else: a branch (local or remote) other than
    /// the current one. A branch another working copy holds keeps this
    /// offer — the row leads to that copy ([`SwitchAction::OpenHolder`]).
    pub switch_to: bool,
    /// That move asks first: an operation or unmerged files to clear (the
    /// `!` in the row's mark seat). A branch another working copy holds
    /// asks nothing and wears no mark (デザイン規約 §進行中の操作から出る).
    pub switch_asks: bool,
    /// A new branch on this row's commit — any ref but a stash, the
    /// current branch and a detached HEAD included
    /// (デザイン規約 §グラフ行の右クリック).
    pub branch_here: bool,
    /// merge / rebase with this row as the far side: needs a current
    /// branch to move, and a row that is not it.
    pub integrate_from: bool,
    /// `git pull` — the branch the working tree is on, brought in line
    /// with its upstream.
    ///
    /// Offered on exactly the two ends of that comparison (the current
    /// branch and its upstream's remote-tracking ref): a pull moves the
    /// branch it is run on, so another branch's upstream would move a
    /// branch the row does not name (デザイン規約 §取り込んで合流させる).
    /// The upstream has to be there — git refuses a pull with no tracking
    /// information (デザイン規約 §メニュー「対象が存在しない」は席ごと消える).
    pub pull: bool,
    /// The everyday delete. git refuses the branch the tree is on or one
    /// any other working copy has checked out (`cannot delete branch …
    /// used by worktree at …`), locked or not.
    pub delete: bool,
    /// The branch's remote reading, deleted without touching the local
    /// one — still possible in both of the refused cases above, and out
    /// where the two have drifted (`remote_drifted` in [`ref_menu`]).
    pub delete_remote: bool,
    /// The tag sent to the remote this repository pushes to. Only a tag —
    /// a branch goes out through the toolbar, where its counts are
    /// (デザイン規約 §リモートへ送る). The working tree has no bearing; a
    /// tag only a remote has is not sent.
    pub push_tag: bool,
    /// The tag taken off the remote, leaving whatever is here. Only where a
    /// remote was last heard to carry the name: the delete is leased to the
    /// commit that reading shows ([`crate::remote::delete_remote_tag`]), so
    /// with no reading there is nothing to lease to.
    pub delete_remote_tag: bool,
    /// Both at once, for the name that stands on both sides.
    pub delete_tag_everywhere: bool,
    /// Which remote branch this local one is measured against. Local
    /// branches only, given a remote to ask about; the current branch and
    /// one another working copy holds take it too — this writes
    /// configuration, not the branch.
    pub set_upstream: bool,
    /// The row is the branch HEAD is on — what the delete rows' refusal
    /// names first.
    pub on_current_branch: bool,
}

impl RefMenuOffers {
    /// The offers as words, the shape `GitFacts.refMenuOffers` answers with.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words: Vec<&'static str> = Vec::new();
        if self.switch_to {
            words.push("switch");
        }
        if self.switch_asks {
            words.push("asks");
        }
        if self.branch_here {
            words.push("branch-here");
        }
        if self.integrate_from {
            words.push("integrate");
        }
        if self.pull {
            words.push("pull");
        }
        if self.delete {
            words.push("delete");
        }
        if self.delete_remote {
            words.push("delete-remote");
        }
        if self.push_tag {
            words.push("push-tag");
        }
        if self.delete_remote_tag {
            words.push("delete-remote-tag");
        }
        if self.delete_tag_everywhere {
            words.push("delete-tag-everywhere");
        }
        if self.set_upstream {
            words.push("set-upstream");
        }
        if self.on_current_branch {
            words.push("current");
        }
        words
    }
}

/// The offers themselves.
///
/// `full` is the row's display name as the chip wears it (`feat`,
/// `origin/feat`), not [`crate::refs::RefEntry::name`]'s spelling. `open`
/// is the tab-lifecycle gate (as [`crate::remote::push_standing`] holds
/// it); `op_text` is the operation badge, empty exactly when nothing is
/// standing (bisect counts); `held_by_worktree` is the path of the other
/// working copy holding this row's branch, empty when none does;
/// `remote_counterpart` is the remote reading a local branch carries;
/// `remote_drifted` says that reading stands on another commit
/// ([`crate::session::BranchItem::upstream_drifted`] /
/// [`crate::session::RefsSnapshot::tag_drifts`]); `default_remote` is
/// where pushes go, empty with no remote; `tag_sides` is
/// [`TagSides::from_word`]'s word (ignored for other kinds);
/// `current_upstream` is what the tree's branch tracks
/// (`WorkingTreeModel.upstream`), not this row's — a pull goes there
/// whichever end the menu opened on.
#[expect(clippy::too_many_arguments)]
pub fn ref_menu(
    kind: RefKind,
    full: &str,
    oid_hex: &str,
    open: bool,
    busy_count: i32,
    current_branch: &str,
    detached: bool,
    op_text: &str,
    conflict_count: i32,
    held_by_worktree: &str,
    remote_counterpart: &str,
    remote_drifted: bool,
    default_remote: &str,
    tag_sides: &str,
    current_upstream: &str,
) -> RefMenuOffers {
    let branchy = matches!(kind, RefKind::Branch | RefKind::Remote);
    let busy = busy_count > 0;
    let op_standing = !op_text.is_empty();
    let held = !held_by_worktree.is_empty();
    let on_current_branch = kind == RefKind::Branch && full == current_branch;
    // Only a tag has sides; every other row's local half is simply there.
    let sides = (kind == RefKind::Tag)
        .then(|| TagSides::from_word(tag_sides))
        .flatten();
    // Unread reads as "here", keeping the everyday delete on a row whose
    // section has not answered; rows needing a remote reading stay out
    // until there is one.
    let tag_here = sides.is_none_or(TagSides::here);
    let tag_on_remote = sides.is_some_and(TagSides::on_remote);
    RefMenuOffers {
        // A working copy leads to itself — unless the tab already stands
        // in it, which is when the caller hands no holder.
        switch_to: (branchy && full != current_branch) || (kind == RefKind::Worktree && held),
        switch_asks: !held && (op_standing || conflict_count > 0),
        branch_here: kind != RefKind::Stash && !oid_hex.is_empty() && !busy && !op_standing,
        integrate_from: open
            && !busy
            && !detached
            && !current_branch.is_empty()
            && !op_standing
            && !full.is_empty()
            && full != current_branch,
        // What integrating asks for, on one of the two ends of the tree's
        // own comparison.
        pull: open
            && !busy
            && !detached
            && !current_branch.is_empty()
            && !op_standing
            && !current_upstream.is_empty()
            && match kind {
                RefKind::Branch => full == current_branch,
                RefKind::Remote => full == current_upstream,
                RefKind::Tag | RefKind::Stash | RefKind::Worktree => false,
            },
        // A working copy's own delete is the WORKTREE card's
        // ([`worktree_card`]).
        delete: !busy
            && kind != RefKind::Worktree
            && !(kind == RefKind::Branch && (full == current_branch || held))
            // A remote-only tag leaves `tag --delete` nothing to name.
            && tag_here,
        delete_remote: !busy && !remote_counterpart.is_empty() && !remote_drifted,
        // Not gated on drift: a remote holding the name elsewhere turns
        // the row into the leased overwrite (デザイン規約 §相手の履歴を置き換える).
        push_tag: kind == RefKind::Tag && !busy && !default_remote.is_empty() && tag_here,
        delete_remote_tag: !busy && !default_remote.is_empty() && tag_on_remote && !remote_drifted,
        delete_tag_everywhere: !busy
            && !default_remote.is_empty()
            && tag_on_remote
            && tag_here
            && !remote_drifted,
        set_upstream: kind == RefKind::Branch && !busy && !default_remote.is_empty(),
        on_current_branch,
    }
}

/// What the right-click menu on a commit row may offer, decided as it
/// opens and frozen while it stands (`CommitMenuState.openRowMenu`).
/// A stash opens its own menu and takes only [`Self::stash_write`]; every
/// other flag is the commit menu's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommitMenuOffers {
    /// cherry-pick / revert — they only add a commit, so they run detached
    /// or not, on any commit.
    pub sequence: bool,
    /// merge / rebase with this commit as the far side: needs a current
    /// branch, and a commit other than HEAD's.
    pub integrate: bool,
    /// squash / reword / drop, HEAD's commit included (the one most often
    /// meant). Not mid-operation: mid-merge a soft reset refuses and the
    /// other two abandon the merge silently.
    pub edit_history: bool,
    /// reset — the branch taken back to this commit. Asks the same of the
    /// repository as [`Self::integrate`].
    pub move_branch: bool,
    /// A new branch on this commit, standing on it — detached too (the way
    /// back out, デザイン規約 §ブランチ・コミットへの移動), and on HEAD's
    /// commit.
    pub branch_here: bool,
    /// The stash menu's writes.
    pub stash_write: bool,
}

impl CommitMenuOffers {
    /// The offers as words, the list `GitFacts.commitMenuOffers` answers
    /// with.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words: Vec<&'static str> = Vec::new();
        if self.sequence {
            words.push("sequence");
        }
        if self.integrate {
            words.push("integrate");
        }
        if self.edit_history {
            words.push("edit-history");
        }
        if self.move_branch {
            words.push("move-branch");
        }
        if self.branch_here {
            words.push("branch-here");
        }
        if self.stash_write {
            words.push("stash-write");
        }
        words
    }
}

/// The offers themselves. `stash_ref` is the stash selector (`""` on an
/// ordinary commit) and routes between the two menus; `head_oid` is the
/// commit HEAD is on. The other inputs read as in [`ref_menu`].
#[expect(clippy::too_many_arguments)]
pub fn commit_menu(
    open: bool,
    busy_count: i32,
    current_branch: &str,
    detached: bool,
    op_text: &str,
    oid_hex: &str,
    head_oid: &str,
    stash_ref: &str,
) -> CommitMenuOffers {
    let busy = busy_count > 0;
    let op_standing = !op_text.is_empty();
    let commit_row = !oid_hex.is_empty() && stash_ref.is_empty();
    let on_branch =
        open && !busy && !detached && !current_branch.is_empty() && !op_standing && commit_row;
    let elsewhere = oid_hex != head_oid;
    CommitMenuOffers {
        sequence: !busy && !op_standing && commit_row,
        integrate: on_branch && elsewhere,
        edit_history: on_branch,
        move_branch: on_branch && elsewhere,
        branch_here: open && !busy && !op_standing && commit_row,
        stash_write: !busy && !stash_ref.is_empty(),
    }
}

/// Why the WORKTREE card's `worktree remove` row stands greyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOut {
    /// `git worktree lock`: git refuses until it is unlocked.
    Locked,
    /// The tab stands in that copy: git runs inside the folder it would be
    /// deleting, and on Windows stops part-way through it.
    Here,
    /// Another write is out.
    Busy,
}

/// What the WORKTREE card offers for one working copy, decided as the
/// menu opens (`RefWorktreeMenu.standOn`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorktreeCardOffers {
    /// `worktree remove` has a row: every copy but the repository's own,
    /// which git never removes (`is a main working tree`).
    pub remove: bool,
    /// Why that row is greyed, where it is; the lock first, since it is the
    /// one reason the reader has to go and undo.
    pub remove_out: Option<RemoveOut>,
}

impl WorktreeCardOffers {
    /// The offers as words, the shape `GitFacts.worktreeCardOffers` answers
    /// with: `remove`, then `out-locked` / `out-here` / `out-busy`.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words: Vec<&'static str> = Vec::new();
        if self.remove {
            words.push("remove");
        }
        match self.remove_out {
            Some(RemoveOut::Locked) => words.push("out-locked"),
            Some(RemoveOut::Here) => words.push("out-here"),
            Some(RemoveOut::Busy) => words.push("out-busy"),
            None => {}
        }
        words
    }
}

/// The WORKTREE card's rule. `main` is the repository's own copy, `here`
/// the one the asking tab stands in.
pub fn worktree_card(main: bool, locked: bool, here: bool, busy_count: i32) -> WorktreeCardOffers {
    if main {
        return WorktreeCardOffers::default();
    }
    let remove_out = if locked {
        Some(RemoveOut::Locked)
    } else if here {
        Some(RemoveOut::Here)
    } else if busy_count > 0 {
        Some(RemoveOut::Busy)
    } else {
        None
    };
    WorktreeCardOffers {
        remove: true,
        remove_out,
    }
}

/// What a new working copy can be made to stand on, from the row a menu
/// was opened on: the two rows that make one (デザイン規約 §作業コピーを作る).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CopyOffers {
    /// `Create worktree here…` — a new branch on this row's commit, out in
    /// a new copy. Any row with a commit but a stash, mid-operation too:
    /// nothing in this copy is touched, so it is the one way to start on
    /// another branch while one is stopped here.
    pub here: bool,
    /// `worktree add` — the row's own branch out in a new copy, where git
    /// would take it.
    pub checkout: Option<CopyCheckout>,
}

/// How the row's branch goes out in the new copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyCheckout {
    /// A local branch no copy has out: checked out as it is.
    Branch,
    /// A remote branch with no local one of its name: a local one is made
    /// to follow it, as `switch` makes one.
    Track,
}

impl CopyOffers {
    /// The offers as words, the shape `GitFacts.copyOffers` answers with:
    /// `here`, then `checkout-branch` / `checkout-track`.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words: Vec<&'static str> = Vec::new();
        if self.here {
            words.push("here");
        }
        match self.checkout {
            Some(CopyCheckout::Branch) => words.push("checkout-branch"),
            Some(CopyCheckout::Track) => words.push("checkout-track"),
            None => {}
        }
        words
    }
}

/// The rule. `kind` is the row's (`None` on a commit row that draws no
/// name); `held_by_worktree` the copy holding the row's branch, a remote
/// row's through the local branch of its name, as in [`ref_menu`];
/// `local_exists` whether a remote row's local branch is there already.
///
/// The branch the tree is on and one another copy has out are git's to
/// refuse (`is already used by worktree at`), so they offer no
/// `worktree add` — the `switch` row's `Open` leads to the holder. A
/// remote branch whose local one exists leaves it to that branch's own
/// row: making the copy off the remote would mean moving the local one.
#[expect(clippy::too_many_arguments)]
pub fn copy_rows(
    kind: Option<RefKind>,
    full: &str,
    oid_hex: &str,
    open: bool,
    busy_count: i32,
    current_branch: &str,
    held_by_worktree: &str,
    local_exists: bool,
) -> CopyOffers {
    let free = open && busy_count <= 0;
    let held = !held_by_worktree.is_empty();
    let checkout = match kind {
        Some(RefKind::Branch) if !held && full != current_branch => Some(CopyCheckout::Branch),
        Some(RefKind::Remote) if !held && !local_exists => Some(CopyCheckout::Track),
        _ => None,
    };
    CopyOffers {
        here: free && !oid_hex.is_empty() && kind != Some(RefKind::Stash),
        checkout: checkout.filter(|_| free && !full.is_empty()),
    }
}

#[cfg(test)]
mod commit_tests;
#[cfg(test)]
mod moves_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod worktree_tests;
