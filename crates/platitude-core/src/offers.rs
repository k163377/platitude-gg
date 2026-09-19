//! Pure rules for what the standing repository state lets the UI offer:
//! the ref and commit menus' rows, and — in [`moves`] — where a press on
//! a ref lands and what leaving a stopped operation costs. Every refusal
//! encoded here is git's own, measured (the doc on
//! each item).
//!
//! Nothing here runs git — callers pass values already in hand, and the
//! menus freeze the answers as they open (app-ui.md §メニュー), so each
//! function is asked once and its result held while the card stands.

mod message;
mod moves;

pub use message::{MessageEdit, message_edit};
pub use moves::{
    SwitchAction, leave_code, leaving_undoes, move_landed, moves_blocked, skip_is_free,
    switch_action,
};

/// The kind of ref a menu row stands on, in the words the chip records
/// carry (`branch` / `remote` / `tag` / `stash`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Branch,
    Remote,
    Tag,
    Stash,
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
            _ => None,
        }
    }
}

/// Which sides of a tag exist, in the words the menu asks with.
///
/// A tag has no namespace, so the same bare name stands for the one here
/// and the one a remote carries, and the rows that act on it are not the
/// same rows: `tag --delete` needs a local one, `push --delete` needs a
/// remote one, and sending needs something local to send. The sidebar
/// lists a name held on both sides once, so the row cannot say this on
/// its own (`NavSectionModel.tagSides`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagSides {
    /// Made here and no remote is known to carry the name.
    Here,
    /// A remote carries it and this repository does not — the one kind of
    /// tag row that names a commit no local ref points at.
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
    /// `switch` lands somewhere else: only a branch (local or remote)
    /// names a place to move to, and the branch the tree is on is not a
    /// move. **A branch another working copy holds keeps this offer** —
    /// everything that stands in the move's way is answered by the
    /// question the press raises.
    pub switch_to: bool,
    /// That move raises a question before it moves: an operation or
    /// unmerged files to clear first, or the branch out in another
    /// working copy. Worn as the `!` in the row's mark seat.
    pub switch_asks: bool,
    /// A new branch on this row's commit. Every ref that names a commit
    /// takes one — the current branch and a detached HEAD included, which
    /// is where one is most often wanted. A stash is the exception: it is
    /// nobody's history to carry on from (デザイン規約 §グラフ行の右クリック).
    pub branch_here: bool,
    /// merge / rebase with this row as the far side. Both make the current
    /// branch the sentence's subject: detached or unborn there is nothing
    /// to move, and the branch itself is nothing to bring in.
    pub integrate_from: bool,
    /// `git pull` — the branch the working tree is on, brought in line
    /// with its upstream in one command.
    ///
    /// **Two rows, one command, and it is always the same one**: the
    /// branch the tree is on, and the remote-tracking ref that is that
    /// branch's upstream — the two ends of the one comparison the window
    /// is already drawing (ahead / behind). Nothing else takes it: a
    /// pull moves the branch it is *run on*, so a remote-tracking ref
    /// somebody else's branch tracks would move a branch the row does
    /// not name (デザイン規約 §取り込んで合流させる — bringing *that* in
    /// is what merge and rebase are for).
    ///
    /// **The upstream has to be there.** git turns a plain pull down for
    /// a branch with no tracking information (measured, 2.55), and a row
    /// offered on a guess would be a press that only ever prints that
    /// refusal (デザイン規約 §メニュー:「対象が存在しない」は席ごと消える).
    pub pull: bool,
    /// The everyday delete. **git refuses to delete the branch the
    /// working tree is on — or one any other working copy has checked
    /// out** (`error: cannot delete branch … used by worktree at …`,
    /// measured), locked or not: a lock stops `worktree remove` /
    /// `worktree move`, a different question.
    pub delete: bool,
    /// The branch's remote reading, deleted without touching the local
    /// one — still possible in both of the refused cases above, and out
    /// where the two have drifted ([`Self::remote_drifted`]).
    pub delete_remote: bool,
    /// The tag sent to the remote this repository pushes to. Only a tag:
    /// a branch goes out through the toolbar, which is where the counts
    /// that decide how hard it may push are (デザイン規約 §リモートへ送る),
    /// and a remote-tracking ref is a reading of what is already there.
    /// Nothing about the working tree stands in its way — a tag names a
    /// commit, and where HEAD is has no bearing on sending it. **A tag
    /// only a remote has is not sent**: there is nothing here to send.
    pub push_tag: bool,
    /// The tag taken off the remote, leaving whatever is here. Offered
    /// wherever a remote was last heard to carry the name — and only
    /// there, because the qualified `--delete` git needs does not fail on
    /// a name the remote has not got (measured — [`crate::remote::
    /// delete_remote_tag`]), so a row offered on a guess would report
    /// success for having done nothing.
    pub delete_remote_tag: bool,
    /// Both at once, for the name that stands on both sides.
    pub delete_tag_everywhere: bool,
    /// Which remote branch this local one is measured against. Only a
    /// local branch has the setting at all — a remote-tracking ref is the
    /// far side of somebody's, and it has none of its own — and the
    /// answer is a question, so the row needs a remote to ask about
    /// and nothing else. **The branch the tree is on
    /// takes it, and so does one another working copy holds** (measured): this
    /// writes configuration about a branch.
    pub set_upstream: bool,
    /// The row is the branch HEAD is on — what the delete rows' refusal
    /// names first.
    pub on_current_branch: bool,
}

impl RefMenuOffers {
    /// The offers as packed words (`switch asks branch-here integrate
    /// pull delete delete-remote push-tag delete-remote-tag
    /// delete-tag-everywhere set-upstream current`), the shape
    /// `GitFacts.refMenuOffers` answers with and the opening function
    /// decodes mechanically.
    pub fn words(&self) -> String {
        let mut words: Vec<&str> = Vec::new();
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
        words.join(" ")
    }
}

/// The offers themselves, off values already in hand — no git runs.
///
/// `full` is the row's whole display name as the chip wears it —
/// `feat`, or `origin/feat` for a remote reading: the UI's
/// vocabulary, which [`crate::refs::RefEntry::name`] spells its own way.
/// `open` is the tab-lifecycle gate and stays the caller's to answer
/// (as [`crate::remote::push_standing`] holds it); `op_text` is the
/// operation badge, empty exactly when nothing is standing — bisect
/// counts; `held_by_worktree` is the path of the other working copy
/// holding the branch this row lands on, empty when none does;
/// `remote_counterpart` is the remote reading a local branch also
/// carries, empty where it has none; `remote_drifted` says that reading
/// stands on another commit — an upstream left behind
/// ([`crate::session::BranchItem::upstream_drifted`]) or a remote's copy
/// of a tag ([`crate::session::RefsSnapshot::tag_drifts`]), and false
/// where there is no reading to have drifted; `default_remote` is where
/// this repository's pushes go, empty where it has no remote at all;
/// `tag_sides` says which sides a tag row's name stands on
/// ([`TagSides::from_word`] — ignored for every other kind);
/// `current_upstream` is what the branch the working tree is on tracks
/// (`WorkTreeModel.upstream`), empty where it tracks nothing —
/// **the tree's, not this row's**, because a pull goes to that one
/// whichever of its two ends the menu was opened on. Per-row
/// kind choices (a tag's row keeping rebase for the tag's own gestures)
/// stay with the rows.
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
    // Unread is read as "here": that is what a tag row was before any
    // remote was asked, and it keeps the everyday delete on a row whose
    // section has not answered yet. The rows that need a remote reading
    // stay out until there is one.
    let tag_here = sides.is_none_or(TagSides::here);
    let tag_on_remote = sides.is_some_and(TagSides::on_remote);
    RefMenuOffers {
        switch_to: branchy && full != current_branch,
        switch_asks: held || op_standing || conflict_count > 0,
        branch_here: kind != RefKind::Stash && !oid_hex.is_empty() && !busy && !op_standing,
        integrate_from: open
            && !busy
            && !detached
            && !current_branch.is_empty()
            && !op_standing
            && !full.is_empty()
            && full != current_branch,
        // The same standing the rows that integrate ask for — a pull is
        // an integration with a fetch in front of it — and then one of
        // the two ends of the tree's own comparison.
        pull: open
            && !busy
            && !detached
            && !current_branch.is_empty()
            && !op_standing
            && !current_upstream.is_empty()
            && match kind {
                RefKind::Branch => full == current_branch,
                RefKind::Remote => full == current_upstream,
                RefKind::Tag | RefKind::Stash => false,
            },
        delete: !busy
            && !(kind == RefKind::Branch && (full == current_branch || held))
            // A tag only a remote has leaves `tag --delete` nothing to
            // name; the row below it is the one that reaches it.
            && tag_here,
        delete_remote: !busy && !remote_counterpart.is_empty() && !remote_drifted,
        // Sending is about the tag on this row, and a remote holding the
        // name elsewhere is what turns that row into the leased
        // overwrite (デザイン規約 §相手の履歴を置き換える).
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
/// Each flag answers for the menu its row is in: a stash opens its own
/// menu and takes only [`Self::stash_write`], every other flag is the
/// commit menu's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommitMenuOffers {
    /// cherry-pick / revert — the pair that only add a commit, and so ask
    /// less of the repository than the rest: they run detached or not,
    /// and the commit under the pointer is fair game.
    pub sequence: bool,
    /// merge / rebase with this commit as the far side. The current
    /// branch is the subject, and the commit the tree is standing on is
    /// nothing to bring in.
    pub integrate: bool,
    /// squash / reword / drop — rewriting this commit's place in the
    /// history. Unlike the rows that integrate or move, the newest commit
    /// is fair game: that is the one a fold or a drop most often means.
    /// Not offered mid-operation — mid-merge a soft reset refuses
    /// outright and the other two abandon the merge without a word.
    pub edit_history: bool,
    /// reset — the branch taken back to this commit. Asks the same of the
    /// repository as [`Self::integrate`].
    pub move_branch: bool,
    /// A new branch on this commit, standing on it. The one row that asks
    /// nothing of where the working tree is now: a detached HEAD may take
    /// it — it is the way back out (デザイン規約 §ブランチ・コミットへの移動) — and so
    /// may the commit the tree is already standing on.
    pub branch_here: bool,
    /// The stash menu's writes. A stash is a commit git keeps off to one
    /// side of every branch, so none of the rows above land on it and it
    /// gets its own menu.
    pub stash_write: bool,
}

impl CommitMenuOffers {
    /// The offers as packed words (`sequence integrate edit-history
    /// move-branch branch-here stash-write`), the shape
    /// `GitFacts.commitMenuOffers` answers with.
    pub fn words(&self) -> String {
        let mut words: Vec<&str> = Vec::new();
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
        words.join(" ")
    }
}

/// The offers themselves. `stash_ref` is the selector git answers to
/// when the row is a stash (`""` on an ordinary commit) — it is what
/// routes between the two menus; `head_oid` is the commit HEAD is on.
/// The other inputs read as in [`ref_menu`].
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

#[cfg(test)]
mod commit_tests;
#[cfg(test)]
mod moves_tests;
#[cfg(test)]
mod tests;
