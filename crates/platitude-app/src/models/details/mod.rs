use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use platitude_core::session::SelectionRead;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{Landed, Landing};
use crate::hub::{DetailsMsg, Feed, Hub};

use super::pathtree::DirNode;
use super::qml_register;

#[cfg(test)]
mod details_tests;
mod qobject;
mod tree;

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
    /// The rename's source, cut back as far as `name` is
    /// (`encode::rename_source`) — display only; a diff is addressed by
    /// the whole `orig_path`.
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
    /// `file:` URL of this author's assigned picture, empty when none.
    /// Refreshed on every read and on every assignment (`refresh_avatar`).
    avatar_url: String,
    /// The `Co-authored-by` trailers (`encode::mates_of`).
    co_authors: crate::encode::Mates,
    committer_name: String,
    committer_email: String,
    committer_avatar: i32,
    committer_avatar_url: String,
    /// Whether somebody other than the author committed it, and at another
    /// moment than it was written (decided in `drain`).
    committer_differs: bool,
    commit_time_differs: bool,
    committer_time: i64,
    message_subject: String,
    message_body: String,
    /// How many commits the choice holds, and whether the file list is
    /// the two-commit comparison rather than what the set changed
    /// (デザイン規約 §複数のコミットを選ぶ). 0 / false while one commit is read.
    selection_count: i32,
    comparing: bool,
    /// Whether the files below a choice are its own answer. Read it before
    /// the list: a failed read leaves it empty and not loading — the same
    /// shape as a choice that changed nothing (app-ui.md §UI 自動化).
    selection_loaded: bool,
    /// The two ends of a comparison, oldest first — what the header
    /// names. Empty unless `comparing`.
    compare_from: String,
    compare_to: String,
    loading: bool,
    requested: String,
    requested_generation: Option<u64>,
    requested_at: Option<Instant>,
    feed: Option<Arc<Feed<DetailsMsg>>>,
    tab_id: i32,
}

impl Default for DetailsModel {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            raw_files: Vec::new(),
            file_total: 0,
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
            co_authors: crate::encode::Mates::default(),
            committer_name: String::new(),
            committer_email: String::new(),
            committer_avatar: 0,
            committer_avatar_url: String::new(),
            committer_differs: false,
            commit_time_differs: false,
            committer_time: 0,
            message_subject: String::new(),
            message_body: String::new(),
            selection_count: 0,
            comparing: false,
            selection_loaded: false,
            compare_from: String::new(),
            compare_to: String::new(),
            loading: false,
            requested: String::new(),
            requested_generation: None,
            requested_at: None,
            feed: None,
            tab_id: 0,
        }
    }
}

impl DetailsModel {
    /// Lays a file list into the CHANGES rows, a commit's or a choice's
    /// alike.
    pub(super) fn take_files(&mut self, files: &[platitude_core::parse::name_status::FileChange]) {
        self.raw_files = files
            .iter()
            .map(|f| FileItem {
                change: f.status.to_string(),
                path: f.path.clone(),
                orig_path: f.orig_path.clone().unwrap_or_default(),
                name: f.path.clone(),
                // Whole in the flat view; the tree cuts both names
                // together (`build_file_tree`).
                orig_name: f.orig_path.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect();
        self.file_total = self.raw_files.len() as i32;
        self.folder_overrides.clear();
        self.rebuild_rows();
        self.reset();
        if crate::harness::memprobe::enabled() {
            crate::harness::memprobe::note("details-files", self.tab_id, &self.raw_files);
        }
    }

    /// Empties everything that describes one commit — a choice of several
    /// has no author, message or hash, so the card goes.
    pub(super) fn clear_commit(&mut self) {
        self.sha_hex.clear();
        self.sha8.clear();
        self.parent_hex.clear();
        self.author_name.clear();
        self.author_email.clear();
        self.author_time = 0;
        self.avatar = 0;
        self.avatar_url.clear();
        self.co_authors = crate::encode::Mates::default();
        self.committer_name.clear();
        self.committer_email.clear();
        self.committer_avatar = 0;
        self.committer_avatar_url.clear();
        self.committer_differs = false;
        self.commit_time_differs = false;
        self.committer_time = 0;
        self.message_subject.clear();
        self.message_body.clear();
    }

    /// Puts the pane back on one commit, whatever choice it was holding.
    pub(super) fn clear_selection(&mut self) {
        self.selection_count = 0;
        self.comparing = false;
        self.selection_loaded = false;
        self.compare_from.clear();
        self.compare_to.clear();
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
