//! Pure rules for what the standing repository state lets the UI offer:
//! the ref and commit menus' rows, and — in [`moves`] — where a press on
//! a ref lands and what leaving a stopped operation costs. Every refusal
//! encoded here is git's own (the doc on each item).
//!
//! Nothing here runs git; the menus freeze the answers as they open
//! (rules/app-ui.md「メニューは `AppMenu` 系で書く」). A git command
//! still running is no input: a menu opened under one keeps these rows
//! and greys them until it ends (`AppMenu.runningWhy`), so its shape does
//! not depend on when it was opened.
//!
//! **These are the rules, and the app holds no copy.** It asks by value
//! through `GitFacts` (`refMenu` — one answer per stand, read by both
//! entrances through `RefMenuFacts` — and `commitMenuOffers`, frozen into
//! the menus' `can*` as `RefRowMenu.offerOn` / `CommitMenuState.openRowMenu`
//! open; `switchAction` for `RepoPage.switchToRef`), and what follows from
//! the standing state arrives as `WorktreeModel` properties derived at
//! drain (`movesBlocked` / `leaveUndoes` / `leaveCode` / `opSkipFree`).
//! QML keeps only the lookups that feed them, the `qsTr` wording and each
//! row's kind condition.

mod far;
mod message;
mod moves;
mod pull;
mod tree;

pub use far::{BranchFar, FarSide, Reading, TagDeleteHeld, TagFar, TagSides};
pub use message::{MessageEdit, message_edit};
pub use moves::{
    SwitchAction, SwitchTarget, leave_code, leaving_undoes, move_landed, moves_blocked,
    skip_is_free, switch_action,
};
pub use pull::{PullOffer, PullOut, PullRuns, pull_row};
pub use tree::{TreeStanding, is_current_branch};

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

/// The row a menu stands on: its kind, and the name the row is asked by —
/// a branch's, a remote branch's and a tag's as the chip wears it (`feat`,
/// `origin/feat`), not [`crate::refs::RefEntry::name`]'s spelling; a
/// stash's selector; a worktree's path. Never empty. Which row is not
/// where a press on it lands: a remote row's move goes to a branch here of
/// another name ([`SwitchTarget`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefTarget<'a> {
    kind: RefKind,
    name: &'a str,
}

impl<'a> RefTarget<'a> {
    /// `None` for no name: a row that names nothing is no row to offer on.
    fn new(kind: RefKind, name: &'a str) -> Option<Self> {
        (!name.is_empty()).then_some(Self { kind, name })
    }

    /// The row the two words a menu asks with stand for; `None` where the
    /// kind is none a menu names itself by ([`RefKind::from_word`]) or the
    /// name is empty.
    pub fn from_words(kind: &str, name: &'a str) -> Option<Self> {
        Self::new(RefKind::from_word(kind)?, name)
    }
}

/// A ref's row as its menu stands on it: the row, whether it points at a
/// commit, whether the tab is open, where the tree stands, and where the
/// row leads beside it. Built once per stand; the menu's rules read it
/// whole ([`ref_menu`], [`pull_row`], [`worktree_rows`]).
///
/// Whether the tab is open is not the tree's ([`TreeStanding`]), and where
/// the row leads is not the row's: the worktrees say it.
#[derive(Debug, Clone, Copy)]
pub struct RowStanding<'a> {
    /// The row; `None` on a commit row that draws no name, which has the
    /// rows of its commit and none of a name.
    pub target: Option<RefTarget<'a>>,
    /// The row points at a commit — what anything made on it stands on.
    pub has_commit: bool,
    /// The tab-lifecycle gate.
    pub open: bool,
    pub tree: &'a TreeStanding,
    pub leads: Leads,
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

/// Where a row stands on a menu that decides its seat apart from whether it
/// is live (デザイン規約 §メニュー「「今できない」行は無効で残す」).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Seat {
    /// No row: what it would act on is not there.
    #[default]
    Gone,
    Live,
    /// There and greyed, for the reason the card words.
    Greyed,
}

impl Seat {
    /// The word a menu's record carries it in; empty for [`Self::Gone`].
    pub fn word(self) -> &'static str {
        match self {
            Self::Gone => "",
            Self::Live => "live",
            Self::Greyed => "greyed",
        }
    }
}

/// What the right-click menu on a ref may offer, decided as it opens and
/// as it stands again on the tree as it is (`RefRowMenu.offerOn` /
/// `standAgain`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefMenuOffers {
    /// `switch` lands somewhere else: a branch (local or remote) other than
    /// the current one. A branch another worktree holds keeps this
    /// offer — the row leads to that worktree ([`SwitchAction::OpenHolder`]).
    pub switch_to: bool,
    /// That move asks first: an operation or unmerged files to clear (the
    /// `!` in the row's mark seat), read off the answer the press itself
    /// reads ([`TreeStanding::moves_blocked`]). A branch another worktree
    /// holds asks nothing and wears no mark (デザイン規約 §進行中の操作から出る).
    pub switch_asks: bool,
    /// A new branch on this row's commit — any ref but a stash, the
    /// current branch and a detached HEAD included
    /// (デザイン規約 §ブランチ・コミットへの移動).
    pub branch_here: bool,
    /// merge / rebase with this row as the far side: needs a current
    /// branch to move, and a row that is not it.
    pub integrate_from: bool,
    /// An operation stands: [`Self::branch_here`] and
    /// [`Self::integrate_from`] keep their seats, greyed until it is
    /// finished or put down — one operation at a time, and `switch
    /// --create` is a move (デザイン規約 §進行中の操作から出る).
    pub standing: bool,
    /// The everyday delete. git refuses the branch the tree is on or one
    /// any other worktree has checked out (`cannot delete branch …
    /// used by worktree at …`), locked or not.
    pub delete: bool,
    /// The branch's remote reading, deleted without touching the local
    /// one — still possible in both of the refused cases above, and out
    /// where the two have drifted ([`Reading::Drifted`]).
    pub delete_remote: bool,
    /// The tag sent to the remote this repository pushes to. Only a tag —
    /// a branch goes out through the toolbar, where its counts are
    /// (デザイン規約 §リモートへ送る). The working tree has no bearing; a
    /// tag only a remote has is not sent.
    pub push_tag: bool,
    /// The tag taken off the remote, leaving whatever is here. Only where a
    /// remote was last heard to carry the name: the delete is leased to the
    /// commit that reading shows ([`crate::remote::delete_remote_tag`]), so
    /// with no reading there is nothing to lease to. Greyed while the
    /// deletes over there are held back ([`TagFar::held_back`]): the row
    /// keeps its seat and says why.
    pub delete_remote_tag: Seat,
    /// Both at once, for the name that stands on both sides — greyed as
    /// [`Self::delete_remote_tag`] is, on a name here.
    pub delete_tag_everywhere: Seat,
    /// Which remote branch this local one is measured against. Local
    /// branches only, given a remote to ask about; the current branch and
    /// one another worktree holds take it too — this writes
    /// configuration, not the branch.
    pub set_upstream: bool,
    /// The upstream taken off, leaving the branch measured against
    /// nothing. Only a local branch that has one, listed or gone; like
    /// [`Self::set_upstream`] it writes configuration, so the current
    /// branch and one another worktree holds take it, and it needs no
    /// remote — an upstream may be a branch here.
    pub unset_upstream: bool,
    /// The row is the branch HEAD is on — what the delete rows' refusal
    /// names first.
    pub on_current_branch: bool,
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

/// The offers themselves: `far` is what the row's card reads over there
/// ([`FarSide::Unread`] where it reads nothing). Nothing on a row that
/// names no ref. The `pull` row is [`pull_row`]'s.
pub fn ref_menu(row: &RowStanding<'_>, far: FarSide) -> RefMenuOffers {
    let Some(RefTarget { kind, name }) = row.target else {
        return RefMenuOffers::default();
    };
    let tree = row.tree;
    let branchy = matches!(kind, RefKind::Branch | RefKind::Remote);
    let current = tree.is_current(name);
    let on_current_branch = kind == RefKind::Branch && current;
    let (upstream, tag) = match far.on(kind) {
        FarSide::Unread => (None, None),
        FarSide::Branch(upstream) => (Some(upstream), None),
        FarSide::Tag(tag) => (None, Some(tag)),
    };
    // Every row but a tag's has its local half simply there, and an
    // unread tag reads as one here ([`TagFar::sides`]).
    let sides = tag.and_then(|tag| tag.sides);
    let tag_here = sides.is_none_or(TagSides::here);
    let tag_on_remote = sides.is_some_and(TagSides::on_remote);
    let tag_reaches = tag.is_some_and(|tag| tag.has_remote);
    let tag_held = tag.is_some_and(|tag| tag.held_back != TagDeleteHeld::No);
    // A hold names the reading it holds back, so it keeps the seat it greys
    // — on a name here for the pair.
    let tag_seat = |reaches: bool, held: bool| {
        if held {
            Seat::Greyed
        } else if reaches {
            Seat::Live
        } else {
            Seat::Gone
        }
    };
    RefMenuOffers {
        // A worktree leads to itself — unless the tab already stands in it.
        switch_to: (branchy && !current) || row.leads == Leads::Itself,
        // Leading elsewhere asks nothing: the press opens that worktree.
        switch_asks: row.leads == Leads::Nowhere && tree.moves_blocked,
        branch_here: kind != RefKind::Stash && row.has_commit,
        integrate_from: row.open && tree.on_branch() && !current,
        standing: tree.op_standing(),
        // A worktree's own delete is the WORKTREE card's
        // ([`worktree_card`]).
        delete: kind != RefKind::Worktree
            && !(kind == RefKind::Branch && (current || row.leads == Leads::Holder))
            // A remote-only tag leaves `tag --delete` nothing to name.
            && tag_here,
        delete_remote: upstream.is_some_and(|upstream| upstream.reading == Reading::Together),
        // Not gated on drift: a remote holding the name elsewhere turns
        // the row into the leased overwrite (デザイン規約 §相手の履歴を置き換える).
        push_tag: tag_reaches && tag_here,
        delete_remote_tag: tag_seat(tag_reaches && tag_on_remote, tag_held),
        delete_tag_everywhere: tag_seat(
            tag_reaches && tag_on_remote && tag_here,
            tag_held && tag_here,
        ),
        set_upstream: upstream.is_some_and(|upstream| upstream.has_remote),
        unset_upstream: upstream.is_some_and(|upstream| upstream.tracked),
        on_current_branch,
    }
}

/// What the right-click menu on a commit row may offer, decided as it
/// opens and as it stands again on the tree as it is
/// (`CommitMenuState.openRowMenu` / `standAgain`).
/// A stash opens its own menu and takes only [`Self::stash_write`]; every
/// other flag is the commit menu's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommitMenuOffers {
    /// cherry-pick — it only adds a commit, so it runs detached or not, on
    /// any commit.
    pub cherry_pick: bool,
    /// revert — as [`Self::cherry_pick`], but what it undoes has to be in the
    /// history ([`Self::on_provisional`]).
    pub revert: bool,
    /// merge / rebase with this commit as the far side: needs a current
    /// branch, and a commit other than HEAD's.
    pub integrate: bool,
    /// squash / reword / drop, HEAD's commit included (the one most often
    /// meant).
    pub edit_history: bool,
    /// reset — the branch taken back to this commit. Asks the same of the
    /// repository as [`Self::integrate`]: a branch (a detached HEAD has
    /// none to move), a commit row (a stash is no branch's history), and a
    /// commit other than HEAD's (nothing would move). False takes the card
    /// and the title row that opens it away together. Under a standing
    /// operation the card stays, greyed ([`Self::standing`]): mid-merge
    /// `--soft` refuses and `--mixed` / `--hard` drop `MERGE_HEAD` silently,
    /// abandoning the merge; the way out is the exit card's `--continue` /
    /// `--abort`.
    pub move_branch: bool,
    /// A new branch on this commit, standing on it — detached too (the way
    /// back out, デザイン規約 §ブランチ・コミットへの移動), and on HEAD's
    /// commit.
    pub branch_here: bool,
    /// An operation stands: every row above keeps its seat, greyed until
    /// it is finished or put down — they all start or rewrite history,
    /// and mid-merge a soft reset refuses while squash and drop would
    /// abandon the merge silently. As [`RefMenuOffers::standing`] greys
    /// the ref menu's.
    pub standing: bool,
    /// The stash menu's writes.
    pub stash_write: bool,
}

impl CommitMenuOffers {
    /// The offers as words, the list `GitFacts.commitMenuOffers` answers
    /// with.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words: Vec<&'static str> = Vec::new();
        if self.cherry_pick {
            words.push("cherry-pick");
        }
        if self.revert {
            words.push("revert");
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
        if self.standing {
            words.push("standing");
        }
        if self.stash_write {
            words.push("stash-write");
        }
        words
    }

    /// The same offers on a commit the discard log's picked entry draws on
    /// the graph, a display mode of its own (user decision — ask before
    /// changing): what the mode can never do has no row, not a greyed one, and
    /// what stays greys under [`Self::standing`] as anywhere (デザイン規約
    /// §メニュー). It can do only what adds and leaves every ref where it is —
    /// cherry-pick and a new name on the commit. `revert` would undo what the
    /// branch never had; squash / drop are refused off the branch, and on a
    /// commit still in it (an entry that took only a name) would rewrite what
    /// the entry is not about; merge / rebase / reset would take the discard
    /// back by moving a ref (破棄記録仕様.md「復元は追加の操作」).
    #[must_use]
    pub fn on_provisional(self) -> Self {
        Self {
            revert: false,
            integrate: false,
            edit_history: false,
            move_branch: false,
            ..self
        }
    }
}

/// The offers themselves. `oid_hex` is the row's commit; `stash` routes
/// between the two menus. `open` and `tree` read as in [`RowStanding`].
pub fn commit_menu(
    open: bool,
    tree: &TreeStanding,
    oid_hex: &str,
    stash: bool,
) -> CommitMenuOffers {
    let commit_row = !oid_hex.is_empty() && !stash;
    let on_branch = open && tree.on_branch() && commit_row;
    let elsewhere = !tree.is_head(oid_hex);
    CommitMenuOffers {
        cherry_pick: commit_row,
        revert: commit_row,
        integrate: on_branch && elsewhere,
        edit_history: on_branch,
        move_branch: on_branch && elsewhere,
        branch_here: open && commit_row,
        standing: tree.op_standing() && commit_row,
        stash_write: stash,
    }
}

/// Why the WORKTREE card's `worktree remove` row stands greyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOut {
    /// `git worktree lock`: git refuses until it is unlocked.
    Locked,
    /// The tab stands in that worktree: git runs inside the folder it would be
    /// deleting, and on Windows stops part-way through it.
    Here,
}

/// What the WORKTREE card offers for one worktree, decided as the
/// menu opens (`RefWorktreeMenu.standOn`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorktreeCardOffers {
    /// `worktree remove` has a row: every worktree but the repository's own,
    /// which git never removes (`is a main working tree`).
    pub remove: bool,
    /// Why that row is greyed, where it is; the lock first, since it is the
    /// one reason the reader has to go and undo.
    pub remove_out: Option<RemoveOut>,
}

impl WorktreeCardOffers {
    /// The offers as words, the shape `GitFacts.worktreeCardOffers` answers
    /// with: `remove`, then `out-locked` / `out-here`.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words: Vec<&'static str> = Vec::new();
        if self.remove {
            words.push("remove");
        }
        match self.remove_out {
            Some(RemoveOut::Locked) => words.push("out-locked"),
            Some(RemoveOut::Here) => words.push("out-here"),
            None => {}
        }
        words
    }
}

/// The WORKTREE card's rule. `main` is the repository's own worktree, `here`
/// the one the asking tab stands in.
pub fn worktree_card(main: bool, locked: bool, here: bool) -> WorktreeCardOffers {
    if main {
        return WorktreeCardOffers::default();
    }
    let remove_out = if locked {
        Some(RemoveOut::Locked)
    } else if here {
        Some(RemoveOut::Here)
    } else {
        None
    };
    WorktreeCardOffers {
        remove: true,
        remove_out,
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
    /// removes it ([`worktree_card`]).
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

/// What a new worktree can be made to stand on, from the row a menu
/// was opened on: the two rows that make one (デザイン規約 §worktree を作る).
/// Both give it a branch — no row makes a detached worktree: a branch row
/// takes its branch, a commit or tag row a new one, a remote row a local
/// one of the same name set to follow it, as `switch` does there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorktreeOffers {
    /// `Create worktree here…` — a new branch on this row's commit, out in
    /// a new worktree. Any row with a commit but a stash, mid-operation too:
    /// nothing in this worktree is touched, so it is the one way to start on
    /// another branch while one is stopped here.
    pub here: bool,
    /// `worktree add` — the row's own branch out in a new worktree.
    pub checkout: Option<WorktreeCheckout>,
    /// Why that row is greyed, where it is.
    pub checkout_out: Option<WorktreeAddOut>,
}

/// Why `worktree add` cannot take the row's branch out right now: git keeps a
/// branch to one worktree (`is already used by worktree at`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeAddOut {
    /// The tree this tab stands in has it out.
    Current,
    /// Another worktree has it out.
    Held,
    /// A rebase or bisect stopped in this tree holds it: the tree is
    /// detached, and git still counts the branch it names as out here.
    Stopped,
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

impl WorktreeAddOut {
    /// The word the menu's record carries it in (`WorktreeMaking`).
    pub fn word(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Held => "held",
            Self::Stopped => "stopped",
        }
    }
}

impl WorktreeCheckout {
    /// The word the menu's record carries it in (`WorktreeMaking`).
    pub fn word(self) -> &'static str {
        match self {
            Self::Branch => "branch",
            Self::Track => "track",
        }
    }
}

/// The rule. `stopped_here` is whether a rebase or bisect stopped in this
/// tree holds the row's branch; `local_listed` whether a remote row's
/// local branch is there already.
///
/// The branch the tree is on, one a stopped operation here holds and one
/// another worktree has out are git's to refuse (`is already used by
/// worktree at`): their `worktree add` stands greyed ([`WorktreeAddOut`]),
/// and the `switch` row's `Open` leads to the holder. A remote branch whose
/// local one exists leaves it to that branch's own row: making the
/// worktree off the remote would mean moving the local one.
pub fn worktree_rows(
    row: &RowStanding<'_>,
    stopped_here: bool,
    local_listed: bool,
) -> WorktreeOffers {
    let kind = row.target.map(|target| target.kind);
    let held = row.leads == Leads::Holder;
    let checkout = match kind {
        Some(RefKind::Branch) => Some(WorktreeCheckout::Branch),
        Some(RefKind::Remote) if !held && !local_listed => Some(WorktreeCheckout::Track),
        _ => None,
    }
    .filter(|_| row.open);
    let on_current = row
        .target
        .is_some_and(|target| row.tree.is_current(target.name));
    let checkout_out = match checkout {
        Some(WorktreeCheckout::Branch) if on_current => Some(WorktreeAddOut::Current),
        Some(WorktreeCheckout::Branch) if held => Some(WorktreeAddOut::Held),
        Some(WorktreeCheckout::Branch) if stopped_here => Some(WorktreeAddOut::Stopped),
        _ => None,
    };
    WorktreeOffers {
        here: row.open && row.has_commit && kind != Some(RefKind::Stash),
        checkout,
        checkout_out,
    }
}

#[cfg(test)]
mod testkit;
#[cfg(test)]
mod tests;
