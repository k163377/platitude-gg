//! The file rows a stage or unstage has moved on the screen while git is
//! still doing it (デザイン規約 §バケツごとの一覧と、ペインの分け方): which
//! presses are out, which status answers each, and what a status looks like
//! with them in it.

use std::collections::{HashMap, HashSet};

use platitude_core::session::EolMark;
use platitude_core::status::{StatusItem, WorktreeStatus};

/// Which way a press moves rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// `git add`: an unstaged or untracked row goes to the staged bucket.
    Stage,
    /// `git restore --staged` (`git reset` for the whole bucket, `git rm
    /// --cached` before the first commit): a staged row goes back.
    Unstage,
}

impl Side {
    /// Whether a press this way takes rows from `bucket`, as the lists name
    /// the buckets: staging from the unstaged heading's and the conflicts
    /// (`git add` marks those resolved), unstaging from the staged one's.
    fn takes_from(self, bucket: &str) -> bool {
        match self {
            Side::Stage => matches!(bucket, "unstaged" | "untracked" | "conflicts"),
            Side::Unstage => bucket == "staged",
        }
    }
}

/// What one press reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// A bucket's `Stage all` / `Unstage all`: every row on that side.
    All,
    /// The rows of these paths — one row's `+` / `−`, the lit rows that move
    /// with it, the diff pane's `Stage file`.
    Paths(HashSet<String>),
}

impl Reach {
    fn reaches(&self, path: &str) -> bool {
        match self {
            Self::All => true,
            Self::Paths(paths) => paths.contains(path),
        }
    }
}

/// The readers one status is drawn by, each on a turn of its own: the three
/// bucket lists (one feed each — `hub::Feeds`) and the worktree's counts
/// (`WorktreeModel`, the Commit button's file count).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reader {
    Conflicts,
    Unstaged,
    Staged,
    Counts,
}

impl Reader {
    /// The list drawing the bucket run `run` (`NavSectionModel::run`);
    /// `None` for a list that draws no status of this window's tree — another
    /// worktree's rows go by a feed of their own and nothing here moves them.
    #[must_use]
    pub fn of_run(run: &str) -> Option<Self> {
        match run {
            "conflicts" => Some(Self::Conflicts),
            "unstaged" => Some(Self::Unstaged),
            "staged" => Some(Self::Staged),
            _ => None,
        }
    }

    fn seat(self) -> usize {
        match self {
            Self::Conflicts => 0,
            Self::Unstaged => 1,
            Self::Staged => 2,
            Self::Counts => 3,
        }
    }
}

/// A status with the presses in it, and the line-ending marks moved with
/// their rows (a mark says which side's change it is about —
/// `EolMark::staged`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub status: WorktreeStatus,
    pub marks: Vec<EolMark>,
}

/// One press, from the queue's acceptance to the status that shows it.
#[derive(Debug)]
struct StagePress {
    id: u64,
    side: Side,
    reach: Reach,
    /// git answered and did it.
    landed: bool,
    /// The smallest stamp a status that looked after this write can carry
    /// (`WriteFinished::reads_from`); zero until it has landed.
    reads_from: u64,
}

impl StagePress {
    /// Whether a status that looked at `looked` is still drawn with this press
    /// in it: until it has landed, and then while the status is from before the
    /// write ended.
    fn draws_over(&self, looked: u64) -> bool {
        !(self.landed && looked >= self.reads_from)
    }
}

/// The stage and unstage presses whose rows the screen has moved ahead of
/// git, from the press to the status that shows what git did.
///
/// 1. [`Self::asked`] — the press, under the id the queue accepted. Its rows
///    move at once: every reader draws its status through [`Self::picture`].
///    Where the queue took nothing (no session) nothing moves.
/// 2. [`Self::answered`] — git's answer to that id alone. Refused, the press
///    goes and its rows come back on the same turn, so the refusal is about
///    rows on screen; landed, the rows stay where they went.
/// 3. A status read after that write — `looked` at or above the answer's
///    `reads_from` — has git's own word on the rows, and is drawn as it is.
///    **Measured, never counted**: the answer and the statuses arrive on
///    separate feeds in no fixed order, and a status in flight as the write
///    ended (a paced read) lands after the answer still holding the old rows —
///    drawn without the press, it would take the moved rows back under the
///    hand. The stamp is per reader ([`StagePress::draws_over`] against the status
///    that reader holds), and the press is let go once all four have drawn
///    past it ([`Self::drawn`]): let go on the first, a list still holding the
///    old status would be drawn without it until its own read landed.
///
/// **A move is worked out on the status each reader holds, not stored**
/// ([`moved`]): a press is "stage these paths", not "these rows". So a press
/// drawn over a status that already has git's answer in it moves nothing (the
/// row is on its side already), a press over a row somebody changed since
/// (edited on both sides, staged in a terminal) answers for the row as it is
/// now, and two presses on one row compose in the order they were made — the
/// order the queue runs them in.
///
/// **Only a row whose next state the row alone decides moves**; the rest wait
/// for the status (the list and why: [`moved`]).
///
/// **Commit waits while any press stands** ([`Self::unseen`]), until every
/// reader has drawn the status after it: before that, what the rows show is
/// the press's word, not git's — a row left to the status, a file `git add
/// --all` takes that no status listed, a file changed outside between the
/// press and git. Once Commit can be pressed again, the screen is git's.
///
/// Held by the hub per tab (`hub::stage_ahead`), not on the page: the four
/// readers are models of their own, each drawing in its own drain, and each
/// asks here as it draws. Reset with the session (`Hub::release_tab` /
/// `Hub::restand_tab`): the next session's status is the truth, and its
/// stamps count from the start.
#[derive(Debug, Default)]
pub struct StageAhead {
    /// In the order they were pressed — the order the queue runs them in.
    presses: Vec<StagePress>,
    /// The status each reader drew last ([`Reader::seat`]). The last, not the
    /// largest: within a session a reader's statuses only grow, and one of a
    /// session gone that lands after [`Self::session_gone`] must not stand
    /// over the next session's stamps, which count from the start.
    drawn: [u64; 4],
    /// Whether each path asked is a directory on disk — a gitlink, in a
    /// tracked row — asked once per path while presses are out (`moved`).
    dirs: HashMap<String, bool>,
    /// A press was let go while a reader drew — Commit can be pressed again
    /// where it was the last — and the tab has not been told
    /// ([`Self::take_notice`]).
    notice: bool,
}

impl StageAhead {
    /// A press went to the queue under `accepted`; says whether it was taken
    /// — whether the readers draw it from now on. `None` (the session is
    /// closed) is not taken: no answer is coming to put its rows back.
    pub fn asked(&mut self, side: Side, reach: Reach, accepted: Option<u64>) -> bool {
        let Some(id) = accepted.filter(|id| *id != 0) else {
            tracing::debug!("stage the queue took nothing for moves no row");
            return false;
        };
        if let Reach::Paths(paths) = &reach
            && paths.is_empty()
        {
            return false;
        }
        self.presses.push(StagePress {
            id,
            side,
            reach,
            landed: false,
            reads_from: 0,
        });
        true
    }

    /// git answered a write; only a press's own answer moves anything. Says
    /// whether it was one of these — the rows that come back, or the press no
    /// longer out (its paths leave [`Self::moving_paths`]).
    pub fn answered(&mut self, id: u64, failed: bool, reads_from: u64) -> bool {
        let Some(at) = self.presses.iter().position(|p| p.id == id && !p.landed) else {
            return false;
        };
        if failed {
            self.presses.remove(at);
            self.forget_dirs_when_idle();
            return true;
        }
        let press = &mut self.presses[at];
        press.landed = true;
        press.reads_from = reads_from;
        self.let_go();
        true
    }

    /// `reader` has drawn a status that looked at `looked`.
    pub fn drawn(&mut self, reader: Reader, looked: u64) {
        self.drawn[reader.seat()] = looked;
        if self.let_go() {
            self.notice = true;
        }
    }

    /// `base` — the status a reader holds, which looked at `looked` — with the
    /// presses it is still drawn with; `None` where they move nothing in it.
    ///
    /// `is_dir` answers whether a path is a directory on disk; each path is
    /// asked once while presses are out.
    pub fn picture(
        &mut self,
        looked: u64,
        base: &WorktreeStatus,
        marks: &[EolMark],
        mut is_dir: impl FnMut(&str) -> bool,
    ) -> Option<Picture> {
        let Self { presses, dirs, .. } = self;
        let over: Vec<&StagePress> = presses.iter().filter(|p| p.draws_over(looked)).collect();
        if over.is_empty() {
            return None;
        }
        let mut dir = |path: &str| -> bool {
            if let Some(known) = dirs.get(path) {
                return *known;
            }
            let found = is_dir(path);
            dirs.insert(path.to_string(), found);
            found
        };
        picture_of(base, marks, &over, &mut dir)
    }

    /// The paths named by presses still waiting for git's answer, sorted and
    /// each once: the rows whose `+` / `−` wait meanwhile, and the diff pane
    /// open on one of them. A whole bucket's press reaches every row, which
    /// [`Self::all_out`] says.
    #[must_use]
    pub fn moving_paths(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .presses
            .iter()
            .filter(|p| !p.landed)
            .filter_map(|p| match &p.reach {
                Reach::Paths(paths) => Some(paths.iter().cloned()),
                Reach::All => None,
            })
            .flatten()
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Whether a whole bucket's press is waiting for git's answer — the
    /// buckets' `Stage all` / `Unstage all` wait meanwhile.
    #[must_use]
    pub fn all_out(&self) -> bool {
        self.presses
            .iter()
            .any(|p| !p.landed && p.reach == Reach::All)
    }

    /// Whether a press standing — answered or not — reaches `path`'s row in
    /// `bucket`: one it names, or every row for a whole bucket's, on the side
    /// it takes rows from. That row leaves its side with the status after the
    /// press — moved there already, or left to it (`moved`) — so the open
    /// diff walks on past it (`RepoPage.followEmptySide`). Not a path the
    /// disk said is a directory (`dirs`, asked under a press of rows): a
    /// gitlink's row may stay where it is (`git add` stages its commit, not
    /// the changes inside), so the diff waits on it for the status, as one
    /// moving at every status does. A whole bucket's press asks the disk
    /// nothing (`picture_of`), and a gitlink under it counts as reached like
    /// any row.
    #[must_use]
    pub fn reaches(&self, bucket: &str, path: &str) -> bool {
        self.dirs.get(path) != Some(&true)
            && self
                .presses
                .iter()
                .any(|p| p.side.takes_from(bucket) && p.reach.reaches(path))
    }

    /// Whether a press stands the screen has not caught up with — any, until
    /// every reader has drawn the status after it: Commit waits meanwhile, and
    /// so does the open diff's landing on its file's own new side
    /// (`RepoPage.followEmptySide`). Without one nothing is out, landed or
    /// not, and a status is drawn as it is.
    #[must_use]
    pub fn unseen(&self) -> bool {
        !self.presses.is_empty()
    }

    /// Whether a press was let go while a reader drew, and spends the news:
    /// the tab is the one to read it (`TabMsg::PressLetGo`).
    pub fn take_notice(&mut self) -> bool {
        std::mem::take(&mut self.notice)
    }

    /// The session behind this tab is gone, and with it every answer still
    /// owed and every stamp the readers drew: the next session's first status
    /// is the truth, whatever the old one's writes did.
    pub fn session_gone(&mut self) {
        *self = Self::default();
    }

    /// A landed press goes once every reader has drawn a status past it —
    /// none of them draws it any more. Says whether one went.
    fn let_go(&mut self) -> bool {
        let low = self.drawn.iter().copied().min().unwrap_or_default();
        let before = self.presses.len();
        self.presses.retain(|p| !(p.landed && low >= p.reads_from));
        self.forget_dirs_when_idle();
        self.presses.len() != before
    }

    /// What was found on disk is about this run of presses: a folder can be
    /// replaced by a file between two.
    fn forget_dirs_when_idle(&mut self) {
        if self.presses.is_empty() {
            self.dirs.clear();
        }
    }
}

/// `base` with `presses` in it, in their order — `None` where nothing moved.
fn picture_of(
    base: &WorktreeStatus,
    marks: &[EolMark],
    presses: &[&StagePress],
    dir: &mut dyn FnMut(&str) -> bool,
) -> Option<Picture> {
    // A path git lists twice — a staged deletion beside an untracked file of
    // the same name — is no one row's to move.
    let mut seen: HashSet<&str> = HashSet::new();
    let twice: HashSet<&str> = base
        .items
        .iter()
        .map(StatusItem::path)
        .filter(|path| !seen.insert(path))
        .collect();
    let mut items = base.items.clone();
    // Which side each moved path ended on: its marks go with it.
    let mut ends: HashMap<String, bool> = HashMap::new();
    for press in presses {
        for item in &mut items {
            if !press.reach.reaches(item.path()) {
                continue;
            }
            let next = if twice.contains(item.path()) {
                None
            } else if press.reach == Reach::All {
                // Not one look at the disk per row on the press's turn: a
                // whole bucket can be thousands. A gitlink is drawn moved
                // like any row until the status lands — Commit waits on that
                // status for a whole bucket's press anyway (`unseen`).
                moved(press.side, item, &mut |_| false)
            } else {
                moved(press.side, item, dir)
            };
            if let Some(next) = next {
                ends.insert(item.path().to_string(), press.side == Side::Stage);
                *item = next;
            }
        }
    }
    if items == base.items {
        return None;
    }
    let marks = marks
        .iter()
        .map(|mark| match ends.get(&mark.path) {
            Some(&staged) => EolMark {
                staged,
                ..mark.clone()
            },
            None => mark.clone(),
        })
        .collect();
    let picture = Picture {
        status: WorktreeStatus {
            branch_oid: base.branch_oid,
            branch_head: base.branch_head.clone(),
            upstream: base.upstream.clone(),
            upstream_tracked: base.upstream_tracked,
            ahead: base.ahead,
            behind: base.behind,
            items,
            staged_print: base.staged_print,
        },
        marks,
    };
    Some(picture)
}

/// The row `item` becomes when `side` reaches it, where the row alone decides
/// that; `None` leaves it to the status.
///
/// Moved: a change on one side only — `git add` of an edit, a deletion, a
/// type change or an intent-to-add makes the same letter staged, and of an
/// untracked file an addition; unstaging gives the edit, deletion or type
/// change back unstaged, and an addition back untracked. A one-sided row never
/// leaves the list when staged: git lists it only where the worktree, read
/// the way `git add` reads it (line endings normalised), differs from an index
/// that is HEAD's, so the index `git add` writes differs from HEAD too.
///
/// Left to the status:
/// - **a file changed on both sides** (`MM`, `AM`, `MD` …): what the index
///   ends up holding against HEAD is a comparison of contents — staging a
///   file whose staged half was edited back to HEAD leaves no row at all
/// - **a staged rename or copy**: unstaging the new name leaves the old
///   name's deletion staged (`git restore --staged` takes one path)
/// - **a path git lists twice** (see `picture_of`)
/// - **a conflict**: `Mark resolved` is git's word on how it was resolved
/// - **a directory**: an untracked one (`nest/`) is a repository git did not
///   open; a tracked one — asked of the disk under a press of rows, not under
///   a whole bucket's (below) — is a gitlink, of which `git add` stages the
///   commit and none of the files, so a submodule with edits inside and no
///   new commit stays unstaged; and a deleted file whose path is a folder now
///   takes the folder's files with it
///
/// Not told apart, and so drawn wrong until the status lands:
/// - a gitlink under a whole bucket's press, which asks the disk nothing
///   (`picture_of`): drawn moved like a file
/// - a forced-in ignored file unstaged (it leaves the list rather than going
///   untracked)
/// - an intent-to-add file deleted since (`.D` like any deletion): staging it
///   takes the entry out, and the row goes rather than turning `D.`
/// - a moved file's two halves staged together (a deletion and an untracked
///   file): drawn as the two rows, `D.` and `A.`, where the status pairs them
///   into one rename — the same commit, one file fewer in the count
fn moved(side: Side, item: &StatusItem, dir: &mut dyn FnMut(&str) -> bool) -> Option<StatusItem> {
    // A rename or a copy carries `R` / `C`, which no arm takes: every row
    // moved here has no old name.
    match (side, item) {
        (
            Side::Stage,
            StatusItem::Tracked {
                staged: '.',
                unstaged: letter @ ('M' | 'D' | 'T' | 'A'),
                path,
                ..
            },
        ) if !dir(path) => Some(StatusItem::Tracked {
            staged: *letter,
            unstaged: '.',
            path: path.clone(),
            orig_path: None,
        }),
        (Side::Stage, StatusItem::Untracked { path }) if !path.ends_with('/') => {
            Some(StatusItem::Tracked {
                staged: 'A',
                unstaged: '.',
                path: path.clone(),
                orig_path: None,
            })
        }
        (
            Side::Unstage,
            StatusItem::Tracked {
                staged: letter @ ('M' | 'D' | 'T'),
                unstaged: '.',
                path,
                ..
            },
        ) if !dir(path) => Some(StatusItem::Tracked {
            staged: '.',
            unstaged: *letter,
            path: path.clone(),
            orig_path: None,
        }),
        (
            Side::Unstage,
            StatusItem::Tracked {
                staged: 'A',
                unstaged: '.',
                path,
                ..
            },
        ) if !dir(path) => Some(StatusItem::Untracked { path: path.clone() }),
        _ => None,
    }
}
