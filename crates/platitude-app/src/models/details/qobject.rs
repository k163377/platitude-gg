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
        self.requested = oid_hex;
        self.requested_at = Some(Instant::now());
        self.requested_generation = crate::hub::from_session(self.tab_id, |s| s.load_details(oid))
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
        if let Some(t0) = self.requested_at.take() {
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
        self.raw_files = details
            .files
            .iter()
            .map(|f| FileItem {
                change: f.status.to_string(),
                path: f.path.clone(),
                orig_path: f.orig_path.clone().unwrap_or_default(),
                name: f.path.clone(),
                // The flat view spells every row whole, both names with
                // it; the tree cuts them together (`build_file_tree`).
                orig_name: f.orig_path.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect();
        self.file_total = self.raw_files.len() as i32;
        self.folder_overrides.clear();
        self.rebuild_rows();
        self.reset();
        if crate::memprobe::enabled() {
            crate::memprobe::note("details-files", self.tab_id, &self.raw_files);
        }
        self.changed();
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
