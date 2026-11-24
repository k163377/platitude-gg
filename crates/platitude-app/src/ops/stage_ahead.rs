//! The file rows a stage or unstage has moved on the screen while git is
//! still doing it (デザイン規約 §バケツごとの一覧と、ペインの分け方): which
//! presses are out, which status answers each, and what a status looks like
//! with them in it.

use std::collections::{HashMap, HashSet};

use platitude_core::session::EolMark;
use platitude_core::stage::DiscardSide;
use platitude_core::status::{StatusItem, WorktreeStatus};

use super::stage_parts::{DiffSpot, Discard, Moves, Next, Part, PartVerb};

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
        use crate::encode::Routing;
        let Some(routing) = Routing::of_word(bucket) else {
            return false;
        };
        match self {
            Side::Stage => routing != Routing::Staged,
            Side::Unstage => routing == Routing::Staged,
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

/// What one press does to the rows it reaches.
#[derive(Debug)]
enum Act {
    /// Whole files staged or unstaged ([`moved`]).
    Whole { side: Side, reach: Reach },
    /// Part of one file's diff ([`Part::next`]).
    Part(Part),
    /// Chosen rows discarded ([`Discard::next`]).
    Discard(Discard),
}

impl Act {
    /// Whether the press names `path`'s row.
    fn reaches(&self, path: &str) -> bool {
        match self {
            Self::Whole { reach, .. } => reach.reaches(path),
            Self::Part(part) => part.path == path,
            Self::Discard(discard) => discard.reaches(path),
        }
    }

    /// The paths the press names; `None` for a whole bucket's.
    fn paths(&self) -> Option<Vec<String>> {
        match self {
            Self::Whole {
                reach: Reach::Paths(paths),
                ..
            } => Some(paths.iter().cloned().collect()),
            Self::Whole {
                reach: Reach::All, ..
            } => None,
            Self::Part(part) => Some(vec![part.path.clone()]),
            Self::Discard(discard) => Some(discard.paths().cloned().collect()),
        }
    }

    /// Whether git is to write the index — what Commit, the stash, HEAD's
    /// Save and `--continue` take whole ([`StageAhead::index_unseen`]). A
    /// discard of the disk alone leaves the index as it was.
    fn moves_index(&self) -> bool {
        match self {
            Self::Whole { .. } => true,
            Self::Part(part) => part.verb != PartVerb::Discard,
            Self::Discard(discard) => discard.moves_index(),
        }
    }

    /// Whether `path`'s row in `bucket` leaves its side with the status
    /// after the press ([`StageAhead::reaches`]).
    fn leaves(&self, bucket: &str, path: &str) -> bool {
        match self {
            Self::Whole { side, reach } => side.takes_from(bucket) && reach.reaches(path),
            Self::Part(part) => part.leaves(bucket, path),
            Self::Discard(discard) => discard.leaves(bucket, path),
        }
    }

    /// The row `item` becomes; `None` leaves it to the status. A whole
    /// bucket's press asks the disk nothing (see [`picture_of`]).
    fn next(&self, item: &StatusItem, dir: &mut dyn FnMut(&str) -> bool) -> Option<Next> {
        match self {
            Self::Whole {
                side,
                reach: Reach::All,
            } => moved(*side, item, &mut |_| false).map(Next::Row),
            Self::Whole { side, .. } => moved(*side, item, dir).map(Next::Row),
            Self::Part(part) => part.next(item),
            Self::Discard(discard) => discard.next(item, dir),
        }
    }

    /// Which side a row a whole file's press moved ended on — its
    /// line-ending marks go with it; `None` for the other two, which leave
    /// the marks where they were or take them with the row.
    fn ends_staged(&self) -> Option<bool> {
        match self {
            Self::Whole { side, .. } => Some(*side == Side::Stage),
            Self::Part(_) | Self::Discard(_) => None,
        }
    }
}

/// A part's press as the open diff draws it ([`StageAhead::diff_parts`]):
/// where it stands in the drawing it was pressed on, the side it moves, and
/// whether git has done it. `id` tells one press from another, so a reader
/// can tell whether what it draws has moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPart {
    pub id: u64,
    pub part: platitude_core::parts::Part,
    pub landed: bool,
}

/// A part's press as the pane draws it, from the press to the first read the
/// pane asked for after git answered it. `spot` is where it stands in the
/// read the pane draws now — moved onto each newer read it stays placeable
/// in ([`StageAhead::diff_drawn`]).
#[derive(Debug)]
struct DiffPress {
    id: u64,
    spot: DiffSpot,
    moves: Moves,
    landed: bool,
    /// The pane's first ask after git answered it
    /// ([`StageAhead::diff_asked`]): a read asked at or after this is git's
    /// word on it.
    truth_from: Option<u64>,
}

/// What the pictures drawn of one press have done with the paths it names
/// ([`StageAhead::picture`]) — the one thing about a move that is kept
/// (why: [`StageAhead`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pictured {
    /// No reader has drawn it over a status: no row on screen is its word.
    Never,
    /// Every picture so far moved every path it names.
    AllMoved,
    /// A picture left a path it names to the status, or held no row for it.
    /// So does one over a status that already holds git's answer, the row
    /// on its new side already: the press moves nothing there, and the
    /// picture cannot tell that from a row left to the status.
    ///
    /// For good: that reader's rows showed nothing of the press there, and a
    /// later picture that moves every path takes nothing back — the doors
    /// stay held until the press goes with the status after it (the safe
    /// side).
    SomeLeft,
}

/// One press, from the queue's acceptance to the status that shows it.
#[derive(Debug)]
struct StagePress {
    id: u64,
    act: Act,
    /// git answered and did it.
    landed: bool,
    /// The smallest stamp a status that looked after this write can carry
    /// (`WriteFinished::reads_from`); zero until it has landed.
    reads_from: u64,
    /// The status the queue read behind the write failed
    /// ([`StageAhead::read_failed`]). The press stands all the same, its
    /// rows drawn, until a status that looked after the write lands.
    unread: bool,
    pictured: Pictured,
}

impl StagePress {
    /// Whether a status that looked at `looked` is still drawn with this press
    /// in it: until it has landed, and then while the status is from before the
    /// write ended.
    fn draws_over(&self, looked: u64) -> bool {
        !(self.landed && looked >= self.reads_from)
    }

    /// A picture drew this press and moved `moved` rows with it. Every path
    /// moved is as many rows as the press names paths: a path is one row at
    /// most (`picture_of` leaves one git lists twice). A whole bucket's
    /// press, a part and a discard name no such count, and are never
    /// [`Pictured::AllMoved`].
    fn pictured(&mut self, moved: usize) {
        let every = matches!(
            &self.act,
            Act::Whole { reach: Reach::Paths(paths), .. } if paths.len() == moved
        );
        self.pictured = match (self.pictured, every) {
            (Pictured::Never | Pictured::AllMoved, true) => Pictured::AllMoved,
            _ => Pictured::SomeLeft,
        };
    }

    /// Whether the writes that take the index whole wait on this press
    /// ([`StageAhead::index_unseen`]).
    fn holds_index(&self) -> bool {
        self.act.moves_index() && !(self.unread && self.pictured == Pictured::AllMoved)
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
/// **Commit waits while any press that writes the index stands**
/// ([`Self::index_unseen`]), until every reader has drawn the status after
/// it: before that, what the rows show is the press's word, not git's — a
/// row left to the status, a file `git add --all` takes that no status
/// listed, a file changed outside between the press and git. Once Commit can
/// be pressed again, the screen is git's. A discard of the disk alone leaves
/// the index — what Commit, the stash's staged half, HEAD's Save and
/// `--continue` take — where it was, and holds none of them.
///
/// **The one exception: where the status read behind a press failed, the wait
/// is let go for a press of rows that moved every one it names, and for no
/// other** ([`Self::read_failed`], user decision — ask before changing). What
/// failed is the one read the queue made behind that write: the next status
/// that lands — the paced read of a tab on screen (`session::pace`), the read
/// behind another write — lets the press go as any does, and the wait would
/// end there too, but not for as long as the read keeps failing. Let off, the
/// doors open on rows that are still the press's word: each row it names
/// shows what the press did, drawn wrong where [`moved`] cannot tell two
/// shapes apart. A press that left a row to the status keeps the wait — that
/// row is drawn as it was before the press, so a commit would record what no
/// row shows — and so do a whole bucket's (`git add --all` takes files no
/// status listed), a part and a discard.
///
/// **Whether a press moved every row is written down as it is drawn**
/// ([`Pictured`], [`Self::picture`]), where the move itself is not: the
/// readers hold a status each and draw on turns of their own, so there is no
/// one status to work it out on when the failure is told. Every picture of a
/// press adds to its record — one that left a path where it was, or held no
/// row for it, marks the press for good, whatever the other readers drew —
/// and the wait is read off the record as it stands, so a picture drawn after
/// the failure was told moves the doors too (the tab hears of it as of a
/// press let go, [`Self::take_notice`]).
///
/// **Three kinds of press, one life** ([`Act`]): whole files staged or
/// unstaged, part of one file's diff ([`Self::asked_part`]) and chosen rows
/// discarded ([`Self::asked_discard`]). A part is drawn by a fifth reader
/// too, the open diff ([`Self::diff_parts`]), which lets it go on an order
/// of its own: **the pane's asks**. A read the pane asked once git had
/// answered the part is git's word on it; one asked before can land after
/// the answer still holding the pressed hunk, and the part stays drawn over
/// it ([`Self::diff_asked`], [`Self::diff_drawn`]).
///
/// Held by the hub per tab (`hub::stage_ahead`), not on the page: the four
/// status readers and the open diff are models of their own, each drawing
/// in its own drain, and each asks here as it draws. Reset with the session (`Hub::release_tab` /
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
    /// A reader's drawing moved the presses — a press let go, so Commit can
    /// be pressed again where it was the last, the doors of a press whose
    /// status read failed moved with a picture ([`Self::picture`]), or the
    /// open diff's parts moved with its read — and the tab has not been told
    /// ([`Self::take_notice`]).
    notice: bool,
    /// The parts as the open diff draws them, in the order they were
    /// pressed; each goes with its answer where refused, and once landed
    /// when the diff drawn is not the one it was made on.
    diff_presses: Vec<DiffPress>,
    /// The diff the pane drew last — its key and fingerprint — `None` where
    /// it holds none of this window's tree ([`Self::diff_drawn`]).
    diff_seen: Option<(String, u64)>,
    /// That diff as the pane was handed it: what a part pressed on it while
    /// others are in front is placed by (`stage::PressedOn`).
    diff_read: Option<DiffRead>,
}

/// A diff as the pane was handed it.
pub type DiffRead = std::sync::Arc<Vec<platitude_core::parse::diff::FilePatch>>;

impl StageAhead {
    /// A press went to the queue under `accepted`; says whether it was taken
    /// — whether the readers draw it from now on. `None` (the session is
    /// closed) is not taken: no answer is coming to put its rows back.
    pub fn asked(&mut self, side: Side, reach: Reach, accepted: Option<u64>) -> bool {
        if let Reach::Paths(paths) = &reach
            && paths.is_empty()
        {
            return false;
        }
        self.push(Act::Whole { side, reach }, accepted).is_some()
    }

    /// The same for part of one file's diff: its row moves where the row
    /// and the diff decide it ([`Part::next`]), and the open diff draws the
    /// part taken out of it ([`Self::diff_parts`]).
    pub fn asked_part(&mut self, part: Part, accepted: Option<u64>) -> bool {
        let spot = part.spot.clone();
        let moves = part.verb.moves();
        let Some(id) = self.push(Act::Part(part), accepted) else {
            return false;
        };
        self.diff_presses.push(DiffPress {
            id,
            spot,
            moves,
            landed: false,
            truth_from: None,
        });
        true
    }

    /// The same for a discard of chosen rows (`<path>` and the side each was
    /// chosen on): the rows go at the press ([`Discard::next`]).
    pub fn asked_discard(
        &mut self,
        chosen: &[(String, DiscardSide)],
        accepted: Option<u64>,
    ) -> bool {
        let discard = Discard::new(chosen);
        if discard.is_empty() {
            return false;
        }
        self.push(Act::Discard(discard), accepted).is_some()
    }

    /// Writes a press down under the id the queue took it under; `None`
    /// where it took nothing.
    fn push(&mut self, act: Act, accepted: Option<u64>) -> Option<u64> {
        let Some(id) = accepted.filter(|id| *id != 0) else {
            tracing::debug!("stage the queue took nothing for moves no row");
            return None;
        };
        self.presses.push(StagePress {
            id,
            act,
            landed: false,
            reads_from: 0,
            unread: false,
            pictured: Pictured::Never,
        });
        Some(id)
    }

    /// The status the queue read behind write `id` failed
    /// (`TabMsg::StatusUnread`): git answered the press, and the one read
    /// asked for behind it drew nothing. Written down on the press — whether
    /// the doors stop waiting on it is [`Self::index_unseen`]'s to say — and
    /// its rows stay where it put them until the next status that lands,
    /// which lets it go as any does. Nothing for a write that is no press of
    /// these, or one git has not answered: no read was made behind that.
    pub fn read_failed(&mut self, id: u64) {
        if let Some(press) = self.presses.iter_mut().find(|p| p.id == id && p.landed) {
            press.unread = true;
        }
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
            self.refused_part(id);
            self.forget_dirs_when_idle();
            return true;
        }
        let press = &mut self.presses[at];
        press.landed = true;
        press.reads_from = reads_from;
        for part in self.diff_presses.iter_mut().filter(|p| p.id == id) {
            part.landed = true;
        }
        // Landed off the drawing — the pane closed or moved to another file
        // while it ran: no read of the pane's will speak for it.
        let seen = self.diff_seen.as_ref();
        self.diff_presses.retain(|p| {
            !p.landed
                || seen
                    .is_some_and(|(key, print)| p.spot.key == *key && p.spot.fingerprint == *print)
        });
        self.let_go();
        true
    }

    /// The pane asked for a read under its number `asked`: every part git
    /// has answered is spoken for by this read and any asked after it —
    /// asked once the write was done, they look at what it left.
    pub fn diff_asked(&mut self, asked: u64) {
        for part in self.diff_presses.iter_mut().filter(|p| p.landed) {
            part.truth_from.get_or_insert(asked);
        }
    }

    /// The pane draws `seen` — its key and fingerprint — read as `read`,
    /// answering the ask numbered `asked`; `None`, a diff of none of this
    /// window's files, which no part is drawn over.
    ///
    /// **What the pane holds of its own is the parts; a read is git's word.**
    /// A part this read speaks for — its write answered before the read was
    /// asked ([`Self::diff_asked`]) — goes, whatever the read holds: git did
    /// it, or somebody put the file back since, and the read says which. The
    /// others are moved onto the read: a read asked before their answers
    /// holds the bytes they were pressed on, or the bytes some of them left
    /// — git runs them in the order pressed — and where it is exactly the
    /// diff the first of them leave (`parts::after`), the rest stand on it
    /// where they stood. Where it is none of those, the file moved under
    /// them: they leave the drawing, and git turns them down as moved.
    ///
    /// **By the order of asking, not of arrival**: a read asked before an
    /// answer can land after it holding the bytes from before the write —
    /// one asked as a status came in mid-write, or one asked at the answer of
    /// the part in front while the next one ran — and drawn as the word on
    /// that write, it would put the part back under the hand.
    pub fn diff_drawn(&mut self, seen: Option<(String, u64)>, read: Option<DiffRead>, asked: u64) {
        let before = self.diff_presses.len();
        let was = self.diff_seen.take();
        let held = self.diff_read.take();
        match (&seen, &read) {
            (Some((key, print)), Some(now)) => {
                let on_held = was
                    .as_ref()
                    .filter(|(at, _)| at == key)
                    .map(|(_, print)| *print);
                self.move_parts(
                    key,
                    on_held,
                    held.as_deref().map(Vec::as_slice),
                    now,
                    *print,
                    asked,
                );
            }
            // Nothing of the file is drawn: a part git has answered has
            // nothing left to draw it over.
            _ => self.diff_presses.retain(|p| !p.landed),
        }
        let moved = was != seen || self.diff_presses.len() != before;
        self.diff_seen = seen;
        self.diff_read = read;
        if moved {
            self.notice = true;
        }
    }

    /// The parts on file `key` — pressed on the read `held` drew as `print`
    /// — moved onto `now` (fingerprint `now_print`, answering ask `asked`):
    /// see [`Self::diff_drawn`].
    fn move_parts(
        &mut self,
        key: &str,
        print: Option<u64>,
        held: Option<&[platitude_core::parse::diff::FilePatch]>,
        now: &[platitude_core::parse::diff::FilePatch],
        now_print: u64,
        asked: u64,
    ) {
        let on = |p: &DiffPress| p.spot.key == key && Some(p.spot.fingerprint) == print;
        let parts: Vec<platitude_core::parts::Part> = self
            .diff_presses
            .iter()
            .filter(|p| on(p))
            .map(|p| platitude_core::parts::Part {
                hunk: p.spot.hunk,
                line: p.spot.line,
                moves: p.moves,
            })
            .collect();
        let spoken = self
            .diff_presses
            .iter()
            .filter(|p| on(p))
            .take_while(|p| p.truth_from.is_some_and(|from| from <= asked))
            .count();
        let done = held.and_then(|held| {
            (spoken..=parts.len()).rev().find(|done| {
                let laid = platitude_core::parts::after(held, &parts[..*done]);
                laid.exact && platitude_core::parts::same_lines(&laid.patches, now)
            })
        });
        let mut index = 0;
        self.diff_presses.retain_mut(|p| {
            if !on(p) {
                // Pressed on other bytes or another file: nothing here to
                // draw it over.
                return !p.landed;
            }
            let at = index;
            index += 1;
            match done {
                Some(done) if at >= done => {
                    p.spot.fingerprint = now_print;
                    true
                }
                _ => false,
            }
        });
    }

    /// The parts drawn over the diff named `key` as read with `fingerprint`,
    /// in the order they were pressed: those made on these very bytes,
    /// landed or not — and none over other bytes, which their places do not
    /// address.
    #[must_use]
    pub fn diff_parts(&self, key: &str, fingerprint: u64) -> Vec<DiffPart> {
        self.diff_presses
            .iter()
            .filter(|p| p.spot.key == key && p.spot.fingerprint == fingerprint)
            .map(|p| DiffPart {
                id: p.id,
                part: platitude_core::parts::Part {
                    hunk: p.spot.hunk,
                    line: p.spot.line,
                    moves: p.moves,
                },
                landed: p.landed,
            })
            .collect()
    }

    /// What a part pressed now on the diff `key` read with `fingerprint` is
    /// placed by: those bytes as read, and the parts pressed on them still
    /// standing, in order (`stage::PressedOn`). `None` where the pane draws
    /// other bytes.
    #[must_use]
    pub fn pressed_on(
        &self,
        key: &str,
        fingerprint: u64,
    ) -> Option<(DiffRead, Vec<platitude_core::parts::Part>)> {
        let (seen_key, seen_print) = self.diff_seen.as_ref()?;
        if seen_key != key || *seen_print != fingerprint {
            return None;
        }
        let read = self.diff_read.clone()?;
        let ahead = self
            .diff_parts(key, fingerprint)
            .into_iter()
            .map(|p| p.part)
            .collect();
        Some((read, ahead))
    }

    /// Whether write `id` is a part on the diff the pane draws — one the
    /// drawing has in it already, which leaves the other parts of it
    /// pressable (`RepoTab.diffWritesOut`).
    #[must_use]
    pub fn part_on_drawn(&self, id: u64) -> bool {
        let Some((key, print)) = self.diff_seen.as_ref() else {
            return false;
        };
        self.diff_presses
            .iter()
            .any(|p| p.id == id && p.spot.key == *key && p.spot.fingerprint == *print)
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
    ///
    /// Each press drawn is told what this picture did with its paths
    /// ([`Pictured`]); where that moves the doors of one whose status read
    /// failed ([`Self::index_unseen`]), the tab is owed the news
    /// ([`Self::take_notice`]).
    pub fn picture(
        &mut self,
        looked: u64,
        base: &WorktreeStatus,
        marks: &[EolMark],
        mut is_dir: impl FnMut(&str) -> bool,
    ) -> Option<Picture> {
        let held = self.index_unseen();
        let Self { presses, dirs, .. } = self;
        let mut over: Vec<&mut StagePress> = presses
            .iter_mut()
            .filter(|p| p.draws_over(looked))
            .collect();
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
        let picture = picture_of(base, marks, &mut over, &mut dir);
        if self.index_unseen() != held {
            self.notice = true;
        }
        picture
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
            .filter_map(|p| p.act.paths())
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
            .any(|p| !p.landed && p.act.paths().is_none())
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
            && self.presses.iter().any(|p| p.act.leaves(bucket, path))
    }

    /// Whether a press that takes `path`'s row off `bucket` still waits for
    /// git's answer. The open diff landed on the file's new side at the press
    /// reads there only after it (`RepoPage.diffHeld`): read before, that
    /// side shows itself as it was before the write.
    #[must_use]
    pub fn leaving_out(&self, bucket: &str, path: &str) -> bool {
        self.presses
            .iter()
            .any(|p| !p.landed && p.act.leaves(bucket, path))
    }

    /// Whether a press stands the status readers have not caught up with —
    /// any, until every reader has drawn the status after it: an open diff
    /// waiting in place for where its file went waits meanwhile
    /// (`RepoPage.followLanding`). Without one nothing is out, landed or
    /// not, and a status is drawn as it is. A status read that failed
    /// ([`Self::read_failed`]) lets nothing go here: the diff waiting in
    /// place, and automation's `staging` wait ([`Self::ahead_unseen`]), stay
    /// on the next status that lands.
    #[must_use]
    pub fn unseen(&self) -> bool {
        !self.presses.is_empty()
    }

    /// [`Self::unseen`] for the presses that write the index — what Commit,
    /// the stash, HEAD's Save and `--continue` wait on ([`Act::moves_index`]).
    /// Not a press whose status read failed ([`Self::read_failed`]) where
    /// every picture drawn of it moved every row it names
    /// ([`Pictured::AllMoved`]): waited on, it would hold them for as long as
    /// the read keeps failing. Any other press stands here until the status
    /// after it is drawn (which press and why: [`StageAhead`]).
    #[must_use]
    pub fn index_unseen(&self) -> bool {
        self.presses.iter().any(StagePress::holds_index)
    }

    /// Whether anything on screen is still a press's word: a press the
    /// status readers have not caught up with, or a part the open diff has
    /// not drawn past.
    #[must_use]
    pub fn ahead_unseen(&self) -> bool {
        !self.presses.is_empty() || !self.diff_presses.is_empty()
    }

    /// Whether what a reader drew moved the presses — one let go, or the open
    /// diff's parts moved onto its read or gone with it — and spends the
    /// news: the tab is the one to read it (`TabMsg::PressLetGo`).
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

    /// git refused write `id`: where it was a part, it leaves the drawing,
    /// and so does every part pressed after it on the same bytes — each was
    /// placed in a drawing with this one in it, which git now does not print
    /// (core refuses them the same way, `stage::PressedOn`). Their rows wait
    /// on their own answers.
    fn refused_part(&mut self, id: u64) {
        let Some(at) = self.diff_presses.iter().position(|p| p.id == id) else {
            return;
        };
        let refused = self.diff_presses.remove(at);
        let mut index = 0;
        self.diff_presses.retain(|p| {
            let behind = index >= at
                && p.spot.key == refused.spot.key
                && p.spot.fingerprint == refused.spot.fingerprint;
            index += 1;
            !behind
        });
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
/// Each press is told how many rows it moved ([`StagePress::pictured`]).
fn picture_of(
    base: &WorktreeStatus,
    marks: &[EolMark],
    presses: &mut [&mut StagePress],
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
    let mut items: Vec<Option<StatusItem>> = base.items.iter().cloned().map(Some).collect();
    // Which side each moved path ended on — its marks go with it — or
    // `None` where the row left the list, and its marks with it.
    let mut ends: HashMap<String, Option<bool>> = HashMap::new();
    for press in presses.iter_mut() {
        let mut moved = 0;
        for slot in &mut items {
            let Some(item) = slot else {
                continue;
            };
            // A whole bucket's press asks no disk on the press's turn: a
            // bucket can be thousands of rows. A gitlink is drawn moved
            // like any row until the status lands — Commit waits on that
            // status for a whole bucket's press anyway (`index_unseen`).
            if !press.act.reaches(item.path()) || twice.contains(item.path()) {
                continue;
            }
            match press.act.next(item, dir) {
                Some(Next::Row(next)) => {
                    if let Some(staged) = press.act.ends_staged() {
                        ends.insert(item.path().to_string(), Some(staged));
                    }
                    *item = next;
                    moved += 1;
                }
                Some(Next::Gone) => {
                    ends.insert(item.path().to_string(), None);
                    *slot = None;
                    moved += 1;
                }
                None => {}
            }
        }
        press.pictured(moved);
    }
    let items: Vec<StatusItem> = items.into_iter().flatten().collect();
    if items == base.items {
        return None;
    }
    let marks = marks
        .iter()
        .filter_map(|mark| match ends.get(&mark.path) {
            Some(Some(staged)) => Some(EolMark {
                staged: *staged,
                ..mark.clone()
            }),
            Some(None) => None,
            None => Some(mark.clone()),
        })
        .collect();
    let picture = Picture {
        status: WorktreeStatus {
            branch_oid: base.branch_oid,
            branch_head: base.branch_head.clone(),
            upstream: base.upstream.clone(),
            upstream_counts: base.upstream_counts,
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
