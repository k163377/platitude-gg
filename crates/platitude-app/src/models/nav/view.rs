use super::*;

// A list model that answers by value rather than by reference: `data()`
// computes the field a role asks for, where `QListModel::get` would have
// handed out a borrow of a stored `NavItem` and so forced every row to
// exist. Nothing here is a tree — `parent` is always invalid and rows hang
// off the root — but `QAbstractItemModel` is the base that lets a row be
// answered instead of held (and the one CXX-Qt expects, via
// `QAbstractListModel`).
impl QAbstractItemModel for NavSectionModel {
    fn index(&self, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex {
        let out_of_list = parent.is_valid()
            || column != 0
            || usize::try_from(row).is_ok_and(|row| row >= self.shown_rows());
        if out_of_list || row < 0 {
            return QModelIndex::default();
        }
        self.create_index(row, column, 0)
    }

    fn parent(&self, _child: &QModelIndex) -> QModelIndex {
        QModelIndex::default()
    }

    fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            0
        } else {
            self.shown_rows() as i32
        }
    }

    fn column_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() { 0 } else { 1 }
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let (Some(role), Some(row)) = (
            Role::of(role),
            usize::try_from(index.row())
                .ok()
                .and_then(|at| self.row_at(at)),
        ) else {
            return QVariant::default();
        };
        self.field(row, role).variant()
    }

    /// The names QML resolves a role by. Spelled where the answers are
    /// (`Role`), and held to the item's own derived table by the test at
    /// the foot of `role.rs`.
    fn role_names(&self) -> QHash<i32, QByteArray> {
        let mut names = QHash::default();
        for (number, role) in Role::ALL.iter().enumerate() {
            names.insert(&(number as i32), &QByteArray::from(role.spelling()));
        }
        names
    }
}

impl NavSectionModel {
    /// Shapes the arrived rows into the ones on screen — indented, folded,
    /// filtered — and finds the current entry among them.
    pub(super) fn arrange(&mut self) {
        let needle = self.filter.to_lowercase();
        self.tree_named =
            needle.is_empty() && matches!(self.section.as_str(), "branches" | "remotes");
        self.arranged = if needle.is_empty() {
            match self.section.as_str() {
                "branches" | "remotes" => Some(self.build_tree()),
                // The worktree keeps its group runs (conflicts → unstaged →
                // staged) and trees each run independently.
                "worktree" if self.tree_view => {
                    let mut out = Vec::new();
                    let mut at = 0;
                    while at < self.all.len() {
                        let run = self.run_of(at);
                        let mut end = at + 1;
                        while end < self.all.len() && self.run_of(end) == run {
                            end += 1;
                        }
                        self.wt_tree_into(at..end, run, &mut out);
                        at = end;
                    }
                    Some(out)
                }
                _ => None,
            }
        } else {
            // Filtering shows flat full names (folders would hide context).
            Some(
                (0..self.all.len())
                    .filter(|at| {
                        self.all
                            .entry(*at)
                            .is_some_and(|of| of.name().to_lowercase().contains(&needle))
                    })
                    .map(|at| Arranged::At {
                        at: at as u32,
                        depth: 0,
                        from: 0,
                    })
                    .collect(),
            )
        };
        self.head_row = (0..self.shown_rows())
            .find(|at| {
                self.row_at(*at).is_some_and(|row| {
                    self.field(row, Role::IsHead).flag() && !self.field(row, Role::Folder).flag()
                })
            })
            .map_or(-1, |row| row as i32);
    }

    /// Shapes the rows again and tells the view its whole list changed.
    /// The begin/end pair is written out — `QAbstractItemModel` has no
    /// `reset` wrapper to inherit.
    pub(super) fn reshape(&mut self) {
        self.begin_reset_model();
        self.arrange();
        self.end_reset_model();
    }

    pub(super) fn shown_rows(&self) -> usize {
        match &self.arranged {
            Some(arranged) => arranged.len(),
            None => self.all.len(),
        }
    }

    pub(super) fn row_at(&self, at: usize) -> Option<Row<'_>> {
        match &self.arranged {
            Some(arranged) => match arranged.get(at)? {
                Arranged::At { at, depth, from } => Some(Row::Shown {
                    of: self.all.entry(*at as usize)?,
                    depth: *depth,
                    from: *from as usize,
                }),
                Arranged::Made(item) => Some(Row::Made(item)),
            },
            None => Some(Row::Shown {
                of: self.all.entry(at)?,
                depth: 0,
                from: 0,
            }),
        }
    }

    pub(super) fn shown_name(of: Entry<'_>, from: usize) -> &str {
        let whole = of.name();
        whole.get(from..).unwrap_or(whole)
    }

    /// The local name a remote-tracking ref would take, read from the
    /// snapshot that made the remote row visible. Keeping this answer with
    /// that snapshot avoids a separate tab-level feed racing the first
    /// double-click on a remote row.
    pub(super) fn local_name_of(&self, remote_ref: &str) -> String {
        let Source::Remotes(snapshot) = &self.all else {
            return remote_ref.to_string();
        };
        snapshot
            .remote_names
            .iter()
            .filter_map(|remote| {
                remote_ref
                    .strip_prefix(remote.as_str())
                    .and_then(|rest| rest.strip_prefix('/'))
                    .filter(|rest| !rest.is_empty())
            })
            // A remote may itself contain `/`; the shortest remainder is
            // the longest configured remote prefix.
            .min_by_key(|rest| rest.len())
            .unwrap_or(remote_ref)
            .to_string()
    }

    /// What the row identified by one field answers for another.
    ///
    /// Asks the section's whole source rather than the visible rows, so
    /// an active filter or a collapsed folder does not hide the answer —
    /// and asks it undented, because a name given from outside is the
    /// whole one git knows.
    pub(super) fn told(&self, known: Role, text: &str, wanted: Role) -> String {
        (0..self.all.len())
            .filter_map(|at| self.all.entry(at))
            .map(|of| Row::Shown {
                of,
                depth: 0,
                from: 0,
            })
            .find(|row| self.field(*row, known).as_str() == text)
            .map(|row| self.field(row, wanted).as_str().to_string())
            .unwrap_or_default()
    }

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
            .map(|row| {
                format!(
                    "{}:{}",
                    self.field(*row, Role::Bucket).as_str(),
                    self.field(*row, Role::Full).as_str()
                )
            })
            .unwrap_or_default()
    }

    /// The file row `way` steps from `path` in `bucket`, as
    /// `<row>\u{1e}<bucket>\u{1e}<path>`. Empty where the walk has nowhere
    /// left to go — which is how the arrows stop at the ends rather than
    /// wrapping — and empty where the path is not shown at all
    /// (デザイン規約 §diff のファイル一覧). Only the sign of `way` is read:
    /// one press is one file.
    ///
    /// **The rows on screen, not the source.** A folder the reader closed is
    /// a folder the walk does not enter, and a file a filter hid is hidden
    /// from the arrows too. Folder rows themselves are stepped over, since a
    /// folder has no diff to move to. The buckets *are* crossed: this is one
    /// list, and the heading below the last unstaged file is something
    /// walking down goes past rather than stops at.
    ///
    /// The path comes last because it is the only field git lets hold the
    /// separator.
    pub(super) fn step(&self, bucket: &str, path: &str, way: i32) -> String {
        let shown = self.shown_rows();
        let Some(from) = (0..shown).find(|at| {
            self.row_at(*at)
                .is_some_and(|row| self.is(row, bucket, path))
        }) else {
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
        landed
            .and_then(|at| self.row_at(at).map(|row| (at, row)))
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

    /// What one row on screen shows for one role (empty out of range).
    pub(super) fn shows(&self, row: i32, role: Role) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|at| self.row_at(at))
            .map(|row| self.field(row, role).as_str().to_string())
            .unwrap_or_default()
    }

    /// The display run a source row sits in — what the working tree's
    /// list is built one of at a time, so that a folder of the same name
    /// under two of them folds apart.
    fn run_of(&self, at: usize) -> &'static str {
        match self.all.entry(at) {
            Some(Entry::File { bucket, .. }) => bucket.run(),
            _ => "",
        }
    }

    /// Points the section at what just arrived, and answers whether the
    /// rows it shows moved.
    ///
    /// A poll tick republishes refs and status whether or not they moved;
    /// swapping identical data in would still reset the Qt model — every
    /// delegate rebuilt, the inner list scrolled back. The entries
    /// themselves are compared: one moved branch is not a reason for the
    /// tag section to rebuild forty-five thousand delegates.
    pub(super) fn take(&mut self, arrived: Source) -> bool {
        let moved = match (&self.all, &arrived) {
            (Source::Locals(held), Source::Locals(fresh)) => held.locals != fresh.locals,
            (Source::Remotes(held), Source::Remotes(fresh)) => held.remotes != fresh.remotes,
            (Source::Tags(held), Source::Tags(fresh)) => held.tags != fresh.tags,
            (Source::Stashes(held), Source::Stashes(fresh)) => held != fresh,
            (
                Source::Worktrees {
                    list: held,
                    current: was,
                },
                Source::Worktrees {
                    list: fresh,
                    current,
                },
            ) => held != fresh || was != current,
            (Source::Files { status: held, .. }, Source::Files { status: fresh, .. }) => {
                held.items != fresh.items
            }
            _ => true,
        };
        self.all = arrived;
        moved
    }

    /// Filed as two lines on purpose: `arranged` being nothing is what
    /// says the view is reading the source directly, and a single number
    /// would hide the day that stops being true.
    pub(super) fn note_footprint(&self) {
        crate::memprobe::note_bytes(
            &format!("nav-{}-all", self.section),
            self.tab_id,
            platitude_core::mem::Footprint::heap_bytes(&self.all),
            self.all.len(),
        );
        crate::memprobe::note_bytes(
            &format!("nav-{}-arranged", self.section),
            self.tab_id,
            self.arranged
                .as_ref()
                .map_or(0, platitude_core::mem::Footprint::heap_bytes),
            self.arranged.as_ref().map_or(0, Vec::len),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;
    use crate::encode::FIELD_SEP;

    #[test]
    fn a_filtered_remote_row_shows_its_whole_name() {
        let mut model = section(
            "remotes",
            Source::Remotes(snapshot(
                vec![remote("origin/feature/one"), remote("origin/main")],
                Vec::new(),
            )),
        );
        model.filter = "feature".to_string();
        model.arrange();

        assert_eq!(model.shown_rows(), 1);
        assert_eq!(says(&model, 0, Role::Name), "origin/feature/one");
        assert_eq!(says(&model, 0, Role::Full), "");
    }

    #[test]
    fn a_remote_row_derives_its_local_name_from_its_own_snapshot() {
        let model = section(
            "remotes",
            Source::Remotes(Arc::new(platitude_core::session::RefsSnapshot {
                remote_names: vec!["my".into(), "my/fork".into()],
                ..Default::default()
            })),
        );

        assert_eq!(model.local_name_of("my/fork/feature/one"), "feature/one");
        assert_eq!(model.local_name_of("unknown/feature"), "unknown/feature");
    }
    /// The arrows walk the rows on screen, one file per press, and stop at
    /// each end rather than wrapping (規約 §diff のファイル一覧).
    #[test]
    fn the_arrows_walk_the_files_and_stop_at_the_ends() {
        let mut model = section("worktree", Source::files(pending()));
        model.tree_view = false;
        model.arrange();

        // Down out of the unstaged run and into the next heading's file: one
        // list, and the band between them is something walking goes past.
        assert_eq!(
            model.step("unstaged", "src/b.txt", 1),
            format!("2{FIELD_SEP}untracked{FIELD_SEP}c.txt")
        );
        assert_eq!(
            model.step("unstaged", "src/b.txt", -1),
            format!("0{FIELD_SEP}conflicts{FIELD_SEP}a.txt")
        );
        // Only the sign is read: one press is one file.
        assert_eq!(
            model.step("unstaged", "src/b.txt", 9),
            model.step("unstaged", "src/b.txt", 1)
        );
        // Both ends, and the two rows of the file that is on both sides:
        // asked by bucket, each walks on from its own row.
        assert_eq!(model.step("conflicts", "a.txt", -1), "");
        assert_eq!(model.step("staged", "d.txt", 1), "");
        assert_eq!(
            model.step("staged", "src/b.txt", 1),
            format!("4{FIELD_SEP}staged{FIELD_SEP}d.txt")
        );
        // A path no row of this list holds has nowhere to walk from.
        assert_eq!(model.step("unstaged", "nowhere.txt", 1), "");
    }
    /// A folder is not a file: the walk steps over its row rather than
    /// landing on one that has no diff behind it.
    #[test]
    fn the_arrows_step_over_a_folder_row() {
        let mut model = section("worktree", Source::files(pending()));
        model.tree_view = true;
        model.arrange();

        // The tree puts the folder `src` between the conflicted file and the
        // leaf under it, and a folder has no diff to land on.
        assert!(flags(&model, 1, Role::Folder));
        assert_eq!(
            model.step("conflicts", "a.txt", 1),
            format!("2{FIELD_SEP}unstaged{FIELD_SEP}src/b.txt")
        );
        assert!(flags(&model, 4, Role::Folder));
        assert_eq!(
            model.step("untracked", "c.txt", 1),
            format!("5{FIELD_SEP}staged{FIELD_SEP}src/b.txt")
        );
    }
    /// A snapshot that carries the same tags is not a reason to rebuild
    /// tens of thousands of delegates.
    #[test]
    fn a_republished_snapshot_moves_nothing() {
        let mut model = section("tags", Source::default());
        assert!(model.take(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", true, true)]
        ))));
        assert!(!model.take(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", true, true)]
        ))));
        assert!(model.take(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.1", true, true)]
        ))));
    }
}
