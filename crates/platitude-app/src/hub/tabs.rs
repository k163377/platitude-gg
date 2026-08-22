//! The tab registry: opening, reserving, closing, and what the
//! sessions behind them are asked for.

use super::sink::BridgeSink;
use super::*;

impl Hub {
    /// Installs the hub into the main thread. Call once before `QApp::run`.
    ///
    /// A run that was turned away from the settings files is handed an
    /// empty store and `held_elsewhere` names the directory it did not get.
    pub fn install(runtime: tokio::runtime::Runtime, store: Store, held_elsewhere: String) {
        let settings = store.load_settings();
        let state = store.load_state();
        tracing::info!(
            settings = ?store.settings_path(),
            state = ?store.state_path(),
            "settings store"
        );
        HUB.with(|h| {
            *h.borrow_mut() = Some(Hub {
                runtime: Some(runtime),
                executor: GitExecutor::new(),
                tabs: HashMap::new(),
                next_tab_id: 0,
                store,
                settings,
                saved_state: state.clone(),
                state,
                held_elsewhere,
            });
        });
        Hub::with(Hub::forget_missing_avatars);
    }

    /// Runs `f` with the hub; logs and returns `None` when uninstalled
    /// (only possible before `install`, i.e. never during normal slots).
    pub fn with<R>(f: impl FnOnce(&mut Hub) -> R) -> Option<R> {
        HUB.with(|h| {
            let mut guard = h.borrow_mut();
            match guard.as_mut() {
                Some(hub) => Some(f(hub)),
                None => {
                    tracing::error!("hub used before install");
                    None
                }
            }
        })
    }

    /// Shuts down all sessions and the runtime. Call after `QApp::run`.
    pub fn shutdown() {
        let hub = HUB.with(|h| h.borrow_mut().take());
        if let Some(mut hub) = hub {
            hub.flush_state();
            for (_, tab) in hub.tabs.drain() {
                if let Some(session) = tab.session {
                    session.close();
                }
            }
            if let Some(rt) = hub.runtime.take() {
                rt.shutdown_timeout(std::time::Duration::from_secs(2));
            }
        }
    }

    pub fn runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self.runtime.as_ref().map(|r| r.handle().clone())
    }

    pub fn executor(&self) -> GitExecutor {
        self.executor.clone()
    }

    /// Asks whether `path` can be opened, without opening anything.
    ///
    /// Only the picker goes through here — a folder that is no repository
    /// must never become a (remembered) tab. Every other way in (a
    /// restored tab, a worktree row, `PG_AUTO_OPEN`) opens straight away
    /// and fails on the page's own failure screen
    /// (デザイン規約 §可否・警告の出し場所).
    pub fn probe_repo(&self, path: PathBuf, feed: Arc<Feed<PickMsg>>) -> bool {
        let Some(handle) = self.runtime_handle() else {
            return false;
        };
        let executor = self.executor();
        handle.spawn(async move {
            let cancel = tokio_util::sync::CancellationToken::new();
            let msg = match platitude_core::repo::open(&executor, &path, &cancel).await {
                Ok(_) => PickMsg::Accepted { path },
                Err(e) => {
                    let (kind, message) = match &e {
                        platitude_core::GitError::NotARepository { bare, .. } => {
                            (if *bare { "bare" } else { "plain" }, String::new())
                        }
                        _ => ("other", e.to_string()),
                    };
                    PickMsg::Rejected {
                        near: crate::urlpath::picker_folder_url(&path),
                        path,
                        kind,
                        message,
                    }
                }
            };
            feed.push(msg);
        });
        true
    }

    /// Opens a repository in a new tab; returns the tab id.
    pub fn open_tab(&mut self, path: PathBuf) -> Option<i32> {
        let id = self.reserve_tab(path)?;
        self.ensure_open(id);
        Some(id)
    }

    /// Takes a tab id for a repository without opening it. The feeds exist
    /// from the start, so the page attaches to them as usual and simply
    /// stays on "loading" until [`Hub::ensure_open`] fills them.
    pub fn reserve_tab(&mut self, path: PathBuf) -> Option<i32> {
        self.runtime_handle()?;
        self.next_tab_id += 1;
        let id = self.next_tab_id;
        self.tabs.insert(
            id,
            Tab {
                session: None,
                path,
                feeds: Arc::new(Feeds::default()),
                draft: Draft::default(),
            },
        );
        Some(id)
    }

    /// Opens the session of a reserved tab, if it has not been opened yet.
    pub fn ensure_open(&mut self, id: i32) {
        let Some(handle) = self.runtime_handle() else {
            return;
        };
        let executor = self.executor.clone();
        let Some(tab) = self.tabs.get(&id) else {
            return;
        };
        if tab.session.is_some() {
            return;
        }
        let path = tab.path.clone();
        // Nothing queued here can be about the session about to be opened,
        // because there is no session yet — so anything waiting came from
        // one that has been released, pushed by a task that had already
        // worked out its answer when cancellation reached it. Read as the
        // new session's, one such message is enough to leave the graph
        // holding a generation the new stream never reaches
        // (`Feed::clear_queued`).
        tab.feeds.clear_queued_all();
        let feeds = Arc::clone(&tab.feeds);
        let applied = self.settings.for_repo(&path.to_string_lossy());
        let sink = Arc::new(BridgeSink { feeds });
        let session = RepoSession::open(executor, handle, path, sink);
        apply_repo_settings(&session, &applied);
        // The saved tags flag takes the same door the settings do: the
        // page's restore runs before this session exists, so its
        // `setTagsShown` cannot be the write that lands here — without
        // this the eye read "hidden" while the walk drew every tag.
        session.set_include_tags(self.state.layout.tags_shown);
        // Straight after the settings, because they are the permission:
        // the session holds this until it knows where the repository is,
        // and fetches before it reads anything (`fetch_on_open`).
        session.fetch_on_open();
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.session = Some(session);
        }
        tracing::info!(
            tab = id,
            auto_fetch_minutes = applied.auto_fetch_minutes,
            network_timeout_secs = applied.network_timeout_secs,
            "opened repository tab"
        );
    }

    /// Puts the settings in force on every open tab. A repository with a
    /// setting of its own keeps it.
    pub(super) fn reapply_settings(&self) {
        for tab in self.tabs.values() {
            let Some(session) = &tab.session else {
                continue;
            };
            apply_repo_settings(
                session,
                &self.settings.for_repo(&tab.path.to_string_lossy()),
            );
        }
    }

    /// Lets go of everything a tab read while it was in front, leaving the
    /// tab itself in the strip.
    ///
    /// What is left afterwards is a tab in exactly the state
    /// [`Hub::reserve_tab`] leaves one in: a path, empty feeds, and no
    /// session. Selecting it again runs [`Hub::ensure_open`], which opens
    /// a new session and reads the repository from the start — so this is
    /// only ever a matter of memory, never of what the reader can get
    /// back to.
    ///
    /// **The models are the other half.** This releases what the *hub*
    /// holds; the rows already handed to QML are released by the page
    /// being taken down with the tab (`Main.qml`), which is also why the
    /// memory report is told to forget this tab's models — their last
    /// reported footprints describe objects that no longer exist.
    ///
    /// Does nothing to a tab that has no session, which is the normal
    /// case for a tab nobody has looked at yet.
    pub fn release_tab(&mut self, id: i32) {
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        let Some(session) = tab.session.take() else {
            return;
        };
        session.close();
        tab.feeds.release_all();
        crate::memprobe::forget(id);
        tracing::info!(tab = id, "released repository tab");
    }

    /// Puts the words typed into a tab's commit editor somewhere that
    /// outlives its page (see [`Draft`]).
    pub fn hold_draft(&mut self, id: i32, draft: Draft) {
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.draft = draft;
        }
    }

    /// What was typed there, for the page that comes back. Empty for a tab
    /// nobody has written in — which is also what an unknown tab answers,
    /// since there is nothing to put back either way.
    pub fn draft(&self, id: i32) -> Draft {
        self.tabs
            .get(&id)
            .map(|tab| tab.draft.clone())
            .unwrap_or_default()
    }

    /// Closes a tab and cancels its session.
    pub fn close_tab(&mut self, id: i32) {
        if let Some(tab) = self.tabs.remove(&id) {
            if let Some(session) = tab.session {
                session.close();
            }
            crate::memprobe::forget(id);
            tracing::info!(tab = id, "closed repository tab");
        }
    }

    /// Re-reads the author configuration of every open tab. Each session
    /// caches what it read when the repository opened, so a write made
    /// outside them (the app-level identity screen) leaves them stale.
    pub fn refresh_authors(&self) {
        for tab in self.tabs.values() {
            if let Some(session) = &tab.session {
                session.refresh_author();
            }
        }
    }

    pub fn session(&self, id: i32) -> Option<Arc<RepoSession>> {
        self.tabs.get(&id).and_then(|t| t.session.clone())
    }

    pub fn feeds(&self, id: i32) -> Option<Arc<Feeds>> {
        self.tabs.get(&id).map(|t| Arc::clone(&t.feeds))
    }

    /// How many messages are waiting in each tab's feeds, as
    /// `<name>#<tab>:<depth>` for the ones holding anything. The feeds
    /// walked are the one list `Feeds::each` carries — a copy kept by
    /// hand here once under-reported exactly the queue this report exists
    /// to catch.
    pub fn feed_depths(&self) -> String {
        let mut waiting: Vec<String> = Vec::new();
        for (id, tab) in &self.tabs {
            for (name, feed) in tab.feeds.each() {
                let depth = feed.depth();
                if depth > 0 {
                    waiting.push(format!("{name}#{id}:{depth}"));
                }
            }
        }
        waiting.sort();
        waiting.join(" ")
    }

    /// Every open session with its tab number, lowest first — what the
    /// memory report walks.
    pub fn sessions(&self) -> Vec<(i32, Arc<RepoSession>)> {
        let mut open: Vec<(i32, Arc<RepoSession>)> = self
            .tabs
            .iter()
            .filter_map(|(id, tab)| tab.session.clone().map(|s| (*id, s)))
            .collect();
        open.sort_by_key(|(id, _)| *id);
        open
    }
}

/// Asks a tab's session something, or answers `None` because there is no
/// session to ask: a tab that has been closed, or one nobody has looked at
/// yet ([`Hub::ensure_open`] is what opens one).
pub fn from_session<R>(tab_id: i32, f: impl FnOnce(&Arc<RepoSession>) -> R) -> Option<R> {
    let session = Hub::with(|hub| hub.session(tab_id))??;
    Some(f(&session))
}

/// The same for telling it to do something, where there being no session
/// means there is nothing to do.
pub fn with_session(tab_id: i32, f: impl FnOnce(&Arc<RepoSession>)) {
    from_session(tab_id, f);
}

fn minutes_to_interval(minutes: u32) -> Option<std::time::Duration> {
    (minutes > 0).then(|| std::time::Duration::from_secs(u64::from(minutes) * 60))
}

/// Puts one repository's settings in force on its session. Every value the
/// settings file holds passes through here, so a key that is written but
/// never applied cannot go unnoticed.
fn apply_repo_settings(
    session: &Arc<RepoSession>,
    applied: &platitude_core::settings::RepoSettings,
) {
    session.set_auto_fetch(minutes_to_interval(applied.auto_fetch_minutes));
    session.set_network_timeout(std::time::Duration::from_secs(applied.network_timeout_secs));
}
