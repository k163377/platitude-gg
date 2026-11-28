//! Whether a row can be pressed now, read off everything known.

use super::class::{Below, Chosen, Class, Shape, Tie};
use super::row::Row;
use crate::Oid;
use crate::offers::{MovedBranch, PullRuns, RefKind, TagDeleteHeld, TreeStanding};
use crate::remote::PushStanding;
use crate::stash::StashStanding;
use crate::worktrees::Keeper;

/// The state of what the [`Shape`] says is there: what the lines read
/// beside it, and the rows never do.
#[derive(Debug, Clone, Copy)]
pub struct State<'a> {
    /// The commit the row points at; `None` on a branch with no commits
    /// yet.
    pub commit: Option<Oid>,
    /// How the branch HEAD is on has that commit; `None` where it is not
    /// known ([`super::Subject::history`]).
    pub history: Option<History>,
    /// HEAD, the operation standing on it, and the tab's branch against its
    /// upstream.
    pub tree: &'a TreeStanding,
    /// Another worktree has the row's branch out — a remote row's through
    /// its local namesake.
    pub held: bool,
    /// A remote row's local namesake is there.
    pub local_listed: bool,
    /// The remote branch a local one reads stands on another commit.
    pub drifted: bool,
    /// How many commits a local branch has that its upstream lacks, as of
    /// the last fetch.
    pub ahead: u32,
    /// The repository has a remote to name an upstream on.
    pub has_remote: bool,
    /// A tag's rows over there have a remote to go to.
    pub tag_remote: bool,
    /// Why a tag's deletes over there are held back.
    pub tag_held: TagDeleteHeld,
    /// What a push of the row's branch can do.
    pub push: PushStanding,
    /// The worktree the row names or leads to is locked.
    pub locked: bool,
    pub placing: Placing,
}

impl State<'_> {
    /// A local branch of the remote row's name is there, or out in another
    /// worktree: its own row takes it out, and `worktree add` on the remote
    /// row would mean moving it ([`Why::LocalBranchThere`],
    /// [`super::Subject::worktree_checkout`]).
    pub fn namesake_there(&self) -> bool {
        self.local_listed || self.held
    }
}

/// How the branch HEAD is on has a row's commit, off the rows the graph
/// has drawn (`GraphModel.historyOf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum History {
    /// Down HEAD's first parents with no merge on the way — git's
    /// `is_linear_history`: a rebase onto it has nothing to move
    /// (`Current branch … is up to date.`).
    Straight,
    /// In the history past a merge — one on the way down HEAD's first
    /// parents, or the commit on a merge's other side: a rebase onto it
    /// lays the branch's commits again in one line, the merges left out
    /// — with `rebase.rebaseMerges` set it lays them merges and all, and
    /// the branch stands as it was, the live row costing nothing (measured
    /// on 2.55).
    PastMerge,
    /// Not in the history.
    Off,
}

impl History {
    const ALL: [Self; 3] = [Self::Straight, Self::PastMerge, Self::Off];

    /// The word the graph answers it in (`GraphModel.historyOf`).
    pub fn word(self) -> &'static str {
        match self {
            Self::Straight => "straight",
            Self::PastMerge => "past-merge",
            Self::Off => "off",
        }
    }

    /// The answer [`Self::word`] wrote; `None` for any other word, the
    /// empty one the graph gives where it cannot say included.
    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|history| history.word() == word)
    }

    /// In the history at all.
    pub fn holds(self) -> bool {
        self != Self::Off
    }
}

/// Where a new worktree goes, as the worktree listing tells it — one answer
/// for both rows that make one (`Create worktree here…`, `worktree add`).
///
/// Only those two rows read it: they need the place before git runs. The
/// rest of what the listing tells — another worktree holding the row's
/// branch, a lock — reads as nothing while it has not arrived, and git
/// answers at the press.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Placing {
    /// The listing has not arrived: where a new one goes is not known —
    /// nowhere included.
    #[default]
    Unread,
    /// Nowhere, and why.
    Nowhere(Nowhere),
    /// In the folder beside the repository's own worktree
    /// (`worktrees::worktrees_folder`); `taken`: something stands where the
    /// row's own branch would go out.
    Beside { taken: bool },
}

impl Placing {
    const ALL: [Self; 5] = [
        Self::Unread,
        Self::Nowhere(Nowhere::Bare),
        Self::Nowhere(Nowhere::DriveRoot),
        Self::Beside { taken: false },
        Self::Beside { taken: true },
    ];

    /// The word a row's lead carries it in (`NavSectionModel.leadOf`).
    pub fn word(self) -> &'static str {
        match self {
            Self::Unread => "unread",
            Self::Nowhere(Nowhere::Bare) => "bare",
            Self::Nowhere(Nowhere::DriveRoot) => "drive-root",
            Self::Beside { taken: false } => "free",
            Self::Beside { taken: true } => "taken",
        }
    }

    /// The answer [`Self::word`] wrote; `None` for any other word.
    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|placing| placing.word() == word)
    }

    /// Why no new worktree can be made off the row: no place known yet, or
    /// none at all; `None` where one goes beside the repository's own.
    fn none_why(self) -> Option<Why> {
        match self {
            Self::Unread => Some(Why::WorktreesUnread),
            Self::Nowhere(nowhere) => Some(Why::Nowhere(nowhere)),
            Self::Beside { .. } => None,
        }
    }
}

/// Why new worktrees have nowhere to go: the folder they go in is beside the
/// repository's own worktree, and there is none to go beside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nowhere {
    /// The repository is bare: it has no worktree of its own.
    Bare,
    /// The repository's own worktree stands at the root of its drive, with
    /// no folder above it.
    DriveRoot,
}

/// Everything a ref's or a graph row's line reads: what is there, and its
/// state.
#[derive(Debug, Clone, Copy)]
pub struct Facts<'a> {
    pub shape: Shape<'a>,
    pub state: State<'a>,
}

/// What a menu is opened on, with everything its rows read: what is there
/// decides the class ([`Class::on`]), the rest only greys.
#[derive(Debug, Clone, Copy)]
pub enum On<'a> {
    /// A ref's row of the left pane, or a graph row.
    Ref(Facts<'a>),
    /// Another worktree's uncommitted row on the graph: no ref and no commit
    /// to act on. Unmerged paths stand in this tree (`conflicts_here`), and
    /// in that worktree as of its last read (`conflicts_there`).
    Carried {
        conflicts_here: bool,
        conflicts_there: bool,
    },
    /// The files chosen in the working tree pane, and whether git takes a
    /// stash of this tree at all (`stash::standing`) — a question about the
    /// tree, not the chosen files.
    Files {
        chosen: Chosen,
        stash: StashStanding,
    },
    /// A step of a plan, by what stands below it.
    Step(Below),
    /// A remote's own row; `marked`: both keys already name it — a remote
    /// only one key names keeps the mark live, to finish it (デザイン規約
    /// §リモートを書き留める).
    Remote { marked: bool },
}

/// One drawn row: its kind, and whether it can be pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    pub row: Row,
    pub can: Can,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Can {
    Yes,
    /// Greyed, saying why on hover.
    No(Why),
}

/// Why a row is greyed — the one reason its tip says, picked in order where
/// several hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The tab has not finished opening.
    Opening,
    /// The row's branch has no commits yet — for the working tree's files,
    /// the branch HEAD is on: git has no ref to start on, bring in, send,
    /// measure or stash onto (`You do not have the initial commit yet`).
    NoCommitsYet,
    /// An operation stands: one at a time, so whatever would start or
    /// rewrite history waits until it is finished or put down — `switch
    /// --create` is a move (デザイン規約 §進行中の操作から出る), and `Create
    /// tag here…` answers as `Create branch here…` does (user decision — ask
    /// before changing; git itself tags mid-operation). The way out is the
    /// exit card's `--continue` / `--abort`. Mid-merge a soft reset refuses,
    /// `--mixed` / `--hard` would abandon the merge silently (`MERGE_HEAD`
    /// goes with the reset), and `squash` / `drop` are turned down before
    /// anything is stashed (`session::build`'s `carry_across`) — measured on
    /// 2.55. Mid-rebase git refuses no `merge`: it writes the merge on the
    /// detached HEAD.
    StoppedOperation,
    /// HEAD is on a branch with no commits yet
    /// ([`TreeStanding::head_unborn`]): nothing on it to undo a commit on,
    /// to rebase or to rewrite. `cherry-pick`, `merge` and `reset` stay
    /// live — they take the branch to a commit, `merge` as a fast-forward:
    /// under `merge.ff=false` git refuses one into an empty head (`Non-
    /// fast-forward commit does not make sense into an empty head`).
    HeadUnborn,
    /// The commit is not in the history of the branch HEAD is on: a rewrite
    /// takes only the branch's own commits, and refuses before git runs
    /// (`report::rewrite_off_branch`).
    OffBranch,
    /// The row stands on HEAD's commit: git brings nothing in (`Already up
    /// to date.` / `Current branch … is up to date.`), and a cherry-pick of
    /// it comes out empty, which the press walks past (`integrate::pick`).
    SameCommit,
    /// The row's commit is in the history of the branch HEAD is on, behind
    /// its tip: a merge brings nothing in (`Already up to date.`), and a
    /// rebase onto it has nothing to move where the branch runs down to it
    /// straight ([`History::Straight`]). Past a merge a rebase lays the
    /// branch again, and a cherry-pick is no part of it: git makes the
    /// commit again where the branch undid it since, stops on a conflict
    /// where its lines were written over since, and comes out empty only
    /// where nothing touched them (measured on 2.55). Said where the drawn
    /// rows tell ([`State::history`]); elsewhere git answers at the press.
    InHistory,
    /// The row stands on HEAD's commit, where the branch the tab is on
    /// already is: the RESET card takes it nowhere. `--mixed` and `--hard`
    /// would still unstage or discard what is not committed — no move of the
    /// branch, which is all the card is for.
    AlreadyThere,
    /// The branch the tab is on: git deletes no branch HEAD names, nor takes
    /// one out in a second worktree.
    OnThisBranch,
    /// Another worktree has the branch out, locked or not: git refuses to
    /// delete it (`cannot delete branch … used by worktree at …`), to take
    /// it out again (`is already used by worktree at`) and to fetch into it.
    /// The `Open` row leads to that worktree.
    CheckedOutElsewhere,
    /// A rebase or bisect stopped in this tree keeps the branch
    /// ([`MovedBranch::Kept`]): the tree is detached, and git still counts
    /// the branch as out here — it deletes it no more than another
    /// worktree's (`cannot delete branch … used by worktree at …`), nor
    /// takes it out in a second worktree. The word says which operation
    /// (`rebasing` / `bisecting`), since the tip names it.
    KeptBy(Keeper),
    /// A local branch of the remote row's name is there: its own row takes
    /// it out — making the worktree off the remote would mean moving the
    /// local one.
    LocalBranchThere,
    /// The worktree listing has not arrived yet: where a new worktree goes
    /// is not known ([`Placing::Unread`]).
    WorktreesUnread,
    /// New worktrees have nowhere to go, and why ([`Placing::Nowhere`]).
    Nowhere(Nowhere),
    /// Something stands where the new worktree would go.
    PlaceTaken,
    /// The tab's branch and its upstream have both moved: merging and
    /// rebasing are the reader's to choose between, and git's settings do
    /// not get to choose for them (user decision — ask before changing).
    /// The tip says where to choose (`Words.pullDiverged`: the remote branch
    /// row's `rebase <current> onto it`, or `merge`); the same reason pins
    /// the pull to `--ff-only` (`remote::pull`). Told by the tree's standing
    /// ([`TreeStanding::diverged`]).
    Diverged,
    /// The branch has commits its upstream lacks, so no fast-forward reaches
    /// it — git would answer `[rejected] (non-fast-forward)` — and a press
    /// must not move a branch somewhere unexpected (user decision — ask
    /// before changing). Only a pull from the branch itself can take them
    /// in: switch to it and use its row (a diverged one there says where
    /// next). Its tip says what cannot happen, then why, naming the upstream
    /// (user decision — ask before changing; `Words.pullOwnCommits`).
    OwnCommits,
    /// The remote branch the local one reads stands on another commit: the
    /// delete over there would take commits this row does not show.
    ReadingDrifted,
    /// No remote to go to.
    NoRemote,
    /// The remote reached has the tag on another commit.
    TagDrifted,
    /// Several remotes carry the tag, and none is named.
    TagUnnamed,
    /// `git worktree lock`: git refuses until it is unlocked.
    Locked,
    /// The tab shows that worktree: git runs inside the folder it would be
    /// deleting, and on Windows stops part-way through it.
    ShownHere,
    /// The remote has every commit of it — the row stands greyed saying so
    /// (user decision — ask before changing).
    NothingToSend,
    /// The push resolves to this repository.
    StaysHere,
    /// Unmerged paths stand in this tree: git applies nothing over an
    /// unmerged index, and stashes nothing — one unmerged path anywhere and
    /// it refuses the whole command, named paths or not (`needs merge`).
    ConflictsHere,
    /// The other worktree holds unmerged paths: which of their sides to
    /// bring is not decided.
    ConflictsThere,
    /// Every step below is a `drop`: a fold would land in none — git stops
    /// on such a todo (`cannot 'squash' without a previous commit`).
    NothingToFoldInto,
    /// Both keys name the remote already: the mark is there.
    AlreadyOrigin,
    /// The branch listing has not caught up with the row, so where its push
    /// goes and what it would send are not known: the listing still marks
    /// it as the branch HEAD is on while the tree reads another, or does
    /// not name it yet (`RefsSnapshot::branch_push`, closed). The menu
    /// stands again as the listing moves.
    Unread,
}

impl Why {
    /// The word the menu's record carries it in. The QML says each in one
    /// sentence (`Words.menuWhy`): a word with none would grey its row
    /// saying nothing of why, so `cargo xtask structure` holds the two
    /// lists equal (`structure::menu_words`).
    pub fn word(self) -> &'static str {
        match self {
            Self::Opening => "opening",
            Self::NoCommitsYet => "no-commits",
            Self::StoppedOperation => "standing",
            Self::HeadUnborn => "head-unborn",
            Self::OffBranch => "off-branch",
            Self::SameCommit => "same-commit",
            Self::InHistory => "in-history",
            Self::AlreadyThere => "already-there",
            Self::OnThisBranch => "current",
            Self::CheckedOutElsewhere => "held",
            Self::KeptBy(Keeper::Rebase) => "rebasing",
            Self::KeptBy(Keeper::Bisect) => "bisecting",
            Self::LocalBranchThere => "local",
            Self::WorktreesUnread => "worktrees-unread",
            Self::Nowhere(Nowhere::Bare) => "bare",
            Self::Nowhere(Nowhere::DriveRoot) => "drive-root",
            Self::PlaceTaken => "taken",
            Self::Diverged => "diverged",
            Self::OwnCommits => "own",
            Self::ReadingDrifted => "drifted",
            Self::NoRemote => "no-remote",
            Self::TagDrifted => "tag-drifted",
            Self::TagUnnamed => "tag-unnamed",
            Self::Locked => "locked",
            Self::ShownHere => "here",
            Self::NothingToSend => "clean",
            Self::StaysHere => "stays-here",
            Self::ConflictsHere => "conflicts-here",
            Self::ConflictsThere => "conflicts-there",
            Self::NothingToFoldInto => "no-fold",
            Self::AlreadyOrigin => "already-origin",
            Self::Unread => "unread",
        }
    }
}

/// The first reason that holds, or [`Can::Yes`].
fn first<const N: usize>(reasons: [(bool, Why); N]) -> Can {
    reasons
        .into_iter()
        .find_map(|(holds, why)| holds.then_some(Can::No(why)))
        .unwrap_or(Can::Yes)
}

/// The first reason given, or [`Can::Yes`] — [`first`] for reasons a fact
/// names itself ([`Placing`]).
fn first_given<const N: usize>(reasons: [Option<Why>; N]) -> Can {
    reasons
        .into_iter()
        .flatten()
        .next()
        .map_or(Can::Yes, Can::No)
}

/// The menu opened on `on`: every row its class has, in drawing order, each
/// with whether it can be pressed. `None` where it offers nothing. Until the
/// tab has finished opening (`open`), every row greys under one reason
/// ([`Why::Opening`]).
pub fn lines(open: bool, on: &On<'_>) -> Option<Vec<Line>> {
    let class = Class::on(on)?;
    Some(
        class
            .rows()
            .into_iter()
            .map(|row| Line {
                row,
                can: if open {
                    can(row, on)
                } else {
                    Can::No(Why::Opening)
                },
            })
            .collect(),
    )
}

/// Why git takes no stash of the tree, in the menus' reasons: what the
/// file rows' `stash` says, and the toolbar's button with it (one cause,
/// one sentence); `None` where git takes one.
pub fn stash_why(stash: StashStanding) -> Option<Why> {
    match stash {
        StashStanding::Unborn => Some(Why::NoCommitsYet),
        StashStanding::Conflicts => Some(Why::ConflictsHere),
        StashStanding::Clean | StashStanding::Ready => None,
    }
}

/// What `row` of the class `on` makes can do, read off `on`'s facts. The
/// files' rows other than `stash` stand on nothing but the files; a plan's
/// rows only edit the todo, which the plan's own button runs whatever the
/// repository stands in.
fn can(row: Row, on: &On<'_>) -> Can {
    match *on {
        On::Ref(facts) => ref_can(row, &facts),
        // A conflict on either side greys it (user decision — ask before
        // changing) — here first, so with both the reason says here. That
        // worktree's count is its row's, as old as its last read
        // (`session::pace`); the write reads the tree afresh when its turn
        // comes (`RepoSession::apply_carried`).
        On::Carried {
            conflicts_here,
            conflicts_there,
        } => match row {
            Row::ApplyChanges => first([
                (conflicts_here, Why::ConflictsHere),
                (conflicts_there, Why::ConflictsThere),
            ]),
            _ => Can::Yes,
        },
        On::Files { stash, .. } => match row {
            Row::StashFiles => stash_why(stash).map_or(Can::Yes, Can::No),
            _ => Can::Yes,
        },
        // `drop` below every step leaves a fold nowhere to land: git stops
        // on such a todo (`RebasePlanModel`'s `fold_lands`, the line its
        // demotion of a stranded fold holds too).
        On::Step(below) => match row {
            Row::StepSquash | Row::StepFixup => {
                first([(below == Below::Dropped, Why::NothingToFoldInto)])
            }
            _ => Can::Yes,
        },
        On::Remote { marked } => match row {
            Row::MarkOrigin => first([(marked, Why::AlreadyOrigin)]),
            _ => Can::Yes,
        },
    }
}

/// What `row` of a ref's or a graph row's class can do on `facts`.
fn ref_can(row: Row, facts: &Facts<'_>) -> Can {
    let Facts { shape, state } = facts;
    let tie = shape.tie();
    let unborn = state.commit.is_none();
    let standing = standing(shape, state);
    let current = tie == Tie::Head;
    let held = state.held;
    // Nothing else holds a branch a stopped operation here keeps — HEAD is
    // on none, and git keeps a branch to one worktree —, so its reason is
    // said first on the rows that take the branch itself (its deletes,
    // `worktree add`). The rows that move it say the operation, as they do
    // for HEAD's branch (`Why::StoppedOperation`).
    let kept = match tie {
        Tie::Kept(by) => Some(Can::No(Why::KeptBy(by))),
        Tie::Head | Tie::Apart { .. } => None,
    };
    match row {
        Row::BranchHere | Row::TagHere => first([
            (unborn, Why::NoCommitsYet),
            (standing, Why::StoppedOperation),
        ]),
        // Nothing in this worktree is touched: mid-operation too.
        Row::WorktreeHere => first_given([
            unborn.then_some(Why::NoCommitsYet),
            state.placing.none_why(),
        ]),
        // What stands in the move's way presses through to a question.
        Row::Reach | Row::DropStash | Row::UnsetUpstream | Row::DeleteTag => Can::Yes,
        // What stands in a stash's way — an unmerged index, local changes
        // it would overwrite — is git's to say, on the bar, as for the
        // details pane's band (`StashActionsBand`).
        Row::ApplyStash | Row::PopStash => Can::Yes,
        Row::CherryPick
        | Row::Revert
        | Row::Squash
        | Row::Drop
        | Row::RebaseInteractive
        | Row::Merge
        | Row::Rebase
        | Row::ResetSoft
        | Row::ResetMixed
        | Row::ResetHard => history(row, state, standing),
        Row::Pull => pull(shape, state),
        // A branch with no commit has no ref, so no listing names it and its
        // answer stays closed until the first commit: that is the reason.
        Row::Push if unborn => Can::No(Why::NoCommitsYet),
        // Closed answers no press: a plain one would send with nothing asked
        // where the branch goes (`asks` is `Publish`'s alone). The listing
        // catching up with the tree answers it.
        Row::Push => match state.push {
            PushStanding::Closed => Can::No(Why::Unread),
            PushStanding::Unborn => Can::No(Why::NoCommitsYet),
            PushStanding::StaysHere => Can::No(Why::StaysHere),
            PushStanding::Clean => Can::No(Why::NothingToSend),
            PushStanding::Publish
            | PushStanding::Elsewhere
            | PushStanding::Ready
            | PushStanding::Behind
            | PushStanding::Diverged => Can::Yes,
        },
        Row::DeleteBranch | Row::DeleteRemoteBranch | Row::DeleteBoth => {
            delete_branch(row, shape, state, kept)
        }
        Row::SetUpstream => first([
            (unborn, Why::NoCommitsYet),
            (!state.has_remote, Why::NoRemote),
        ]),
        Row::WorktreeAdd => {
            let remote = shape.kind == Some(RefKind::Remote);
            kept.unwrap_or(first_given([
                current.then_some(Why::OnThisBranch),
                (remote && state.namesake_there()).then_some(Why::LocalBranchThere),
                (!remote && held).then_some(Why::CheckedOutElsewhere),
                state.placing.none_why(),
                (state.placing == Placing::Beside { taken: true }).then_some(Why::PlaceTaken),
            ]))
        }
        // The lock first: the one reason the reader has to go and undo.
        Row::RemoveWorktree => first([
            (state.locked, Why::Locked),
            (
                shape.worktree.is_some_and(|worktree| worktree.here),
                Why::ShownHere,
            ),
        ]),
        Row::PushTag => first([(!state.tag_remote, Why::NoRemote)]),
        Row::DeleteRemoteTag | Row::DeleteTagBoth => first([
            (state.tag_held == TagDeleteHeld::Drifted, Why::TagDrifted),
            (state.tag_held == TagDeleteHeld::Unnamed, Why::TagUnnamed),
            (!state.tag_remote, Why::NoRemote),
        ]),
        // No class of a ref or a graph row has these: `Class::of` makes no
        // carried row's, files', step's or remote's class, and `can` reads
        // their own facts.
        Row::ApplyChanges
        | Row::KeepOurs
        | Row::TakeTheirs
        | Row::MergeTool
        | Row::StashFiles
        | Row::Discard
        | Row::StepPick
        | Row::StepReword
        | Row::StepEdit
        | Row::StepSquash
        | Row::StepFixup
        | Row::StepDrop
        | Row::RemoteUrl
        | Row::MarkOrigin => Can::Yes,
    }
}

/// The rows that write on HEAD or move the branch the tab moves: greyed
/// while an operation stands (`standing`), where HEAD's branch has no
/// commits to undo, rebase or rewrite yet, on a commit a rewrite cannot
/// reach, and where the press would do nothing on a commit HEAD's branch
/// has already — HEAD's own, which nothing is brought in from or moved to,
/// or one behind it, which a merge brings nothing in from and a rebase
/// moves nothing onto down a straight line.
fn history(row: Row, state: &State<'_>, standing: bool) -> Can {
    let tree = state.tree;
    let unborn = state.commit.is_none();
    let head_unborn = tree.head_unborn();
    let on_head = state
        .commit
        .is_some_and(|commit| tree.is_head(&commit.to_hex()));
    let in_history = state.history.is_some_and(History::holds);
    let straight = state.history == Some(History::Straight);
    match row {
        // It only adds a commit, on any HEAD — an unborn one takes it as its
        // first. One behind HEAD is not said: what git makes of it is in
        // the lines since (`Why::InHistory`).
        Row::CherryPick => first([
            (standing, Why::StoppedOperation),
            (on_head, Why::SameCommit),
        ]),
        // Into an unborn branch it takes the branch to the commit.
        Row::Merge => first([
            (unborn, Why::NoCommitsYet),
            (standing, Why::StoppedOperation),
            (on_head, Why::SameCommit),
            (in_history, Why::InHistory),
        ]),
        Row::Rebase => first([
            (unborn, Why::NoCommitsYet),
            (standing, Why::StoppedOperation),
            (head_unborn, Why::HeadUnborn),
            (on_head, Why::SameCommit),
            (straight, Why::InHistory),
        ]),
        Row::ResetSoft | Row::ResetMixed | Row::ResetHard => first([
            (standing, Why::StoppedOperation),
            (on_head, Why::AlreadyThere),
        ]),
        // They rewrite what HEAD's branch has, HEAD's commit included — an
        // unborn branch has nothing, and a commit off its history is not
        // theirs to rewrite.
        Row::Squash | Row::Drop | Row::RebaseInteractive => first([
            (standing, Why::StoppedOperation),
            (head_unborn, Why::HeadUnborn),
            (state.history == Some(History::Off), Why::OffBranch),
        ]),
        // Revert, the rest `ref_can` hands here: it undoes any commit on top
        // of HEAD — an unborn branch has nothing to undo it on.
        _ => first([
            (standing, Why::StoppedOperation),
            (head_unborn, Why::HeadUnborn),
        ]),
    }
}

/// The rows that take a branch's name away — the local branch, the remote
/// branch it reads, or both —, `kept` the reason a stopped operation's hold
/// on the branch gives first.
fn delete_branch(row: Row, shape: &Shape<'_>, state: &State<'_>, kept: Option<Can>) -> Can {
    let current = shape.tie() == Tie::Head;
    match row {
        // The reading goes on its own: what holds the local branch back
        // holds nothing over there, and a remote row's own delete reaches it
        // whatever stands here.
        Row::DeleteRemoteBranch => first([(
            shape.kind == Some(RefKind::Branch) && state.drifted,
            Why::ReadingDrifted,
        )]),
        // The local half's reasons first, since `DeleteBoth` runs it first.
        _ => kept.unwrap_or(first([
            (current, Why::OnThisBranch),
            (state.held, Why::CheckedOutElsewhere),
            (row == Row::DeleteBoth && state.drifted, Why::ReadingDrifted),
        ])),
    }
}

/// An operation stands: the badge says so, or HEAD's reading names a branch
/// a stopped rebase or bisect keeps. The two land apart, HEAD's first, so
/// the rows that move the kept branch grey off the same reading that makes
/// them ([`Shape::moved`]), whichever lands first.
fn standing(shape: &Shape<'_>, state: &State<'_>) -> bool {
    state.tree.op_standing() || matches!(shape.moved, Some(MovedBranch::Kept { .. }))
}

/// What the `pull` row runs where the class has one: `git pull` on either
/// end of the tab's own comparison — the tab's branch, or the remote branch
/// it reads —, else the row's own branch fast-forwarded to its upstream.
pub fn pull_runs(shape: &Shape<'_>) -> PullRuns {
    if shape.tie() == Tie::Head || shape.kind == Some(RefKind::Remote) {
        PullRuns::Here
    } else {
        PullRuns::Aside
    }
}

/// The `pull` row: greyed by what stands in this tree where it runs here,
/// and where it runs aside by what holds that branch first. Nothing is
/// pulled while an operation stands (user decision — ask before changing).
/// Every judgement is by counts alone, as of the last fetch: a far side
/// that moved since leaves the row live, git refuses the press, and the log
/// comes up. An upstream no row of the listing answers to has no row at all
/// ([`super::UpstreamShape::Gone`]): where git has no ref by its name
/// either, there is nothing to pull from as of the last fetch — git answers
/// `no such ref was fetched` (measured on 2.55) —, and a fetch that finds
/// the branch made again reads it back.
fn pull(shape: &Shape<'_>, state: &State<'_>) -> Can {
    let standing = standing(shape, state);
    match pull_runs(shape) {
        // Over a diverged tree the operation is what is said.
        PullRuns::Here => first([
            (standing, Why::StoppedOperation),
            (state.tree.diverged, Why::Diverged),
        ]),
        // The holder first: nothing done here lifts it — the pull is done
        // in its tab. The operation is finished here, and own commits are
        // taken in from the branch itself, which the reader cannot switch
        // to from here.
        PullRuns::Aside => first([
            (state.held, Why::CheckedOutElsewhere),
            (standing, Why::StoppedOperation),
            (state.ahead > 0, Why::OwnCommits),
        ]),
    }
}
