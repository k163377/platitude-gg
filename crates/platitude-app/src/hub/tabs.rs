//! The tab registry: opening, reserving, closing, and what the
//! sessions behind them are asked for.

use super::sink::BridgeSink;
use super::*;

/// What becomes of the page reading a tab's feeds when the session
/// behind it is let go ([`Hub::let_go_of_session`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageAfter {
    /// It goes with the session: the tab left the front, and its models
    /// are destroyed (`RepoPageStack`).
    TakenDown,
    /// It stays and reads the next session on this tab — the tab
    /// standing in another working copy (`Hub::restand_tab`).
    Standing,
}

impl Hub {
    /// Asks where `path` opens, without opening anything: the working
    /// copy the folder is in, and its repository
    /// ([`platitude_core::repo::place`]). Every road in asks this, and no
    /// tab is shown until it answers (デザイン規約 §タブの所作
    /// 「判定は 1 か所に置く」).
    ///
    /// `false` with no runtime: nothing will ever answer.
    pub fn place_repo(&self, path: PathBuf, feed: Arc<Feed<OpenMsg>>) -> bool {
        let Some(handle) = self.runtime_handle() else {
            return false;
        };
        let executor = self.executor();
        handle.spawn(async move {
            let cancel = tokio_util::sync::CancellationToken::new();
            let msg = match platitude_core::repo::place(&executor, &path, &cancel).await {
                Ok(place) => OpenMsg::Placed { place },
                Err(e) => {
                    let (kind, message) = match &e {
                        platitude_core::GitError::NotARepository { bare, .. } => {
                            (if *bare { "bare" } else { "plain" }, String::new())
                        }
                        _ => ("other", e.to_string()),
                    };
                    OpenMsg::Refused {
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

    /// Clones `url` into `into`, which becomes a tab once it is there.
    /// Answers with the token that stops it, or `None` with no runtime.
    ///
    /// Outside every session, since there is no repository yet
    /// (デザイン規約 §リポジトリを取り寄せる). The budget is the settings'
    /// network timeout, as for a fetch.
    pub fn clone_repo(
        &self,
        url: String,
        into: PathBuf,
        feed: Arc<Feed<CloneMsg>>,
    ) -> Option<tokio_util::sync::CancellationToken> {
        let handle = self.runtime_handle()?;
        let executor = self.executor();
        let timeout = std::time::Duration::from_secs(self.settings.defaults.network_timeout_secs);
        let cancel = tokio_util::sync::CancellationToken::new();
        let token = cancel.clone();
        handle.spawn(async move {
            let msg = match platitude_core::remote::clone(&executor, &url, &into, timeout, &cancel)
                .await
            {
                Ok(()) => CloneMsg::Done { path: into },
                // Stopped by the reader: the dialog went with the press.
                Err(e) if e.is_cancelled() => return,
                Err(e) => CloneMsg::Failed {
                    message: git_said(&e),
                },
            };
            feed.push(msg);
        });
        Some(token)
    }

    /// Opens a repository in a new tab; returns the tab id. `home` is
    /// the repository's own working copy — the same for every copy of
    /// one repository, and equal to `path` for a tab standing in it.
    pub fn open_tab(&mut self, path: PathBuf, home: PathBuf) -> Option<i32> {
        let id = self.reserve_tab(path, home)?;
        self.ensure_open(id);
        Some(id)
    }

    /// Takes a tab id for a repository without opening it. The feeds exist
    /// from the start, so the page attaches to them as usual and simply
    /// stays on "loading" until [`Hub::ensure_open`] fills them.
    pub fn reserve_tab(&mut self, path: PathBuf, home: PathBuf) -> Option<i32> {
        self.runtime_handle()?;
        self.next_tab_id += 1;
        let id = self.next_tab_id;
        self.tabs.insert(
            id,
            Tab {
                session: None,
                runs: 0,
                drawn: None,
                path,
                home,
                feeds: Arc::new(Feeds::default()),
                sink: None,
                drafts: HashMap::new(),
                stand_in: crate::ops::StandIn::default(),
            },
        );
        Some(id)
    }

    /// Where a tab whose copy would not open is stood back: the
    /// repository's own working copy, `None` for a tab already in it
    /// (デザイン規約 §タブの所作「立てない所へは立たない」).
    ///
    /// Asked by the `RepoTab` drain before a refusal is shown, by tab id —
    /// the one thing a page knows about itself; and on every opening, for
    /// the picker's folder (`RepoTab::picker_folder_url`).
    pub fn home_copy(&self, id: i32) -> Option<String> {
        let tab = self.tabs.get(&id)?;
        (tab.home != tab.path).then(|| tab.home.to_string_lossy().into_owned())
    }

    /// Stands a tab in another working copy of the repository it shows:
    /// the same tab id, pointed at `path` and read again from there
    /// (`TabsModel::switch_copy`). Answers whether the tab is this hub's.
    ///
    /// Keeping the id keeps the page (`RepoPageStack`): the new session's
    /// first walk replaces the graph in one go ([`FirstPass::Swapped`]),
    /// and the page drops what the copy owned itself (`RepoPage.leaveCopy`).
    /// Drafts stay, filed per copy (`Tab::drafts`); `Tab::home` stays, as
    /// the repository does not move. A tab off the front holds no session
    /// and is read when it comes to the front.
    pub fn restand_tab(&mut self, id: i32, path: PathBuf) -> bool {
        if !self.tabs.contains_key(&id) {
            return false;
        }
        let standing = self.let_go_of_session(id, PageAfter::Standing);
        let Some(tab) = self.tabs.get_mut(&id) else {
            return false;
        };
        tab.path = path;
        if standing {
            self.open_session(id, FirstPass::Swapped);
        }
        tracing::info!(tab = id, standing, "repository tab stood in another copy");
        true
    }

    /// Opens the session of a reserved tab, if it has not been opened yet.
    pub fn ensure_open(&mut self, id: i32) {
        self.open_session(id, FirstPass::Streamed);
    }

    /// The opening itself; `first_pass` says what the first graph pass
    /// does about a graph already on screen ([`FirstPass`]).
    fn open_session(&mut self, id: i32, first_pass: FirstPass) {
        let Some(handle) = self.runtime_handle() else {
            return;
        };
        let executor = self.executor.clone();
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        if tab.session.is_some() {
            return;
        }
        tab.runs += 1;
        let run = tab.runs;
        // Taken on every opening, so none outlives it
        // (`Hub::let_go_of_session` is the only writer).
        let drawn = tab.drawn.take();
        let path = tab.path.clone();
        // Anything queued came from a released session (a task that had
        // its answer when cancellation reached it); read as the new
        // session's, one message can leave the graph on a generation the
        // new stream never reaches (`Feed::clear_queued`). The command log
        // keeps its messages: they name their session
        // (`Feeds::clear_queued_reads`).
        tab.feeds.clear_queued_reads();
        let feeds = Arc::clone(&tab.feeds);
        let applied = self.settings.defaults.clone();
        let sink = Arc::new(BridgeSink::new(feeds, run));
        let hooks = crate::harness::pass_hooks();
        let session = match first_pass {
            FirstPass::Streamed => {
                RepoSession::open(executor, handle, path, Arc::clone(&sink) as _, hooks)
            }
            FirstPass::Swapped => RepoSession::open_standing_in(
                executor,
                handle,
                path,
                Arc::clone(&sink) as _,
                hooks,
                drawn,
            ),
        };
        apply_repo_settings(&session, &applied);
        // The saved tags flag, here because the page's restoring
        // `setTagsShown` runs before this session exists.
        session.set_include_tags(self.state.layout.tags_shown);
        // After the settings: they are its permission (`fetch_on_open`).
        session.fetch_on_open();
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.session = Some(session);
            tab.sink = Some(sink);
        }
        tracing::info!(
            tab = id,
            auto_fetch_minutes = applied.auto_fetch_minutes,
            network_timeout_secs = applied.network_timeout_secs,
            initial_commits = applied.initial_commits.unwrap_or(0),
            "opened repository tab"
        );
    }

    /// Puts the settings in force on every open tab.
    pub(super) fn reapply_settings(&self) {
        for tab in self.tabs.values() {
            let Some(session) = &tab.session else {
                continue;
            };
            apply_repo_settings(session, &self.settings.defaults);
        }
    }

    /// Lets go of everything a tab read while in front, leaving it in the
    /// strip in the state [`Hub::reserve_tab`] leaves one: a path, empty
    /// feeds, no session. Selecting it again ([`Hub::ensure_open`]) reads
    /// the repository from the start.
    ///
    /// The rows handed to QML go with the page (`RepoPageStack`), which
    /// is why the memory report forgets this tab's models. What the
    /// session read goes even with a write still running
    /// (`RepoSession::close`).
    ///
    /// Does nothing to a tab with no session.
    pub fn release_tab(&mut self, id: i32) {
        if !self.let_go_of_session(id, PageAfter::TakenDown) {
            return;
        }
        crate::harness::memprobe::forget(id);
        tracing::info!(tab = id, "released repository tab");
    }

    /// Lets go of the session behind a tab and answers whether there was
    /// one — shared by [`Hub::release_tab`] and [`Hub::restand_tab`];
    /// `page` is the whole difference.
    fn let_go_of_session(&mut self, id: i32, page: PageAfter) -> bool {
        let Some(tab) = self.tabs.get_mut(&id) else {
            return false;
        };
        let Some(session) = tab.session.take() else {
            return false;
        };
        // The sink first, before the close: it keeps a late write's
        // answer out of the next session's page, and makes the graph
        // taken below the last word on what is on screen. A staying page
        // keeps the log's half (`BridgeSink::retire_reads`).
        if let Some(sink) = tab.sink.take() {
            match page {
                PageAfter::TakenDown => sink.retire(),
                PageAfter::Standing => sink.retire_reads(),
            }
        }
        // Taken before the close, which throws it away (`DrawnGraph`).
        tab.drawn = match page {
            PageAfter::TakenDown => None,
            PageAfter::Standing => Some(session.take_drawn_graph()),
        };
        session.close();
        match page {
            PageAfter::TakenDown => tab.feeds.release_all(),
            // The invokers stay: the page attaches once, when built, and
            // would be left attached to nothing. Queued reads are the old
            // copy's; the log's messages stay.
            PageAfter::Standing => tab.feeds.clear_queued_reads(),
        }
        // Rows a delete took off go back and the lists' drawn readings
        // are dropped: the answer that would put the rows back is this
        // session's, kept out by the retired sink, and the readings carry
        // this session's stamps (`ops::StandIn::session_gone`).
        tab.stand_in.session_gone();
        self.park_writes_of(&session);
        true
    }

    /// Files a tab's commit editor words under the copy it stands in (see
    /// [`Draft`]). An empty draft is filed too — otherwise a cleared
    /// message would come back.
    pub fn hold_draft(&mut self, id: i32, draft: Draft) {
        if let Some(tab) = self.tabs.get_mut(&id) {
            let copy = tab.path.to_string_lossy().into_owned();
            tab.drafts.insert(copy, draft);
        }
    }

    /// What was typed in the copy this tab stands in; empty for none, or
    /// for an unknown tab.
    pub fn draft(&self, id: i32) -> Draft {
        self.tabs
            .get(&id)
            .and_then(|tab| tab.drafts.get(tab.path.to_string_lossy().as_ref()))
            .cloned()
            .unwrap_or_default()
    }

    /// Closes a tab and cancels its session.
    pub fn close_tab(&mut self, id: i32) {
        if let Some(tab) = self.tabs.remove(&id) {
            if let Some(sink) = tab.sink {
                sink.retire();
            }
            if let Some(session) = tab.session {
                session.close();
                self.park_writes_of(&session);
            }
            crate::harness::memprobe::forget(id);
            tracing::info!(tab = id, "closed repository tab");
        }
    }

    /// Keeps a closed session's write loop while it has a local write to
    /// finish (`RepoSession::close`), so [`Hub::writes_settled`] and
    /// [`Hub::shutdown`] still see it. Call after the close: no new write
    /// can start then, so zero pending means the queue is done.
    ///
    /// Only the waiting is the hub's: a reopened session queues behind
    /// this write through the working tree's order
    /// (`platitude_core::session::write_order`), not through this handle.
    fn park_writes_of(&mut self, session: &Arc<RepoSession>) {
        if session.local_writes_pending() == 0 {
            return;
        }
        if let Some(write) = session.take_write_join() {
            self.parked_writes.push(write);
        }
    }

    /// Whether every write git was asked for is done: open tabs' local
    /// writes, the parked writes of closed tabs, and the saves
    /// (`hub::saves`). The quit gate (`WindowQuitGate`) keeps the window
    /// until this is true, so the join in [`Hub::shutdown`] normally waits
    /// for nothing.
    pub fn writes_settled(&mut self) -> bool {
        self.parked_writes.retain(|write| !write.is_finished());
        self.parked_writes.is_empty()
            && self.saves.pending() == 0
            && self.tabs.values().all(|tab| {
                tab.session
                    .as_ref()
                    .is_none_or(|session| session.local_writes_pending() == 0)
            })
    }

    /// Re-reads every open tab's author configuration: sessions cache it,
    /// so a write from the app-level identity screen leaves them stale.
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

    /// Messages waiting in each tab's feeds, as `<name>#<tab>:<depth>` for
    /// the non-empty ones. Walks `Feeds::each`, never a hand-kept list.
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

/// What git said, without the command line around it — the half a screen
/// quotes (デザイン規約 §リポジトリを取り寄せる「赤は git の文言だけ」).
/// The error's `Display` where git said nothing (never started, or killed
/// by the budget).
fn git_said(e: &platitude_core::GitError) -> String {
    let stderr = match e {
        platitude_core::GitError::Failed { stderr, .. }
        | platitude_core::GitError::Reported { stderr, .. } => stderr.trim(),
        _ => "",
    };
    if stderr.is_empty() {
        e.to_string()
    } else {
        stderr.to_string()
    }
}

/// Asks a tab's session something, or `None` with no session: a closed
/// tab, or one not yet opened ([`Hub::ensure_open`]).
pub fn from_session<R>(tab_id: i32, f: impl FnOnce(&Arc<RepoSession>) -> R) -> Option<R> {
    let session = Hub::with(|hub| hub.session(tab_id))??;
    Some(f(&session))
}

/// The same, dropping the answer — a slot that must hand a write's id
/// back to the page uses [`from_session`].
pub fn with_session<R>(tab_id: i32, f: impl FnOnce(&Arc<RepoSession>) -> R) {
    from_session(tab_id, f);
}

/// The sidebar section `section` has applied a listing to its rows,
/// stamped `at` as the read was taken (`session::Standing::stamp`).
///
/// Recorded for that section's own row only (`ops::Row::drawn_by`): the
/// three refs sections draw the same read on separate turns, and a section
/// that stands nothing in is not recorded. Which write it answers is the
/// stand-in's to decide (`ops::StandIn::look_again`).
pub fn listing_applied(tab_id: i32, section: &str, at: u64) {
    let Some(row) = crate::ops::Row::drawn_by(section) else {
        return;
    };
    stand_in(tab_id, |gone| gone.listing_applied(row, at));
}

/// Moves a tab's delete along and answers with the rows it leaves showing
/// as gone (`ops::StandIn`) — none for a tab this hub does not have.
///
/// The caller is handed the rows: the borrow is over before this returns,
/// so the copy a property is read from is taken with the hub free (the
/// re-entrant borrow the bridge panics on — rules/app-ui.md
/// §Qt Bridges・QML の不変条件).
pub fn stand_in(tab_id: i32, f: impl FnOnce(&mut crate::ops::StandIn)) -> crate::ops::Rows {
    Hub::with(|hub| match hub.tabs.get_mut(&tab_id) {
        Some(tab) => {
            f(&mut tab.stand_in);
            tab.stand_in.rows().clone()
        }
        None => crate::ops::Rows::default(),
    })
    .unwrap_or_default()
}

fn minutes_to_interval(minutes: u32) -> Option<std::time::Duration> {
    (minutes > 0).then(|| std::time::Duration::from_secs(u64::from(minutes) * 60))
}

/// Puts the settings in force on one session — every settings value passes
/// through here, so none is written but never applied.
fn apply_repo_settings(session: &Arc<RepoSession>, applied: &platitude_core::settings::Defaults) {
    session.set_auto_fetch(minutes_to_interval(applied.auto_fetch_minutes));
    session.set_network_timeout(std::time::Duration::from_secs(applied.network_timeout_secs));
    // Off holds for an opening's pass too, not only the page's pace.
    session.set_copies_pace(applied.copies_pace());
    session.set_pace_bounds(applied.pace_bounds());
    // Walks nothing twice on a new session: with no workdir yet the
    // restart returns, and the opening's own `restart_log` reads this.
    session.set_log_limit(applied.initial_commits);
}
