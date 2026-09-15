//! Everything QML sees of one commit: the fields the card binds to, the
//! feed it drains, and the questions it asks of the changed files.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

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

    /// Re-reads this author's assigned picture. Called when an assignment
    /// changes: the commit on screen did not, so there is nothing to ask
    /// git for.
    #[qslot]
    fn refresh_avatar(&mut self) {
        let author = Hub::with(|hub| hub.avatar_url(&self.author_email)).unwrap_or_default();
        // The committer wears a face of their own in the author card, and
        // it comes from the same store, so an assignment reaches both.
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

    /// Requests details of `oid_hex` (graph row selection).
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

    /// Requests what a choice of several commits changed. `packed` is
    /// their ids **newest first**, joined by `\u{1f}` — the order the
    /// graph stands in.
    ///
    /// `compare` picks which question is being asked: two commits are
    /// read as what differs between them, three or more as what all of
    /// them changed (デザイン規約 §複数のコミットを選ぶ). The header is
    /// settled here rather than when the files land, so the pane turns
    /// over with the press instead of a round trip later.
    #[qslot]
    fn request_selection(&mut self, packed: String, compare: bool) {
        let mut oids = Vec::new();
        for hex in packed.split('\u{1f}').filter(|h| !h.is_empty()) {
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
        // **The last commit's files go with it** — but one choice's do
        // not go when another choice replaces it. Left standing, a
        // commit's files would sit under a band naming several; emptied
        // between two choices, the list under this one collapses and the
        // pane below it takes the whole place for the length of a round
        // trip, which is a flash every time a commit joins or leaves
        // (the diff pane keeps its rows across a re-read for the same
        // reason — `DiffModel::begin_request`).
        if self.selection_count == 0 {
            self.take_files(&[]);
        }
        self.selection_count = oids.len() as i32;
        self.comparing = compare;
        self.selection_loaded = false;
        self.compare_from.clear();
        self.compare_to.clear();
        if compare {
            // Oldest first — the side a comparison is measured from.
            // Whole ids: what reads these is the patch behind a row of
            // the list (`DiffModel::request_range_file`), not a caption.
            self.compare_from = oids.last().map(Oid::to_hex).unwrap_or_default();
            self.compare_to = oids.first().map(Oid::to_hex).unwrap_or_default();
        }
        // Named by the newest of the choice, so a failure addressed to
        // it still finds a reader (the drain's `Failed` arm).
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
            // What a choice of several commits changed. Only the file
            // list arrives — the header was settled by the request, the
            // commits themselves being named by rows already on screen.
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
        // Held for the two marks rather than taken at the first of them:
        // one says when the answer arrived and the other when it is in the
        // model, and between them is what this call costs.
        let asked = self.requested_at.take();
        if let Some(t0) = &asked {
            // Data arrival only; PagePerfDriver separately observes a frame.
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
        self.co_authors = crate::encode::encode_co_authors(&details.co_authors);
        self.author_time = details.author_time;
        self.committer_name = details.committer_name.clone();
        self.committer_email = details.committer_email.clone();
        self.committer_avatar = crate::encode::avatar_code(&details.committer_name);
        self.committer_avatar_url =
            Hub::with(|hub| hub.avatar_url(&details.committer_email)).unwrap_or_default();
        // Who wrote it and who put it here are the same person on an
        // ordinary commit; a patch applied by somebody else, a web merge
        // or a rebase is what makes them two. The address decides, the
        // way it decides everywhere a person is identified here — the
        // spellings are already mailmapped by the time they arrive.
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
            // The rows are in the model and the signals are out; what is
            // left before the frame is the view and the painting.
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "details rows applied"
            );
        }
    }

    /// Switches the CHANGES list between tree and flat-path display.
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

    /// The changed file `way` steps from `path` among the rows this list
    /// shows, as `<row>\u{1e}<bucket>\u{1e}<path>`. Empty where the walk has
    /// nowhere left to go — which is how the arrows stop at the ends rather
    /// than wrapping — and empty where the path is not shown at all
    /// (デザイン規約 §diff のファイル一覧).
    ///
    /// Only the sign of `way` is read: one press is one file.
    ///
    /// Folder rows are stepped over, since a folder has no diff to move to,
    /// and the rows walked are the ones on screen — a folder the reader
    /// closed is one the walk does not enter.
    ///
    /// The bucket field is always empty here (a commit's changed files sit
    /// in no bucket); it is in the record so that one walk reads both file
    /// lists. The path comes last because it is the only field git lets hold
    /// the separator.
    #[qslot]
    pub(super) fn step_file(&self, _bucket: String, path: String, way: i32) -> String {
        let Some(from) = self.files.iter().position(|f| !f.folder && f.path == path) else {
            return String::new();
        };
        let file = |at: &usize| self.files.get(*at).is_some_and(|f| !f.folder);
        let landed = if way < 0 {
            (0..from).rev().find(file)
        } else {
            (from + 1..self.files.len()).find(file)
        };
        landed
            .and_then(|at| self.files.get(at).map(|f| (at, f)))
            .map(|(at, f)| {
                format!(
                    "{at}{sep}{sep}{path}",
                    sep = crate::encode::FIELD_SEP,
                    path = f.path
                )
            })
            .unwrap_or_default()
    }

    /// Where a renamed file came from, by path — whole, the way a diff wants
    /// it (a rename's diff is read by naming both of its sides). The row's
    /// own `orig_path` is the same answer; this is for the callers holding a
    /// path and not a row.
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
