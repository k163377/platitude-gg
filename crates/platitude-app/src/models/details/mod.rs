use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::pathtree::DirNode;
use super::qml_register;

#[cfg(test)]
mod details_tests;
mod qobject;
mod tree;

// The three siblings reach the imports above through `use super::*`, which
// sees what this module can see -- the block stays whole here rather than
// being dealt out three ways.

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
