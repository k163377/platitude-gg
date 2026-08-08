use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

// ---------------------------------------------------------------------------
// DetailsModel: commit metadata + changed files
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct FileItem {
    change: String,
    /// Full path (diff request + tooltip); folder rows carry their
    /// directory path here, which doubles as the fold toggle key.
    path: String,
    orig_path: String,
    /// Display text: the last segment in tree view, the full path in
    /// path view.
    name: String,
    depth: i32,
    folder: bool,
    collapsed: bool,
}

/// Turns flat changed-file entries into an indented tree: directories
/// first (alphabetical), single-child directory chains compacted into one
/// row (`a/b/c`), leaves labeled by their last segment.
fn build_file_tree(raw: &[FileItem], overrides: &HashMap<String, bool>) -> Vec<FileItem> {
    #[derive(Default)]
    struct DirNode {
        dirs: std::collections::BTreeMap<String, DirNode>,
        files: Vec<FileItem>,
    }
    let mut root = DirNode::default();
    for entry in raw {
        let mut node = &mut root;
        let mut rest = entry.path.as_str();
        while let Some((dir, tail)) = rest.split_once('/') {
            node = node.dirs.entry(dir.to_string()).or_default();
            rest = tail;
        }
        let mut leaf = entry.clone();
        leaf.name = rest.to_string();
        node.files.push(leaf);
    }
    fn emit(
        node: &DirNode,
        prefix: &str,
        depth: i32,
        overrides: &HashMap<String, bool>,
        out: &mut Vec<FileItem>,
    ) {
        for (dir_name, child) in &node.dirs {
            let mut label = dir_name.clone();
            let mut target = child;
            while target.files.is_empty() && target.dirs.len() == 1 {
                let Some((next_name, next)) = target.dirs.iter().next() else {
                    break;
                };
                label.push('/');
                label.push_str(next_name);
                target = next;
            }
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
        for f in &node.files {
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
    files: Vec<FileItem>,
    /// Flat entries in git output order; display rows derive from these.
    raw_files: Vec<FileItem>,
    file_total: i32,
    tree_view: bool,
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
    committer: String,
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
            committer: String::new(),
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
    fn rebuild_rows(&mut self) {
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
    qproperty!("committer", Member = committer, Notify = changed);
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
        let url = Hub::with(|hub| hub.avatar_url(&self.author_email)).unwrap_or_default();
        if url != self.avatar_url {
            self.avatar_url = url;
            self.changed();
        }
    }

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.details);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
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
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.load_details(oid);
        }
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
        self.committer = format!("{} <{}>", details.committer_name, details.committer_email);
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
                ..Default::default()
            })
            .collect();
        self.file_total = self.raw_files.len() as i32;
        self.folder_overrides.clear();
        self.rebuild_rows();
        self.reset();
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
