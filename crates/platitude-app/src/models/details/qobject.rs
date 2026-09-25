//! Everything QML sees of one commit: the fields the card binds to, the
//! feed it drains, and the questions it asks of the changed files.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (structure.md §分割).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl DetailsModel {
    qproperty!("shaHex", Member = sha_hex, Notify = changed);
    qproperty!("sha8", Member = sha8, Notify = changed);
    qproperty!("parentHex", Member = parent_hex, Notify = changed);
    qproperty!("authorName", Member = author_name, Notify = changed);
    qproperty!("authorEmail", Member = author_email, Notify = changed);
    qproperty!("authorTime", Member = author_time, Notify = changed);
    qproperty!("avatar", Member = avatar, Notify = changed);
    qproperty!("avatarUrl", Member = avatar_url, Notify = changed);
    qproperty!("coAuthors", Member = co_authors, Notify = changed);
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

    #[qsignal]
    fn changed(&mut self);

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
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.details, invoker);
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
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let Some(msg) = feed.drain().pop() else {
            return;
        };
        if Some(msg.generation()) != self.requested_generation {
            return;
        }
        let details = match msg {
            crate::hub::DetailsMsg::Loaded { details, .. } => details,
            // A choice: only the file list arrives — the request settled
            // the header.
            crate::hub::DetailsMsg::Selection { files, .. } => {
                self.requested_at = None;
                self.loading = false;
                self.selection_loaded = true;
                self.take_files(&files);
                self.changed();
                return;
            }
            crate::hub::DetailsMsg::Failed {
                oid_hex, message, ..
            } => {
                // Only the spinner comes down (the failure itself is on
                // the error surface); an answer already on screen stays.
                if oid_hex == self.requested {
                    self.loading = false;
                    self.requested_at = None;
                    Hub::with(|hub| {
                        if let Some(feeds) = hub.feeds(self.tab_id) {
                            feeds.tab.push(crate::hub::TabMsg::OpError { message });
                        }
                    });
                    self.changed();
                }
                return;
            }
        };
        let hex = details.oid.to_hex();
        if hex != self.requested {
            return; // stale response for a previous selection
        }
        // For the read / apply marks (ci/baseline/perf-windows-x64.md
        // §操作 1 点の内訳).
        let asked = self.requested_at.take();
        if let Some(t0) = &asked {
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "details request round trip"
            );
        }
        self.sha_hex = hex;
        self.sha8 = details.oid.short_hex(8);
        self.parent_hex = details.parents.first().map(Oid::to_hex).unwrap_or_default();
        self.author_name = details.author_name.clone();
        self.author_email = details.author_email.clone();
        // Same input as the graph rows (author name) → same identicon.
        self.avatar = crate::encode::avatar_code(&details.author_name);
        self.avatar_url =
            Hub::with(|hub| hub.avatar_url(&details.author_email)).unwrap_or_default();
        self.co_authors = crate::encode::mates_of(&details.co_authors);
        self.author_time = details.author_time;
        self.committer_name = details.committer_name.clone();
        self.committer_email = details.committer_email.clone();
        self.committer_avatar = crate::encode::avatar_code(&details.committer_name);
        self.committer_avatar_url =
            Hub::with(|hub| hub.avatar_url(&details.committer_email)).unwrap_or_default();
        // The address tells two people apart (デザイン規約 §アバターを与える);
        // it arrives already mailmapped.
        self.committer_differs = !details
            .committer_email
            .eq_ignore_ascii_case(&details.author_email);
        self.commit_time_differs = details.committer_time != details.author_time;
        self.committer_time = details.committer_time;
        // Subject / body split mirrors the commit-editor fields.
        let (subject, body) = details
            .message
            .split_once('\n')
            .map(|(s, b)| (s.to_string(), b.trim_start_matches('\n').to_string()))
            .unwrap_or_else(|| (details.message.clone(), String::new()));
        self.message_subject = subject;
        self.message_body = body;
        self.loading = false;
        self.take_files(&details.files);
        self.changed();
        if let Some(t0) = asked {
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "details rows applied"
            );
        }
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
    /// Nothing past either end, or for a path not shown (デザイン規約
    /// §diff のファイル一覧). Only the sign of `way` is read.
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
