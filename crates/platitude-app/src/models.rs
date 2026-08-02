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
    auto_wip: bool,
    scroll_to: String,
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
            auto_wip: std::env::var("PG_AUTO_WIP").as_deref() == Ok("1"),
            // Smoke-test hook: "top" / "bottom" jumps the graph after load.
            scroll_to: std::env::var("PG_SCROLL_TO").unwrap_or_default(),
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
    qproperty!("autoWip", Member = auto_wip, Constant);
    qproperty!("scrollTo", Member = scroll_to, Constant);

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
    /// Write commands currently in flight (they are serialized per session,
    /// but requests can queue up).
    busy_count: i32,
    busy_op: String,
    /// Last answer to `checkPublish`: how much of a range a remote has.
    publish_range: String,
    publish_total: i32,
    publish_published: i32,
    /// Author identity; `identityReady` false means git cannot commit yet
    /// and the UI should ask for a name and address.
    author_name: String,
    author_email: String,
    identity_ready: bool,
    /// Whether this repository signs commits or tags, and how.
    signing_active: bool,
    signing_format: String,
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
            busy_count: 0,
            busy_op: String::new(),
            publish_range: String::new(),
            publish_total: 0,
            publish_published: 0,
            author_name: String::new(),
            author_email: String::new(),
            // Assumed fine until the check says otherwise, so nothing
            // flashes a warning during startup.
            identity_ready: true,
            signing_active: false,
            signing_format: String::new(),
            feed: None,
        }
    }
}

impl RepoTab {
    /// Runs `f` with this tab's session, if the tab is still open.
    fn with_session(&self, f: impl FnOnce(&Arc<platitude_core::session::RepoSession>)) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            f(&session);
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
    qproperty!("busyCount", Member = busy_count, Notify = changed);
    qproperty!("busyOp", Member = busy_op, Notify = changed);
    qproperty!("publishRange", Member = publish_range, Notify = changed);
    qproperty!("publishTotal", Member = publish_total, Notify = changed);
    qproperty!(
        "publishPublished",
        Member = publish_published,
        Notify = changed
    );
    qproperty!("authorName", Member = author_name, Notify = changed);
    qproperty!("authorEmail", Member = author_email, Notify = changed);
    qproperty!("identityReady", Member = identity_ready, Notify = changed);
    qproperty!("signingActive", Member = signing_active, Notify = changed);
    qproperty!("signingFormat", Member = signing_format, Notify = changed);

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
                TabMsg::Author {
                    name,
                    email,
                    complete,
                    signing,
                    signing_format,
                } => {
                    self.author_name = name;
                    self.author_email = email;
                    self.identity_ready = complete;
                    self.signing_active = signing;
                    self.signing_format = signing_format;
                }
                TabMsg::Publish {
                    range,
                    total,
                    published,
                } => {
                    self.publish_range = range;
                    self.publish_total = total;
                    self.publish_published = published;
                }
                TabMsg::WriteState { op, running } => {
                    if running {
                        self.busy_count += 1;
                        self.busy_op = op;
                    } else {
                        self.busy_count = (self.busy_count - 1).max(0);
                        if self.busy_count == 0 {
                            self.busy_op = String::new();
                        }
                    }
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

    // --- write operations -----------------------------------------------
    //
    // Every one of these is fire-and-forget: the session serializes them,
    // reports progress through `busyCount` and routes git's own error text
    // into `lastError`. Paths arrive one per call — a git path may contain
    // anything except NUL, so there is no separator safe enough to pack a
    // list into one string.

    #[qslot]
    fn stage_path(&mut self, path: String) {
        self.with_session(|s| s.stage_paths(vec![path.clone()]));
    }

    #[qslot]
    fn unstage_path(&mut self, path: String) {
        self.with_session(|s| s.unstage_paths(vec![path.clone()]));
    }

    /// Throws away unstaged modifications of a tracked file (destructive).
    #[qslot]
    fn discard_path(&mut self, path: String) {
        self.with_session(|s| s.discard_paths(vec![path.clone()]));
    }

    /// Deletes an untracked file or directory (destructive).
    #[qslot]
    fn remove_untracked(&mut self, path: String) {
        self.with_session(|s| s.remove_untracked(vec![path.clone()]));
    }

    #[qslot]
    fn stage_all(&mut self) {
        self.with_session(|s| s.stage_all());
    }

    #[qslot]
    fn unstage_all(&mut self) {
        self.with_session(|s| s.unstage_all());
    }

    /// Stages (or unstages) part of one file's diff. `kind` is the diff-key
    /// prefix (`unstaged` / `staged` / `untracked`) and `spec` is
    /// `"<hunk>[:<line>,<line>...];..."` — see `encode::parse_hunk_selection`.
    #[qslot]
    fn stage_selection(&mut self, kind: String, path: String, orig_path: String, spec: String) {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection staging asked for a non-worktree diff");
            return;
        };
        let selects = crate::encode::parse_hunk_selection(&spec);
        if selects.is_empty() {
            return;
        }
        self.with_session(|s| s.apply_partial(target.clone(), selects.clone()));
    }

    /// Commits the index. An empty message is only valid with `amend`,
    /// where it keeps the existing one.
    #[qslot]
    fn commit(&mut self, message: String, amend: bool) {
        let options = platitude_core::commit::CommitOptions {
            amend,
            ..Default::default()
        };
        self.with_session(|s| s.commit(message.clone(), options));
    }

    #[qslot]
    fn checkout_branch(&mut self, name: String) {
        let target = platitude_core::branch::CheckoutTarget::Branch { name };
        self.with_session(|s| s.checkout(target.clone()));
    }

    /// Checks out any commit-ish, detaching HEAD.
    #[qslot]
    fn checkout_detached(&mut self, rev: String) {
        let target = platitude_core::branch::CheckoutTarget::Detach { rev };
        self.with_session(|s| s.checkout(target.clone()));
    }

    /// Creates a local branch tracking a remote-tracking ref and switches.
    #[qslot]
    fn checkout_remote(&mut self, remote_ref: String, local: String) {
        let target = platitude_core::branch::CheckoutTarget::Track { remote_ref, local };
        self.with_session(|s| s.checkout(target.clone()));
    }

    /// Creates a branch at `start_point` (HEAD when empty).
    #[qslot]
    fn create_branch(&mut self, name: String, start_point: String, switch_to: bool) {
        let start = (!start_point.is_empty()).then_some(start_point);
        self.with_session(|s| s.create_branch(name.clone(), start.clone(), switch_to));
    }

    /// Deletes a local branch. Without `force`, git refuses an unmerged one.
    #[qslot]
    fn delete_branch(&mut self, name: String, force: bool) {
        self.with_session(|s| s.delete_branch(name.clone(), force));
    }

    #[qslot]
    fn rename_branch(&mut self, from: String, to: String, force: bool) {
        self.with_session(|s| s.rename_branch(from.clone(), to.clone(), force));
    }

    /// `git stash push`.
    #[qslot]
    fn push_stash(&mut self, message: String, include_untracked: bool, keep_index: bool) {
        let options = platitude_core::stash::PushOptions {
            include_untracked,
            keep_index,
            staged_only: false,
        };
        self.with_session(|s| s.stash_push(message.clone(), options));
    }

    /// `git stash pop` on the given selector (stash-row action).
    #[qslot]
    fn pop_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_pop(selector.clone()));
    }

    /// `git stash apply` on the given selector (keeps the stash).
    #[qslot]
    fn apply_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_apply(selector.clone()));
    }

    /// `git stash drop` (destructive).
    #[qslot]
    fn drop_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_drop(selector.clone()));
    }

    /// `git fetch --prune`; an empty remote fetches all of them.
    #[qslot]
    fn fetch(&mut self, remote: String) {
        let remote = (!remote.is_empty()).then_some(remote);
        self.with_session(|s| s.fetch(remote.clone()));
    }

    /// `git push`. `force` is `""` / `"lease"` / `"force"`; `lease_expect`
    /// pins the remote commit the user saw (empty = bare lease).
    #[qslot]
    fn push_branch(
        &mut self,
        remote: String,
        local: String,
        remote_branch: String,
        set_upstream: bool,
        force: String,
        lease_expect: String,
    ) {
        use platitude_core::remote::PushForce;
        let force = match force.as_str() {
            "lease" => PushForce::WithLease {
                expect: (!lease_expect.is_empty()).then_some(lease_expect),
            },
            "force" => PushForce::Force,
            _ => PushForce::None,
        };
        let spec = platitude_core::remote::PushSpec {
            remote,
            local,
            remote_branch,
            set_upstream,
            force,
        };
        self.with_session(|s| s.push(spec.clone()));
    }

    /// `git push <remote> --delete <branch>` (destructive).
    #[qslot]
    fn delete_remote_branch(&mut self, remote: String, branch: String) {
        self.with_session(|s| s.delete_remote_branch(remote.clone(), branch.clone()));
    }

    /// `git merge <rev>`.
    #[qslot]
    fn merge(&mut self, rev: String, no_ff: bool, ff_only: bool, message: String) {
        let options = platitude_core::integrate::MergeOptions {
            no_ff,
            ff_only,
            squash: false,
            message: (!message.trim().is_empty()).then_some(message),
        };
        self.with_session(|s| s.merge(rev.clone(), options.clone()));
    }

    /// `git rebase <upstream>`; an empty `onto` uses `upstream` as the base.
    #[qslot]
    fn rebase(&mut self, upstream: String, onto: String, autostash: bool, update_refs: bool) {
        let options = platitude_core::integrate::RebaseOptions {
            onto: (!onto.is_empty()).then_some(onto),
            branch: None,
            autostash,
            update_refs,
        };
        self.with_session(|s| s.rebase(upstream.clone(), options.clone()));
    }

    #[qslot]
    fn cherry_pick(&mut self, rev: String) {
        self.with_session(|s| s.cherry_pick(vec![rev.clone()]));
    }

    #[qslot]
    fn revert(&mut self, rev: String) {
        self.with_session(|s| s.revert(vec![rev.clone()]));
    }

    /// Continues / aborts / skips whatever is in progress. `how` is
    /// `"continue"` / `"abort"` / `"skip"` / `"quit"`.
    #[qslot]
    fn resolve_operation(&mut self, how: String) {
        use platitude_core::integrate::Continuation;
        let continuation = match how.as_str() {
            "continue" => Continuation::Continue,
            "abort" => Continuation::Abort,
            "skip" => Continuation::Skip,
            "quit" => Continuation::Quit,
            other => {
                tracing::warn!(how = other, "unknown continuation");
                return;
            }
        };
        self.with_session(|s| s.resolve_current(continuation));
    }

    /// Resolves one conflicted path by taking a side (`"ours"`/`"theirs"`).
    ///
    /// During a rebase the sides are reversed: the commits being replayed
    /// are "theirs".
    #[qslot]
    fn take_side(&mut self, path: String, side: String) {
        use platitude_core::conflict::Side;
        let side = match side.as_str() {
            "ours" => Side::Ours,
            "theirs" => Side::Theirs,
            other => {
                tracing::warn!(side = other, "unknown conflict side");
                return;
            }
        };
        self.with_session(|s| s.take_side(vec![path.clone()], side));
    }

    /// Hands a conflicted path to `git mergetool` (empty = all of them).
    #[qslot]
    fn open_mergetool(&mut self, path: String) {
        let paths = if path.is_empty() {
            Vec::new()
        } else {
            vec![path]
        };
        self.with_session(|s| s.mergetool(paths.clone()));
    }

    /// Asks how much of `range` is already on a remote; the answer arrives
    /// as `publishRange` / `publishTotal` / `publishPublished`.
    #[qslot]
    fn check_publish(&mut self, range: String) {
        self.with_session(|s| s.check_publish(range.clone()));
    }

    /// Records `user.name` / `user.email`. `global` writes the user's own
    /// configuration, which is the right default for a first-run prompt:
    /// the answer is about the person, not the project.
    #[qslot]
    fn set_identity(&mut self, name: String, email: String, global: bool) {
        let scope = if global {
            platitude_core::identity::ConfigScope::Global
        } else {
            platitude_core::identity::ConfigScope::Local
        };
        self.with_session(|s| s.set_identity(name.clone(), email.clone(), scope));
    }

    /// Time budget for fetch / push, in seconds. Zero is ignored.
    #[qslot]
    fn set_network_timeout(&mut self, seconds: i32) {
        if seconds <= 0 {
            return;
        }
        let timeout = std::time::Duration::from_secs(seconds as u64);
        self.with_session(|s| s.set_network_timeout(timeout));
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
    /// `stash@{n}` when the row is a stash; empty otherwise.
    stash_ref: String,
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
    /// Completed stream passes (direct + tag swap + reloads). QML watches
    /// this edge to re-anchor the viewport after each model reset.
    finish_count: i32,
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
    qproperty!("finishCount", Member = finish_count, Notify = stats_changed);
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
                        self.finish_count += 1;
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

    /// Reflog selector when the commit is a stash row (empty otherwise).
    #[qslot]
    fn stash_ref_of(&self, oid_hex: String) -> String {
        self.rows
            .iter()
            .find(|r| r.oid_hex == oid_hex)
            .map(|r| r.stash_ref.clone())
            .unwrap_or_default()
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
        stash_ref: row.stash_ref.clone(),
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
    /// Display grouping of worktree rows (GitKraken-style): untracked
    /// files count as `unstaged` here while `bucket` keeps the real
    /// routing for diffs and staging.
    group: String,
    orig_path: String,
    is_head: bool,
    has_remote: bool,
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
    /// Current branch (branches section only) — feeds the pinned row
    /// shown under the section header.
    head_name: String,
    head_oid: String,
    head_has_remote: bool,
    head_has_pr: bool,
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
        self.items = match self.section.as_str() {
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
        };
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
    qproperty!("refsLoaded", Member = refs_loaded, Notify = changed);
    qproperty!("treeView", Member = tree_view, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    /// Wires this instance to one section's data feed.
    /// `section`: `branches` / `remotes` / `worktree` / `stashes` / `tags`.
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
            // Message only — the reflog selector (stash@{0}) stays hidden
            // by request (Phase 2 stash ops will resolve it internally).
            // The commit id makes rows clickable: the details pane then
            // shows the stashed changes.
            self.all = stashes
                .into_iter()
                .map(|s| NavItem {
                    name: s.message,
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
    conflict_count: i32,
    /// Rebase progress; both zero when nothing is stepping.
    op_step: i32,
    op_steps: i32,
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
    qproperty!("conflictCount", Member = conflict_count, Notify = changed);
    qproperty!("opStep", Member = op_step, Notify = changed);
    qproperty!("opSteps", Member = op_steps, Notify = changed);

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
        let Some(StatusMsg {
            status,
            op_state,
            progress,
        }) = feed.drain().pop()
        else {
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
        self.conflict_count = status.conflicted().count() as i32;
        (self.op_step, self.op_steps) = match progress {
            Some(p) => (p.current as i32, p.total as i32),
            None => (0, 0),
        };
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
    qproperty!("committer", Member = committer, Notify = changed);
    qproperty!("committerTime", Member = committer_time, Notify = changed);
    qproperty!("messageSubject", Member = message_subject, Notify = changed);
    qproperty!("messageBody", Member = message_body, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);
    qproperty!("treeView", Member = tree_view, Notify = changed);
    qproperty!("fileTotal", Member = file_total, Notify = changed);

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
