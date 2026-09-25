//! The walk the arrow keys take over the file rows: a step, and where a
//! walk crossing in from another bucket lands (デザイン規約 §diff のファイル一覧).

use super::*;

impl NavSectionModel {
    /// Whether this section holds `path` in `bucket`. Both halves are
    /// needed: an `MM` file has a row under each side.
    pub(super) fn holds(&self, bucket: &str, path: &str) -> bool {
        self.rows().any(|row| self.is(row, bucket, path))
    }

    /// Where the reader lands when `path` leaves `bucket`: the next file
    /// under the same heading, else the previous. Empty when the heading
    /// holds nothing else or the path is not in it — the caller asks while
    /// it still is (`RepoPage.noteDiffNeighbour`). The answer carries its
    /// bucket: untracked files sit under the unstaged heading
    /// (`NavItem.group`).
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

    /// The file row on screen at `at` as `<bucket>:<path>`; empty for a
    /// folder row and past the end. The pane asks here, not its delegates,
    /// which exist only for the viewport
    /// (rules-refs/app-ui.md「WIP の選択は模型が解く」).
    pub(super) fn file_key(&self, at: usize) -> String {
        self.row_at(at)
            .filter(|row| !self.field(*row, Role::Folder).flag())
            .map(|row| self.keyed(row))
            .unwrap_or_default()
    }

    /// One row as `<bucket>:<path>`, the one spelling of the pane's key.
    /// The first colon divides: buckets hold none, paths may.
    fn keyed(&self, row: Row) -> String {
        format!(
            "{}:{}",
            self.field(row, Role::Bucket).as_str(),
            self.field(row, Role::Full).as_str()
        )
    }

    /// The file row `way` steps to from `path` in `bucket`
    /// (`encode::Landing`); only the sign of `way` is read. Nothing at
    /// this list's ends or where the path is not shown.
    ///
    /// Walks the rows on screen: closed folders and filtered files are not
    /// entered, folder rows are stepped over. Crossing into the next
    /// bucket's list is the pane's step (`FileRowWalk` → [`Self::edge`]).
    pub(super) fn step(&self, bucket: &str, path: &str, way: i32) -> Landed {
        let shown = self.shown_rows();
        let Ok(from) = usize::try_from(self.row_of_file(bucket, path)) else {
            return Landed::none();
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

    /// Which row on screen holds `path` in `bucket`; -1 when none does
    /// (both halves: [`Self::holds`]; on screen: [`Self::step`]).
    pub(super) fn row_of_file(&self, bucket: &str, path: &str) -> i32 {
        (0..self.shown_rows())
            .find(|at| {
                self.row_at(*at)
                    .is_some_and(|row| self.is(row, bucket, path))
            })
            .map_or(-1, |at| at as i32)
    }

    /// Where a walk crossing into this list lands: its first file row for
    /// a forward `way`, its last for a backward one. Nothing where the
    /// list holds no file row, so the walk passes an empty bucket.
    pub(super) fn edge(&self, way: i32) -> Landed {
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

    /// Where a walk landed, shared by `step` and `edge`. Nothing for no
    /// row.
    fn landing(&self, at: Option<usize>) -> Landed {
        Landed::new(
            at.and_then(|at| self.row_at(at).map(|row| (at, row)))
                .map(|(at, row)| Landing {
                    row: i32::try_from(at).unwrap_or(-1),
                    bucket: self.field(row, Role::Bucket).as_str().to_string(),
                    path: self.field(row, Role::Full).as_str().to_string(),
                }),
        )
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
