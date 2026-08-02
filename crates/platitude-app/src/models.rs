//! QML-facing objects. This layer only maps platitude-core results onto Qt
//! models/properties (thin bridge, swappable per 実装計画 §2.1).
//!
//! Conventions:
//! - every object is registered under the `platitude` QML module via the
//!   `qml_register!` macro (NoQmlElement + manual QmlRegister)
//! - per-tab objects wire themselves with `attach(tabId)` and consume their
//!   feed in `drain` (the only slot ever called through an invoker)
//! - `#[derive(QModelItem)]` requires `HashMap` in scope (macro hygiene).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::details::DiffTarget;
use platitude_core::session::LogRow;
use platitude_core::{Oid, version};
use qtbridge::qtbridge_type_lib::QModelIndex;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{DiffRow, diff_key, encode_geometry, encode_labels, flatten_patches};
use crate::hub::{Feed, GraphMsg, Hub, StatusMsg, TabMsg};
use crate::urlpath::file_url_to_path;

/// Registers a type under the `platitude` QML module.
macro_rules! qml_register {
    ($ty:ty, $name:literal, singleton = $singleton:literal) => {
        impl qtbridge::QmlRegister for $ty {
            const URI: &str = "platitude";
            const ELEMENT_NAME: &str = $name;
            const MAJOR_VERSION: u8 = 1;
            const MINOR_VERSION: u8 = 0;
            const IS_SINGLETON: bool = $singleton;
        }
    };
}

/// Batch append with one begin/endInsertRows pair (qtbridge has no batch
/// insert; this mirrors QListModelBase::push via the public proxy API —
/// see CLAUDE.md "Qt Bridges の要点").
macro_rules! impl_extend_notified {
    ($ty:ty, $field:ident, $item:ty) => {
        impl $ty {
            #[expect(unsafe_code)]
            fn extend_notified(&mut self, batch: Vec<$item>) {
                if batch.is_empty() {
                    return;
                }
                let Some(proxy) = self.try_get_rust_proxy_ptr() else {
                    self.$field.extend(batch);
                    return;
                };
                let first = self.$field.len() as i32;
                let last = first + batch.len() as i32 - 1;
                // SAFETY: same pattern as QListModelBase::push — the proxy
                // pointer stays valid while the QObject side is attached,
                // and we are on the Qt main thread inside a slot.
                unsafe { &mut *proxy }.base_begin_insert_rows(
                    &mut *self,
                    &QModelIndex::default(),
                    first,
                    last,
                );
                self.$field.extend(batch);
                // SAFETY: see above.
                unsafe { &mut *proxy }.base_end_insert_rows(&mut *self);
            }
        }
    };
}

// ---------------------------------------------------------------------------
// AppBackend (singleton): git version gate + automation hooks
// ---------------------------------------------------------------------------

enum GitCheckMsg {
    Ok { version: String },
    Missing { message: String },
    Unsupported { message: String },
    Error { message: String },
}

pub struct AppBackend {
    git_state: String,
    git_version: String,
    git_error: String,
    auto_open: String,
    shot_dir: String,
    auto_quit_ms: i32,
    auto_select: bool,
    auto_scroll: bool,
    check_feed: Arc<Feed<GitCheckMsg>>,
}

impl Default for AppBackend {
    fn default() -> Self {
        Self {
            git_state: "checking".into(),
            git_version: String::new(),
            git_error: String::new(),
            auto_open: std::env::var("PG_AUTO_OPEN").unwrap_or_default(),
            shot_dir: std::env::var("PG_SHOT_DIR")
                .unwrap_or_default()
                .replace('\\', "/"),
            auto_quit_ms: std::env::var("PG_AUTO_QUIT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            auto_select: std::env::var("PG_AUTO_SELECT").as_deref() == Ok("1"),
            auto_scroll: std::env::var("PG_AUTO_SCROLL").as_deref() == Ok("1"),
            check_feed: Arc::new(Feed::default()),
        }
    }
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl AppBackend {
    qproperty!("gitState", Member = git_state, Notify = git_state_changed);
    qproperty!(
        "gitVersion",
        Member = git_version,
        Notify = git_state_changed
    );
    qproperty!("gitError", Member = git_error, Notify = git_state_changed);
    qproperty!("autoOpen", Member = auto_open, Constant);
    qproperty!("shotDir", Member = shot_dir, Constant);
    qproperty!("autoQuitMs", Member = auto_quit_ms, Constant);
    qproperty!("autoSelect", Member = auto_select, Constant);
    qproperty!("autoScroll", Member = auto_scroll, Constant);

    #[qsignal]
    fn git_state_changed(&mut self);

    /// Benchmark/automation reporting channel (QML → tracing).
    #[qslot]
    fn report(&self, message: String) {
        tracing::info!(target: "bench", "{message}");
    }

    /// Starts the git version check (call once from QML on startup).
    #[qslot]
    fn initialize(&mut self) {
        self.check_feed.attach(self.get_qml_method_invoker());
        let feed = Arc::clone(&self.check_feed);
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let msg = match version::ensure_supported(&executor, &cancel).await {
                    Ok(v) => GitCheckMsg::Ok { version: v.raw },
                    Err(e @ platitude_core::GitError::GitNotFound { .. }) => GitCheckMsg::Missing {
                        message: e.to_string(),
                    },
                    Err(e @ platitude_core::GitError::UnsupportedVersion { .. }) => {
                        GitCheckMsg::Unsupported {
                            message: e.to_string(),
                        }
                    }
                    Err(e) => GitCheckMsg::Error {
                        message: e.to_string(),
                    },
                };
                feed.push(msg);
            });
            true
        })
        .unwrap_or(false);
        if !spawned {
            self.git_state = "error".into();
            self.git_error = "internal: runtime unavailable".into();
            self.git_state_changed();
        }
    }

    #[qslot]
    fn drain(&mut self) {
        for msg in self.check_feed.drain() {
            match msg {
                GitCheckMsg::Ok { version } => {
                    self.git_state = "ok".into();
                    self.git_version = version;
                }
                GitCheckMsg::Missing { message } => {
                    self.git_state = "missing".into();
                    self.git_error = message;
                }
                GitCheckMsg::Unsupported { message } => {
                    self.git_state = "unsupported".into();
                    self.git_error = message;
                }
                GitCheckMsg::Error { message } => {
                    self.git_state = "error".into();
                    self.git_error = message;
                }
            }
        }
        self.git_state_changed();
    }
}
qml_register!(AppBackend, "AppBackend", singleton = true);

// ---------------------------------------------------------------------------
// TabsModel: open repositories (the tab strip)
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct TabItem {
    tab_id: i32,
    title: String,
    repo_path: String,
}

#[derive(Default)]
pub struct TabsModel {
    items: Vec<TabItem>,
    current_index: i32,
}

impl QListModel for TabsModel {
    type Item = TabItem;

    fn len(&self) -> usize {
        self.items.len()
    }
    fn get(&self, index: usize) -> Option<&TabItem> {
        self.items.get(index)
    }
    fn remove_unnotified(&mut self, index: usize) -> TabItem {
        self.items.remove(index)
    }
    fn reset_unnotified(&mut self) {
        self.items.clear();
    }
    fn push_unnotified(&mut self, value: TabItem) {
        self.items.push(value);
    }
}

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl TabsModel {
    qproperty!(
        "currentIndex",
        Member = current_index,
        Notify = current_index_changed
    );

    #[qsignal]
    fn current_index_changed(&mut self);

    /// Opens the folder picked in a FolderDialog (a `file://` URL).
    #[qslot]
    fn open_repository_url(&mut self, url: String) {
        self.open_repository_path(file_url_to_path(&url).to_string_lossy().into_owned());
    }

    /// Opens a plain filesystem path.
    #[qslot]
    fn open_repository_path(&mut self, path: String) {
        let path_buf = std::path::PathBuf::from(path.trim());
        if path_buf.as_os_str().is_empty() {
            return;
        }
        let title = path_buf
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let Some(Some(tab_id)) = Hub::with(|hub| hub.open_tab(path_buf)) else {
            return;
        };
        self.push(TabItem {
            tab_id,
            title,
            repo_path: path,
        });
        self.current_index = self.items.len() as i32 - 1;
        self.current_index_changed();
    }

    #[qslot]
    fn close_tab(&mut self, tab_id: i32) {
        Hub::with(|hub| hub.close_tab(tab_id));
        if let Some(pos) = self.items.iter().position(|t| t.tab_id == tab_id) {
            self.remove(pos);
            let len = self.items.len() as i32;
            if self.current_index >= len {
                self.current_index = len - 1;
            }
            self.current_index_changed();
        }
    }

    #[qslot]
    fn set_current_index(&mut self, index: i32) {
        if index != self.current_index && index >= -1 && index < self.items.len() as i32 {
            self.current_index = index;
            self.current_index_changed();
        }
    }
}
qml_register!(TabsModel, "TabsModel", singleton = false);

// ---------------------------------------------------------------------------
// RepoTab: per-tab lifecycle + error surface + refresh entry points
// ---------------------------------------------------------------------------

pub struct RepoTab {
    tab_id: i32,
    state: String,
    title: String,
    repo_path: String,
    error: String,
    last_error: String,
    tags_shown: bool,
    feed: Option<Arc<Feed<TabMsg>>>,
}

impl Default for RepoTab {
    fn default() -> Self {
        Self {
            tab_id: 0,
            state: String::new(),
            title: String::new(),
            repo_path: String::new(),
            error: String::new(),
            last_error: String::new(),
            // Mirrors core LogOptions::default().
            tags_shown: true,
            feed: None,
        }
    }
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl RepoTab {
    qproperty!("state", Member = state, Notify = changed);
    qproperty!("title", Member = title, Notify = changed);
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    qproperty!("error", Member = error, Notify = changed);
    qproperty!("lastError", Member = last_error, Notify = changed);
    qproperty!("tagsShown", Member = tags_shown, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        self.state = "loading".into();
        self.changed();
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.tab);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        for msg in feed.drain() {
            match msg {
                TabMsg::Opened { title, path } => {
                    self.state = "open".into();
                    self.title = title;
                    self.repo_path = path;
                }
                TabMsg::OpenFailed { message } => {
                    self.state = "error".into();
                    self.error = message;
                }
                TabMsg::OpError { message } => {
                    self.last_error = message;
                }
            }
        }
        self.changed();
    }

    /// Cheap refresh: refs + status + stashes (window focus, post-op).
    #[qslot]
    fn refresh_quick(&mut self) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.refresh_quick();
        }
    }

    /// Full refresh: restarts the log stream as well (manual refresh).
    #[qslot]
    fn refresh_all(&mut self) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.restart_log();
            session.refresh_quick();
        }
    }

    #[qslot]
    fn clear_last_error(&mut self) {
        self.last_error = String::new();
        self.changed();
    }

    /// Shows/hides tags in the graph walk (restarts the stream).
    #[qslot]
    fn set_tags_shown(&mut self, shown: bool) {
        if self.tags_shown == shown {
            return;
        }
        self.tags_shown = shown;
        self.changed();
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.set_include_tags(shown);
        }
    }
}
qml_register!(RepoTab, "RepoTab", singleton = false);

// ---------------------------------------------------------------------------
// GraphModel: the commit graph rows
// ---------------------------------------------------------------------------

// Kept lean: one instance per commit in the window. The short sha is
// derived in QML from `oid_hex` (mechanical substring); `avatar` is a
// packed local identicon code (see encode::avatar_code).
#[derive(QModelItem, Default, Clone)]
pub struct GraphRowItem {
    oid_hex: String,
    author: String,
    atime: i64,
    subject: String,
    node_lane: i32,
    node_color: i32,
    row_width: i32,
    avatar: i32,
    geometry: String,
    labels: String,
}

#[derive(Default)]
pub struct GraphModel {
    rows: Vec<GraphRowItem>,
    generation: u64,
    loading: bool,
    row_total: i32,
    max_lanes: i32,
    first_chunk_ms: i32,
    total_ms: i32,
    truncated: bool,
    /// Lanes running off the end of the window (`lane.color;...`), drawn
    /// by the truncation footer.
    tail_geometry: String,
    error: String,
    started_at: Option<Instant>,
    feed: Option<Arc<Feed<GraphMsg>>>,
    tab_id: i32,
}

impl QListModel for GraphModel {
    type Item = GraphRowItem;

    fn len(&self) -> usize {
        self.rows.len()
    }
    fn get(&self, index: usize) -> Option<&GraphRowItem> {
        self.rows.get(index)
    }
    fn set_unnotified(&mut self, index: usize, value: GraphRowItem) -> bool {
        match self.rows.get_mut(index) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }
    fn reset_unnotified(&mut self) {
        self.rows.clear();
    }
}

impl_extend_notified!(GraphModel, rows, GraphRowItem);

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl GraphModel {
    qproperty!("loading", Member = loading, Notify = stats_changed);
    qproperty!("rowTotal", Member = row_total, Notify = stats_changed);
    qproperty!("maxLanes", Member = max_lanes, Notify = stats_changed);
    qproperty!(
        "firstChunkMs",
        Member = first_chunk_ms,
        Notify = stats_changed
    );
    qproperty!("totalMs", Member = total_ms, Notify = stats_changed);
    qproperty!("truncated", Member = truncated, Notify = stats_changed);
    qproperty!(
        "tailGeometry",
        Member = tail_geometry,
        Notify = stats_changed
    );
    qproperty!("error", Member = error, Notify = stats_changed);

    #[qsignal]
    fn stats_changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.graph);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        for msg in feed.drain() {
            match msg {
                GraphMsg::Started { generation } => {
                    if generation > self.generation {
                        self.generation = generation;
                        self.reset();
                        self.loading = true;
                        self.row_total = 0;
                        self.max_lanes = 1;
                        self.first_chunk_ms = -1;
                        self.total_ms = -1;
                        self.truncated = false;
                        self.error = String::new();
                        self.started_at = Some(Instant::now());
                    }
                }
                GraphMsg::Chunk { generation, rows } => {
                    if generation != self.generation {
                        continue;
                    }
                    if self.first_chunk_ms < 0
                        && let Some(t0) = self.started_at
                    {
                        self.first_chunk_ms = t0.elapsed().as_millis() as i32;
                        tracing::info!(first_chunk_ms = self.first_chunk_ms, "graph first chunk");
                    }
                    let items: Vec<GraphRowItem> = rows.iter().map(to_row_item).collect();
                    for item in &items {
                        self.max_lanes = self.max_lanes.max(item.row_width);
                    }
                    self.extend_notified(items);
                    self.row_total = self.rows.len() as i32;
                }
                GraphMsg::Labels { rows } => {
                    for (row, labels) in rows {
                        let idx = row as usize;
                        if let Some(existing) = self.rows.get(idx) {
                            let mut updated = existing.clone();
                            updated.labels = encode_labels(&labels);
                            self.set(idx, updated);
                        }
                    }
                }
                GraphMsg::Finished {
                    generation,
                    total,
                    elapsed_ms,
                    truncated,
                } => {
                    if generation == self.generation {
                        self.loading = false;
                        self.total_ms = elapsed_ms as i32;
                        self.row_total = total as i32;
                        self.truncated = truncated;
                        self.tail_geometry = if truncated {
                            self.rows
                                .last()
                                .map(|r| crate::encode::tail_lanes(&r.geometry))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        // Release Vec growth slack after the stream ends.
                        self.rows.shrink_to_fit();
                        tracing::info!(total, elapsed_ms, truncated, "graph stream finished");
                    }
                }
                GraphMsg::Failed {
                    generation,
                    message,
                } => {
                    if generation == self.generation {
                        self.loading = false;
                        self.error = message;
                    }
                }
            }
        }
        self.stats_changed();
    }

    /// Row index of a commit (sidebar jump); -1 when absent.
    #[qslot]
    fn row_of(&self, oid_hex: String) -> i32 {
        self.rows
            .iter()
            .position(|r| r.oid_hex == oid_hex)
            .map_or(-1, |i| i as i32)
    }

    /// Full commit id at a row (selection / keyboard navigation).
    #[qslot]
    fn oid_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| r.oid_hex.clone())
            .unwrap_or_default()
    }
}
qml_register!(GraphModel, "GraphModel", singleton = false);

fn to_row_item(row: &LogRow) -> GraphRowItem {
    GraphRowItem {
        oid_hex: row.oid_hex.clone(),
        author: row.author.clone(),
        atime: row.time,
        subject: row.subject.clone(),
        node_lane: i32::from(row.node_lane),
        node_color: i32::from(row.node_color),
        row_width: i32::from(row.width),
        avatar: crate::encode::avatar_code(&row.author),
        geometry: encode_geometry(&row.segments),
        labels: encode_labels(&row.labels),
    }
}

// ---------------------------------------------------------------------------
// NavSectionModel: one sidebar section (branches / remotes / worktree /
// stashes / tags). Five QML instances share this type; each attaches to its
// section's feed and owns its inner scrolling list, so section headers can
// stay fixed in the sidebar while contents scroll.
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct NavItem {
    /// Display text: the last path segment in tree mode, the full name in
    /// flat/filter mode.
    name: String,
    /// Full ref/path (tooltips; folder rows carry their folder path here,
    /// which doubles as the toggle key).
    full: String,
    oid_hex: String,
    change: String,
    bucket: String,
    orig_path: String,
    is_head: bool,
    has_remote: bool,
    depth: i32,
    folder: bool,
    collapsed: bool,
}

#[derive(Default)]
pub struct NavSectionModel {
    section: String,
    all: Vec<NavItem>,
    items: Vec<NavItem>,
    filter: String,
    total: i32,
    /// Explicit folder open/close choices (key = folder path); anything
    /// absent uses the section default.
    folder_overrides: HashMap<String, bool>,
    refs_feed: Option<Arc<Feed<platitude_core::session::RefsSnapshot>>>,
    status_feed: Option<Arc<Feed<StatusMsg>>>,
    stash_feed: Option<Arc<Feed<Vec<platitude_core::stash::StashEntry>>>>,
    tab_id: i32,
}

impl QListModel for NavSectionModel {
    type Item = NavItem;

    fn len(&self) -> usize {
        self.items.len()
    }
    fn get(&self, index: usize) -> Option<&NavItem> {
        self.items.get(index)
    }
    fn reset_unnotified(&mut self) {
        let needle = self.filter.to_lowercase();
        if !needle.is_empty() {
            // Filtering shows flat full names (folders would hide context).
            self.items = self
                .all
                .iter()
                .filter(|i| i.name.to_lowercase().contains(&needle))
                .cloned()
                .collect();
            return;
        }
        if matches!(self.section.as_str(), "branches" | "remotes") {
            self.items = self.build_tree();
        } else {
            self.items = self.all.clone();
        }
    }
}

impl NavSectionModel {
    /// Section default: remote roots (one per remote) start collapsed —
    /// that is the per-repository fold — everything else starts open.
    fn folder_expanded(&self, key: &str, depth: i32) -> bool {
        self.folder_overrides
            .get(key)
            .copied()
            .unwrap_or(!(self.section == "remotes" && depth == 0))
    }

    /// Turns the flat sorted name list into an indented tree with
    /// collapsible folder rows for every `/` level.
    fn build_tree(&self) -> Vec<NavItem> {
        let mut out = Vec::new();
        let mut open_path: Vec<String> = Vec::new();
        // Depth at which a collapsed folder swallows its descendants.
        let mut collapsed_at: Option<usize> = None;

        for leaf in &self.all {
            let segments: Vec<&str> = leaf.name.split('/').collect();
            let folder_count = segments.len() - 1;

            // Longest common folder prefix with the previous entry.
            let mut common = 0;
            while common < open_path.len()
                && common < folder_count
                && open_path[common] == segments[common]
            {
                common += 1;
            }
            open_path.truncate(common);
            if let Some(depth) = collapsed_at
                && depth >= open_path.len()
            {
                collapsed_at = None;
            }

            for (depth, segment) in segments.iter().enumerate().take(folder_count).skip(common) {
                open_path.push((*segment).to_string());
                if collapsed_at.is_some() {
                    continue;
                }
                let key = open_path.join("/");
                let expanded = self.folder_expanded(&key, depth as i32);
                out.push(NavItem {
                    name: (*segment).to_string(),
                    full: key,
                    depth: depth as i32,
                    folder: true,
                    collapsed: !expanded,
                    ..Default::default()
                });
                if !expanded {
                    collapsed_at = Some(depth);
                }
            }
            if collapsed_at.is_none() {
                let mut item = leaf.clone();
                item.full = leaf.name.clone();
                item.name = segments[folder_count].to_string();
                item.depth = folder_count as i32;
                out.push(item);
            }
        }
        out
    }
}

fn branch_nav_items(list: &[platitude_core::session::BranchItem]) -> Vec<NavItem> {
    list.iter()
        .map(|b| NavItem {
            name: b.short.clone(),
            oid_hex: b.oid_hex.clone(),
            is_head: b.is_head,
            has_remote: b.has_remote,
            ..Default::default()
        })
        .collect()
}

/// Working-tree entries in display order (conflicts → staged → unstaged →
/// untracked).
fn status_nav_items(status: &platitude_core::status::WorkTreeStatus) -> Vec<NavItem> {
    let push = |out: &mut Vec<NavItem>, bucket: &str, change: String, path: &str, orig: String| {
        out.push(NavItem {
            name: path.to_string(),
            change,
            bucket: bucket.into(),
            orig_path: orig,
            ..Default::default()
        });
    };
    let mut rows = Vec::new();
    for entry in status.conflicted() {
        if let platitude_core::status::StatusItem::Unmerged { ours, theirs, path } = entry {
            push(
                &mut rows,
                "conflicts",
                format!("{ours}{theirs}"),
                path,
                String::new(),
            );
        }
    }
    for entry in status.staged() {
        if let platitude_core::status::StatusItem::Tracked {
            staged,
            path,
            orig_path,
            ..
        } = entry
        {
            push(
                &mut rows,
                "staged",
                staged.to_string(),
                path,
                orig_path.clone().unwrap_or_default(),
            );
        }
    }
    for entry in status.unstaged() {
        if let platitude_core::status::StatusItem::Tracked { unstaged, path, .. } = entry {
            push(
                &mut rows,
                "unstaged",
                unstaged.to_string(),
                path,
                String::new(),
            );
        }
    }
    for entry in status.untracked() {
        push(
            &mut rows,
            "untracked",
            "?".to_string(),
            entry.path(),
            String::new(),
        );
    }
    rows
}

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl NavSectionModel {
    qproperty!("total", Member = total, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    /// Wires this instance to one section's data feed.
    /// `section`: `branches` / `remotes` / `worktree` / `stashes` / `tags`.
    #[qslot]
    fn attach_section(&mut self, tab_id: i32, section: String) {
        self.tab_id = tab_id;
        self.section = section;
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let invoker = self.get_qml_method_invoker();
        match self.section.as_str() {
            "branches" => {
                let feed = Arc::clone(&feeds.refs_branches);
                feed.attach(invoker);
                self.refs_feed = Some(feed);
            }
            "remotes" => {
                let feed = Arc::clone(&feeds.refs_remotes);
                feed.attach(invoker);
                self.refs_feed = Some(feed);
            }
            "tags" => {
                let feed = Arc::clone(&feeds.refs_tags);
                feed.attach(invoker);
                self.refs_feed = Some(feed);
            }
            "worktree" => {
                let feed = Arc::clone(&feeds.status_nav);
                feed.attach(invoker);
                self.status_feed = Some(feed);
            }
            "stashes" => {
                let feed = Arc::clone(&feeds.stash);
                feed.attach(invoker);
                self.stash_feed = Some(feed);
            }
            other => tracing::warn!(section = other, "unknown sidebar section"),
        }
    }

    #[qslot]
    fn drain(&mut self) {
        if let Some(feed) = self.refs_feed.clone()
            && let Some(snapshot) = feed.drain().pop()
        {
            self.all = match self.section.as_str() {
                "branches" => branch_nav_items(&snapshot.locals),
                "remotes" => branch_nav_items(&snapshot.remotes),
                _ => snapshot
                    .tags
                    .iter()
                    .map(|t| NavItem {
                        name: t.short.clone(),
                        oid_hex: t.oid_hex.clone(),
                        ..Default::default()
                    })
                    .collect(),
            };
        }
        if let Some(feed) = self.status_feed.clone()
            && let Some(StatusMsg { status, .. }) = feed.drain().pop()
        {
            self.all = status_nav_items(&status);
        }
        if let Some(feed) = self.stash_feed.clone()
            && let Some(stashes) = feed.drain().pop()
        {
            // The reflog selector (stash@{0}) is an implementation detail;
            // Phase 2 stash operations will carry it in a hidden role.
            self.all = stashes
                .into_iter()
                .map(|s| NavItem {
                    name: s.message,
                    ..Default::default()
                })
                .collect();
        }
        self.total = self.all.len() as i32;
        self.reset();
        self.changed();
    }

    #[qslot]
    fn set_filter(&mut self, filter: String) {
        if self.filter != filter {
            self.filter = filter;
            self.reset();
            self.changed();
        }
    }

    /// Opens/closes one folder row (key = its path, e.g. `origin/feature`).
    #[qslot]
    fn toggle_folder(&mut self, key: String) {
        let depth = key.matches('/').count() as i32;
        let current = self.folder_expanded(&key, depth);
        self.folder_overrides.insert(key, !current);
        self.reset();
    }

    /// Filtered row count (the header shows `shown/total` while filtering).
    #[qslot]
    fn shown(&self) -> i32 {
        self.items.len() as i32
    }
}
qml_register!(NavSectionModel, "NavSectionModel", singleton = false);

// ---------------------------------------------------------------------------
// WorkTreeModel: always-on header state (branch / ops / conflicts / counts).
// The working-tree file list itself lives in the sidebar (NavSectionModel).
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct WorkTreeModel {
    branch: String,
    detached: bool,
    upstream: String,
    ahead: i32,
    behind: i32,
    op_text: String,
    has_conflicts: bool,
    staged_count: i32,
    unstaged_count: i32,
    untracked_count: i32,
    feed: Option<Arc<Feed<StatusMsg>>>,
    tab_id: i32,
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl WorkTreeModel {
    qproperty!("branch", Member = branch, Notify = changed);
    qproperty!("detached", Member = detached, Notify = changed);
    qproperty!("upstream", Member = upstream, Notify = changed);
    qproperty!("ahead", Member = ahead, Notify = changed);
    qproperty!("behind", Member = behind, Notify = changed);
    qproperty!("opText", Member = op_text, Notify = changed);
    qproperty!("hasConflicts", Member = has_conflicts, Notify = changed);
    qproperty!("stagedCount", Member = staged_count, Notify = changed);
    qproperty!("unstagedCount", Member = unstaged_count, Notify = changed);
    qproperty!("untrackedCount", Member = untracked_count, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.status);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let Some(StatusMsg { status, op_state }) = feed.drain().pop() else {
            return;
        };

        self.branch = status.branch_head.clone().unwrap_or_default();
        self.detached = status.branch_head.is_none() && status.branch_oid.is_some();
        self.upstream = status.upstream.clone().unwrap_or_default();
        self.ahead = status.ahead;
        self.behind = status.behind;
        self.has_conflicts = status.has_conflicts();
        let mut ops: Vec<&str> = Vec::new();
        if op_state.rebasing {
            ops.push("REBASING");
        }
        if op_state.merging {
            ops.push("MERGING");
        }
        if op_state.cherry_picking {
            ops.push("CHERRY-PICKING");
        }
        if op_state.reverting {
            ops.push("REVERTING");
        }
        if op_state.bisecting {
            ops.push("BISECTING");
        }
        self.op_text = ops.join(" · ");
        self.staged_count = status.staged().count() as i32;
        self.unstaged_count = status.unstaged().count() as i32;
        self.untracked_count = status.untracked().count() as i32;
        self.changed();
    }
}
qml_register!(WorkTreeModel, "WorkTreeModel", singleton = false);

// ---------------------------------------------------------------------------
// DetailsModel: commit metadata + changed files
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct FileItem {
    change: String,
    path: String,
    orig_path: String,
}

#[derive(Default)]
pub struct DetailsModel {
    files: Vec<FileItem>,
    sha_hex: String,
    sha8: String,
    parent_hex: String,
    author_name: String,
    author_email: String,
    author_time: i64,
    avatar: i32,
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
    qproperty!("committer", Member = committer, Notify = changed);
    qproperty!("committerTime", Member = committer_time, Notify = changed);
    qproperty!("messageSubject", Member = message_subject, Notify = changed);
    qproperty!("messageBody", Member = message_body, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

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
        self.files = details
            .files
            .iter()
            .map(|f| FileItem {
                change: f.status.to_string(),
                path: f.path.clone(),
                orig_path: f.orig_path.clone().unwrap_or_default(),
            })
            .collect();
        self.reset();
        self.changed();
    }

    /// Path of the changed file at `row` (keyboard navigation, automation).
    #[qslot]
    fn file_path_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.files.get(i))
            .map(|f| f.path.clone())
            .unwrap_or_default()
    }

    /// Original path of the file at `row` (empty unless renamed/copied).
    #[qslot]
    fn file_orig_path_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.files.get(i))
            .map(|f| f.orig_path.clone())
            .unwrap_or_default()
    }
}
qml_register!(DetailsModel, "DetailsModel", singleton = false);

// ---------------------------------------------------------------------------
// DiffModel: unified diff lines for one file
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct DiffLineItem {
    kind: String,
    old_no: i32,
    new_no: i32,
    text: String,
}

#[derive(Default)]
pub struct DiffModel {
    lines: Vec<DiffLineItem>,
    title: String,
    is_binary: bool,
    loading: bool,
    current_key: String,
    feed: Option<Arc<Feed<crate::hub::DiffMsg>>>,
    tab_id: i32,
}

impl QListModel for DiffModel {
    type Item = DiffLineItem;

    fn len(&self) -> usize {
        self.lines.len()
    }
    fn get(&self, index: usize) -> Option<&DiffLineItem> {
        self.lines.get(index)
    }
    fn reset_unnotified(&mut self) {
        self.lines.clear();
    }
}

impl_extend_notified!(DiffModel, lines, DiffLineItem);

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl DiffModel {
    qproperty!("title", Member = title, Notify = changed);
    qproperty!("isBinary", Member = is_binary, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.diff);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
    }

    /// Diff of one file of a commit (vs its first parent).
    #[qslot]
    fn request_commit_file(
        &mut self,
        oid_hex: String,
        parent_hex: String,
        path: String,
        orig_path: String,
    ) {
        let Ok(oid) = Oid::from_hex_str(oid_hex.trim()) else {
            return;
        };
        let parent = Oid::from_hex_str(parent_hex.trim()).ok();
        let target = DiffTarget::Commit {
            oid,
            parent,
            path: path.clone(),
            orig_path: (!orig_path.is_empty()).then_some(orig_path),
        };
        self.begin_request(path, target);
    }

    /// Diff of a working-tree entry (bucket: staged/unstaged/untracked/
    /// conflicts).
    #[qslot]
    fn request_work_tree(&mut self, bucket: String, path: String, orig_path: String) {
        let target = match bucket.as_str() {
            "staged" => DiffTarget::Staged {
                path: path.clone(),
                orig_path: (!orig_path.is_empty()).then_some(orig_path),
            },
            "untracked" => DiffTarget::Untracked { path: path.clone() },
            // Conflicted files show their working-tree state.
            _ => DiffTarget::Unstaged { path: path.clone() },
        };
        self.begin_request(path, target);
    }

    #[qslot]
    fn clear(&mut self) {
        self.current_key = String::new();
        self.title = String::new();
        self.is_binary = false;
        self.loading = false;
        self.reset();
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
        if diff_key(&msg.target) != self.current_key {
            return; // stale response
        }
        self.loading = false;
        self.is_binary = msg.patches.iter().any(|p| p.is_binary);
        self.reset();
        let rows = flatten_patches(&msg.patches)
            .into_iter()
            .map(|r: DiffRow| DiffLineItem {
                kind: r.kind.to_string(),
                old_no: r.old_no,
                new_no: r.new_no,
                text: r.text,
            })
            .collect();
        self.extend_notified(rows);
        self.changed();
    }
}
qml_register!(DiffModel, "DiffModel", singleton = false);

impl DiffModel {
    fn begin_request(&mut self, title: String, target: DiffTarget) {
        self.current_key = diff_key(&target);
        self.title = title;
        self.is_binary = false;
        self.loading = true;
        self.reset();
        self.changed();
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.load_diff(target);
        }
    }
}
