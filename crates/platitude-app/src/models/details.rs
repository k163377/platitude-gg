use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::pathtree::DirNode;
use super::qml_register;

// ---------------------------------------------------------------------------
// DetailsModel: commit metadata + changed files
// ---------------------------------------------------------------------------

// Every field is `pub(super)` because the rows are also what
// `details_tests` builds a commit out of: the tests sit beside the model
// rather than in it (.claude/rules/structure.md), and `models` is as far as
// the widening reaches.
#[derive(QModelItem, Default, Clone)]
pub struct FileItem {
    pub(super) change: String,
    /// Full path (diff request + tooltip); folder rows carry their
    /// directory path here, which doubles as the fold toggle key.
    pub(super) path: String,
    pub(super) orig_path: String,
    /// Display text: the last segment in tree view, the full path in
    /// path view.
    pub(super) name: String,
    /// The same for a rename's source, cut back exactly as far as `name`
    /// is (`encode::rename_source`). `orig_path` stays whole beside it —
    /// that one addresses a diff, this one is only read.
    pub(super) orig_name: String,
    pub(super) depth: i32,
    pub(super) folder: bool,
    pub(super) collapsed: bool,
}

impl platitude_core::mem::Footprint for FileItem {
    fn heap_bytes(&self) -> usize {
        self.change.heap_bytes()
            + self.path.heap_bytes()
            + self.orig_path.heap_bytes()
            + self.name.heap_bytes()
            + self.orig_name.heap_bytes()
    }
}

/// Turns flat changed-file entries into an indented tree: directories
/// first (alphabetical), single-child directory chains compacted into one
/// row (`a/b/c`), leaves labeled by their last segment.
fn build_file_tree(raw: &[FileItem], overrides: &HashMap<String, bool>) -> Vec<FileItem> {
    let mut root = DirNode::default();
    for entry in raw {
        root.insert(&entry.path, |cut| {
            let mut leaf = entry.clone();
            leaf.name = entry.path[cut..].to_string();
            // The folders above the row spell this much of its path; a
            // rename's source gives up the same prefix when it had one.
            leaf.orig_name =
                crate::encode::rename_source(&entry.orig_path, &entry.path, cut).to_string();
            leaf
        });
    }
    fn emit(
        node: &DirNode<FileItem>,
        prefix: &str,
        depth: i32,
        overrides: &HashMap<String, bool>,
        out: &mut Vec<FileItem>,
    ) {
        for (label, target) in node.folders() {
            let key = format!("{prefix}{label}");
            let expanded = overrides.get(&key).copied().unwrap_or(true);
            out.push(FileItem {
                name: label,
                path: key.clone(),
                depth,
                folder: true,
                collapsed: !expanded,
                ..Default::default()
            });
            if expanded {
                emit(target, &format!("{key}/"), depth + 1, overrides, out);
            }
        }
        for f in node.files() {
            let mut item = f.clone();
            item.depth = depth;
            out.push(item);
        }
    }
    let mut out = Vec::new();
    emit(&root, "", 0, overrides, &mut out);
    out
}

pub struct DetailsModel {
    pub(super) files: Vec<FileItem>,
    /// Flat entries in git output order; display rows derive from these.
    pub(super) raw_files: Vec<FileItem>,
    file_total: i32,
    pub(super) tree_view: bool,
    /// Explicit folder open/close choices (key = directory path); cleared
    /// per commit, anything absent defaults to open.
    folder_overrides: HashMap<String, bool>,
    sha_hex: String,
    sha8: String,
    parent_hex: String,
    author_name: String,
    author_email: String,
    author_time: i64,
    avatar: i32,
    /// `file:` URL of this author's assigned picture, empty when they have
    /// none. Refreshed on every details read and whenever an assignment
    /// changes, so the card follows the settings list without a reload.
    avatar_url: String,
    /// Packed `Co-authored-by` trailers (see `encode::encode_co_authors`).
    co_authors: String,
    committer_name: String,
    committer_email: String,
    committer_avatar: i32,
    committer_avatar_url: String,
    /// Whether the commit was put here by somebody other than its author,
    /// and whether that happened at another moment than it was written.
    /// Both are answered here rather than in QML: the address is what
    /// tells two people apart (デザイン規約 §アバターを与える), and the
    /// card that reads these must not carry a second copy of that rule.
    committer_differs: bool,
    commit_time_differs: bool,
    committer_time: i64,
    message_subject: String,
    message_body: String,
    loading: bool,
    requested: String,
    requested_at: Option<Instant>,
    feed: Option<Arc<Feed<platitude_core::details::CommitDetails>>>,
    tab_id: i32,
}

impl Default for DetailsModel {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            raw_files: Vec::new(),
            file_total: 0,
            // Tree view is the default look of the CHANGES list.
            tree_view: true,
            folder_overrides: HashMap::new(),
            sha_hex: String::new(),
            sha8: String::new(),
            parent_hex: String::new(),
            author_name: String::new(),
            author_email: String::new(),
            author_time: 0,
            avatar: 0,
            avatar_url: String::new(),
            co_authors: String::new(),
            committer_name: String::new(),
            committer_email: String::new(),
            committer_avatar: 0,
            committer_avatar_url: String::new(),
            committer_differs: false,
            commit_time_differs: false,
            committer_time: 0,
            message_subject: String::new(),
            message_body: String::new(),
            loading: false,
            requested: String::new(),
            requested_at: None,
            feed: None,
            tab_id: 0,
        }
    }
}

impl DetailsModel {
    /// Rebuilds display rows from the raw entries for the current view.
    pub(super) fn rebuild_rows(&mut self) {
        self.files = if self.tree_view {
            build_file_tree(&self.raw_files, &self.folder_overrides)
        } else {
            self.raw_files.clone()
        };
    }
}

impl QListModel for DetailsModel {
    type Item = FileItem;

    fn len(&self) -> usize {
        self.files.len()
    }
    fn get(&self, index: usize) -> Option<&FileItem> {
        self.files.get(index)
    }
    fn reset_unnotified(&mut self) {}
}

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
        self.loading = true;
        self.changed();
        crate::hub::with_session(self.tab_id, |s| s.load_details(oid));
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let Some(details) = feed.drain().pop() else {
            return;
        };
        let hex = details.oid.to_hex();
        if hex != self.requested {
            return; // stale response for a previous selection
        }
        if let Some(t0) = self.requested_at.take() {
            // The 100ms interaction budget is measured here (click → data).
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
