use super::*;

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
        // independent for the same reason `total` is, and hidden rows
        // come off it by `total`'s own rule. Sections without a run
        // (everything but the worktree buckets) answer 0: the property
        // is the bucket heading's, and a tags list publishing its whole
        // row count under this name would only invite a wrong reader.
        self.run_files = if self.run.is_empty() {
            0
        } else {
            (0..self.all.len())
                .filter(|at| self.in_run(*at) && !self.hidden_at(*at))
                .count() as i32
        };
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

    /// The same lookup for a role whose answer is a flag, and `None`
    /// where no row answers to `text` at all.
    ///
    /// Apart from [`Self::told`] because a flag has no spelling — the
    /// value's `as_str` is empty for one, so a caller that went through
    /// there could not tell `false` from "no such row" and would read
    /// every missing row as an answer.
    pub(super) fn told_flag(&self, known: Role, text: &str, wanted: Role) -> Option<bool> {
        (0..self.all.len())
            .filter_map(|at| self.all.entry(at))
            .map(|of| Row::Shown {
                of,
                depth: 0,
                from: 0,
            })
            .find(|row| self.field(*row, known).as_str() == text)
            .map(|row| self.field(row, wanted).flag())
    }

    /// The path of the other working copy holding `branch` — what
    /// `worktree_holding` answers, and why it answers it, is on the slot.
    ///
    /// Two fields decide it, so this cannot go through `told`: the branch
    /// a worktree row shows on its right, and the mark saying the row is
    /// the copy this window is already in — that one is where a switch is
    /// a no-op, not where it is refused.
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

    /// Where a renamed file came from, whole — what `orig_of` answers,
    /// and why it answers it, is on the slot.
    ///
    /// The first row of that path that **names a source**, which is what
    /// keeps it off `told`: that one answers with the first row of the
    /// path whatever the row holds.
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
