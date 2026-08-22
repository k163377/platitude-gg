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
                // The worktree holds every bucket run (conflicts → unstaged
                // → staged) and shows one of them: the run this list is the
                // list of, or all of them where none was named (the tests,
                // which read the source's own order). Trees are built a run
                // at a time, so a folder of the same name under two of them
                // folds apart.
                "worktree" => {
                    let mut out = Vec::new();
                    let mut at = 0;
                    while at < self.all.len() {
                        let run = self.run_of(at);
                        let mut end = at + 1;
                        while end < self.all.len() && self.run_of(end) == run {
                            end += 1;
                        }
                        if self.run.is_empty() || run == self.run {
                            if self.tree_view {
                                self.wt_tree_into(at..end, run, &mut out);
                            } else {
                                out.extend((at..end).map(|at| Arranged::At {
                                    at: at as u32,
                                    depth: 0,
                                    from: 0,
                                }));
                            }
                        }
                        at = end;
                    }
                    Some(out)
                }
                // Nothing arranges these — until a row is being shown as
                // gone, which is an order of its own and has to be written
                // down (`hidden_at`).
                _ if self.hidden.is_empty() => None,
                _ => Some(
                    (0..self.all.len())
                        .filter(|at| !self.hidden_at(*at))
                        .map(|at| Arranged::At {
                            at: at as u32,
                            depth: 0,
                            from: 0,
                        })
                        .collect(),
                ),
            }
        } else {
            // Filtering shows flat full names (folders would hide context).
            Some(
                (0..self.all.len())
                    .filter(|at| self.in_run(*at))
                    .filter(|at| !self.hidden_at(*at))
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
        self.shown_total = self.shown_rows() as i32;
        // The count the section's band shows. **A hidden row is not one
        // of them** — the band is saying how many the repository has, and
        // a row being shown as already deleted has to be gone from the
        // number as well as from the list, or the band reads one more
        // than the reader can count (デザイン規約 §消す操作は先に画面から消す).
        // A *filtered* row still counts: that band answers "how many are
        // there", not "how many match" (`set_filter`).
        self.total = (self.all.len() - self.all.named_count(&self.hidden)) as i32;
        // The bucket heading's number, counted where the rows are cut to
        // the run — so the heading and the list under it cannot come to
        // read the bucket rule (`Bucket::run`) two ways. Filter-
        // independent for the same reason `total` is.
        self.run_files = (0..self.all.len()).filter(|at| self.in_run(*at)).count() as i32;
    }

    /// Whether this source row is one the page is already showing as gone
    /// (`Source::is_named`).
    pub(super) fn hidden_at(&self, at: usize) -> bool {
        !self.hidden.is_empty() && self.all.is_named(at, &self.hidden)
    }

    /// Whether a source row belongs to the run this list shows. Every row
    /// does where no run was named — every section but the worktree.
    fn in_run(&self, at: usize) -> bool {
        self.run.is_empty() || self.run_of(at) == self.run
    }

    /// What the memory report files this list under. The worktree is three
    /// lists, one per bucket run, and three lines under one name would be
    /// read as one list that grew.
    fn named(&self) -> String {
        if self.run.is_empty() {
            self.section.clone()
        } else {
            format!("{}-{}", self.section, self.run)
        }
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

    /// Which of the rows on show a ref sits on, by the name git knows it
    /// by; -1 when it is on none.
    ///
    /// Asked of the rows as they stand, unlike `told`: what this answers
    /// is where to scroll, and a row behind a filter or folded into a
    /// closed folder is nowhere the view can go. The name is read the
    /// way the rows are keyed (`NavList.keyOf`) — the full one, or what
    /// the row shows where there is no full one.
    pub(super) fn row_of(&self, name: &str) -> i32 {
        (0..self.shown_rows())
            .find(|at| {
                self.row_at(*at).is_some_and(|row| {
                    let full = self.field(row, Role::Full);
                    if full.as_str().is_empty() {
                        self.field(row, Role::Name).as_str() == name
                    } else {
                        full.as_str() == name
                    }
                })
            })
            .map_or(-1, |at| at as i32)
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
    /// folder has no diff to move to. **One bucket run, because a list is
    /// one run**: the answer runs out at this list's own end, and crossing
    /// into the next bucket's list is the pane's step (`FileRowWalk`), which
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
    /// rows on screen rather than the source for the reason [`Self::step`]
    /// gives — this is where a walk sets off from, and a row folded away is
    /// nowhere it can stand.
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
    /// a walk goes past an empty bucket instead of stopping in it.
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
            &format!("nav-{}-all", self.named()),
            self.tab_id,
            platitude_core::mem::Footprint::heap_bytes(&self.all),
            self.all.len(),
        );
        crate::memprobe::note_bytes(
            &format!("nav-{}-arranged", self.named()),
            self.tab_id,
            self.arranged
                .as_ref()
                .map_or(0, platitude_core::mem::Footprint::heap_bytes),
            self.arranged.as_ref().map_or(0, Vec::len),
        );
    }
}
