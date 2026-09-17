//! The rows the window is showing as gone while git is being asked to
//! delete them (デザイン規約 §消す操作は先に画面から消す).

/// Which list a stood-in row was taken out of.
///
/// One name per list at most: a delete touches at most one row of each,
/// and the composite (`Delete both`) is the one that touches two lists
/// under a single write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Branch,
    Remote,
    Tag,
    Stash,
}

impl Row {
    /// Which of them the sidebar section called `section` draws, for the
    /// four that draw one at all (`NavSectionModel::attach_section_feed`
    /// names the same words). `None` for a section that stands nothing in
    /// — the working copies, and the working tree's own buckets.
    ///
    /// **Each is a list of its own**, drained on its own turn: the three
    /// refs sections are handed one snapshot and apply it at three
    /// different moments, so what one of them has drawn says nothing
    /// about what the others are still showing.
    #[must_use]
    pub fn drawn_by(section: &str) -> Option<Self> {
        match section {
            "branches" => Some(Self::Branch),
            "remotes" => Some(Self::Remote),
            "tags" => Some(Self::Tag),
            "stashes" => Some(Self::Stash),
            _ => None,
        }
    }
}

/// What is being shown as gone, one name per list — the picture the lists
/// and the graph's chips are drawn from. Empty means nothing is being
/// stood in for, which is also what a refusal goes back to.
///
/// Named the way the rows are named to git — a stash by its selector, a
/// ref by its short name — so a refusal puts back exactly what the press
/// took away.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rows {
    pub branch: String,
    pub remote: String,
    pub tag: String,
    pub stash: String,
}

impl Rows {
    /// Shows one row as gone. An unnamed row stands nothing in: the empty
    /// string is what a list is told when it is to put everything back.
    fn put(&mut self, row: Row, name: &str) {
        if name.is_empty() {
            return;
        }
        let seat = match row {
            Row::Branch => &mut self.branch,
            Row::Remote => &mut self.remote,
            Row::Tag => &mut self.tag,
            Row::Stash => &mut self.stash,
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
        }
    }

    fn is_empty(&self) -> bool {
        self.branch.is_empty()
            && self.remote.is_empty()
            && self.tag.is_empty()
            && self.stash.is_empty()
    }
}

/// When the list that draws each kind of row last applied a reading —
/// **one number per list**.
///
/// The three refs sections are handed the same snapshot and apply it on
/// three separate turns. Folded into a single newest-applied, the
/// branches section drawing the new listing would answer for a tag row
/// the tags section is still showing the old listing of, and the row a
/// delete took away would come back for as long as that section took to
/// catch up.
#[derive(Debug, Default)]
struct Applied {
    branch: u64,
    remote: u64,
    tag: u64,
    stash: u64,
}

impl Applied {
    fn at(&self, row: Row) -> u64 {
        match row {
            Row::Branch => self.branch,
            Row::Remote => self.remote,
            Row::Tag => self.tag,
            Row::Stash => self.stash,
        }
    }

    /// Writes down a reading that list has drawn; the newest stands.
    fn note(&mut self, row: Row, at: u64) {
        let seat = match row {
            Row::Branch => &mut self.branch,
            Row::Remote => &mut self.remote,
            Row::Tag => &mut self.tag,
            Row::Stash => &mut self.stash,
        };
        *seat = (*seat).max(at);
    }
}

/// One delete, from the press to the reading that proves it: the rows it
/// took off the screen, the write they are waiting on, and how far that
/// write has got.
///
/// **A delete takes the row away at the press, and git is asked behind
/// it** (デザイン規約 §消す操作は先に画面から消す). The write itself is
/// the short half — `git branch -d` is one process, and what the reader
/// is actually waiting on is the refs read and the walk that rebuilds the
/// graph behind it. Left to those, the row would sit there through all of
/// it with nothing to say whether the press even landed.
///
/// **Three moments, and each of them is told
/// to this type.**
///
/// 1. [`Self::asked`] — the press. The rows go, and the id the queue
///    accepted the write under is written down. Where it accepted none
///    (the session is closed) **the rows stay put**: an answer is what puts
///    the rows back, and a press nobody will ever answer would leave
///    them gone for good.
/// 2. [`Self::answered`] — git's own answer to that write. Refused, and
///    everything the press took comes straight back, because the row a
///    refusal is about has to be on screen when the reader is told about
///    it. Landed, and the rows stay away — the delete runs on:
///    the lists on screen still hold the rows themselves.
/// 3. [`Self::look_again`] — a list says it has drawn a reading, so what
///    the lists now hold may be the truth and there may be nothing left
///    to stand in for. Every row is asked, and each answers off the list
///    that draws it: the stash's listing is read after the graph is
///    rebuilt (`session::write`), so a dropped entry measured against
///    the refs' would be back on screen for the whole of the rebuild
///    between the two.
///
/// **By id alone** — and the same for the listings, by stamp. The
/// answer that comes back next may be another write's — every stash
/// operation answers under the same word, and a fetch running behind
/// a press answers in the middle of it — so only an answer carrying
/// this id moves the rows.
///
/// **What answers is a listing that saw the write.** The answer
/// and the listings travel feeds of their own and are applied in
/// no fixed order, so a read already in flight when the write
/// ended can be applied after the answer and would, taken as
/// proof, put the row back under the hand that had just taken it
/// away. Each listing says **when it looked**
/// ([`Self::listing_applied`], stamped before git was spawned)
/// and the write's answer says the smallest stamp that can speak
/// for what it left (`reads_from`, `session::Standing::fence`);
/// only a listing at or above it answers, and only it ends the wait.
///
/// **Applied on screen.** The stamps are written down by the list
/// that put the rows on screen, at the moment it put them there — core
/// having published a reading says nothing about what the window is
/// showing, which is the whole of what these rows are about.
///
/// **And by the list that draws that row.** Each kind of row
/// is a section of its own, handed the snapshot on its own turn
/// ([`Applied`]), so a section that has caught up answers for
/// its own row alone — a composite delete's two rows go as their
/// two lists reach them.
///
/// **The stamps belong to the session that numbered them.** They
/// survive a delete ending, because what a list has drawn is the
/// lists' own to keep, and go with the session ([`Self::session_gone`]):
/// the count begins again at zero with the next one.
///
/// **What it still cannot tell apart is the composite.** `Delete both`
/// deletes locally and then pushes, and a remote half that failed answers
/// with the same one error as a local half git would not do — so a landed
/// local delete is put back here too, until the refs read takes it away
/// again. Telling those apart needs a result per half from core
/// (rules-refs/app-ui.md).
#[derive(Debug, Default)]
pub struct StandIn {
    rows: Rows,
    /// The write the rows are waiting on, as the bridge carries an
    /// `OperationId` — zero while none is out.
    waiting: u64,
    /// Whether that write has answered and git did it. A reading answers
    /// for the rows only after it has: a listing already queued when the
    /// press landed says nothing about this delete, and read as though it
    /// did it would put the row straight back under the hand that had
    /// just taken it away.
    landed: bool,
    /// The smallest stamp a listing that saw what this write left can
    /// carry (`WriteFinished::reads_from`); zero while none is out.
    reads_from: u64,
    /// What each list has drawn. **Kept across a delete
    /// ending**, so the order the answer and the listings
    /// reach this type in stops mattering: a listing applied
    /// before the answer is still the proof it was, and the
    /// answer settles on it the moment it lands. It describes
    /// the lists, so it is theirs to keep.
    ///
    /// **The session ending is the one thing that does throw it away**
    /// ([`Self::session_gone`]) — the stamps are a session's own count,
    /// beginning again at zero with the next one (`session::Standing`),
    /// so one kept from a session that is over would sit above every
    /// number the new one can produce and answer for a delete nothing
    /// has read yet.
    applied: Applied,
}

impl StandIn {
    /// Takes this press's rows off the screen, under the id the queue
    /// accepted its write with.
    ///
    /// A press made while another delete is still standing takes the wait
    /// over — one key per list, the later name winning its own. The UI
    /// offers no second delete while a write is running, so this is the
    /// window between an answer and the reading that follows it.
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

    /// git answered a write. Only this one's own answer moves anything.
    ///
    /// `reads_from` is the smallest stamp a listing that saw what this
    /// write left can carry. A landing settles on the spot where a
    /// listing at or above it has **already** been applied — the answer
    /// and the listings arrive in no fixed order, and the one that comes
    /// second is the one that completes the pair.
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
    /// Written down and nothing more: whether it answers for the delete
    /// is [`Self::look_again`]'s to say, and the newest applied stands.
    pub fn listing_applied(&mut self, row: Row, at: u64) {
        self.applied.note(row, at);
    }

    /// A list has drawn what it was handed, so ask every row again
    /// whether the list that draws *it* has caught up.
    ///
    /// **One door, because everything it needs is here.** Which rows
    /// can go is decided from what each list has drawn against the fence
    /// the write named, so a caller naming the list it came from could
    /// only name it wrongly — a stash edge sent to a refs door would
    /// leave the stash row standing for good. The four are asked every
    /// time and each answers for itself; a row whose list has not caught
    /// up stays where it is.
    pub fn look_again(&mut self) {
        for row in [Row::Branch, Row::Remote, Row::Tag, Row::Stash] {
            self.settle_row(row);
        }
    }

    /// The session behind this tab is gone (`Hub::release_tab`), and with
    /// it every list that was drawing these rows and every number they
    /// were measured by.
    ///
    /// **Wider than a delete ending** ([`Self::put_back`]): the
    /// readings a list has drawn outlive one delete and are what the
    /// next one is measured against, and they go with the session that
    /// numbered them. A new session counts its reads from the start
    /// (`session::Standing`), so a stamp kept from the last one sits
    /// above everything the new one can produce and would answer for a
    /// delete no listing has been read for at all.
    pub fn session_gone(&mut self) {
        *self = Self::default();
    }

    /// The rows to draw as gone.
    pub fn rows(&self) -> &Rows {
        &self.rows
    }

    /// The rows come back and the wait is over — git refused, or the
    /// last of them was proved gone. What the lists have drawn is left
    /// alone: it describes them.
    fn put_back(&mut self) {
        self.rows = Rows::default();
        self.waiting = 0;
        self.landed = false;
        self.reads_from = 0;
    }

    /// Whether the list that draws `row` has drawn a listing that speaks
    /// for what the write left. Below the fence it was already in flight
    /// when the write ended; asked of the wrong list, it says nothing
    /// about this row at all.
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
