//! The rows the window is showing as gone while git is being asked to
//! delete them (デザイン規約 §消す操作は先に画面から消す).

/// Which list a stood-in row was taken out of — one name per list at most:
/// a delete touches at most one row of each (`Delete both` touches two
/// lists under one write).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Branch,
    Remote,
    Tag,
    Stash,
    /// A working copy, by its path as git lists it.
    Worktree,
}

impl Row {
    /// Which of them the sidebar section called `section` draws
    /// (`NavSectionModel::attach_section_feed` names the same words);
    /// `None` for a section that stands nothing in.
    #[must_use]
    pub fn drawn_by(section: &str) -> Option<Self> {
        match section {
            "branches" => Some(Self::Branch),
            "remotes" => Some(Self::Remote),
            "tags" => Some(Self::Tag),
            "stashes" => Some(Self::Stash),
            "worktrees" => Some(Self::Worktree),
            _ => None,
        }
    }
}

/// What is being shown as gone, one name per list — what the lists and the
/// graph's chips are drawn from; empty is nothing stood in for.
///
/// Named as git names the rows (a stash by its selector, a ref by its short
/// name), so a refusal puts back exactly what the press took away.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rows {
    pub branch: String,
    pub remote: String,
    pub tag: String,
    pub stash: String,
    pub worktree: String,
}

impl Rows {
    /// Shows one row as gone. An empty name stands nothing in: it is what a
    /// list is told when it is to put everything back.
    fn put(&mut self, row: Row, name: &str) {
        if name.is_empty() {
            return;
        }
        let seat = match row {
            Row::Branch => &mut self.branch,
            Row::Remote => &mut self.remote,
            Row::Tag => &mut self.tag,
            Row::Stash => &mut self.stash,
            Row::Worktree => &mut self.worktree,
        };
        *seat = name.to_string();
    }

    /// Puts one kind back, whatever it was.
    fn clear(&mut self, row: Row) {
        self.seat(row).clear();
    }

    fn seat(&mut self, row: Row) -> &mut String {
        match row {
            Row::Branch => &mut self.branch,
            Row::Remote => &mut self.remote,
            Row::Tag => &mut self.tag,
            Row::Stash => &mut self.stash,
            Row::Worktree => &mut self.worktree,
        }
    }

    fn is_empty(&self) -> bool {
        self.branch.is_empty()
            && self.remote.is_empty()
            && self.tag.is_empty()
            && self.stash.is_empty()
            && self.worktree.is_empty()
    }
}

/// When the list that draws each kind of row last applied a reading — one
/// number per list: the three refs sections apply the same snapshot on
/// three separate turns, and a single newest-applied would let the branches
/// section answer for a tag row the tags section still shows old, bringing
/// the deleted row back until that section caught up.
#[derive(Debug, Default)]
struct Applied {
    branch: u64,
    remote: u64,
    tag: u64,
    stash: u64,
    worktree: u64,
}

impl Applied {
    fn at(&self, row: Row) -> u64 {
        match row {
            Row::Branch => self.branch,
            Row::Remote => self.remote,
            Row::Tag => self.tag,
            Row::Stash => self.stash,
            Row::Worktree => self.worktree,
        }
    }

    /// Writes down a reading that list has drawn; the newest stands.
    fn note(&mut self, row: Row, at: u64) {
        let seat = match row {
            Row::Branch => &mut self.branch,
            Row::Remote => &mut self.remote,
            Row::Tag => &mut self.tag,
            Row::Stash => &mut self.stash,
            Row::Worktree => &mut self.worktree,
        };
        *seat = (*seat).max(at);
    }
}

/// One delete, from the press to the reading that proves it: the rows it
/// took off the screen, the write they are waiting on, and how far that
/// write has got.
///
/// 1. [`Self::asked`] — the press. The rows go and the accepted id is
///    written down. Where none was accepted (the session is closed) the
///    rows stay put: only an answer puts them back.
/// 2. [`Self::answered`] — git's answer to that id alone (every stash
///    operation answers under the same word, and a fetch behind a press
///    answers in the middle of it). Refused, and the rows come straight
///    back, so the refusal is about a row on screen; landed, and they stay
///    away.
/// 3. [`Self::look_again`] — a list has drawn a reading. Each row answers
///    off the list that draws it ([`Applied`]; a composite delete's two
///    rows go as their two lists reach them): the stash listing is read
///    after the graph rebuild (`session::write`), so measured against the
///    refs' a dropped entry would be back for the whole rebuild.
///
/// Only a listing that saw the write answers: the answer and the listings
/// arrive in no fixed order, and a read in flight when the write ended
/// would put the row back under the hand that had just taken it. Each
/// listing is stamped before git is spawned ([`Self::listing_applied`]),
/// and the answer names the smallest stamp that speaks for what it left
/// (`reads_from`, `session::Standing::fence`). The list writes the stamp
/// down when it puts the rows on screen — core having published a reading
/// says nothing about what the window shows.
///
/// Known gap: `Delete both` deletes locally and then pushes, and a failed
/// remote half answers with the same one error as a refused local half —
/// so a landed local delete is put back too, until the refs read takes it
/// away again. Telling them apart needs a result per half from core
/// (rules-refs/app-ui.md).
#[derive(Debug, Default)]
pub struct StandIn {
    rows: Rows,
    /// The write the rows are waiting on, as the bridge carries an
    /// `OperationId` — zero while none is out.
    waiting: u64,
    /// Whether that write has answered and git did it; a reading answers
    /// for the rows only after it has.
    landed: bool,
    /// The smallest stamp a listing that saw what this write left can
    /// carry (`WriteFinished::reads_from`); zero while none is out.
    reads_from: u64,
    /// What each list has drawn. Kept across a delete ending, so a listing
    /// applied before the answer is still proof when the answer lands;
    /// thrown away only with the session ([`Self::session_gone`]).
    applied: Applied,
}

impl StandIn {
    /// Takes this press's rows off the screen, under the id the queue
    /// accepted its write with.
    ///
    /// A press while another delete still stands (between an answer and
    /// its reading — the UI offers no second delete while a write runs)
    /// takes the wait over, the later name winning its own list.
    pub fn asked(&mut self, rows: &[(Row, &str)], accepted: Option<u64>) {
        let Some(id) = accepted.filter(|id| *id != 0) else {
            tracing::debug!("delete the queue took nothing for leaves its rows where they are");
            return;
        };
        for (row, name) in rows {
            self.rows.put(*row, name);
        }
        self.waiting = id;
        self.landed = false;
    }

    /// git answered a write; only this one's own answer moves anything.
    ///
    /// A landing settles at once where a listing at or above `reads_from`
    /// was already applied — whichever of the two arrives second completes
    /// the pair.
    pub fn answered(&mut self, id: u64, failed: bool, reads_from: u64) {
        if self.waiting == 0 || self.waiting != id {
            return;
        }
        if failed {
            self.put_back();
            return;
        }
        self.landed = true;
        self.reads_from = reads_from;
        self.look_again();
    }

    /// The list that draws `row` applied a reading that looked at `at`.
    /// Only written down; [`Self::look_again`] decides what it answers for.
    pub fn listing_applied(&mut self, row: Row, at: u64) {
        self.applied.note(row, at);
    }

    /// A list has drawn what it was handed: ask every row whether the list
    /// that draws *it* has caught up.
    ///
    /// One door for all five, taking no list: a caller naming the list it
    /// came from could name it wrongly, and a stash edge sent to a refs
    /// door would leave the stash row standing for good.
    pub fn look_again(&mut self) {
        for row in [
            Row::Branch,
            Row::Remote,
            Row::Tag,
            Row::Stash,
            Row::Worktree,
        ] {
            self.settle_row(row);
        }
    }

    /// The session behind this tab is gone (`Hub::release_tab`), and with
    /// it every list that was drawing these rows and every number they
    /// were measured by.
    ///
    /// Wider than [`Self::put_back`]: a new session counts its reads from
    /// zero (`session::Standing`), so a stamp kept from the last one would
    /// answer for a delete no listing has been read for.
    pub fn session_gone(&mut self) {
        *self = Self::default();
    }

    /// The rows to draw as gone.
    pub fn rows(&self) -> &Rows {
        &self.rows
    }

    /// The rows come back and the wait is over — git refused, or the
    /// last of them was proved gone. What the lists have drawn stays.
    fn put_back(&mut self) {
        self.rows = Rows::default();
        self.waiting = 0;
        self.landed = false;
        self.reads_from = 0;
    }

    /// Whether the list that draws `row` has drawn a listing that speaks
    /// for what the write left.
    fn answers_for(&self, row: Row) -> bool {
        self.landed && self.applied.at(row) >= self.reads_from
    }

    fn settle_row(&mut self, row: Row) {
        if !self.answers_for(row) {
            return;
        }
        self.rows.clear(row);
        self.settle();
    }

    /// Nothing left to stand in for: the wait goes with the last row, so
    /// a late answer under the spent id moves nothing.
    fn settle(&mut self) {
        if self.rows.is_empty() {
            self.put_back();
        }
    }
}
