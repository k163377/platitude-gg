use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, Hub, StatusMsg};

use super::qml_register;

// ---------------------------------------------------------------------------
// NavSectionModel: one section list (branches / remotes / worktree /
// worktrees / stashes / tags). Six QML instances share this type; each
// attaches to its section's feed and owns its inner scrolling list, so
// section headers can stay fixed while contents scroll. All render in the
// sidebar except `worktree` (the changed files), which the right pane's
// WIP view shows.
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
    /// Display grouping of worktree rows (GitKraken-style): untracked
    /// files count as `unstaged` here while `bucket` keeps the real
    /// routing for diffs and staging.
    group: String,
    orig_path: String,
    is_head: bool,
    has_remote: bool,
    /// The remote branch a local one speaks for (`origin/main`), empty for
    /// every other kind of row. What a rename of this row offers to carry
    /// over, and what the badge beside it is about.
    upstream: String,
    /// PR-state badge. Real data arrives in Phase 4 (ls-remote refs/pull
    /// matching); until then PG_FAKE_PR previews the look.
    has_pr: bool,
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
    /// Current branch (branches section only) — feeds the sticky row
    /// that stands in for it while its own row is scrolled off.
    head_name: String,
    head_oid: String,
    head_has_remote: bool,
    head_has_pr: bool,
    /// Visible row of the current entry, or -1 when it has none (a
    /// filter or a collapsed folder hides it, or HEAD is detached). The
    /// sticky row needs it to tell whether the real row is on screen.
    head_row: i32,
    /// True once a refs snapshot arrived (distinguishes "no head yet"
    /// from "detached / no local branches" for the default selection).
    refs_loaded: bool,
    /// Worktree section only: tree (default) vs flat-path display.
    tree_view: bool,
    /// Explicit folder open/close choices (key = folder path); anything
    /// absent uses the section default.
    folder_overrides: HashMap<String, bool>,
    refs_feed: Option<Arc<Feed<platitude_core::session::RefsSnapshot>>>,
    status_feed: Option<Arc<Feed<StatusMsg>>>,
    stash_feed: Option<Arc<Feed<Vec<platitude_core::stash::StashEntry>>>>,
    worktrees_feed: Option<Arc<Feed<Vec<platitude_core::worktrees::WorktreeEntry>>>>,
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
        self.items = if needle.is_empty() {
            match self.section.as_str() {
                "branches" | "remotes" => self.build_tree(),
                // The worktree keeps its group runs (conflicts → unstaged →
                // staged) and trees each run independently.
                "worktree" if self.tree_view => {
                    let mut out = Vec::new();
                    let mut i = 0;
                    while i < self.all.len() {
                        let group = self.all[i].group.clone();
                        let mut j = i + 1;
                        while j < self.all.len() && self.all[j].group == group {
                            j += 1;
                        }
                        wt_tree_into(&self.all[i..j], &group, &self.folder_overrides, &mut out);
                        i = j;
                    }
                    out
                }
                _ => self.all.clone(),
            }
        } else {
            // Filtering shows flat full names (folders would hide context).
            self.all
                .iter()
                .filter(|i| i.name.to_lowercase().contains(&needle))
                .cloned()
                .collect()
        };
        self.head_row = self
            .items
            .iter()
            .position(|i| i.is_head && !i.folder)
            .map_or(-1, |row| row as i32);
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

/// `remote` strips the remote prefix before the PR lookup, so
/// `origin/main` matches a PR on `main`.
fn branch_nav_items(list: &[platitude_core::session::BranchItem], remote: bool) -> Vec<NavItem> {
    list.iter()
        .map(|b| {
            let pr_key = if remote {
                b.short.split_once('/').map_or(b.short.as_str(), |(_, r)| r)
            } else {
                b.short.as_str()
            };
            NavItem {
                name: b.short.clone(),
                oid_hex: b.oid_hex.clone(),
                is_head: b.is_head,
                has_remote: b.has_remote,
                upstream: b.upstream.clone(),
                has_pr: crate::encode::fake_pr_set().contains(pr_key),
                ..Default::default()
            }
        })
        .collect()
}

/// Trees one group run of worktree entries: single-child directory
/// chains compact into one `a/b/c` row; fold-toggle keys are
/// group-prefixed so equal paths in different groups fold apart.
fn wt_tree_into(
    entries: &[NavItem],
    group: &str,
    overrides: &HashMap<String, bool>,
    out: &mut Vec<NavItem>,
) {
    #[derive(Default)]
    struct DirNode {
        dirs: std::collections::BTreeMap<String, DirNode>,
        files: Vec<NavItem>,
    }
    let mut root = DirNode::default();
    for entry in entries {
        let mut node = &mut root;
        let mut rest = entry.full.as_str();
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
        group: &str,
        prefix: &str,
        depth: i32,
        overrides: &HashMap<String, bool>,
        out: &mut Vec<NavItem>,
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
            let path = format!("{prefix}{label}");
            let key = format!("{group}:{path}");
            let expanded = overrides.get(&key).copied().unwrap_or(true);
            out.push(NavItem {
                name: label,
                full: key.clone(),
                group: group.to_string(),
                depth,
                folder: true,
                collapsed: !expanded,
                ..Default::default()
            });
            if expanded {
                emit(
                    target,
                    group,
                    &format!("{path}/"),
                    depth + 1,
                    overrides,
                    out,
                );
            }
        }
        for f in &node.files {
            let mut item = f.clone();
            item.depth = depth;
            out.push(item);
        }
    }
    emit(&root, group, "", 0, overrides, out);
}

/// Working-tree entries in GitKraken display order: conflicts → unstaged
/// (untracked files count as unstaged for display, via `group`, while
/// `bucket` keeps the real diff/staging routing) → staged. `full` always
/// carries the real path (tree leaves rename `name` to their last
/// segment).
fn status_nav_items(status: &platitude_core::status::WorkTreeStatus) -> Vec<NavItem> {
    let push = |out: &mut Vec<NavItem>,
                bucket: &str,
                group: &str,
                change: String,
                path: &str,
                orig: String| {
        out.push(NavItem {
            name: path.to_string(),
            full: path.to_string(),
            change,
            bucket: bucket.into(),
            group: group.into(),
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
                "conflicts",
                format!("{ours}{theirs}"),
                path,
                String::new(),
            );
        }
    }
    for entry in status.unstaged() {
        if let platitude_core::status::StatusItem::Tracked { unstaged, path, .. } = entry {
            push(
                &mut rows,
                "unstaged",
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
            "unstaged",
            "?".to_string(),
            entry.path(),
            String::new(),
        );
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
                "staged",
                staged.to_string(),
                path,
                orig_path.clone().unwrap_or_default(),
            );
        }
    }
    rows
}

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl NavSectionModel {
    qproperty!("total", Member = total, Notify = changed);
    qproperty!("headName", Member = head_name, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("headHasRemote", Member = head_has_remote, Notify = changed);
    qproperty!("headHasPr", Member = head_has_pr, Notify = changed);
    qproperty!("headRow", Member = head_row, Notify = changed);
    qproperty!("refsLoaded", Member = refs_loaded, Notify = changed);
    qproperty!("treeView", Member = tree_view, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    /// Wires this instance to one section's data feed. `section`:
    /// `branches` / `remotes` / `worktree` / `worktrees` / `stashes` /
    /// `tags`.
    #[qslot]
    fn attach_section(&mut self, tab_id: i32, section: String) {
        self.tab_id = tab_id;
        self.section = section;
        self.tree_view = true;
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
            "worktrees" => {
                let feed = Arc::clone(&feeds.worktrees);
                feed.attach(invoker);
                self.worktrees_feed = Some(feed);
            }
            other => tracing::warn!(section = other, "unknown sidebar section"),
        }
    }

    #[qslot]
    fn drain(&mut self) {
        if let Some(feed) = self.refs_feed.clone()
            && let Some(snapshot) = feed.drain().pop()
        {
            self.refs_loaded = true;
            self.all = match self.section.as_str() {
                "branches" => {
                    let items = branch_nav_items(&snapshot.locals, false);
                    let head = items.iter().find(|b| b.is_head);
                    self.head_name = head.map(|b| b.name.clone()).unwrap_or_default();
                    self.head_oid = head.map(|b| b.oid_hex.clone()).unwrap_or_default();
                    self.head_has_remote = head.is_some_and(|b| b.has_remote);
                    self.head_has_pr = head.is_some_and(|b| b.has_pr);
                    items
                }
                "remotes" => branch_nav_items(&snapshot.remotes, true),
                _ => snapshot
                    .tags
                    .iter()
                    .map(|t| NavItem {
                        name: t.short.clone(),
                        oid_hex: t.oid_hex.clone(),
                        // Same badge as a branch: nothing means this tag
                        // is only here. Real data needs ls-remote (Phase
                        // 4); PG_FAKE_REMOTE_TAGS previews the look.
                        has_remote: crate::encode::fake_remote_tag_set().contains(&t.short),
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
        if let Some(feed) = self.worktrees_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            // The current worktree is marked like the current branch;
            // `bucket` carries the branch (empty = detached) and `full`
            // the absolute path (tooltip + click-to-open).
            let current = Hub::with(|hub| hub.session(self.tab_id).and_then(|s| s.workdir()))
                .flatten()
                .map(|p| p.to_string_lossy().replace('\\', "/").to_lowercase())
                .unwrap_or_default();
            self.all = list
                .into_iter()
                .filter(|w| !w.bare)
                .map(|w| {
                    let norm = w.path.replace('\\', "/");
                    let name = norm.rsplit('/').next().unwrap_or(norm.as_str()).to_string();
                    let has_pr = w
                        .branch
                        .as_deref()
                        .is_some_and(|b| crate::encode::fake_pr_set().contains(b));
                    NavItem {
                        name,
                        is_head: norm.to_lowercase() == current,
                        full: w.path,
                        bucket: w.branch.unwrap_or_default(),
                        has_pr,
                        ..Default::default()
                    }
                })
                .collect();
        }
        if let Some(feed) = self.stash_feed.clone()
            && let Some(stashes) = feed.drain().pop()
        {
            // The message is the whole of what a stash shows: the reflog
            // selector (stash@{0}) stays out of sight by request, but it
            // rides along in `full` because it is what names the entry to
            // git — renaming one takes the selector, not the message.
            // The commit id makes rows clickable: the details pane then
            // shows the stashed changes.
            self.all = stashes
                .into_iter()
                .map(|s| NavItem {
                    name: s.message,
                    full: s.name,
                    oid_hex: s.oid.to_hex(),
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
        self.reset();
        self.changed();
    }

    /// Filtered row count (`total` counts all rows; this counts what the
    /// filter lets through).
    #[qslot]
    fn shown(&self) -> i32 {
        self.items.len() as i32
    }

    /// Commit id of the ref with this name; empty when there is none.
    ///
    /// Looks in the section's whole list rather than the visible rows, so
    /// an active filter or a collapsed folder does not hide the answer.
    #[qslot]
    fn oid_of_name(&self, name: String) -> String {
        self.all
            .iter()
            .find(|item| item.name == name)
            .map(|item| item.oid_hex.clone())
            .unwrap_or_default()
    }

    /// The remote branch this one speaks for (`origin/main`); empty when
    /// it speaks for none, and for every section but the branches.
    #[qslot]
    fn upstream_of(&self, name: String) -> String {
        self.all
            .iter()
            .find(|item| item.name == name)
            .map(|item| item.upstream.clone())
            .unwrap_or_default()
    }

    /// What one row shows, and what git knows it by (empty out of range).
    ///
    /// Roles are only visible to a delegate, so this is how automation
    /// reaches a row it has to act on.
    #[qslot]
    fn name_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.items.get(row))
            .map(|item| item.name.clone())
            .unwrap_or_default()
    }

    #[qslot]
    fn full_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.items.get(row))
            .map(|item| item.full.clone())
            .unwrap_or_default()
    }
}
qml_register!(NavSectionModel, "NavSectionModel", singleton = false);
