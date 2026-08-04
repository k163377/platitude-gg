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
use platitude_core::process::GitExecutor;
use platitude_core::session::{
    AUTO_FETCH_OP, LogRow, RefLabel, RefsSnapshot, RepoSession, SessionEvent, SessionSink,
};
use platitude_core::stash::StashEntry;
use platitude_core::status::WorkTreeStatus;
use qtbridge::QmlMethodInvoker;

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
    /// A move was refused: uncommitted work stands in the way and nothing
    /// happened. `kind` says which way out is on offer — "changes" can be
    /// merged across, "untracked" can only be stashed aside.
    MoveBlocked {
        kind: String,
    },
    /// How much of a range a remote already has (rewrite warning).
    Publish {
        range: String,
        total: i32,
        published: i32,
    },
    /// Author identity and signing state.
    Author {
        name: String,
        email: String,
        complete: bool,
        signing: bool,
        signing_format: String,
    },
    /// Configured remote names — where a branch with no upstream can go.
    Remotes {
        names: Vec<String>,
    },
    /// HEAD's message, for prefilling an amend.
    HeadMessage {
        message: String,
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

#[derive(Debug)]
pub struct StatusMsg {
    pub status: WorkTreeStatus,
    pub op_state: OpState,
    /// "commit N of M" while a rebase is stepping.
    pub progress: Option<platitude_core::conflict::Progress>,
}

#[derive(Debug)]
pub struct DiffMsg {
    pub target: DiffTarget,
    pub patches: Vec<FilePatch>,
    pub preview: Option<FilePreview>,
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
    pub refs_branches: Arc<Feed<RefsSnapshot>>,
    pub refs_remotes: Arc<Feed<RefsSnapshot>>,
    pub refs_tags: Arc<Feed<RefsSnapshot>>,
    /// Status headline consumer (WorkTreeModel: header props/counts).
    pub status: Arc<Feed<StatusMsg>>,
    /// Status list consumer (working-tree sidebar section).
    pub status_nav: Arc<Feed<StatusMsg>>,
    pub stash: Arc<Feed<Vec<StashEntry>>>,
    pub worktrees: Arc<Feed<Vec<platitude_core::worktrees::WorktreeEntry>>>,
    pub details: Arc<Feed<platitude_core::details::CommitDetails>>,
    pub diff: Arc<Feed<DiffMsg>>,
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
            SessionEvent::LabelsChanged { rows } => {
                self.feeds.graph.push(GraphMsg::Labels { rows });
            }
            SessionEvent::RefsLoaded { snapshot } => {
                self.feeds.tab.push(TabMsg::Remotes {
                    names: snapshot.remote_names.clone(),
                });
                self.feeds.refs_branches.push_replace(snapshot.clone());
                self.feeds.refs_remotes.push_replace(snapshot.clone());
                self.feeds.refs_tags.push_replace(snapshot);
            }
            SessionEvent::StatusLoaded {
                status,
                op_state,
                progress,
            } => {
                self.feeds.status_nav.push_replace(StatusMsg {
                    status: status.clone(),
                    op_state,
                    progress,
                });
                self.feeds.status.push_replace(StatusMsg {
                    status,
                    op_state,
                    progress,
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
            } => {
                self.feeds.diff.push_replace(DiffMsg {
                    target,
                    patches,
                    preview,
                });
            }
            SessionEvent::OpFailed { op, error } => self.feeds.tab.push(TabMsg::OpError {
                message: format!("{op}: {error}"),
            }),
            SessionEvent::MoveBlocked { block } => {
                use platitude_core::branch::CheckoutBlock;
                self.feeds.tab.push(TabMsg::MoveBlocked {
                    kind: match block {
                        CheckoutBlock::LocalChanges => "changes",
                        CheckoutBlock::UntrackedFiles => "untracked",
                    }
                    .to_string(),
                });
            }
            SessionEvent::AuthorLoaded { config } => {
                self.feeds.tab.push(TabMsg::Author {
                    complete: config.identity.is_complete(),
                    name: config.identity.name.unwrap_or_default(),
                    email: config.identity.email.unwrap_or_default(),
                    signing: config.signing.is_active(),
                    signing_format: config.signing.format.as_str().to_string(),
                });
            }
            SessionEvent::PublishChecked { range, state } => {
                self.feeds.tab.push(TabMsg::Publish {
                    range,
                    total: state.total as i32,
                    published: state.published() as i32,
                });
            }
            SessionEvent::HeadMessageLoaded { message } => {
                self.feeds.tab.push(TabMsg::HeadMessage { message });
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
    session: Arc<RepoSession>,
    feeds: Arc<Feeds>,
}

/// Application-wide state living on the Qt main thread.
pub struct Hub {
    runtime: Option<tokio::runtime::Runtime>,
    executor: GitExecutor,
    tabs: HashMap<i32, Tab>,
    next_tab_id: i32,
    /// Auto-fetch interval, applied to every session including tabs opened
    /// later. `None` is off. Application-wide because the answer is about
    /// how often this computer should talk to remotes at all.
    auto_fetch: Option<std::time::Duration>,
}

thread_local! {
    static HUB: RefCell<Option<Hub>> = const { RefCell::new(None) };
}

impl Hub {
    /// Installs the hub into the main thread. Call once before `QApp::run`.
    pub fn install(runtime: tokio::runtime::Runtime) {
        HUB.with(|h| {
            *h.borrow_mut() = Some(Hub {
                runtime: Some(runtime),
                executor: GitExecutor::new(),
                tabs: HashMap::new(),
                next_tab_id: 0,
                auto_fetch: Some(std::time::Duration::from_secs(
                    u64::from(platitude_core::session::AUTO_FETCH_DEFAULT_MINUTES) * 60,
                )),
            });
        });
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
            for (_, tab) in hub.tabs.drain() {
                tab.session.close();
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

    /// Opens a repository in a new tab; returns the tab id.
    pub fn open_tab(&mut self, path: PathBuf) -> Option<i32> {
        let handle = self.runtime_handle()?;
        self.next_tab_id += 1;
        let id = self.next_tab_id;
        let feeds = Arc::new(Feeds::default());
        let sink = Arc::new(BridgeSink {
            feeds: Arc::clone(&feeds),
        });
        let session = RepoSession::open(self.executor.clone(), handle, path, sink);
        session.set_auto_fetch(self.auto_fetch);
        self.tabs.insert(id, Tab { session, feeds });
        tracing::info!(tab = id, "opened repository tab");
        Some(id)
    }

    /// Changes the auto-fetch interval everywhere at once.
    pub fn set_auto_fetch(&mut self, interval: Option<std::time::Duration>) {
        self.auto_fetch = interval;
        for tab in self.tabs.values() {
            tab.session.set_auto_fetch(interval);
        }
    }

    /// Closes a tab and cancels its session.
    pub fn close_tab(&mut self, id: i32) {
        if let Some(tab) = self.tabs.remove(&id) {
            tab.session.close();
            tracing::info!(tab = id, "closed repository tab");
        }
    }

    /// Re-reads the author configuration of every open tab. Each session
    /// caches what it read when the repository opened, so a write made
    /// outside them (the app-level identity screen) leaves them stale.
    pub fn refresh_authors(&self) {
        for tab in self.tabs.values() {
            tab.session.refresh_author();
        }
    }

    pub fn session(&self, id: i32) -> Option<Arc<RepoSession>> {
        self.tabs.get(&id).map(|t| Arc::clone(&t.session))
    }

    pub fn feeds(&self, id: i32) -> Option<Arc<Feeds>> {
        self.tabs.get(&id).map(|t| Arc::clone(&t.feeds))
    }
}
