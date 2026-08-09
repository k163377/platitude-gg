//! Main-thread hub: tab registry, per-tab event feeds and the session sink.
//!
//! Threading model (spike-proven): background tasks push messages into
//! `Feed` queues and wake the owning QML object with a queued `drain`
//! invocation; the object pulls its messages on the Qt main thread. The
//! only method ever invoked through a `QmlMethodInvoker` is `drain` — a
//! single lowercase word, immune to camel-case conversion accidents.

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use platitude_core::details::DiffTarget;
use platitude_core::opstate::OpState;
use platitude_core::parse::diff::FilePatch;
use platitude_core::preview::FilePreview;
use platitude_core::process::{CommandEnd, GitExecutor};
use platitude_core::session::{
    AUTO_FETCH_OP, LogRow, RefLabel, RefsSnapshot, RepoSession, SessionEvent, SessionSink,
};
use platitude_core::settings::{Settings, State, Store};
use platitude_core::stash::StashEntry;
use platitude_core::status::WorkTreeStatus;
use qtbridge::QmlMethodInvoker;

/// What came back about a folder somebody picked, before it is a tab.
#[derive(Debug)]
pub enum PickMsg {
    /// Open it as a tab. Either it can be opened, or the check itself
    /// could not say — and a screen that cannot name what went wrong has
    /// no business inventing a line for it. The tab's own failure screen
    /// and its command log are where git's words already are
    /// (デザイン規約 §git が言ったことを読む場所).
    ///
    /// The path is the one that was picked, not the root git resolved it
    /// to: the tab is opened exactly the way every other road opens one.
    Open { path: PathBuf },
    /// It is no repository this application can show, and which of the
    /// two it is can be said plainly. `bare` picks the wording.
    Rejected {
        path: PathBuf,
        /// The folder to reopen the picker at — the one this sits in,
        /// where the repository the person was after usually is.
        near: String,
        bare: bool,
    },
}

/// Tab-level messages (open lifecycle + background errors + write state).
#[derive(Debug)]
pub enum TabMsg {
    Opened {
        title: String,
        path: String,
    },
    OpenFailed {
        message: String,
    },
    OpError {
        message: String,
    },
    /// A write command started / ended. A failed write also arrives as an
    /// `OpError`, so the existing error surface needs no special case;
    /// `error` is here as well so an editor can tell whether the write it
    /// asked for is the one that failed.
    WriteState {
        op: String,
        running: bool,
        error: String,
    },
    /// A branch move would leave commits unreachable and was not made.
    MoveNeedsAsk {
        local: String,
        start: String,
    },
    /// How much of a range a remote already has (rewrite warning).
    Publish {
        range: String,
        total: i32,
        published: i32,
    },
    /// Whether HEAD can reach a commit — whether a rewrite may start there.
    InHistory {
        oid: String,
        in_history: bool,
    },
    /// Whether anything besides the current branch still reaches its tip,
    /// which is what a rewrite here would cost.
    HeadReach {
        reached_elsewhere: bool,
    },
    /// Whether a remote already carries a branch name, as of now rather
    /// than as of the last fetch. The question rides along: the box that
    /// asked may be on another name by the time this arrives.
    RemoteBranch {
        remote: String,
        branch: String,
        exists: bool,
        /// False when the remote could not be asked at all.
        reached: bool,
    },
    /// Merge tool names the settings field can offer. Empty is an answer.
    /// `settled` false is the fast half, with the slow read still out.
    MergeTools {
        names: Vec<String>,
        settled: bool,
    },
    /// Author identity and signing state.
    Author {
        name: String,
        email: String,
        complete: bool,
        /// `commit.gpgsign`, not `is_active()`: the commit editor is what
        /// says so, and `tag.gpgsign` would make it say it falsely.
        sign_commits: bool,
        signing_format: String,
    },
    /// One commit's signature: the outcome the UI shows (empty for an
    /// unsigned commit), git's own `%G?` letter behind it, and who signed.
    Signature {
        oid: String,
        kind: String,
        code: String,
        signer: String,
    },
    /// Configured remote names — where a branch with no upstream can go.
    Remotes {
        names: Vec<String>,
    },
    /// HEAD's message and author, for prefilling an amend.
    HeadCommit {
        message: String,
        author_name: String,
        author_email: String,
    },
    /// Auto fetch started or ended. Kept off the shared error surface: a
    /// laptop that is simply offline must not raise a fresh banner every
    /// interval, so the toolbar indicator carries this state instead.
    AutoFetch {
        running: bool,
        error: String,
    },
}

/// Graph-model messages (log stream lifecycle).
#[derive(Debug)]
pub enum GraphMsg {
    Started {
        generation: u64,
    },
    Chunk {
        generation: u64,
        rows: Vec<LogRow>,
    },
    Labels {
        generation: u64,
        rows: Vec<(u32, Vec<RefLabel>)>,
    },
    Finished {
        generation: u64,
        total: u32,
        elapsed_ms: u64,
        truncated: bool,
    },
    /// Whole-graph replacement in one message (see
    /// [`SessionEvent::LogReplaced`]): applied in place so no empty
    /// model is ever visible.
    Replaced {
        generation: u64,
        rows: Vec<LogRow>,
        elapsed_ms: u64,
        truncated: bool,
    },
    Failed {
        generation: u64,
        message: String,
    },
}

/// Command-log messages: one git invocation, start and end.
#[derive(Debug)]
pub enum CommandMsg {
    Started {
        id: u64,
        display: String,
        full: String,
        at_ms: i64,
    },
    Finished {
        id: u64,
        /// Exit code, or `None` when git never ran to a code of its own
        /// (killed by the timeout, cancelled, failed to start).
        code: Option<i32>,
        /// Why there is no code, for the row to say instead.
        note: String,
        /// The code is an answer this command was asked for, so the row
        /// is not a failure however it exited.
        answered: bool,
        elapsed_ms: i64,
        message: String,
    },
}

#[derive(Debug)]
pub struct StatusMsg {
    pub status: WorkTreeStatus,
    pub op_state: OpState,
    /// "commit N of M" while a rebase is stepping.
    pub progress: Option<platitude_core::conflict::Progress>,
    /// What the two sides of a conflict are called. Empty names while
    /// nothing is stopped, or where git left nothing to name one by.
    pub sides: platitude_core::conflict::Sides,
    /// The merge tool git would launch. Empty with none configured, or
    /// while nothing is conflicted. Names the menu row; the launch reads
    /// the config again rather than trusting this.
    pub merge_tool: String,
}

#[derive(Debug)]
pub struct DiffMsg {
    pub target: DiffTarget,
    pub patches: Vec<FilePatch>,
    pub preview: Option<FilePreview>,
    /// Fingerprint of the diff's source bytes; selections carry it back
    /// so a partial write can refuse a drifted diff.
    pub fingerprint: u64,
}

/// A queue whose consumer is one QML object on the Qt main thread.
pub struct Feed<T> {
    state: Mutex<FeedState<T>>,
}

struct FeedState<T> {
    queue: VecDeque<T>,
    invoker: Option<QmlMethodInvoker>,
}

impl<T> Default for Feed<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(FeedState {
                queue: VecDeque::new(),
                invoker: None,
            }),
        }
    }
}

impl<T> Feed<T> {
    fn lock(&self) -> MutexGuard<'_, FeedState<T>> {
        match self.state.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// Appends a message and wakes the attached consumer.
    pub fn push(&self, item: T) {
        let guard = {
            let mut s = self.lock();
            s.queue.push_back(item);
            s
        };
        Self::wake(guard);
    }

    /// Replaces any queued messages (snapshot semantics) and wakes.
    pub fn push_replace(&self, item: T) {
        let guard = {
            let mut s = self.lock();
            s.queue.clear();
            s.queue.push_back(item);
            s
        };
        Self::wake(guard);
    }

    fn wake(guard: MutexGuard<'_, FeedState<T>>) {
        // Invoking while holding the lock is fine: it only posts a queued
        // call to the Qt event loop and never runs the slot synchronously.
        if let Some(inv) = &guard.invoker {
            inv.invoke_method("drain");
        }
    }

    /// Registers the consumer (Qt main thread) and wakes it if messages
    /// are already waiting.
    pub fn attach(&self, invoker: QmlMethodInvoker) {
        let mut s = self.lock();
        let has_pending = !s.queue.is_empty();
        if has_pending {
            invoker.invoke_method("drain");
        }
        s.invoker = Some(invoker);
    }

    /// Takes everything queued (called from the consumer's `drain` slot).
    pub fn drain(&self) -> Vec<T> {
        self.lock().queue.drain(..).collect()
    }
}

/// All feeds of one tab. Each feed is independently `Arc`-shared with the
/// QML object that consumes it (one consumer per feed; status fans out to
/// two consumers via a second feed).
#[derive(Default)]
pub struct Feeds {
    pub tab: Arc<Feed<TabMsg>>,
    pub graph: Arc<Feed<GraphMsg>>,
    /// Refs fan out to one feed per sidebar section (one consumer each).
    /// Shared, not copied: each section reads its own part of the same
    /// snapshot, and a deep copy per section duplicates tens of thousands
    /// of strings for nobody.
    pub refs_branches: Arc<Feed<Arc<RefsSnapshot>>>,
    pub refs_remotes: Arc<Feed<Arc<RefsSnapshot>>>,
    pub refs_tags: Arc<Feed<Arc<RefsSnapshot>>>,
    /// Status headline consumer (WorkTreeModel: header props/counts).
    pub status: Arc<Feed<StatusMsg>>,
    /// Status list consumer (the `worktree` NavSectionModel — the changed
    /// files the right pane's WIP view lists).
    pub status_nav: Arc<Feed<StatusMsg>>,
    pub stash: Arc<Feed<Vec<StashEntry>>>,
    pub worktrees: Arc<Feed<Vec<platitude_core::worktrees::WorktreeEntry>>>,
    pub details: Arc<Feed<platitude_core::details::CommitDetails>>,
    pub diff: Arc<Feed<DiffMsg>>,
    pub commands: Arc<Feed<CommandMsg>>,
}

/// Routes core session events into the per-tab feeds. Runs on background
/// tokio threads; must never block beyond the short feed locks.
struct BridgeSink {
    feeds: Arc<Feeds>,
}

impl SessionSink for BridgeSink {
    fn event(&self, event: SessionEvent) {
        match event {
            SessionEvent::Opened { info } => {
                let title = info
                    .workdir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| info.workdir.to_string_lossy().into_owned());
                self.feeds.tab.push(TabMsg::Opened {
                    title,
                    path: info.workdir.to_string_lossy().into_owned(),
                });
            }
            SessionEvent::OpenFailed { error } => self.feeds.tab.push(TabMsg::OpenFailed {
                message: error.to_string(),
            }),
            SessionEvent::LogStarted { generation } => {
                self.feeds.graph.push(GraphMsg::Started { generation });
            }
            SessionEvent::LogChunk { generation, rows } => {
                self.feeds.graph.push(GraphMsg::Chunk { generation, rows });
            }
            SessionEvent::LogFinished {
                generation,
                total,
                elapsed_ms,
                truncated,
            } => self.feeds.graph.push(GraphMsg::Finished {
                generation,
                total,
                elapsed_ms,
                truncated,
            }),
            SessionEvent::LogFailed { generation, error } => {
                self.feeds.graph.push(GraphMsg::Failed {
                    generation,
                    message: error,
                })
            }
            SessionEvent::LogReplaced {
                generation,
                rows,
                elapsed_ms,
                truncated,
            } => self.feeds.graph.push(GraphMsg::Replaced {
                generation,
                rows,
                elapsed_ms,
                truncated,
            }),
            SessionEvent::LabelsChanged { generation, rows } => {
                self.feeds.graph.push(GraphMsg::Labels { generation, rows });
            }
            SessionEvent::RefsLoaded { snapshot } => {
                self.feeds.tab.push(TabMsg::Remotes {
                    names: snapshot.remote_names.clone(),
                });
                self.feeds.refs_branches.push_replace(Arc::clone(&snapshot));
                self.feeds.refs_remotes.push_replace(Arc::clone(&snapshot));
                self.feeds.refs_tags.push_replace(snapshot);
            }
            SessionEvent::StatusLoaded {
                status,
                op_state,
                progress,
                sides,
                merge_tool,
            } => {
                self.feeds.status_nav.push_replace(StatusMsg {
                    status: status.clone(),
                    op_state,
                    progress,
                    sides: sides.clone(),
                    merge_tool: merge_tool.clone(),
                });
                self.feeds.status.push_replace(StatusMsg {
                    status,
                    op_state,
                    progress,
                    sides,
                    merge_tool,
                });
            }
            SessionEvent::StashesLoaded { stashes } => self.feeds.stash.push_replace(stashes),
            SessionEvent::WorktreesLoaded { worktrees } => {
                self.feeds.worktrees.push_replace(worktrees)
            }
            SessionEvent::DetailsLoaded { details } => self.feeds.details.push_replace(details),
            SessionEvent::DiffLoaded {
                target,
                patches,
                preview,
                fingerprint,
            } => {
                self.feeds.diff.push_replace(DiffMsg {
                    target,
                    patches,
                    preview,
                    fingerprint,
                });
            }
            SessionEvent::CommandStarted {
                id,
                display,
                full,
                at_ms,
            } => self.feeds.commands.push(CommandMsg::Started {
                id,
                display,
                full,
                at_ms,
            }),
            SessionEvent::CommandFinished {
                id,
                end,
                elapsed_ms,
                message,
            } => {
                let (code, note) = match end {
                    CommandEnd::Exited(code) | CommandEnd::Answered(code) => {
                        (Some(code), String::new())
                    }
                    CommandEnd::TimedOut => (None, "timed out".to_string()),
                    CommandEnd::Cancelled => (None, "cancelled".to_string()),
                    CommandEnd::Failed => (None, "did not run".to_string()),
                };
                self.feeds.commands.push(CommandMsg::Finished {
                    id,
                    code,
                    note,
                    // An answer by exit code is not a failure, whatever
                    // the code says.
                    answered: matches!(end, CommandEnd::Answered(_)),
                    elapsed_ms: elapsed_ms as i64,
                    message,
                });
            }
            SessionEvent::OpFailed { op, error } => self.feeds.tab.push(TabMsg::OpError {
                message: format!("{op}: {error}"),
            }),
            SessionEvent::MoveNeedsAsk { local, start } => {
                self.feeds.tab.push(TabMsg::MoveNeedsAsk { local, start })
            }
            SessionEvent::AuthorLoaded { config } => {
                self.feeds.tab.push(TabMsg::Author {
                    complete: config.identity.is_complete(),
                    name: config.identity.name.unwrap_or_default(),
                    email: config.identity.email.unwrap_or_default(),
                    sign_commits: config.signing.sign_commits,
                    signing_format: config.signing.format.as_str().to_string(),
                });
            }
            SessionEvent::RemoteBranchChecked {
                remote,
                branch,
                exists,
                reached,
            } => {
                self.feeds.tab.push(TabMsg::RemoteBranch {
                    remote,
                    branch,
                    exists,
                    reached,
                });
            }
            SessionEvent::SignatureChecked { oid, signature } => {
                use platitude_core::identity::SignatureStatus;
                // Eight verdicts, three outcomes: only `G` may read as
                // verified (`SignatureStatus::is_trusted`), `B` is the one
                // that says the content moved, and everything else in
                // between is a signature nobody here can judge. The letter
                // rides along so the tooltip can say which one it was.
                self.feeds.tab.push(TabMsg::Signature {
                    oid,
                    kind: match signature.status {
                        SignatureStatus::Absent => "",
                        SignatureStatus::Good => "verified",
                        SignatureStatus::Bad => "bad",
                        _ => "signed",
                    }
                    .to_string(),
                    code: signature.status.code().to_string(),
                    signer: signature.signer,
                });
            }
            SessionEvent::PublishChecked { range, state } => {
                self.feeds.tab.push(TabMsg::Publish {
                    range,
                    total: state.total as i32,
                    published: state.published() as i32,
                });
            }
            SessionEvent::InHistoryChecked { oid, in_history } => {
                self.feeds.tab.push(TabMsg::InHistory { oid, in_history });
            }
            SessionEvent::HeadReachChecked { reached_elsewhere } => {
                self.feeds.tab.push(TabMsg::HeadReach { reached_elsewhere });
            }
            SessionEvent::MergeToolsLoaded { names, settled } => {
                self.feeds.tab.push(TabMsg::MergeTools { names, settled });
            }
            SessionEvent::HeadCommitLoaded { head } => {
                self.feeds.tab.push(TabMsg::HeadCommit {
                    message: head.message,
                    author_name: head.author_name,
                    author_email: head.author_email,
                });
            }
            SessionEvent::WriteStarted { op } if op == AUTO_FETCH_OP => {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    running: true,
                    error: String::new(),
                });
            }
            SessionEvent::WriteFinished { op, error } if op == AUTO_FETCH_OP => {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    running: false,
                    error: error.unwrap_or_default(),
                });
            }
            SessionEvent::WriteStarted { op } => self.feeds.tab.push(TabMsg::WriteState {
                op: op.to_string(),
                running: true,
                error: String::new(),
            }),
            SessionEvent::WriteFinished { op, error } => {
                if let Some(message) = &error {
                    self.feeds.tab.push(TabMsg::OpError {
                        message: format!("{op}: {message}"),
                    });
                }
                self.feeds.tab.push(TabMsg::WriteState {
                    op: op.to_string(),
                    running: false,
                    error: error.unwrap_or_default(),
                });
            }
        }
    }
}

struct Tab {
    /// `None` until the tab is first looked at. Restoring a window full of
    /// tabs must not spend a `RepoSession` — and the git it spawns — on
    /// repositories nobody has asked to see yet.
    session: Option<Arc<RepoSession>>,
    /// Kept after opening too: it is the name a per-repository setting is
    /// filed under, so a change to the defaults can be re-resolved against
    /// every tab without asking the UI where they point.
    path: PathBuf,
    feeds: Arc<Feeds>,
}

/// Application-wide state living on the Qt main thread.
pub struct Hub {
    runtime: Option<tokio::runtime::Runtime>,
    executor: GitExecutor,
    tabs: HashMap<i32, Tab>,
    next_tab_id: i32,
    store: Store,
    settings: Settings,
    /// What the window looks like now, and what is already on disk. The
    /// flush compares the two rather than trusting a dirty flag: the UI
    /// reports the whole layout on a timer, so most reports say nothing
    /// new and a flag would be set by all of them.
    state: State,
    saved_state: State,
    /// Empty unless another process is already using the files this one
    /// would have used; then it names their directory. The window has
    /// nothing else to say which of two builds is in the way, and the
    /// store this hub holds is an empty one by then.
    held_elsewhere: String,
}

thread_local! {
    static HUB: RefCell<Option<Hub>> = const { RefCell::new(None) };
}

/// Addresses that have a picture, and where it is. Held by whoever is
/// building rows so the hub is borrowed once for a whole pass rather than
/// once per commit — the graph draws two thousand of those at a time.
#[derive(Debug, Clone, Default)]
pub struct AvatarUrls {
    by_email: HashMap<String, String>,
}

impl AvatarUrls {
    /// The current assignments. Empty where there is nowhere to keep
    /// pictures, which is the same answer as nobody having one.
    pub fn current() -> Self {
        Hub::with(|hub| hub.avatar_urls()).unwrap_or_default()
    }

    /// The URL for an address, or empty. The address is expected to have
    /// come through `avatar::key` already — the log parser does that on
    /// the way in, which is the only way rows reach here.
    pub fn url_of(&self, email: &str) -> String {
        self.by_email.get(email).cloned().unwrap_or_default()
    }
}

impl Hub {
    /// Installs the hub into the main thread. Call once before `QApp::run`.
    ///
    /// The store comes from `main`, which is where the question "may this
    /// process use these files at all" is answered — a run that was turned
    /// away is handed an empty store here and `held_elsewhere` names the
    /// directory it did not get.
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
        // Once, at the start: a person who emptied the avatars directory
        // by hand has said what they meant, and an assignment pointing at
        // nothing would draw a row in the settings list that never fills.
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
            // The window is already gone, so this is the last chance to
            // keep whatever the timer had not reached yet.
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
    /// The picker's answer goes through here first so a folder that is no
    /// repository never becomes a tab: there would be nothing in it to
    /// read, and the path would go on being remembered across restarts.
    /// Every other way in (a restored tab, a worktree row, `PG_AUTO_OPEN`)
    /// still opens straight away — those are not somebody choosing a
    /// folder, and the page's own failure screen is the right place for
    /// them (デザイン規約 §可否・警告の出し場所).
    pub fn probe_repo(&self, path: PathBuf, feed: Arc<Feed<PickMsg>>) -> bool {
        let Some(handle) = self.runtime_handle() else {
            return false;
        };
        let executor = self.executor();
        handle.spawn(async move {
            let cancel = tokio_util::sync::CancellationToken::new();
            let msg = match platitude_core::repo::open(&executor, &path, &cancel).await {
                Ok(_) => PickMsg::Open { path },
                Err(platitude_core::GitError::NotARepository { bare, .. }) => PickMsg::Rejected {
                    near: crate::urlpath::picker_folder_url(&path),
                    path,
                    bare,
                },
                // git having trouble of its own rather than an answer
                // about the folder: a timeout on a share that stopped
                // answering, or output from `rev-parse` nothing can read.
                // Not something to word a screen around — it opens, and
                // the tab says what git said.
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "could not tell whether this folder is a repository; opening it"
                    );
                    PickMsg::Open { path }
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
        let feeds = Arc::clone(&tab.feeds);
        let applied = self.settings.for_repo(&path.to_string_lossy());
        let sink = Arc::new(BridgeSink { feeds });
        let session = RepoSession::open(executor, handle, path, sink);
        apply_repo_settings(&session, &applied);
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
    /// setting of its own keeps it: the defaults moving is not an
    /// instruction about the ones that were singled out.
    fn reapply_settings(&self) {
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

    /// Closes a tab and cancels its session.
    pub fn close_tab(&mut self, id: i32) {
        if let Some(tab) = self.tabs.remove(&id) {
            if let Some(session) = tab.session {
                session.close();
            }
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

    // -- settings and state -------------------------------------------------

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// The directory another process is holding, empty when this one got
    /// what it asked for.
    pub fn held_elsewhere(&self) -> &str {
        &self.held_elsewhere
    }

    /// Records the auto-fetch interval and puts it in force. Written out at
    /// once rather than on the state timer: this is a decision somebody
    /// made in a dialog, not a size that is still moving.
    pub fn set_auto_fetch_minutes(&mut self, minutes: u32) {
        self.settings.defaults.auto_fetch_minutes = minutes;
        self.reapply_settings();
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
    }

    // -- avatars ------------------------------------------------------------

    /// A `file:` URL for the picture assigned to an address, or empty.
    ///
    /// Empty is the ordinary answer — most authors have no picture, and the
    /// generated identicon is what draws then. Empty is also the answer
    /// wherever there is nowhere to keep pictures (a screenshot run), so
    /// nothing downstream has to know that case exists.
    pub fn avatar_url(&self, email: &str) -> String {
        let (Some(dir), Some(file)) = (
            self.store.avatars_dir(),
            self.settings.avatars.file_of(email),
        ) else {
            return String::new();
        };
        crate::urlpath::file_url(&dir.join(file))
    }

    pub fn avatars(&self) -> &platitude_core::avatar::Avatars {
        &self.settings.avatars
    }

    /// Every assignment resolved to a URL in one go, so a pass over the
    /// graph asks the hub once instead of once per row. Cheap to build —
    /// this is a handful of entries, not one per commit.
    pub fn avatar_urls(&self) -> AvatarUrls {
        let Some(dir) = self.store.avatars_dir() else {
            return AvatarUrls::default();
        };
        AvatarUrls {
            by_email: self
                .settings
                .avatars
                .list()
                .iter()
                .map(|entry| {
                    (
                        entry.email.clone(),
                        crate::urlpath::file_url(&dir.join(&entry.file)),
                    )
                })
                .collect(),
        }
    }

    /// Files a picture against an address and writes the settings out.
    /// Reports git-style: the error is the message, empty means it worked.
    pub fn assign_avatar(&mut self, email: &str, name: &str, source: &std::path::Path) -> String {
        let Some(dir) = self.store.avatars_dir() else {
            return platitude_core::avatar::AvatarError::NoStore.to_string();
        };
        if let Err(error) = self.settings.avatars.assign(&dir, email, name, source) {
            return error.to_string();
        }
        self.save_settings_now();
        String::new()
    }

    /// Takes the picture off an address. The person's own file is untouched.
    pub fn remove_avatar(&mut self, email: &str) {
        let Some(dir) = self.store.avatars_dir() else {
            return;
        };
        if self.settings.avatars.remove(&dir, email) {
            self.save_settings_now();
        }
    }

    /// Drops assignments whose image is no longer there, so a directory
    /// emptied by hand is believed rather than argued with. Runs once at
    /// startup; nothing else deletes those files behind the application's
    /// back often enough to be worth watching for.
    pub fn forget_missing_avatars(&mut self) {
        let Some(dir) = self.store.avatars_dir() else {
            return;
        };
        let gone = self.settings.avatars.forget_missing(&dir);
        if gone > 0 {
            tracing::info!(gone, "avatars whose image is missing");
            self.save_settings_now();
        }
    }

    fn save_settings_now(&mut self) {
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
    }

    pub fn set_window_state(&mut self, window: platitude_core::settings::WindowState) {
        self.state.window = window;
    }

    pub fn set_layout_state(&mut self, layout: platitude_core::settings::LayoutState) {
        self.state.layout = layout;
    }

    pub fn set_tabs_state(&mut self, tabs: platitude_core::settings::TabsState) {
        self.state.tabs = tabs;
    }

    /// Writes the state out if it has moved since the last write.
    pub fn flush_state(&mut self) {
        if self.state == self.saved_state {
            return;
        }
        match self.store.save_state(&self.state) {
            Ok(()) => self.saved_state = self.state.clone(),
            // Left unsaved on purpose: the next tick tries again, and until
            // one succeeds the file on disk is still the last good one.
            Err(error) => tracing::warn!(%error, "state not saved"),
        }
    }
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
