//! The walk the arrow keys take over the file rows: where a step from
//! one lands, where a walk crossing in from the bucket above or below
//! comes in, and the one spelling both answers are given in
//! (デザイン規約 §diff のファイル一覧).

use super::*;

impl NavSectionModel {
    /// Whether this section holds `path` in `bucket`. Both halves are
    /// needed: a file changed on both sides at once has a row under each,
    /// and asking by path alone always answers with the first.
    pub(super) fn holds(&self, bucket: &str, path: &str) -> bool {
        self.rows().any(|row| self.is(row, bucket, path))
    }

    /// Where the reader lands when `path` leaves `bucket`: the file under
    /// the same heading after it, or the one before it where it was the
    /// last. Empty when that heading holds nothing else, and empty when
    /// the path is not in it at all — the caller asks while it still is
    /// (`RepoPage.noteDiffNeighbour`).
    ///
    /// The answer carries its bucket, because the heading a file sits
    /// under is not the bucket it belongs to: untracked files are shown
    /// among the unstaged ones (`NavItem.group`).
    pub(super) fn beside(&self, bucket: &str, path: &str) -> String {
        let rows: Vec<Row> = self.rows().collect();
        let Some(at) = rows.iter().position(|row| self.is(*row, bucket, path)) else {
            return String::new();
        };
        let group = self.field(rows[at], Role::Group).as_str().to_string();
        let under = |row: &&Row| self.field(**row, Role::Group).as_str() == group;
        rows[at + 1..]
            .iter()
            .find(under)
            .or_else(|| rows[..at].iter().rev().find(under))
            .map(|row| self.keyed(*row))
            .unwrap_or_default()
    }

    /// The file row on screen at `at`, keyed the way the pane holds its
    /// choice — `<bucket>:<path>`. Empty for a folder row, and empty past
    /// the end.
    ///
    /// **What lets a choice reach a row nobody has scrolled to.** A view
    /// builds a delegate for the rows it is showing and no others, so a
    /// walk over the delegates answers for the viewport: a Shift-reach
    /// across a long bucket, or the rows a discard is about to take, would
    /// come back cut down to what happens to be on screen — silently, since
    /// the rows it skipped look no different from rows nobody chose. Every
    /// row is answerable here (`data` computes by value), so the pane asks
    /// the model.
    pub(super) fn file_key(&self, at: usize) -> String {
        self.row_at(at)
            .filter(|row| !self.field(*row, Role::Folder).flag())
            .map(|row| self.keyed(row))
            .unwrap_or_default()
    }

    /// One row as `<bucket>:<path>` — the one spelling of the key a pane
    /// holds a working-tree row by, so the pane's own composing of it and
    /// the answers here cannot drift apart. The bucket never holds a
    /// colon, so the first one is the divide; the path may hold as many
    /// as it likes.
    fn keyed(&self, row: Row) -> String {
        format!(
            "{}:{}",
            self.field(row, Role::Bucket).as_str(),
            self.field(row, Role::Full).as_str()
        )
    }

    /// The file row `way` steps from `path` in `bucket`, as
    /// `<row>\u{1e}<bucket>\u{1e}<path>`. Empty where the walk has nowhere
    /// left to go — which is how the arrows stop at the ends — and empty
    /// where the path is not shown at all
    /// (デザイン規約 §diff のファイル一覧). Only the sign of `way` is read:
    /// one press is one file.
    ///
    /// **The rows on screen.** A folder the reader closed is a folder the
    /// walk does not enter, and a file a filter hid is hidden from the
    /// arrows too. Folder rows themselves are stepped over, since a folder
    /// has no diff to move to. **One bucket run, because a list is one
    /// run**: the answer runs out at this list's own end, and crossing into
    /// the next bucket's list is the pane's step (`FileRowWalk`), which
    /// asks that list for the row at its near end ([`Self::edge`]).
    pub(super) fn step(&self, bucket: &str, path: &str, way: i32) -> String {
        let shown = self.shown_rows();
        let Ok(from) = usize::try_from(self.row_of_file(bucket, path)) else {
            return String::new();
        };
        let file = |at: &usize| {
            self.row_at(*at)
                .is_some_and(|row| !self.field(row, Role::Folder).flag())
        };
        let landed = if way < 0 {
            (0..from).rev().find(file)
        } else {
            (from + 1..shown).find(file)
        };
        self.landing(landed)
    }

    /// Which row on screen holds `path` in `bucket`; -1 when none does.
    /// Both halves are read for the reason [`Self::holds`] gives, and the
    /// rows on screen for the reason [`Self::step`] gives — this is where
    /// a walk sets off from, and a row folded away is nowhere it can
    /// stand.
    pub(super) fn row_of_file(&self, bucket: &str, path: &str) -> i32 {
        (0..self.shown_rows())
            .find(|at| {
                self.row_at(*at)
                    .is_some_and(|row| self.is(row, bucket, path))
            })
            .map_or(-1, |at| at as i32)
    }

    /// The file row at one end of this list — the first when `way` reads
    /// forwards, the last when it reads back — in [`Self::step`]'s own
    /// shape. Empty where the list holds no file row at all, which is how
    /// a walk goes past an empty bucket.
    ///
    /// This is the other half of crossing a bucket: `step` runs out at the
    /// end of its own run, and the list the walk carries on into is asked
    /// for the row nearest the edge it comes in by.
    pub(super) fn edge(&self, way: i32) -> String {
        let shown = self.shown_rows();
        let file = |at: &usize| {
            self.row_at(*at)
                .is_some_and(|row| !self.field(row, Role::Folder).flag())
        };
        let landed = if way < 0 {
            (0..shown).rev().find(file)
        } else {
            (0..shown).find(file)
        };
        self.landing(landed)
    }

    /// Where a walk landed, as `<row>\u{1e}<bucket>\u{1e}<path>` — the one
    /// spelling of it, so the two ways to land (a step, and coming in at a
    /// list's edge) cannot drift apart. Empty for no row.
    ///
    /// The path comes last because it is the only field git lets hold the
    /// separator.
    fn landing(&self, at: Option<usize>) -> String {
        at.and_then(|at| self.row_at(at).map(|row| (at, row)))
            .map(|(at, row)| {
                format!(
                    "{at}{sep}{bucket}{sep}{path}",
                    sep = crate::encode::FIELD_SEP,
                    bucket = self.field(row, Role::Bucket).as_str(),
                    path = self.field(row, Role::Full).as_str()
                )
            })
            .unwrap_or_default()
    }

    /// Every source row of this section, in the order the list shows them.
    fn rows(&self) -> impl Iterator<Item = Row<'_>> + '_ {
        (0..self.all.len())
            .filter_map(|at| self.all.entry(at))
            .map(|of| Row::Shown {
                of,
                depth: 0,
                from: 0,
            })
    }

    fn is(&self, row: Row, bucket: &str, path: &str) -> bool {
        self.field(row, Role::Bucket).as_str() == bucket
            && self.field(row, Role::Full).as_str() == path
    }
}
