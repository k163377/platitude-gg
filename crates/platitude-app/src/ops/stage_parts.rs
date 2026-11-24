//! The two presses [`super::StageAhead`] holds beside a whole file's stage:
//! part of one file's diff — a hunk or a line staged, unstaged or discarded
//! — and a discard of chosen rows (デザイン規約 §diff の中のステージ,
//! §消す操作は先に画面から消す). What each does to a status row, and where
//! a part stands in the diff it was made on.

use std::collections::HashMap;

use platitude_core::stage::DiscardSide;
use platitude_core::status::StatusItem;

pub use platitude_core::parts::Moves;

/// What a part of a diff is pressed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartVerb {
    /// `git apply --cached` of part of the unstaged diff.
    Stage,
    /// The same in reverse, of part of the staged diff.
    Unstage,
    /// `git apply --reverse` of part of the unstaged diff, on the disk alone.
    Discard,
}

impl PartVerb {
    #[must_use]
    pub fn moves(self) -> Moves {
        match self {
            Self::Stage => Moves::Old,
            Self::Unstage | Self::Discard => Moves::New,
        }
    }

    /// The bucket the part's file row is pressed from.
    fn bucket(self) -> &'static str {
        match self {
            Self::Stage | Self::Discard => "unstaged",
            Self::Unstage => "staged",
        }
    }
}

/// Where a part stands in the diff it was pressed on: the diff named by
/// `key` (`encode::diff_key`) as read with `fingerprint`, its hunk, and one
/// line of that hunk or (`None`) the whole of it — the indices
/// `platitude_core::patch::HunkSelect` addresses, in the drawing the press
/// was made on: the read with the parts pressed on it before this one in it
/// (`platitude_core::parts::after`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSpot {
    pub key: String,
    pub fingerprint: u64,
    pub hunk: usize,
    pub line: Option<usize>,
}

/// One press on part of one file's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub verb: PartVerb,
    pub path: String,
    /// The press takes every change the diff shows, and the diff is one
    /// that a part can empty: a file's edit, no mode change, no file made or
    /// taken away (`DiffModel::takes_all`). Only then does the file's row
    /// leave the side it was pressed from.
    pub whole: bool,
    pub spot: DiffSpot,
}

/// What a press makes of one status row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    Row(StatusItem),
    /// The row leaves the list — both its sides went.
    Gone,
}

fn tracked(staged: char, unstaged: char, path: &str, orig_path: Option<&String>) -> Next {
    Next::Row(StatusItem::Tracked {
        staged,
        unstaged,
        path: path.to_string(),
        orig_path: orig_path.cloned(),
    })
}

impl Part {
    /// The row `item` becomes, where the row and the diff alone decide it;
    /// `None` leaves it to the status (or nothing about it moves).
    ///
    /// Decided: an edit on the pressed side whose other side is HEAD's (`.M`
    /// staged, `M.` unstaged) — a part leaves a change on both sides, the
    /// whole of it moves the row; a staged addition with an edit on top
    /// (`AM`) staged whole — the staged letter is HEAD's to say, and HEAD
    /// has no such file; and a discard of the whole of an edit, which leaves
    /// the index where it was (`.M` goes, `MM` / `AM` / `RM` … keep their
    /// staged half).
    ///
    /// Left to the status:
    /// - **the staged letter of a row on both sides** (`MM` staged or
    ///   unstaged): what the index ends up holding against HEAD is a
    ///   comparison of contents — the part may put it back to HEAD's
    /// - **a staged rename or copy** unstaged in part: whether it stays one
    ///   is git's similarity count
    /// - **a deletion, a file made or a mode change** (never `whole` —
    ///   [`Part::whole`]): a part moves neither the mode nor the file's
    ///   being there
    #[must_use]
    pub fn next(&self, item: &StatusItem) -> Option<Next> {
        let StatusItem::Tracked {
            staged,
            unstaged,
            path,
            orig_path,
        } = item
        else {
            return None;
        };
        if *path != self.path {
            return None;
        }
        let orig = orig_path.as_ref();
        match (self.verb, *staged, *unstaged, self.whole) {
            (PartVerb::Stage, '.', 'M', true) => Some(tracked('M', '.', path, None)),
            (PartVerb::Stage, '.', 'M', false) => Some(tracked('M', 'M', path, None)),
            (PartVerb::Stage, 'A', 'M', true) => Some(tracked('A', '.', path, None)),
            (PartVerb::Unstage, 'M', '.', true) => Some(tracked('.', 'M', path, None)),
            (PartVerb::Unstage, 'M', '.', false) => Some(tracked('M', 'M', path, None)),
            (PartVerb::Discard, '.', 'M', true) => Some(Next::Gone),
            (PartVerb::Discard, letter @ ('M' | 'A' | 'R' | 'C' | 'T'), 'M', true) => {
                Some(tracked(letter, '.', path, orig))
            }
            _ => None,
        }
    }

    /// Whether `path`'s row in `bucket` leaves its side with the status
    /// after the press: the whole of the side pressed from, emptied.
    #[must_use]
    pub fn leaves(&self, bucket: &str, path: &str) -> bool {
        self.whole && path == self.path && bucket == self.verb.bucket()
    }
}

/// The rows a discard of chosen rows was pressed on, by path, each with
/// the sides it was chosen on (a file changed on both sides has a row in
/// each bucket).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discard {
    sides: HashMap<String, Vec<DiscardSide>>,
}

impl Discard {
    #[must_use]
    pub fn new(chosen: &[(String, DiscardSide)]) -> Self {
        let mut sides: HashMap<String, Vec<DiscardSide>> = HashMap::new();
        for (path, side) in chosen {
            sides.entry(path.clone()).or_default().push(*side);
        }
        Self { sides }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sides.is_empty()
    }

    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.sides.keys()
    }

    #[must_use]
    pub fn reaches(&self, path: &str) -> bool {
        self.sides.contains_key(path)
    }

    /// Whether git is to write the index: a staged row goes back to HEAD on
    /// both sides (`git restore --staged --worktree`).
    #[must_use]
    pub fn moves_index(&self) -> bool {
        self.sides
            .values()
            .flatten()
            .any(|s| *s == DiscardSide::Staged)
    }

    fn chose(&self, path: &str, side: DiscardSide) -> bool {
        self.sides
            .get(path)
            .is_some_and(|sides| sides.contains(&side))
    }

    /// The row `item` becomes, where the row alone decides it; `None` leaves
    /// it to the status. `dir` answers whether a path is a directory on disk.
    ///
    /// Decided: a staged row back to HEAD leaves the list with both its
    /// sides (a staged rename takes its old name along — core reads that
    /// inside the write, `stage::with_old_names`), an unstaged edit,
    /// deletion or type change goes back to the index (the row keeps its
    /// staged half, or leaves), and an untracked file goes.
    ///
    /// Left to the status:
    /// - **a directory**: a gitlink's checkout is not restored by `git
    ///   restore`, and `git clean` skips a repository of its own (`nest/`)
    /// - **an intent-to-add file** (`.A`): its index entry holds no content
    ///   to go back to
    /// - **a conflict**: never chosen (`ops_stage::chosen_row`)
    #[must_use]
    pub fn next(&self, item: &StatusItem, dir: &mut dyn FnMut(&str) -> bool) -> Option<Next> {
        match item {
            StatusItem::Untracked { path } => (self.chose(path, DiscardSide::Untracked)
                && !path.ends_with('/'))
            .then_some(Next::Gone),
            StatusItem::Tracked {
                staged,
                unstaged,
                path,
                orig_path,
            } => {
                if dir(path) {
                    return None;
                }
                if *staged != '.' && self.chose(path, DiscardSide::Staged) {
                    return Some(Next::Gone);
                }
                if !matches!(unstaged, 'M' | 'D' | 'T') || !self.chose(path, DiscardSide::Unstaged)
                {
                    return None;
                }
                Some(match staged {
                    '.' => Next::Gone,
                    letter => tracked(*letter, '.', path, orig_path.as_ref()),
                })
            }
            _ => None,
        }
    }

    /// Whether `path`'s row in `bucket` leaves its side with the status
    /// after the press — a staged row's both sides, and the side chosen.
    /// Not a directory: [`Self::next`] leaves those where they are.
    #[must_use]
    pub fn leaves(&self, bucket: &str, path: &str) -> bool {
        use crate::encode::Routing;
        match Routing::of_word(bucket) {
            Some(Routing::Staged) => self.chose(path, DiscardSide::Staged),
            Some(Routing::Unstaged) => {
                self.chose(path, DiscardSide::Unstaged) || self.chose(path, DiscardSide::Staged)
            }
            Some(Routing::Untracked) => {
                self.chose(path, DiscardSide::Untracked) && !path.ends_with('/')
            }
            Some(Routing::Conflicts) | None => false,
        }
    }
}
