use super::*;

#[qobject(Base = QAbstractItemModel, ConvertToCamelCase, NoQmlElement)]
impl NavSectionModel {
    qproperty!("total", Member = total, Notify = changed);
    qproperty!("headName", Member = head_name, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("headHasRemote", Member = head_has_remote, Notify = changed);
    qproperty!("headHasPr", Member = head_has_pr, Notify = changed);
    qproperty!("headRow", Member = head_row, Notify = changed);
    qproperty!("refsLoaded", Member = refs_loaded, Notify = changed);
    qproperty!("treeView", Member = tree_view, Notify = changed);
    qproperty!(
        "pointedEolPath",
        Member = pointed_eol_path,
        Notify = changed
    );
    qproperty!(
        "pointedEolKind",
        Member = pointed_eol_kind,
        Notify = changed
    );
    qproperty!(
        "pointedEolFrom",
        Member = pointed_eol_from,
        Notify = changed
    );
    qproperty!("pointedEolTo", Member = pointed_eol_to, Notify = changed);
    qproperty!(
        "pointedEolLines",
        Member = pointed_eol_lines,
        Notify = changed
    );
    qproperty!(
        "pointedEolScope",
        Member = pointed_eol_scope,
        Notify = changed
    );
    qproperty!("pointedEolExt", Member = pointed_eol_ext, Notify = changed);

    /// Names the row the pointer is on, so its line-ending sentence can be
    /// built. An empty path clears it. Hover cannot be injected headless,
    /// so this is also what the automation writes — the same one property
    /// a real pointer moves.
    #[qslot]
    fn point_eol(&mut self, path: String) {
        let notice = self
            .eol_marks
            .iter()
            .find(|m| m.path == path)
            .map(|m| &m.notice);
        let words = crate::encode::ending_words(notice);
        if path == self.pointed_eol_path
            && words.kind == self.pointed_eol_kind
            && words.from == self.pointed_eol_from
            && words.to == self.pointed_eol_to
            && words.lines == self.pointed_eol_lines
            && words.scope == self.pointed_eol_scope
            && words.ext == self.pointed_eol_ext
        {
            return;
        }
        self.pointed_eol_path = path;
        self.pointed_eol_kind = words.kind;
        self.pointed_eol_from = words.from;
        self.pointed_eol_to = words.to;
        self.pointed_eol_lines = words.lines;
        self.pointed_eol_scope = words.scope;
        self.pointed_eol_ext = words.ext;
        self.changed();
    }

    #[qsignal]
    fn changed(&mut self);

    // A refs snapshot arrived, whether or not it moved anything.
    //
    // `changed` cannot answer this: it stays deliberately quiet when the
    // rows come out identical, because rebuilding a section holding tens
    // of thousands of tags for the same picture is work the sidebar can
    // see. A page holding a landing owed by a write needs to hear it all
    // the same — a cherry-pick of a commit the branch already has
    // records nothing and leaves the refs exactly as they were, and its
    // landing would otherwise stay armed until something unrelated moved
    // them (`RepoPage.tryPendingHeadSelect`).
    #[qsignal]
    fn refs_settled(&mut self);

    /// Wires this instance to one section's data feed. `section`:
    /// `branches` / `remotes` / `worktree` / `worktrees` / `stashes` /
    /// `tags`.
    #[qslot]
    fn attach_section(&mut self, tab_id: i32, section: String) {
        self.tab_id = tab_id;
        self.section = section;
        self.tree_view = true;
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let invoker = self.get_qml_method_invoker();
        match self.section.as_str() {
            "branches" => self.refs_feed = Some(attached(&feeds.refs_branches, invoker)),
            "remotes" => self.refs_feed = Some(attached(&feeds.refs_remotes, invoker)),
            "tags" => self.refs_feed = Some(attached(&feeds.refs_tags, invoker)),
            "worktree" => self.status_feed = Some(attached(&feeds.status_nav, invoker)),
            "stashes" => self.stash_feed = Some(attached(&feeds.stash, invoker)),
            "worktrees" => self.worktrees_feed = Some(attached(&feeds.worktrees, invoker)),
            other => tracing::warn!(section = other, "unknown sidebar section"),
        }
    }

    #[qslot]
    fn drain(&mut self) {
        // Every push queues its own `drain`, so a second call can find the
        // queue already emptied by the first. Nothing arrived means
        // nothing to rebuild.
        let mut arrived = false;
        // Whether refs were published at all, which is a different
        // question from whether they moved (see `refs_settled`).
        let mut settled = false;
        if let Some(feed) = self.refs_feed.clone()
            && let Some(snapshot) = feed.drain().pop()
        {
            settled = true;
            // The first snapshot is news whatever it holds: the default
            // selection is waiting on `refsLoaded`, and a section that is
            // legitimately empty would otherwise never say so.
            arrived |= !self.refs_loaded;
            self.refs_loaded = true;
            let fresh = !self
                .last_refs
                .as_ref()
                .is_some_and(|last| Arc::ptr_eq(last, &snapshot));
            if fresh {
                self.last_refs = Some(Arc::clone(&snapshot));
                arrived |= match self.section.as_str() {
                    "branches" => {
                        let head = snapshot.locals.iter().find(|b| b.is_head);
                        self.head_name = head.map(|b| b.short.to_string()).unwrap_or_default();
                        self.head_oid = head.map(|b| b.oid.to_hex()).unwrap_or_default();
                        self.head_has_remote = head.is_some_and(|b| b.has_remote);
                        self.head_has_pr = head.is_some_and(|b| {
                            crate::encode::fake_pr_set().contains(b.short.as_str())
                        });
                        self.take(Source::Locals(snapshot))
                    }
                    "remotes" => self.take(Source::Remotes(snapshot)),
                    _ => self.take(Source::Tags(snapshot)),
                };
            }
        }
        if let Some(feed) = self.status_feed.clone()
            && let Some(StatusMsg {
                status, eol_marks, ..
            }) = feed.drain().pop()
        {
            // The marks are the other half of what a file row shows, and
            // they can move on their own — a line-ending answer arrives
            // after the status it is about.
            let marked = self.eol_marks != eol_marks;
            self.eol_marks = eol_marks;
            arrived |= self.take(Source::files(status)) || marked;
        }
        if let Some(feed) = self.worktrees_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            let current = crate::hub::from_session(self.tab_id, |s| s.workdir())
                .flatten()
                .map(|p| p.to_string_lossy().replace('\\', "/").to_lowercase())
                .unwrap_or_default();
            // A bare entry has no working copy to show.
            let list = list.into_iter().filter(|w| !w.bare).collect();
            arrived |= self.take(Source::Worktrees { list, current });
        }
        if let Some(feed) = self.stash_feed.clone()
            && let Some(stashes) = feed.drain().pop()
        {
            arrived |= self.take(Source::Stashes(stashes));
        }
        if arrived {
            self.total = self.all.len() as i32;
            self.reshape();
            self.changed();
        }
        if settled {
            self.refs_settled();
        }
        if crate::memprobe::enabled() {
            self.note_footprint();
        }
    }

    #[qslot]
    fn set_filter(&mut self, filter: String) {
        if self.filter != filter {
            self.filter = filter;
            self.reshape();
            self.changed();
        }
    }

    /// Opens/closes one folder row (key = its path, e.g. `origin/feature`).
    #[qslot]
    fn toggle_folder(&mut self, key: String) {
        let depth = key.matches('/').count() as i32;
        let current = self.folder_expanded(&key, depth);
        self.folder_overrides.insert(key, !current);
        self.reshape();
        // Folding moves rows around (and can swallow the current one).
        self.changed();
    }

    /// Switches the worktree list between tree and flat-path display.
    #[qslot]
    fn set_tree_view(&mut self, tree: bool) {
        if self.tree_view == tree {
            return;
        }
        self.tree_view = tree;
        self.reshape();
        self.changed();
    }

    /// Rows on screen (`total` counts the source's rows; this counts what
    /// filtering and folding leave shown).
    #[qslot]
    fn shown(&self) -> i32 {
        self.shown_rows() as i32
    }

    /// Commit id of the ref with this name; empty when there is none.
    #[qslot]
    pub(super) fn oid_of_name(&self, name: String) -> String {
        self.told(Role::Name, &name, Role::OidHex)
    }

    /// The remote branch this one speaks for (`origin/main`); empty when
    /// it speaks for none, and for every section but the branches.
    #[qslot]
    fn upstream_of(&self, name: String) -> String {
        self.told(Role::Name, &name, Role::Upstream)
    }

    /// What one row shows, and what git knows it by (empty out of range).
    ///
    /// Roles are only visible to a delegate, so this is how automation
    /// reaches a row it has to act on.
    #[qslot]
    pub(super) fn name_at(&self, row: i32) -> String {
        self.shows(row, Role::Name)
    }

    #[qslot]
    fn full_at(&self, row: i32) -> String {
        self.shows(row, Role::Full)
    }

    /// Local branch name a remote-tracking ref would take. The remotes
    /// snapshot owns both the visible row and its configured prefixes.
    #[qslot]
    fn local_name_for(&self, remote_ref: String) -> String {
        self.local_name_of(&remote_ref)
    }

    /// The two stage letters git reports for a working-tree path (`UU`,
    /// `DU`, …); empty for any path that is not in this section.
    ///
    /// The diff pane asks for it by path because that is all it holds: it
    /// is handed a file, not the row the file came from, and a conflict
    /// git prints no patch for is a pane with nothing to say unless it can
    /// name what the two sides did.
    #[qslot]
    pub(super) fn change_of(&self, path: String) -> String {
        self.told(Role::Full, &path, Role::Change)
    }

    /// Which bucket holds a path now. Asked after a write has moved it,
    /// when it sits in exactly one of them.
    #[qslot]
    fn bucket_of(&self, path: String) -> String {
        self.told(Role::Full, &path, Role::Bucket)
    }

    /// Whether `bucket` still holds `path` (see [`Self::holds`]).
    #[qslot]
    fn holds_path(&self, bucket: String, path: String) -> bool {
        self.holds(&bucket, &path)
    }

    /// The row beside `path` under the same heading, as `<bucket>:<path>`
    /// (see [`Self::beside`]). What the diff pane reads next when the file
    /// it is on leaves the side being read.
    #[qslot]
    fn beside_path(&self, bucket: String, path: String) -> String {
        self.beside(&bucket, &path)
    }

    /// The file row the arrows land on, `way` steps from `path` in `bucket`
    /// (see [`Self::step`]). What the file list's own arrows walk.
    #[qslot]
    fn step_file(&self, bucket: String, path: String, way: i32) -> String {
        self.step(&bucket, &path, way)
    }

    /// Where a renamed file came from, by path — whole, the way a diff
    /// wants it (a rename's diff is read by naming both of its sides).
    ///
    /// A headless run has only the path, and without this it opens the
    /// destination alone — which git reads as a file appearing out of
    /// nowhere.
    ///
    /// The first row of that path that names a source, not the first row
    /// of that path: a file renamed and then edited again has a row on
    /// each side, and only the staged one knows where it came from.
    #[qslot]
    fn orig_of(&self, path: String) -> String {
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
}
qml_register!(NavSectionModel, "NavSectionModel", singleton = false);
