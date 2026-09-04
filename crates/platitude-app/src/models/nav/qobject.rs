use super::*;

#[qobject(Base = QAbstractItemModel, ConvertToCamelCase, NoQmlElement)]
impl NavSectionModel {
    qproperty!("total", Member = total, Notify = changed);
    qproperty!("shownRows", Member = shown_total, Notify = changed);
    qproperty!("runFiles", Member = run_files, Notify = changed);
    qproperty!("headName", Member = head_name, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("headHasRemote", Member = head_has_remote, Notify = changed);
    qproperty!("headHasPr", Member = head_has_pr, Notify = changed);
    qproperty!("headRow", Member = head_row, Notify = changed);
    qproperty!("headDepth", Member = head_depth, Notify = changed);
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
    pub(super) fn changed(&mut self);

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
    pub(super) fn refs_settled(&mut self);

    /// A refs snapshot arrived *and it was a new one* — the tick-by-tick
    /// republish of the same `Arc` stays quiet. What a listener that
    /// re-derives something expensive from the refs should wait on:
    /// `refs_settled` fires on every status tick, and hanging a git
    /// spawn off it would poll the repository at the tick rate.
    #[qsignal]
    pub(super) fn refs_moved(&mut self);

    /// A stash listing arrived, whether or not it moved anything.
    ///
    /// **Separate from `refs_settled`, because the two do not land
    /// together**: a write publishes its refs before it rebuilds the
    /// graph and asks for the stashes only after (`session::write`), so a
    /// page that read the refs' arrival as the stashes' would put a
    /// dropped stash back on screen for the length of the rebuild
    /// (デザイン規約 §消す操作は先に画面から消す).
    #[qsignal]
    pub(super) fn stashes_settled(&mut self);

    /// Wires this instance to one section's data feed. `section`:
    /// `branches` / `remotes` / `worktrees` / `stashes` / `tags`. The
    /// working tree's changed files come through [`Self::attach_worktree`]
    /// instead — that section is a list per bucket run, not one list.
    #[qslot]
    fn attach_section(&mut self, tab_id: i32, section: String) {
        self.attach_section_feed(tab_id, section);
    }

    /// Wires this instance to one bucket run of the working tree's changed
    /// files — `conflicts` / `unstaged` / `staged`. Each run is a list of
    /// its own in the WIP pane, with a share of the pane and a scroll of
    /// its own, so each takes an instance and a feed of its own.
    ///
    /// Only what is **shown** is one run's: every instance holds the whole
    /// status, so a page can ask any of them about any file.
    #[qslot]
    fn attach_worktree(&mut self, tab_id: i32, run: String) {
        self.attach_worktree_feed(tab_id, run);
    }

    #[qslot]
    fn drain(&mut self) {
        self.take_feeds();
    }

    /// Shows these rows as already gone: the page hands over the names of
    /// what it has just asked git to delete, separated by U+001F, and an
    /// empty string puts them all back (デザイン規約 §消す操作は先に画面から消す).
    ///
    /// The page owns when they go back, because only the page knows which
    /// answer it is waiting for — this list holds nothing but what it was
    /// told, so a name for a row that is not here costs one comparison and
    /// changes nothing.
    #[qslot]
    fn set_hidden(&mut self, names: String) {
        let hidden: Vec<String> = if names.is_empty() {
            Vec::new()
        } else {
            names.split('\u{1f}').map(str::to_string).collect()
        };
        if self.hidden == hidden {
            return;
        }
        self.hidden = hidden;
        self.reshape();
        self.changed();
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

    /// Which row on show this ref sits on; -1 when it is on none. What a
    /// list asks before scrolling to a row nobody clicked on.
    #[qslot]
    fn row_of_name(&self, name: String) -> i32 {
        self.row_of(&name)
    }

    /// The remote branch this one speaks for (`origin/main`); empty when
    /// it speaks for none, and for every section but the branches.
    #[qslot]
    fn upstream_of(&self, name: String) -> String {
        self.told(Role::Name, &name, Role::Upstream)
    }

    /// Whether that reading stands on another commit
    /// (`session::BranchItem::upstream_drifted`). False for a branch that
    /// speaks for none, and for every section but the branches — nothing
    /// has drifted where there is no pair.
    ///
    /// **What takes the delete rows that reach the remote out of the
    /// table** (デザイン規約 §左メニューの所作 の削除の表): the drifted
    /// reading has a row of its own where it does stand, and that is
    /// where it is deleted from.
    #[qslot]
    fn upstream_drifted(&self, name: String) -> bool {
        self.told_flag(Role::Name, &name, Role::UpstreamDrifted)
            .unwrap_or(false)
    }

    /// Which sides the tag with this name stands on — `here` / `remote` /
    /// `both`, empty when this section holds no such row
    /// (`platitude_core::offers::TagSides`).
    ///
    /// **A tag has no namespace**, so one row carries both sides of a
    /// name held here and over there, and the rows that act on it are not
    /// the same rows: `tag --delete` needs a local one, `push --delete` a
    /// remote one. The row cannot say this on its own, and neither half
    /// alone answers it — `only_remote` is what the sidebar draws, and a
    /// name only a remote has is the one row of TAGS that no local ref
    /// points at.
    #[qslot]
    pub(super) fn tag_sides(&self, name: String) -> String {
        if name.is_empty() || self.section != "tags" {
            return String::new();
        }
        // No row of this name at all — a section that has not been read
        // yet, or a name this one does not carry.
        let Some(only_remote) = self.told_flag(Role::Name, &name, Role::OnlyRemote) else {
            return String::new();
        };
        let carried = self
            .told_flag(Role::Name, &name, Role::HasRemote)
            .unwrap_or(false);
        match (only_remote, carried) {
            (true, _) => "remote".to_string(),
            (false, true) => "both".to_string(),
            (false, false) => "here".to_string(),
        }
    }

    /// Where `remote` carries this tag when that is not where this
    /// repository has it; empty when they agree or that remote has never
    /// been heard to carry the name.
    ///
    /// **What decides the shape of the menu's push row**, and it has to
    /// be answerable as the menu opens: a plain push to a name the remote
    /// already has elsewhere is refused outright, so the row comes up as
    /// the leased overwrite instead and this is the commit the lease is
    /// pinned to (デザイン規約 §相手の履歴を置き換える). Asked of the tags
    /// section, the only one holding the readings.
    #[qslot]
    pub(super) fn remote_tag_drift(&self, name: String, remote: String) -> String {
        if name.is_empty() || remote.is_empty() {
            return String::new();
        }
        self.all.tag_drift(&name, &remote)
    }

    /// The other working copy holding this branch, by the path git lists
    /// it under; empty when no other one has it out.
    ///
    /// **git refuses to move onto, or delete, a branch another worktree
    /// has checked out** — `fatal: 'feat' is already used by worktree at
    /// …` and `error: cannot delete branch 'feat' used by worktree at …`
    /// (measured). It refuses that whether or not the worktree is
    /// **locked**: a lock stops `worktree remove` and `worktree move`,
    /// which is a different question, so the rows that would try ask this
    /// one and not the lock.
    ///
    /// Asked of the worktrees section, the only one holding the list —
    /// and held to it, because a file row keeps its bucket name in the
    /// same slot a worktree row keeps its branch in.
    #[qslot]
    pub(super) fn worktree_holding(&self, branch: String) -> String {
        self.worktree_with(&branch)
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

    /// The file row at `row` keyed the way the WIP pane holds its choice
    /// — `<bucket>:<path>`, empty for a folder row and past the end.
    ///
    /// This is how the choice reaches rows the view has built no delegate
    /// for ([`NavSectionModel::file_key`]).
    #[qslot]
    fn file_key_at(&self, row: i32) -> String {
        usize::try_from(row)
            .map(|at| self.file_key(at))
            .unwrap_or_default()
    }

    /// The other way round: what the row git knows by this name shows.
    /// A stash is the section where the two differ — git is told
    /// `stash@{0}` and the row shows the entry's message — and this is
    /// how a pop reads that message while the entry is still there.
    ///
    /// Asked of every entry, not of the rows on show: a filter typed into
    /// the sidebar does not stop the graph's own menu from popping one.
    #[qslot]
    fn name_of_full(&self, full: String) -> String {
        self.told(Role::Full, &full, Role::Name)
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

    /// Which row on screen this list is showing `path` in `bucket` on; -1
    /// when it is showing it on none (see [`Self::row_of_file`]). How the
    /// pane's walk tells which of its bucket lists it is standing in.
    #[qslot]
    fn row_of_file_in(&self, bucket: String, path: String) -> i32 {
        self.row_of_file(&bucket, &path)
    }

    /// The file row at the end of this list a walk going `way` comes in by
    /// (see [`Self::edge`]). What the arrows land on when they cross out of
    /// the bucket above or below.
    #[qslot]
    fn edge_file(&self, way: i32) -> String {
        self.edge(way)
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
        self.orig_path_of(&path)
    }
}
qml_register!(NavSectionModel, "NavSectionModel", singleton = false);
