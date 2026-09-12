//! The tab registry: opening, reserving, closing, and what the
//! sessions behind them are asked for.

use super::sink::BridgeSink;
use super::*;

impl Hub {
    /// Asks whether `path` can be opened, without opening anything.
    ///
    /// Only the picker goes through here — a folder that is no repository
    /// must never become a (remembered) tab. Every other way in (a
    /// restored tab, a worktree row, `PGG_AUTO_OPEN`) opens straight away
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

    /// Fetches a repository into `into`, which becomes a tab once it is
    /// there. Answers with the token that stops it, or `None` where there
    /// is no runtime to run it on.
    ///
    /// Outside every session, because there is no repository yet to have
    /// one: the dialog that asked is the whole of what is on screen about
    /// this, and it stands until the answer comes back
    /// (デザイン規約 §リポジトリを取り寄せる). The budget is the same one
    /// every other network command is given, so a clone over a slow line
    /// is raised in the same place as a slow fetch (the settings screen's
    /// network timeout).
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
                // A clone the reader stopped is not something to report
                // back at: the dialog it was asked in has gone with the
                // press that stopped it.
                Err(e) if e.is_cancelled() => return,
                Err(e) => CloneMsg::Failed {
                    message: git_said(&e),
                },
            };
            feed.push(msg);
        });
        Some(token)
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
                sink: None,
                draft: Draft::default(),
                stand_in: crate::ops::StandIn::default(),
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
        let applied = self.settings.defaults.clone();
        let sink = Arc::new(BridgeSink::new(feeds));
        let session = RepoSession::open(
            executor,
            handle,
            path,
            Arc::clone(&sink) as _,
            crate::harness::pass_hooks(),
        );
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

    /// Puts the settings in force on every open tab. One set of values, so
    /// every tab is left saying the same thing about the network as the
    /// settings screen does.
    pub(super) fn reapply_settings(&self) {
        for tab in self.tabs.values() {
            let Some(session) = &tab.session else {
                continue;
            };
            apply_repo_settings(session, &self.settings.defaults);
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
    /// **A write still running does not hold any of it back.** The close
    /// keeps the session alive to the end of that write, but the session
    /// lets go of what it had drawn as it closes and reads nothing behind
    /// the write — the graph and the refs of a repository this size are
    /// the largest things in the process, and there is no page left to
    /// publish them to (`RepoSession::close`).
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
        // Before the feeds are let go: a write the close let run on
        // answers late, and the next session on this tab attaches to the
        // same feeds — the retired sink is what keeps that answer out of
        // its page.
        if let Some(sink) = tab.sink.take() {
            sink.retire();
        }
        tab.feeds.release_all();
        // The rows a delete took off the screen go back with it, and so
        // do the readings the lists had drawn: the answer that would have
        // put the rows back is this session's, a retired sink is exactly
        // what keeps it out of the next page, and the numbers those
        // readings were stamped with are this session's own count
        // (`ops::StandIn::session_gone`). What the next session reads is
        // the truth either way.
        tab.stand_in.session_gone();
        self.park_writes_of(&session);
        crate::harness::memprobe::forget(id);
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

    /// Keeps hold of a closed session's write loop while it still has a
    /// local write to finish — the write outlives the close on purpose
    /// (`RepoSession::close`), and this handle is how [`Hub::writes_settled`]
    /// and [`Hub::shutdown`] still see it.
    ///
    /// Read after the close on purpose: once the cancel has landed no new
    /// write can start, so a count of zero here means the queue is done —
    /// a loop with nothing left ends on its own and needs no watching.
    ///
    /// **Only the waiting is the hub's.** What that tail means for the
    /// next session on the same repository is not: the working tree holds
    /// the order its writes run in, and the session a reselected tab opens
    /// joins it by the directory git named, not by the tab it was opened
    /// for (`platitude_core::session::write_order`). So a reopen queues
    /// behind this write wherever it was opened from, a second tab on
    /// another repository waits for none of it, and a hub holding no
    /// handle at all would still get the order right.
    fn park_writes_of(&mut self, session: &Arc<RepoSession>) {
        if session.local_writes_pending() == 0 {
            return;
        }
        if let Some(write) = session.take_write_join() {
            self.parked_writes.push(write);
        }
    }

    /// Whether nothing git was asked to write would outlast the window:
    /// no open tab has a local write queued or running, and every write a
    /// closed tab left running has ended. The quit gate reads this — the
    /// window stays until it answers true (`Main.qml`), so the join in
    /// [`Hub::shutdown`] normally has nothing left to wait for.
    pub fn writes_settled(&mut self) -> bool {
        self.parked_writes.retain(|write| !write.is_finished());
        self.parked_writes.is_empty()
            && self.tabs.values().all(|tab| {
                tab.session
                    .as_ref()
                    .is_none_or(|session| session.local_writes_pending() == 0)
            })
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

/// What git said, without the command line this application built around
/// it — the half a screen quotes (デザイン規約: 赤は git の文言だけ).
///
/// The `Display` of a failed command spells the whole invocation out
/// first, which belongs in the command log and not in a dialog. Where git
/// itself said nothing (it never started, or it was killed by the
/// budget), the error's own words are all there is.
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

/// Asks a tab's session something, or answers `None` because there is no
/// session to ask: a tab that has been closed, or one nobody has looked at
/// yet ([`Hub::ensure_open`] is what opens one).
pub fn from_session<R>(tab_id: i32, f: impl FnOnce(&Arc<RepoSession>) -> R) -> Option<R> {
    let session = Hub::with(|hub| hub.session(tab_id))??;
    Some(f(&session))
}

/// The same for telling it to do something, where there being no session
/// means there is nothing to do. What the session answers — the id of a
/// write it accepted — is let go here; a slot that has to hand that id
/// back to the page asks through [`from_session`] instead.
pub fn with_session<R>(tab_id: i32, f: impl FnOnce(&Arc<RepoSession>) -> R) {
    from_session(tab_id, f);
}

/// Moves a tab's delete along and answers with the rows it leaves showing
/// as gone (`ops::StandIn`) — none at all for a tab this hub does not
/// have, which is also the right picture for one.
///
/// **The caller is handed the rows rather than the machine**: the borrow
/// is over before this returns, so the copy a property is read from is
/// never taken while the hub is held (the re-entrant borrow the bridge
/// panics on — 規約 §Qt Bridges の要点).
/// The sidebar section called `section` has **applied** a listing to its
/// rows, stamped as the read that made it was taken
/// (`session::Standing::stamp`).
///
/// **Recorded, not acted on, and only for that section's own rows.**
/// Which write it answers for — if any — is the stand-in's to decide when
/// the page says a list has drawn (`ops::StandIn::look_again`); all this
/// says is what one list is now showing, which is the only thing a
/// section can speak for. The three refs sections are handed the same
/// read and draw it on three separate turns, so each is written down
/// under its own row (`ops::Row::drawn_by`) and a section that stands
/// nothing in is not written down at all.
pub fn listing_applied(tab_id: i32, section: &str, at: u64) {
    let Some(row) = crate::ops::Row::drawn_by(section) else {
        return;
    };
    stand_in(tab_id, |gone| gone.listing_applied(row, at));
}

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

/// Puts the settings in force on one session. Every value the settings file
/// holds passes through here, so a key that is written but never applied
/// cannot go unnoticed.
fn apply_repo_settings(session: &Arc<RepoSession>, applied: &platitude_core::settings::Defaults) {
    session.set_auto_fetch(minutes_to_interval(applied.auto_fetch_minutes));
    session.set_network_timeout(std::time::Duration::from_secs(applied.network_timeout_secs));
    // Restarts the walk when it changes something, and on the way in it
    // changes nothing that has to be walked twice: a session this new has
    // no workdir yet, so the restart returns without a pass and the
    // opening's own `restart_log` is the one that reads this.
    session.set_log_limit(applied.initial_commits);
}
