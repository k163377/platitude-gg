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
