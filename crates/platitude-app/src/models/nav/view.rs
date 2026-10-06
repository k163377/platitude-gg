use super::*;

impl NavSectionModel {
    /// Shapes the arrived rows into the ones on screen — indented, folded,
    /// filtered — and finds the current entry among them.
    pub(super) fn arrange(&mut self) {
        let needle = self.filter.to_lowercase();
        // Filtered branches stand flat by whole name; remotes sit under a
        // remote row either way (`build_remote_groups`).
        self.tree_named = match self.section.as_str() {
            "remotes" => true,
            "branches" => needle.is_empty(),
            _ => false,
        };
        self.arranged = if needle.is_empty() {
            match self.section.as_str() {
                "branches" | "remotes" => Some(self.build_tree()),
                // Every bucket run is held; this list shows its own `run`,
                // or all of them where none is named (tests). Trees are
                // built per run (`file_tree_into`).
                "files" => {
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
                                self.file_tree_into(at..end, run, &mut out);
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
                // Read straight from the source unless a row is hidden.
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
        } else if self.section == "remotes" {
            Some(self.build_remote_groups(&needle))
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
        self.place_head();
        self.shown_total = self.shown_rows() as i32;
        // The band's count: hidden rows come off it, filtered rows do not
        // (デザイン規約 §消す操作は先に画面から消す).
        self.total = (self.all.len() - self.all.named_count(&self.hidden)) as i32;
        // The bucket heading's number, cut to the run here as the list is,
        // so the two cannot read `Bucket::run` two ways. 0 without a run:
        // the property is only the heading's.
        self.run_files = if self.run.is_empty() {
            0
        } else {
            (0..self.all.len())
                .filter(|at| self.in_run(*at) && !self.hidden_at(*at))
                .count() as i32
        };
    }

    /// Finds the current entry among the rows on show and settles its
    /// stand-in (`HeadPinRow`): its row, the folded row holding it, and
    /// the column and name it takes — the closed folder's column, named
    /// from that folder down; the whole name under a filter (デザイン規約
    /// §左メニューの所作 の張り付き行の表).
    fn place_head(&mut self) {
        let head = (0..self.shown_rows()).find(|at| {
            self.row_at(*at).is_some_and(|row| {
                self.field(row, Role::IsHead).flag() && !self.field(row, Role::Folder).flag()
            })
        });
        self.head_row = head.map_or(-1, |row| row as i32);
        self.head_under_row = if head.is_none() {
            self.folded_over_head()
        } else {
            -1
        };
        let (under_depth, under_cut) = match usize::try_from(self.head_under_row)
            .ok()
            .and_then(|at| self.row_at(at))
        {
            Some(row) => (
                self.field(row, Role::Depth).number(),
                self.field(row, Role::Full)
                    .as_str()
                    .rfind('/')
                    .map_or(0, |at| at + 1),
            ),
            None => (0, 0),
        };
        let depth = match head.and_then(|at| self.row_at(at)) {
            Some(row) => self.field(row, Role::Depth).number(),
            None => under_depth,
        };
        let shown = self
            .head_name
            .get(under_cut..)
            .unwrap_or(&self.head_name)
            .to_string();
        self.head_depth = depth;
        self.head_shown = shown;
    }

    /// The shown row of the closed folder hiding the current branch, or
    /// -1. Only the outermost closed folder is emitted (`build_tree`), so
    /// the folded row whose key prefixes the name is it. A filter
    /// flattens the tree, so it answers -1.
    fn folded_over_head(&self) -> i32 {
        if self.head_name.is_empty() || !self.filter.is_empty() {
            return -1;
        }
        (0..self.shown_rows())
            .find(|at| {
                self.row_at(*at).is_some_and(|row| {
                    self.field(row, Role::Folder).flag()
                        && self.field(row, Role::Change).as_str() == FOLDED
                        && self
                            .head_name
                            .starts_with(&format!("{}/", self.field(row, Role::Full).as_str()))
                })
            })
            .map_or(-1, |at| at as i32)
    }

    /// Whether this source row is one the page is already showing as gone
    /// (`Source::is_named`).
    pub(super) fn hidden_at(&self, at: usize) -> bool {
        !self.hidden.is_empty() && self.all.is_named(at, &self.hidden)
    }

    /// Whether a source row belongs to the run this list shows (every row
    /// where no run was named).
    fn in_run(&self, at: usize) -> bool {
        self.run.is_empty() || self.run_of(at) == self.run
    }

    /// The memory report's name for this list — one per working-tree run, so
    /// three lists do not read as one that grew.
    fn named(&self) -> String {
        if self.run.is_empty() {
            self.section.clone()
        } else {
            format!("{}-{}", self.section, self.run)
        }
    }

    /// Re-arranges under one model reset, spelled as a begin/end pair —
    /// `QAbstractItemModel` has no `reset` wrapper.
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
            Some(arranged) => self.read_arranged(arranged.get(at)?),
            None => Some(Row::Shown {
                of: self.all.entry(at)?,
                depth: 0,
                from: 0,
            }),
        }
    }

    /// One arranged row as the view reads it — the list on screen's, or a
    /// tree arranged for somebody else (`card`).
    pub(super) fn read_arranged<'a>(&'a self, arranged: &'a Arranged) -> Option<Row<'a>> {
        match arranged {
            Arranged::At { at, depth, from } => Some(Row::Shown {
                of: self.all.entry(*at as usize)?,
                depth: *depth,
                from: *from as usize,
            }),
            Arranged::Made(item) => Some(Row::Made(item)),
        }
    }

    pub(super) fn shown_name(of: Entry<'_>, from: usize) -> &str {
        let whole = of.name();
        whole.get(from..).unwrap_or(whole)
    }

    /// The local name a remote-tracking ref would take, from the snapshot
    /// that shows the row — a separate feed would race the first
    /// double-click.
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

    /// What the row identified by one field answers for another, over the
    /// whole source (filters and folds hide nothing) and undented (a name
    /// from outside is the whole one).
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

    /// The path of the other worktree holding `branch` (contract on
    /// the `worktree_holding` slot). Not `told`: two fields decide it — the
    /// row's branch, and not being this window's own worktree.
    pub(super) fn worktree_with(&self, branch: &str) -> String {
        if branch.is_empty() || self.section != "worktrees" {
            return String::new();
        }
        (0..self.all.len())
            .filter_map(|at| self.all.entry(at))
            .map(|of| Row::Shown {
                of,
                depth: 0,
                from: 0,
            })
            .find(|row| {
                self.field(*row, Role::Bucket).as_str() == branch
                    && !self.field(*row, Role::IsHead).flag()
            })
            .map(|row| self.field(row, Role::Full).as_str().to_string())
            .unwrap_or_default()
    }

    /// Where a renamed file came from (contract on the `orig_of` slot).
    /// Not `told`, which takes the path's first row whatever it holds.
    pub(super) fn orig_path_of(&self, path: &str) -> String {
        (0..self.all.len())
            .filter_map(|at| self.all.entry(at))
            .map(|of| Row::Shown {
                of,
                depth: 0,
                from: 0,
            })
            .filter(|row| self.field(*row, Role::Full).as_str() == path)
            .map(|row| self.field(row, Role::OrigPath).as_str().to_string())
            .find(|orig| !orig.is_empty())
            .unwrap_or_default()
    }

    /// Which row on show a ref sits on, by the name git knows it by; -1
    /// when none. Reads the rows on show, unlike `told` — this answers
    /// where to scroll. Rows are keyed as `NavList.keyOf` keys them: the
    /// full name, else the shown one.
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

    /// What one row on screen shows for one role (empty out of range).
    pub(super) fn shows(&self, row: i32, role: Role) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|at| self.row_at(at))
            .map(|row| self.field(row, role).as_str().to_string())
            .unwrap_or_default()
    }

    /// The display run a source row sits in (`Bucket::run`); empty for
    /// non-file rows.
    fn run_of(&self, at: usize) -> &'static str {
        match self.all.entry(at) {
            Some(Entry::File { bucket, .. }) => bucket.run(),
            _ => "",
        }
    }

    /// Points the section at what just arrived, and answers whether its
    /// rows moved. Polls republish unchanged data, and a reset rebuilds
    /// every delegate and scrolls the list back, so each section compares
    /// its own entries.
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

    /// Two lines on purpose: an empty `arranged` shows the view reading
    /// the source directly, which one number would hide.
    pub(super) fn note_footprint(&self) {
        crate::harness::memprobe::note_bytes(
            &format!("nav-{}-all", self.named()),
            self.tab_id,
            platitude_core::mem::Footprint::heap_bytes(&self.all),
            self.all.len(),
        );
        crate::harness::memprobe::note_bytes(
            &format!("nav-{}-arranged", self.named()),
            self.tab_id,
            self.arranged
                .as_ref()
                .map_or(0, platitude_core::mem::Footprint::heap_bytes),
            self.arranged.as_ref().map_or(0, Vec::len),
        );
    }
}
