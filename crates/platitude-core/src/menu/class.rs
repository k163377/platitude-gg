//! What decides which rows a menu has: the subject's kind and the parts it
//! is made of, read off what is there and nothing of its state.

use super::line::On;
use super::row::{Part, Row};
use crate::offers::{MovedBranch, RefKind, TagSides, Tracks};
use crate::worktrees::Keeper;

/// What is there: the row right-clicked, where it stands against the tab,
/// and what of it exists, here or over there as last read. The only facts
/// [`Class::of`] reads — anything else, which can only grey a row, belongs
/// in [`super::State`], which the rows never see.
#[derive(Debug, Clone, Copy)]
pub struct Shape<'a> {
    /// Which row was right-clicked.
    pub seen: Seen,
    /// The ref the row names; `None` on a graph row that names none, which
    /// is its commit alone.
    pub kind: Option<RefKind>,
    /// What the row is asked by: a branch's, a remote branch's or a tag's
    /// name as the chip wears it, a stash's selector, a worktree's path.
    pub name: &'a str,
    /// The branch `merge`, `rebase`, the rewrites and `reset` move, whose
    /// name their rows say, and how the tab holds it: HEAD's, or — while a
    /// stopped rebase or bisect keeps HEAD detached — the one it keeps
    /// (`TreeStanding::moved`). `None` while the tab moves no branch.
    pub moved: Option<MovedBranch<'a>>,
    /// The far end of the comparison the window shows as ↑↓: what the
    /// branch HEAD is on tracks (`origin/main`), empty for nothing. Read
    /// only while HEAD is on a branch (`compared`): a stopped operation's
    /// kept branch compares nothing, whatever it tracks.
    pub tab_upstream: &'a str,
    /// A local branch's configured upstream, where the row is one that has
    /// one.
    pub upstream: Option<UpstreamShape>,
    /// Which sides of a tag hold the name; `None` where nobody has
    /// answered, which reads as a tag here.
    pub tag_sides: Option<TagSides>,
    /// The worktree the row's WORKTREE card stands on: a WORKTREES row's
    /// own, or the one that has a local branch out.
    pub worktree: Option<WorktreeShape>,
}

/// Which row was right-clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    /// A ref's row of the left pane: the menu acts on its name.
    Listed,
    /// A commit's row on the graph: the first level acts on the commit as
    /// well, the cards on the name the chip draws (デザイン規約
    /// §グラフ行の右クリック).
    Graph {
        /// The discard log's picked entry draws it, a display mode of its
        /// own (user decision — ask before changing): what the mode can
        /// never do has no row, not a greyed one, and what stays greys as
        /// anywhere (デザイン規約 §メニュー). It can do only what adds and
        /// leaves every ref where it is — `cherry-pick` and a new name on the
        /// commit —, aimed at no name. `revert` would undo what the branch
        /// never had; `squash` / `drop` are refused off the branch, and on a
        /// commit still in it (an entry that took only a name) would rewrite
        /// what the entry is not about; `merge` / `rebase` / `reset` would
        /// take the discard back by moving a ref (破棄記録仕様.md「復元は追加
        /// の操作」).
        provisional: bool,
    },
}

impl Seen {
    /// A ref's row by the word its entrance hands `GitFacts.menuOn`:
    /// `listed`, `graph`, or `provisional` for the discard log's entry.
    /// `None` for any other word — another worktree's row has an entrance
    /// of its own (`GitFacts.carriedMenu`).
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "listed" => Some(Self::Listed),
            "graph" => Some(Self::Graph { provisional: false }),
            "provisional" => Some(Self::Graph { provisional: true }),
            _ => None,
        }
    }
}

/// Where a local branch's configured upstream stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamShape {
    /// A remote branch the listing has: the one this branch reads, which
    /// its delete rows reach and its pull fetches.
    Read,
    /// A branch of this repository the listing has (`branch.<n>.remote =
    /// .`): `git pull` merges it into the branch HEAD is on, and the
    /// fast-forward aside reaches a remote only.
    Here,
    /// One with no ref: nothing to pull from, as for a branch with none —
    /// the push asks where the branch goes (`Push to…`) the same way (user
    /// decision — ask before changing). Taking it off is still there to do.
    /// On another branch than HEAD's, one no row of the listing answers to
    /// ([`Tracks::Gone`]): an upstream fetched outside the remote branches
    /// reads so too, though git has its ref — the listing cannot tell the
    /// two apart, and its push reads it so as well (that configuration gets
    /// no mechanism of its own: `RefsSnapshot::branch_push`).
    Gone,
    /// The branch HEAD is on has one git counts it against — the window's
    /// ↑↓ — and no row of the listing answers to: one the listing has not
    /// read yet (the two land apart, the tree first), or one fetched
    /// outside the remote branches, which it never holds. Its `git pull`
    /// stands, as the toolbar's push does on the same count. A branch
    /// with no commit yet stands so with no count — git counts nothing
    /// against it, and its pull is how it takes the upstream's first
    /// commits.
    Counted,
}

impl UpstreamShape {
    /// Where the listing says a branch's upstream stands; `None` for none.
    pub fn listed(tracks: Tracks) -> Option<Self> {
        match tracks {
            Tracks::Nothing => None,
            Tracks::Together | Tracks::Drifted => Some(Self::Read),
            Tracks::Here => Some(Self::Here),
            Tracks::Gone => Some(Self::Gone),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreeShape {
    /// The repository's own, which git never removes.
    pub main: bool,
    /// The one this tab shows.
    pub here: bool,
}

/// How the row's name stands against the branch the tab moves: the one
/// reading of it ([`Shape::tie`]), which the class and the lines both take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tie {
    /// The row names the branch HEAD is on.
    Head,
    /// The row names the branch a stopped rebase or bisect keeps
    /// ([`MovedBranch::Kept`]), and says which of them keeps it.
    Kept(Keeper),
    /// The row names anything else; `moves` whether the tab moves a branch
    /// at all.
    Apart { moves: bool },
}

impl Tie {
    /// Where the row's branch stands against the branch the tab moves, which
    /// operation keeps it left out: what the class reads of the tie.
    pub fn stand(self) -> Stand {
        match self {
            Self::Head => Stand::Here,
            Self::Kept(_) => Stand::Kept,
            Self::Apart { moves } => Stand::Apart { moves },
        }
    }
}

impl Shape<'_> {
    /// How the row's name stands against the branch the tab moves. Only a
    /// local branch's row names it: a tag or a remote branch of the same
    /// name is another thing.
    pub fn tie(&self) -> Tie {
        let names = |name: &str| self.kind == Some(RefKind::Branch) && name == self.name;
        match self.moved {
            Some(MovedBranch::Head(name)) if names(name) => Tie::Head,
            Some(MovedBranch::Kept { name, by }) if names(name) => Tie::Kept(by),
            moved => Tie::Apart {
                moves: moved.is_some(),
            },
        }
    }

    /// The row's name is the far end of the comparison the window shows:
    /// the remote branch the branch HEAD is on tracks.
    fn compared(&self) -> bool {
        matches!(self.moved, Some(MovedBranch::Head(_)))
            && !self.tab_upstream.is_empty()
            && self.name == self.tab_upstream
    }
}

/// What the rows are decided by. Each field is a distinction that adds or
/// takes parts ([`Part`]), free of the others; a variant is a case that
/// excludes the others. Built in one place, [`Class::on`]: a ref's or a
/// graph row's off its [`Shape`] alone, the others off what is there of
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A local branch: its BRANCHES row, the WORKTREES row of a worktree
    /// that has it out, or a graph row whose chip draws it.
    Branch(BranchClass),
    Remote(RemoteClass),
    Tag(TagClass),
    /// A stash's row, of the left pane or the graph: on no branch's history,
    /// so no commit's row applies.
    Stash,
    /// The WORKTREES row of a worktree with no branch out, or a graph row
    /// whose chip draws one.
    Worktree(WorktreeClass),
    /// A graph row that names no ref: its commit alone — copied and undone
    /// on HEAD and, where the tab moves a branch (`moves`), brought into
    /// it, rewritten over and taken to.
    Commit {
        moves: bool,
    },
    /// The discard log's entry, a display mode of its own
    /// (`Seen::Graph::provisional`): [`Part::Position`] and [`Part::Copied`]
    /// alone, aimed at no name, whatever the tab moves.
    Provisional,
    /// Another worktree's uncommitted row on the graph: that worktree's door
    /// and its work brought here, and nothing that takes the worktree away
    /// (user decision — ask before changing).
    Carried,
    /// The files chosen in the working tree pane: the rows of every section
    /// the choice reaches, each acting on its own section's files.
    Files(Chosen),
    /// A step of an interactive rebase's plan.
    Step(StepClass),
    /// A remote of the REMOTES section — the remote itself, not a branch of
    /// it.
    RemoteConfig,
}

/// What the files chosen in the working tree pane are: conflicted files
/// ([`Part::Conflicted`]), changes in no conflict ([`Part::Restorable`]), or
/// both. A choice of none opens no menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    Conflicts,
    Changes,
    Both,
}

impl Chosen {
    /// The choice that holds conflicted files where `conflicted`, changes in
    /// no conflict where `changed`; `None` for neither.
    pub fn of(conflicted: bool, changed: bool) -> Option<Self> {
        match (conflicted, changed) {
            (true, false) => Some(Self::Conflicts),
            (false, true) => Some(Self::Changes),
            (true, true) => Some(Self::Both),
            (false, false) => None,
        }
    }

    fn conflicted(self) -> bool {
        self != Self::Changes
    }

    fn changed(self) -> bool {
        self != Self::Conflicts
    }
}

/// What stands below a plan's step: no step (the oldest), only steps
/// dropped from the history, or one that stays — the one a fold lands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Below {
    Nothing,
    Dropped,
    Landing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepClass {
    /// Another step stands below it ([`Part::Fold`]).
    pub below: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchClass {
    pub stand: Stand,
    /// Its configured upstream ([`Part::Upstream`]), and where that stands:
    /// something to pull from ([`Part::Pulled`]), and a remote branch it
    /// reads ([`Part::RemoteBranch`], [`Part::BranchAndReading`]).
    pub upstream: Option<UpstreamShape>,
    pub worktree: Option<WorktreeKind>,
    /// A graph row's chip draws it: its commit's rows stand there too, as
    /// [`Class::Commit`]'s.
    pub on_graph: bool,
}

/// Where a branch stands against the branch the tab moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stand {
    /// The branch HEAD is on.
    Here,
    /// The branch a stopped rebase or bisect keeps: the tab moves it while
    /// HEAD is on none ([`Part::Elsewhere`], [`Part::Sent`]), and nothing is
    /// brought into itself.
    Kept,
    /// Any other ([`Part::Elsewhere`], [`Part::Sent`]), brought into the
    /// branch the tab moves where it moves one ([`Part::IntoHere`]).
    Apart { moves: bool },
}

impl Stand {
    /// The tab moves a branch: HEAD's, the kept one, or one the row does
    /// not name.
    pub fn moves(self) -> bool {
        match self {
            Self::Here | Self::Kept => true,
            Self::Apart { moves } => moves,
        }
    }
}

/// A remote branch's row, which leads to a worktree through its local
/// namesake but stands on none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteClass {
    pub stand: RemoteStand,
    /// A graph row's chip draws it.
    pub on_graph: bool,
}

/// Where a remote branch stands against the branch the tab moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteStand {
    /// The tab moves no branch.
    Still,
    /// The tab moves a branch, which it is brought into
    /// ([`Part::IntoHere`]).
    Moves,
    /// The far end of the comparison the window shows as ↑↓ — the remote
    /// branch the branch HEAD is on tracks —: brought into it, and pulled
    /// ([`Part::IntoHere`], [`Part::Pulled`]). While a stopped rebase or
    /// bisect keeps a branch, HEAD is on none and the window compares
    /// nothing: the kept branch's pull is its own row's.
    Compared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagClass {
    /// The tab moves a branch ([`Part::IntoHere`]).
    pub moves: bool,
    /// [`Part::TagHere`] here, [`Part::TagFar`] over there, both and
    /// [`Part::TagBoth`] on both.
    pub sides: TagSides,
    /// A graph row's chip draws it.
    pub on_graph: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreeClass {
    /// The tab shows it: nowhere else to go ([`Part::Elsewhere`] otherwise),
    /// and never brought into the branch the tab moves, as a branch's own
    /// row brings nothing into itself.
    pub here: bool,
    /// The tab moves a branch, which another worktree is brought into
    /// ([`Part::IntoHere`]).
    pub moves: bool,
    pub kind: WorktreeKind,
    /// A graph row's chip draws it.
    pub on_graph: bool,
}

/// The worktree a row's WORKTREE card stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeKind {
    /// The repository's own.
    Main,
    /// Any other ([`Part::OtherWorktree`]).
    Other,
}

impl WorktreeKind {
    fn of(shape: WorktreeShape) -> Self {
        if shape.main { Self::Main } else { Self::Other }
    }
}

impl Class {
    /// The class of what the menu is opened on. `None` where it offers
    /// nothing ([`Class::of`]).
    pub fn on(on: &On<'_>) -> Option<Self> {
        match *on {
            On::Ref(facts) => Self::of(&facts.shape),
            On::Carried { .. } => Some(Self::Carried),
            On::Files { chosen, .. } => Some(Self::Files(chosen)),
            On::Step(below) => Some(Self::Step(StepClass {
                below: below != Below::Nothing,
            })),
            On::Remote { .. } => Some(Self::RemoteConfig),
        }
    }

    /// The class of what `shape` says is there. `None` for a row of the
    /// left pane that names nothing, or a worktree's whose worktree is not
    /// read, which offer nothing; a graph row's commit offers its rows
    /// meanwhile.
    pub fn of(shape: &Shape<'_>) -> Option<Self> {
        let tie = shape.tie();
        let moves = tie.stand().moves();
        let on_graph = match shape.seen {
            Seen::Listed => false,
            Seen::Graph { provisional: false } => true,
            Seen::Graph { provisional: true } => return Some(Self::Provisional),
        };
        let commit = on_graph.then_some(Self::Commit { moves });
        let Some(kind) = shape.kind else {
            return commit;
        };
        Some(match kind {
            RefKind::Branch => Self::Branch(BranchClass {
                stand: tie.stand(),
                upstream: shape.upstream,
                worktree: shape.worktree.map(WorktreeKind::of),
                on_graph,
            }),
            RefKind::Remote => Self::Remote(RemoteClass {
                stand: if shape.compared() {
                    RemoteStand::Compared
                } else if moves {
                    RemoteStand::Moves
                } else {
                    RemoteStand::Still
                },
                on_graph,
            }),
            RefKind::Tag => Self::Tag(TagClass {
                moves,
                sides: shape.tag_sides.unwrap_or(TagSides::Here),
                on_graph,
            }),
            RefKind::Stash => Self::Stash,
            RefKind::Worktree => {
                // A chip naming a worktree the listing has not read yet: the
                // graph row stands on its commit until it is read.
                let Some(own) = shape.worktree else {
                    return commit;
                };
                Self::Worktree(WorktreeClass {
                    here: own.here,
                    moves,
                    kind: WorktreeKind::of(own),
                    on_graph,
                })
            }
        })
    }

    /// The parts the class is made of: its own case's, and a graph row's
    /// commit's where one is drawn.
    pub fn parts(&self) -> Vec<Part> {
        let mut parts = Vec::new();
        let mut add = |part: Part, there: bool| {
            if there && !parts.contains(&part) {
                parts.push(part);
            }
        };
        if let Some(moves) = self.own_parts(&mut add) {
            add(Part::Position, true);
            add(Part::Copied, true);
            add(Part::Undone, true);
            add(Part::IntoHere, moves);
            add(Part::Rewritten, moves);
            add(Part::MovedTo, moves);
        }
        parts
    }

    /// The parts the class's own case adds. Each case is taken apart whole,
    /// so a distinction added to a class does not compile until it says
    /// which parts it adds. Answers the graph row's commit where one is
    /// drawn, as whether the tab moves a branch.
    fn own_parts(&self, add: &mut impl FnMut(Part, bool)) -> Option<bool> {
        match *self {
            Self::Branch(BranchClass {
                stand,
                upstream,
                worktree,
                on_graph,
            }) => {
                add(Part::Position, true);
                add(Part::LocalBranch, true);
                add(Part::Checkout, true);
                let elsewhere = stand != Stand::Here;
                add(Part::Elsewhere, elsewhere);
                add(Part::Sent, elsewhere);
                add(Part::IntoHere, stand == Stand::Apart { moves: true });
                let read = upstream == Some(UpstreamShape::Read);
                // `git pull` takes any upstream it has a ref for into the
                // branch HEAD is on — and into the one a stopped operation
                // keeps, once it is done: the row stands for it, greyed while
                // the operation does —; the fast-forward aside, a remote's
                // only.
                let pulled_here =
                    matches!(upstream, Some(UpstreamShape::Here | UpstreamShape::Counted))
                        && matches!(stand, Stand::Here | Stand::Kept);
                add(Part::Upstream, upstream.is_some());
                add(Part::Pulled, read || pulled_here);
                add(Part::RemoteBranch, read);
                add(Part::BranchAndReading, read);
                add(Part::OtherWorktree, worktree == Some(WorktreeKind::Other));
                on_graph.then_some(stand.moves())
            }
            Self::Remote(RemoteClass { stand, on_graph }) => {
                add(Part::Position, true);
                add(Part::Elsewhere, true);
                add(Part::RemoteBranch, true);
                add(Part::Checkout, true);
                let moves = stand != RemoteStand::Still;
                add(Part::IntoHere, moves);
                add(Part::Pulled, stand == RemoteStand::Compared);
                on_graph.then_some(moves)
            }
            Self::Tag(TagClass {
                moves,
                sides,
                on_graph,
            }) => {
                add(Part::Position, true);
                add(Part::IntoHere, moves);
                add(
                    Part::TagHere,
                    matches!(sides, TagSides::Here | TagSides::Both),
                );
                add(
                    Part::TagFar,
                    matches!(sides, TagSides::Remote | TagSides::Both),
                );
                add(Part::TagBoth, sides == TagSides::Both);
                on_graph.then_some(moves)
            }
            Self::Stash => {
                add(Part::Stash, true);
                None
            }
            Self::Worktree(WorktreeClass {
                here,
                moves,
                kind,
                on_graph,
            }) => {
                add(Part::Position, true);
                add(Part::Elsewhere, !here);
                add(Part::IntoHere, moves && !here);
                add(Part::OtherWorktree, kind == WorktreeKind::Other);
                on_graph.then_some(moves)
            }
            Self::Commit { moves } => Some(moves),
            Self::Provisional => {
                add(Part::Position, true);
                add(Part::Copied, true);
                None
            }
            Self::Carried => {
                add(Part::Elsewhere, true);
                add(Part::Carried, true);
                None
            }
            Self::Files(chosen) => {
                add(Part::Changes, true);
                add(Part::Conflicted, chosen.conflicted());
                add(Part::Restorable, chosen.changed());
                None
            }
            Self::Step(StepClass { below }) => {
                add(Part::Step, true);
                add(Part::Fold, below);
                None
            }
            Self::RemoteConfig => {
                add(Part::RemoteConfig, true);
                None
            }
        }
    }

    /// The menu's rows, in drawing order: every row of every part the class
    /// has, and no other.
    pub fn rows(&self) -> Vec<Row> {
        let parts = self.parts();
        Row::ALL
            .iter()
            .copied()
            .filter(|row| parts.contains(&row.part()))
            .collect()
    }
}
