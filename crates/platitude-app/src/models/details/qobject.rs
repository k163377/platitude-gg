//! Everything QML sees of one commit: the fields the card binds to and the
//! questions it asks of the changed files. What its feed hands it is
//! `drain`'s.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (structure.md §分割).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl DetailsModel {
    qproperty!("shaHex", Member = sha_hex, Notify = changed);
    qproperty!("sha8", Member = sha8, Notify = changed);
    qproperty!("parentHex", Member = parent_hex, Notify = changed);
    qproperty!("parentHexes", Member = parent_hexes, Notify = changed);
    qproperty!("authorName", Member = author_name, Notify = changed);
    qproperty!("authorEmail", Member = author_email, Notify = changed);
    qproperty!("authorTime", Member = author_time, Notify = changed);
    qproperty!("avatar", Member = avatar, Notify = changed);
    qproperty!("avatarUrl", Member = avatar_url, Notify = changed);
    // Through a getter: a value QML wrote would not reach Rust (encode::tests::wire_qml).
    qproperty!("coAuthors", Read = co_authors, Notify = changed);
    fn co_authors(&self) -> &crate::encode::Mates {
        &self.co_authors
    }
    qproperty!("committerName", Member = committer_name, Notify = changed);
    qproperty!("committerEmail", Member = committer_email, Notify = changed);
    qproperty!(
        "committerAvatar",
        Member = committer_avatar,
        Notify = changed
    );
    qproperty!(
        "committerAvatarUrl",
        Member = committer_avatar_url,
        Notify = changed
    );
    qproperty!(
        "committerDiffers",
        Member = committer_differs,
        Notify = changed
    );
    qproperty!(
        "commitTimeDiffers",
        Member = commit_time_differs,
        Notify = changed
    );
    qproperty!("committerTime", Member = committer_time, Notify = changed);
    qproperty!("messageSubject", Member = message_subject, Notify = changed);
    qproperty!("messageBody", Member = message_body, Notify = changed);
    qproperty!("selectionCount", Member = selection_count, Notify = changed);
    qproperty!("comparing", Member = comparing, Notify = changed);
    qproperty!(
        "selectionLoaded",
        Member = selection_loaded,
        Notify = changed
    );
    qproperty!("compareFrom", Member = compare_from, Notify = changed);
    qproperty!("compareTo", Member = compare_to, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);
    qproperty!("treeView", Member = tree_view, Notify = changed);
    qproperty!("fileTotal", Member = file_total, Notify = changed);
    qproperty!("floorRows", Member = floor_rows, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// Re-reads the author's and committer's assigned pictures after an
    /// assignment changed — no git read, the commit did not change.
    #[qslot]
    fn refresh_avatar(&mut self) {
        let author = Hub::with(|hub| hub.avatar_url(&self.author_email)).unwrap_or_default();
        let committer = Hub::with(|hub| hub.avatar_url(&self.committer_email)).unwrap_or_default();
        if author != self.avatar_url || committer != self.committer_avatar_url {
            self.avatar_url = author;
            self.committer_avatar_url = committer;
            self.changed();
        }
    }

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let consumer = crate::hub::Wake::drain(self, Self::drain);
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.details, consumer);
    }

    #[qslot]
    fn request(&mut self, oid_hex: String) {
        let Ok(oid) = Oid::from_hex_str(oid_hex.trim()) else {
            tracing::warn!(oid_hex, "invalid oid in details request");
            return;
        };
        self.clear_selection();
        self.requested = oid_hex;
        self.requested_at = Some(Instant::now());
        self.requested_generation = crate::hub::from_session(self.tab_id, |s| s.load_details(oid))
            .flatten()
            .map(|task| task.generation());
        self.loading = self.requested_generation.is_some();
        self.changed();
    }

    /// Requests what a choice of several commits changed. `ids` are newest
    /// first (graph order). `compare` reads them as what differs between
    /// two rather than what all of them changed (デザイン規約
    /// §複数のコミットを選ぶ). The header is settled here, so the pane
    /// turns over on the press.
    #[qslot]
    fn request_selection(&mut self, ids: Vec<String>, compare: bool) {
        let mut oids = Vec::new();
        for hex in ids.iter().filter(|h| !h.is_empty()) {
            let Ok(oid) = Oid::from_hex_str(hex.trim()) else {
                tracing::warn!(hex, "invalid oid in a selection request");
                return;
            };
            oids.push(oid);
        }
        if oids.len() < 2 {
            return;
        }
        self.clear_commit();
        // A commit's files go (they would sit under a band naming
        // several); a choice's stay until the next choice's arrive —
        // emptying between two choices collapses the list for a round
        // trip, a flash every time a commit joins or leaves.
        if self.selection_count == 0 {
            self.take_files(&[]);
        }
        self.selection_count = oids.len() as i32;
        self.comparing = compare;
        self.selection_loaded = false;
        self.compare_from.clear();
        self.compare_to.clear();
        if compare {
            // Whole ids, oldest as `from`: they address the patch behind
            // a row (`DiffModel::request_range_file`).
            self.compare_from = oids.last().map(Oid::to_hex).unwrap_or_default();
            self.compare_to = oids.first().map(Oid::to_hex).unwrap_or_default();
        }
        // Named by the newest, so a failure addressed to it finds a
        // reader (the drain's `Failed` arm).
        self.requested = oids.first().map(Oid::to_hex).unwrap_or_default();
        self.requested_at = Some(Instant::now());
        let mode = if compare {
            SelectionRead::Compare
        } else {
            SelectionRead::Union
        };
        self.requested_generation =
            crate::hub::from_session(self.tab_id, |s| s.load_selection(oids, mode))
                .flatten()
                .map(|task| task.generation());
        self.loading = self.requested_generation.is_some();
        self.changed();
    }

    #[qslot]
    fn set_tree_view(&mut self, tree: bool) {
        if self.tree_view == tree {
            return;
        }
        self.tree_view = tree;
        self.rebuild_rows();
        self.reset();
        self.changed();
    }

    /// Opens/closes one directory row in tree view (key = its path).
    #[qslot]
    fn toggle_folder(&mut self, key: String) {
        let expanded = self.folder_overrides.get(&key).copied().unwrap_or(true);
        self.folder_overrides.insert(key, !expanded);
        self.rebuild_rows();
        self.reset();
    }

    /// The changed file `way` steps from `path` among the rows shown
    /// (`encode::Landing`), over folder rows and not into closed ones.
    /// Nothing past either end, or for a path not shown
    /// (`FileRowWalk.qml`). Only the sign of `way` is read.
    ///
    /// The bucket is always empty here (a commit's files sit in no
    /// bucket); it is in the record so one walk reads both file lists.
    #[qslot]
    pub(super) fn step_file(&self, _bucket: String, path: String, way: i32) -> Landed {
        let Some(from) = self.files.iter().position(|f| !f.folder && f.path == path) else {
            return Landed::none();
        };
        let file = |at: &usize| self.files.get(*at).is_some_and(|f| !f.folder);
        let landed = if way < 0 {
            (0..from).rev().find(file)
        } else {
            (from + 1..self.files.len()).find(file)
        };
        Landed::new(
            landed
                .and_then(|at| self.files.get(at).map(|f| (at, f)))
                .map(|(at, f)| Landing {
                    row: i32::try_from(at).unwrap_or(-1),
                    bucket: String::new(),
                    path: f.path.clone(),
                }),
        )
    }

    /// The row the changed file `path` is shown at, -1 for a path not
    /// shown (a closed folder over it) — what a caller holding a path puts
    /// on screen. The bucket is empty here, as for `step_file`.
    #[qslot]
    pub(super) fn row_of_file_in(&self, _bucket: String, path: String) -> i32 {
        self.files
            .iter()
            .position(|f| !f.folder && f.path == path)
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1)
    }

    /// Where a renamed file came from, whole (a rename's diff names both
    /// sides) — for callers holding a path rather than a row.
    #[qslot]
    pub(super) fn orig_of(&self, path: String) -> String {
        self.raw_files
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.orig_path.clone())
            .unwrap_or_default()
    }

    /// Path of the changed file at `row` of the flat list (automation).
    #[qslot]
    fn file_path_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.raw_files.get(i))
            .map(|f| f.path.clone())
            .unwrap_or_default()
    }

    /// Original path of the file at `row` (empty unless renamed/copied).
    #[qslot]
    fn file_orig_path_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.raw_files.get(i))
            .map(|f| f.orig_path.clone())
            .unwrap_or_default()
    }
}
qml_register!(DetailsModel, "DetailsModel", singleton = false);
