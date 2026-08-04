//! RepoSession: one open repository = one session (実装計画 §2.3).
//!
//! Owns all git activity for a repository: the streaming log → graph
//! pipeline, parallel snapshot refreshes (refs / status / stash) and
//! on-demand queries (details, diffs). Everything runs on a tokio runtime;
//! results are pushed to the UI through a [`SessionSink`], which must be
//! cheap and non-blocking (the app bridge posts queued invocations to the
//! Qt main thread).
//!
//! Reads run concurrently; writes go through a single queue so two commands
//! can never touch one repository's index or refs at the same time
//! (実装計画 §2.3). A queue rather than a lock, because order is part of
//! the contract: "stage this, now commit" must not run the other way round,
//! and independently spawned tasks racing for a mutex give no such
//! guarantee. Every write refreshes afterwards — including a failed one,
//! because a command that stops halfway (a conflicted merge, an interrupted
//! rebase) has still changed the repository.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio_util::sync::CancellationToken;

use crate::branch::{self, CheckoutTarget};
use crate::commit::{self, CommitOptions};
use crate::conflict;
use crate::details::{self, CommitDetails, DiffTarget};
use crate::error::GitError;
use crate::graph::{GraphBuilder, Segment};
use crate::identity;
use crate::integrate;
use crate::model::CommitMeta;
use crate::oid::Oid;
use crate::opstate::{self, OpState};
use crate::parse::diff::FilePatch;
use crate::parse::log::{LOG_FORMAT_ARG, LogParser};
use crate::patch::HunkSelect;
use crate::preview::{self, FilePreview};
use crate::process::{GitCommand, GitExecutor};
use crate::publish;
use crate::refs::{self, HeadState, RefEntry, RefKind};
use crate::remote;
use crate::repo::{self, RepoInfo};
use crate::sequencer;
use crate::stage;
use crate::stash::{self, StashEntry};
use crate::status::{self, WorkTreeStatus};

/// First chunk is small so the first paint happens as early as possible.
const FIRST_CHUNK_ROWS: usize = 512;
const CHUNK_ROWS: usize = 4096;

/// Op name of the interval-driven fetch. The UI keeps this one off the
/// shared error surface, so both sides have to agree on the spelling.
pub const AUTO_FETCH_OP: &str = "auto-fetch";

/// Longest auto-fetch interval the UI offers, in minutes. Beyond an hour
/// the point of an automatic fetch is gone; use the manual one.
pub const AUTO_FETCH_MAX_MINUTES: u32 = 60;

/// Interval auto fetch starts at when nothing says otherwise.
pub const AUTO_FETCH_DEFAULT_MINUTES: u32 = 1;

/// Default cap on the graph window (GitKraken-like initial view). Bounds
/// memory and stream time on 100k+ commit repositories; the UI shows a
/// truncation hint when the cap is hit.
pub const DEFAULT_LOG_LIMIT: u32 = 2000;

/// What the log stream walks.
///
/// Tags are shown by default (product decision). On tag-heavy repositories
/// they dominate the `--topo-order` frontier setup (JetBrains/kotlin: 44k
/// tags cost ~1.7s extra before the first byte even with a commit-graph),
/// which is why the toggle exists.
#[derive(Debug, Clone, Copy)]
pub struct LogOptions {
    pub include_tags: bool,
    /// `None` walks the full history.
    pub limit: Option<u32>,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            include_tags: true,
            limit: Some(DEFAULT_LOG_LIMIT),
        }
    }
}

/// Kind of a row label chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LabelKind {
    /// Detached-HEAD marker (synthetic `HEAD` chip).
    Head,
    LocalBranch,
    RemoteBranch,
    Tag,
}

/// One label chip on a graph row (branch / tag / HEAD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLabel {
    pub text: String,
    pub kind: LabelKind,
    /// Badge state for local branches (local-only vs has-remote); the PR
    /// dimension is wired in Phase 4.
    pub has_remote: bool,
    /// True when HEAD is on this branch (bold chip).
    pub is_head: bool,
}

/// Display-ready row of the commit graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRow {
    pub row: u32,
    pub oid_hex: String,
    pub short_sha: String,
    pub author: String,
    /// Author time (unix seconds); formatting is presentation.
    pub time: i64,
    pub subject: String,
    pub node_lane: u16,
    pub node_color: u8,
    pub width: u16,
    pub segments: Vec<Segment>,
    pub labels: Vec<RefLabel>,
    /// Reflog selector (`stash@{n}`) when this row is a stash; empty for
    /// ordinary commits and the WIP row.
    pub stash_ref: String,
}

/// Sidebar-ready refs snapshot (sorted).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefsSnapshot {
    pub locals: Vec<BranchItem>,
    pub remotes: Vec<BranchItem>,
    pub tags: Vec<TagItem>,
    pub head: Option<HeadState>,
    /// Names of the configured remotes, sorted. A branch with no upstream
    /// has to be told where to go, and this is the list to offer.
    pub remote_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchItem {
    pub short: String,
    pub full: String,
    pub oid_hex: String,
    pub has_remote: bool,
    pub is_head: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagItem {
    pub short: String,
    /// Peeled commit id (what the graph row is keyed on).
    pub oid_hex: String,
    pub annotated: bool,
    /// Creator date (unix seconds); the sidebar sorts tags newest-first.
    pub created_unix: i64,
}

/// Everything the session can tell the UI.
#[derive(Debug)]
pub enum SessionEvent {
    Opened {
        info: RepoInfo,
    },
    OpenFailed {
        error: GitError,
    },
    /// A (re)load of the graph began; the model must reset.
    LogStarted {
        generation: u64,
    },
    LogChunk {
        generation: u64,
        rows: Vec<LogRow>,
    },
    LogFinished {
        generation: u64,
        total: u32,
        elapsed_ms: u64,
        /// True when the stream stopped at the configured window limit
        /// (older history exists but is not shown).
        truncated: bool,
    },
    LogFailed {
        generation: u64,
        error: String,
    },
    /// A background rebuild finished and replaces the whole graph in one
    /// step. Deliberately one event rather than Started/Chunk/Finished:
    /// those travel as separate queued messages, and a consumer that
    /// drains between them paints an empty model for a frame — visible
    /// as a white flash whenever a background refresh finds changes.
    LogReplaced {
        generation: u64,
        rows: Vec<LogRow>,
        elapsed_ms: u64,
        truncated: bool,
    },
    /// Labels of already-delivered rows changed (refs arrived/refreshed).
    LabelsChanged {
        rows: Vec<(u32, Vec<RefLabel>)>,
    },
    RefsLoaded {
        snapshot: RefsSnapshot,
    },
    StatusLoaded {
        status: WorkTreeStatus,
        op_state: OpState,
        /// "commit N of M" while a rebase is stepping through commits.
        progress: Option<conflict::Progress>,
    },
    /// Answer to [`RepoSession::check_publish`].
    PublishChecked {
        range: String,
        state: publish::PublishState,
    },
    /// Answer to [`RepoSession::check_in_history`].
    InHistoryChecked {
        oid: String,
        in_history: bool,
    },
    /// Answer to [`RepoSession::load_head_message`] — what an amend starts
    /// from. Empty on an unborn branch.
    HeadMessageLoaded {
        message: String,
    },
    /// Author identity and signing configuration. Emitted on open so the
    /// UI can ask for an identity before the first commit fails.
    AuthorLoaded {
        config: identity::AuthorConfig,
    },
    StashesLoaded {
        stashes: Vec<StashEntry>,
    },
    WorktreesLoaded {
        worktrees: Vec<crate::worktrees::WorktreeEntry>,
    },
    DetailsLoaded {
        details: CommitDetails,
    },
    DiffLoaded {
        target: DiffTarget,
        patches: Vec<FilePatch>,
        /// Image bytes / binary sizes when the text diff is not the whole
        /// story (`None` for ordinary text files).
        preview: Option<FilePreview>,
    },
    /// A background refresh/query failed (op is a stable identifier).
    OpFailed {
        op: &'static str,
        error: GitError,
    },
    /// A move was refused because uncommitted work stands in the way.
    /// Nothing changed; the UI asks how to get past it (デザイン規約
    /// §未コミット変更がある状態での移動).
    MoveBlocked {
        block: branch::CheckoutBlock,
    },
    /// A write operation started; the UI can show it as in flight.
    WriteStarted {
        op: &'static str,
    },
    /// A write operation ended. `error` carries git's own message.
    WriteFinished {
        op: &'static str,
        error: Option<String>,
    },
}

/// What a write invalidates once it succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterWrite {
    /// Working tree / index / stash only.
    Snapshots,
    /// History or refs moved, so the graph has to be rebuilt too.
    Graph,
    /// Snapshots plus the author configuration (an identity write).
    Author,
}

/// Receives session events; implementations must be non-blocking.
pub trait SessionSink: Send + Sync + 'static {
    fn event(&self, event: SessionEvent);
}

/// A queued write: what to run, what it invalidates, what to call it.
struct WriteRequest {
    op: &'static str,
    after: AfterWrite,
    #[expect(
        clippy::type_complexity,
        reason = "a boxed async job needs its shape spelled out"
    )]
    run: Box<
        dyn FnOnce(
                GitExecutor,
                RepoInfo,
                CancellationToken,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), GitError>> + Send>,
            > + Send,
    >,
}

/// Shared mutable state between the log task and refs joins.
#[derive(Default)]
struct Shared {
    builder: GraphBuilder,
    /// Label chips per commit id, derived from the last refs snapshot.
    label_map: HashMap<Oid, Vec<RefLabel>>,
    /// Labels currently shown per row (for diffing on refs refresh).
    applied: HashMap<u32, Vec<RefLabel>>,
    /// Rows exactly as delivered to the UI (labels included), kept so a
    /// background rebuild can tell "same picture" from "changed" and skip
    /// the swap entirely — an unchanged repository must not repaint.
    sent_rows: Vec<LogRow>,
}

/// Guards snapshot-replacing ops against out-of-order completion.
#[derive(Default)]
struct OpGate(AtomicU64);

impl OpGate {
    fn begin(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
    fn is_current(&self, generation: u64) -> bool {
        self.0.load(Ordering::SeqCst) == generation
    }
}

pub struct RepoSession {
    executor: GitExecutor,
    runtime: tokio::runtime::Handle,
    sink: Arc<dyn SessionSink>,
    /// Cancelled when the session closes; all ops derive from it.
    root_cancel: CancellationToken,
    info: Mutex<Option<RepoInfo>>,
    shared: Arc<Mutex<Shared>>,
    log_options: Mutex<LogOptions>,
    log_gen: AtomicU64,
    log_cancel: Mutex<Option<CancellationToken>>,
    /// Dirty working tree → the log stream prepends a synthetic WIP row.
    wip_dirty: std::sync::atomic::AtomicBool,
    /// Submission end of the write queue (see the module docs).
    write_tx: tokio::sync::mpsc::UnboundedSender<WriteRequest>,
    /// Time budget for fetch / push (settings, Phase 4, persist this).
    network_timeout: Mutex<std::time::Duration>,
    /// Cancels the running auto-fetch timer, if any.
    auto_fetch: Mutex<Option<CancellationToken>>,
    /// One permit: an auto fetch that is still queued or running holds it,
    /// so a tick that arrives meanwhile is skipped instead of stacking up.
    /// A permit moved into a dropped request is released with it.
    auto_fetch_slot: Arc<tokio::sync::Semaphore>,
    refs_gate: OpGate,
    status_gate: OpGate,
    stash_gate: OpGate,
    worktrees_gate: OpGate,
}

impl RepoSession {
    /// Creates the session and starts opening `path` in the background.
    /// On success everything loads: log stream, refs, status, stashes.
    pub fn open(
        executor: GitExecutor,
        runtime: tokio::runtime::Handle,
        path: PathBuf,
        sink: Arc<dyn SessionSink>,
    ) -> Arc<Self> {
        let (write_tx, write_rx) = tokio::sync::mpsc::unbounded_channel();
        let session = Arc::new(Self {
            executor,
            runtime: runtime.clone(),
            sink,
            root_cancel: CancellationToken::new(),
            info: Mutex::new(None),
            shared: Arc::new(Mutex::new(Shared::default())),
            log_options: Mutex::new(LogOptions::default()),
            log_gen: AtomicU64::new(0),
            log_cancel: Mutex::new(None),
            wip_dirty: std::sync::atomic::AtomicBool::new(false),
            write_tx,
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            auto_fetch: Mutex::new(None),
            auto_fetch_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            refs_gate: OpGate::default(),
            status_gate: OpGate::default(),
            stash_gate: OpGate::default(),
            worktrees_gate: OpGate::default(),
        });
        runtime.spawn(Arc::clone(&session).write_loop(write_rx));

        let s = Arc::clone(&session);
        runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match repo::open(&s.executor, &path, &cancel).await {
                Ok(info) => {
                    s.set_info(info.clone());
                    s.sink.event(SessionEvent::Opened { info });
                    // Before anything else: a missing identity turns the
                    // first commit into a wall of git text, and the UI can
                    // ask for one instead.
                    s.refresh_author();
                    s.restart_log();
                    s.refresh_quick();
                }
                Err(error) => {
                    if !error.is_cancelled() {
                        s.sink.event(SessionEvent::OpenFailed { error });
                    }
                }
            }
        });
        session
    }

    /// Workdir of the opened repository (None until `Opened`).
    pub fn workdir(&self) -> Option<PathBuf> {
        self.lock_info().as_ref().map(|i| i.workdir.clone())
    }

    /// Resolved repository paths (None until `Opened`).
    pub fn repo_info(&self) -> Option<RepoInfo> {
        self.lock_info().clone()
    }

    /// Time budget for fetch / push.
    pub fn network_timeout(&self) -> std::time::Duration {
        match self.network_timeout.lock() {
            Ok(g) => *g,
            Err(e) => *e.into_inner(),
        }
    }

    /// Raises or lowers the fetch / push time budget. Zero is ignored — a
    /// network command must always have a backstop.
    pub fn set_network_timeout(&self, timeout: std::time::Duration) {
        if timeout.is_zero() {
            return;
        }
        if let Ok(mut guard) = self.network_timeout.lock() {
            *guard = timeout;
        }
    }

    /// Cancels everything this session is doing. Idempotent.
    pub fn close(&self) {
        self.root_cancel.cancel();
    }

    /// Starts, restarts or stops the periodic `git fetch --prune`.
    ///
    /// `None` (or zero) turns it off. Only one fetch is ever outstanding:
    /// on a slow link or a repository whose credential helper is taking its
    /// time, a tick that finds the previous fetch unfinished is skipped
    /// rather than queued behind it.
    pub fn set_auto_fetch(self: &Arc<Self>, interval: Option<std::time::Duration>) {
        let mut guard = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(previous) = guard.take() {
            previous.cancel();
        }
        let Some(interval) = interval.filter(|i| !i.is_zero()) else {
            return;
        };
        let cancel = self.root_cancel.child_token();
        *guard = Some(cancel.clone());
        drop(guard);

        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // tokio fires the first tick immediately; opening the
            // repository has just read it, so wait out a full interval.
            ticker.tick().await;
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = ticker.tick() => s.auto_fetch_tick(),
                }
            }
        });
    }

    /// Queues one automatic fetch, unless the previous one is still going.
    ///
    /// Reported under its own op name: a laptop that is simply offline must
    /// not put a fresh error banner on screen every interval.
    fn auto_fetch_tick(self: &Arc<Self>) {
        let Ok(permit) = Arc::clone(&self.auto_fetch_slot).try_acquire_owned() else {
            tracing::debug!("auto fetch skipped: the previous one has not finished");
            return;
        };
        let timeout = self.network_timeout();
        self.write(
            AUTO_FETCH_OP,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let _permit = permit;
                remote::fetch(&exec, &repo.workdir, None, timeout, &cancel).await
            },
        );
    }

    pub fn log_options(&self) -> LogOptions {
        *self.lock_log_options()
    }

    /// Toggles tags in the graph walk and restarts the stream.
    pub fn set_include_tags(self: &Arc<Self>, include_tags: bool) {
        {
            let mut options = self.lock_log_options();
            if options.include_tags == include_tags {
                return;
            }
            options.include_tags = include_tags;
        }
        self.restart_log();
    }

    /// Changes the graph window size (`None` = full history) and restarts.
    pub fn set_log_limit(self: &Arc<Self>, limit: Option<u32>) {
        {
            let mut options = self.lock_log_options();
            if options.limit == limit {
                return;
            }
            options.limit = limit;
        }
        self.restart_log();
    }

    fn lock_log_options(&self) -> std::sync::MutexGuard<'_, LogOptions> {
        match self.log_options.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// Restarts the log → graph stream (used by manual full refresh).
    ///
    /// With tags enabled this runs **two passes**: a fast tag-less pass
    /// paints immediately (tag tips make `--topo-order` frontier setup
    /// cost seconds on tag-heavy repositories), then a tag-inclusive pass
    /// rebuilds in the background and atomically replaces the graph.
    pub fn restart_log(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };

        // Cancel the previous stream and install this run's child token.
        let run_cancel = self.root_cancel.child_token();
        if let Some(prev) = self
            .log_cancel
            .lock()
            .map(|mut g| g.replace(run_cancel.clone()))
            .unwrap_or_default()
        {
            prev.cancel();
        }

        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(async move {
            if options.include_tags {
                let fast = LogOptions {
                    include_tags: false,
                    ..options
                };
                if s.run_direct_pass(&workdir, fast, &run_cancel).await.is_ok() {
                    s.run_swap_pass(&workdir, options, &run_cancel).await;
                }
            } else {
                let _completed = s.run_direct_pass(&workdir, options, &run_cancel).await;
            }
        });
    }

    /// Rebuilds the graph off-screen and swaps it in only when it differs
    /// from what the UI already shows (see [`RepoSession::run_swap_pass`]).
    ///
    /// Background triggers (auto fetch, a finished write, an external
    /// dirty/clean flip) go through here instead of [`RepoSession::restart_log`]:
    /// a reset-and-restream repaints the pane even when history did not
    /// move, which reads as idle flicker once a periodic fetch is on.
    pub fn refresh_log(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let run_cancel = self.root_cancel.child_token();
        if let Some(prev) = self
            .log_cancel
            .lock()
            .map(|mut g| g.replace(run_cancel.clone()))
            .unwrap_or_default()
        {
            prev.cancel();
        }
        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(async move {
            s.run_swap_pass(&workdir, options, &run_cancel).await;
        });
    }

    /// Streams one pass straight to the UI (chunked, resets the graph).
    /// Returns Err after reporting when the pass failed or was cancelled.
    async fn run_direct_pass(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
    ) -> Result<(), ()> {
        let generation = self.log_gen.fetch_add(1, Ordering::SeqCst) + 1;
        {
            // Reset graph state for the new stream under one lock.
            let mut shared = self.lock_shared();
            shared.builder = GraphBuilder::new();
            shared.applied.clear();
            shared.sent_rows.clear();
        }
        self.sink.event(SessionEvent::LogStarted { generation });
        let started = Instant::now();
        match self.stream_log(workdir, generation, options, cancel).await {
            Ok(totals) => {
                self.sink.event(SessionEvent::LogFinished {
                    generation,
                    total: totals.shown,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    // Truncation is a property of the walk: the shown count
                    // drifts from it in both directions (the WIP row adds
                    // one, sifted stash parents subtract), so comparing it
                    // against --max-count would flag the wrong streams.
                    truncated: options.limit.is_some_and(|n| totals.walked >= n),
                });
                Ok(())
            }
            Err(error) => {
                if !matches!(error, GitError::Cancelled { .. }) {
                    self.sink.event(SessionEvent::LogFailed {
                        generation,
                        error: error.to_string(),
                    });
                }
                Err(())
            }
        }
    }

    /// Builds a full pass off-screen, then swaps it in as one reset +
    /// one chunk (the UI drains all three events in a single slot call,
    /// so the replacement is flicker-free).
    async fn run_swap_pass(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
    ) {
        let generation = self.log_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let started = Instant::now();
        let mut builder = GraphBuilder::new();
        let mut rows: Vec<LogRow> = Vec::new();

        let result = self
            .collect_log(workdir, options, cancel, &mut builder, &mut rows)
            .await;
        let walked = match result {
            Ok(walked) => walked,
            Err(error) => {
                // The fast pass is already on screen; report quietly.
                if !matches!(error, GitError::Cancelled { .. }) {
                    self.fail("log", error);
                }
                return;
            }
        };

        let total = rows.len() as u32;
        {
            let mut shared = self.lock_shared();
            if self.log_gen.load(Ordering::SeqCst) != generation {
                return; // superseded by a newer restart
            }
            let mut applied: HashMap<u32, Vec<RefLabel>> = HashMap::new();
            for row in &mut rows {
                if let Ok(oid) = Oid::from_hex_str(&row.oid_hex)
                    && let Some(labels) = shared.label_map.get(&oid)
                {
                    row.labels = labels.clone();
                    applied.insert(row.row, labels.clone());
                }
            }
            let unchanged = shared.sent_rows == rows;
            shared.builder = builder;
            shared.applied = applied;
            if unchanged {
                // The UI already shows exactly this: swapping would only
                // reset the view (scroll anchor, selection re-resolve) for
                // an identical picture. Background refreshes land here on
                // every quiet auto-fetch tick.
                tracing::debug!(generation, total, "graph rebuild unchanged; swap skipped");
                return;
            }
            shared.sent_rows = rows.clone();
        }
        self.sink.event(SessionEvent::LogReplaced {
            generation,
            rows,
            elapsed_ms: started.elapsed().as_millis() as u64,
            // See run_direct_pass: the walk decides truncation, not the
            // shown row count.
            truncated: options.limit.is_some_and(|n| walked >= n),
        });
    }

    /// Refreshes refs, status(+op state), stashes and worktrees
    /// concurrently. Cheap enough for window-focus and post-operation
    /// triggers.
    pub fn refresh_quick(self: &Arc<Self>) {
        self.refresh_refs();
        self.refresh_status();
        self.refresh_stashes();
        self.refresh_worktrees();
    }

    pub fn refresh_refs(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        let op_gen = self.refs_gate.begin();
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            let refs = refs::load(&s.executor, &workdir, &cancel).await;
            let head = refs::head_state(&s.executor, &workdir, &cancel).await;
            // A repository with no remotes is normal, and so is a failure
            // to read the list; neither is a reason to lose the refs.
            let remotes = remote::list(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
            match (refs, head) {
                (Ok(refs), Ok(head)) => {
                    if !s.refs_gate.is_current(op_gen) {
                        return;
                    }
                    let mut snapshot = build_snapshot(&refs, &head);
                    snapshot.remote_names = remotes.into_iter().map(|r| r.name).collect();
                    let label_updates = s.apply_refs(&refs, &head);
                    s.sink.event(SessionEvent::RefsLoaded { snapshot });
                    if !label_updates.is_empty() {
                        s.sink.event(SessionEvent::LabelsChanged {
                            rows: label_updates,
                        });
                    }
                }
                (Err(e), _) | (_, Err(e)) => s.fail("refs", e),
            }
        });
    }

    pub fn refresh_status(self: &Arc<Self>) {
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            // An external change (another tool, the terminal) can make the
            // tree dirty or clean, which adds or removes the WIP row.
            if s.publish_status().await {
                s.refresh_log();
            }
        });
    }

    /// Loads status + op state and publishes them, returning whether
    /// working-tree dirtiness flipped.
    ///
    /// Does not rebuild the graph itself: after a write the caller knows
    /// whether it needs one anyway, and rebuilding on both counts would do
    /// it twice.
    async fn publish_status(self: &Arc<Self>) -> bool {
        let Some(workdir) = self.workdir() else {
            return false;
        };
        let op_gen = self.status_gate.begin();
        let cancel = self.root_cancel.clone();
        let status = status::load(&self.executor, &workdir, &cancel).await;
        let op = opstate::detect(&self.executor, &workdir, &cancel).await;
        match (status, op) {
            (Ok(status), Ok(op_state)) => {
                // Only a stepping rebase has a counter to read, so the
                // common refresh costs nothing extra.
                let progress = if op_state.rebasing {
                    conflict::rebase_progress(&self.executor, &workdir, &cancel)
                        .await
                        .unwrap_or_default()
                } else {
                    None
                };
                if !self.status_gate.is_current(op_gen) {
                    return false;
                }
                let dirty = status.is_dirty();
                let flipped = self.wip_dirty.swap(dirty, Ordering::SeqCst) != dirty;
                self.sink.event(SessionEvent::StatusLoaded {
                    status,
                    op_state,
                    progress,
                });
                flipped
            }
            (Err(e), _) | (_, Err(e)) => {
                self.fail("status", e);
                false
            }
        }
    }

    /// Everything [`RepoSession::refresh_quick`] covers except status.
    fn refresh_side_snapshots(self: &Arc<Self>) {
        self.refresh_refs();
        self.refresh_stashes();
        self.refresh_worktrees();
    }

    /// Re-reads the author identity and signing configuration.
    pub fn refresh_author(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match identity::load(&s.executor, &workdir, &cancel).await {
                Ok(config) => s.sink.event(SessionEvent::AuthorLoaded { config }),
                Err(e) => s.fail("identity", e),
            }
        });
    }

    /// Records `user.name` / `user.email`.
    pub fn set_identity(
        self: &Arc<Self>,
        name: String,
        email: String,
        scope: identity::ConfigScope,
    ) {
        self.write(
            "identity",
            AfterWrite::Author,
            move |exec, repo, cancel| async move {
                identity::set_identity(&exec, &repo.workdir, &name, &email, scope, &cancel).await
            },
        );
    }

    /// Reads HEAD's message so an amend can start from it.
    ///
    /// On demand rather than with every refresh: only the amend path wants
    /// it, and a repository refresh already runs several commands.
    pub fn load_head_message(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match commit::head_message(&s.executor, &workdir, &cancel).await {
                Ok(message) => s.sink.event(SessionEvent::HeadMessageLoaded { message }),
                // An unborn branch has no HEAD to amend; that is a state,
                // not a failure worth an error banner.
                Err(_) => s.sink.event(SessionEvent::HeadMessageLoaded {
                    message: String::new(),
                }),
            }
        });
    }

    pub fn refresh_stashes(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        let op_gen = self.stash_gate.begin();
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match stash::load(&s.executor, &workdir, &cancel).await {
                Ok(stashes) => {
                    if s.stash_gate.is_current(op_gen) {
                        s.sink.event(SessionEvent::StashesLoaded { stashes });
                    }
                }
                Err(e) => s.fail("stash", e),
            }
        });
    }

    pub fn refresh_worktrees(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        let op_gen = self.worktrees_gate.begin();
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match crate::worktrees::load(&s.executor, &workdir, &cancel).await {
                Ok(list) => {
                    if s.worktrees_gate.is_current(op_gen) {
                        s.sink
                            .event(SessionEvent::WorktreesLoaded { worktrees: list });
                    }
                }
                Err(e) => s.fail("worktrees", e),
            }
        });
    }

    // --- writes ---------------------------------------------------------

    /// Runs one write command under the session write lock, reports its
    /// lifecycle and refreshes afterwards.
    ///
    /// The lock is released before refreshing so a queued write is not held
    /// up by snapshot reads.
    fn write<F, Fut>(self: &Arc<Self>, op: &'static str, after: AfterWrite, task: F)
    where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        let request = WriteRequest {
            op,
            after,
            run: Box::new(move |exec, repo, cancel| Box::pin(task(exec, repo, cancel))),
        };
        // Enqueueing is synchronous, so the queue order is the order the UI
        // asked in. Sending only fails once the session has shut down.
        if self.write_tx.send(request).is_err() {
            tracing::debug!(op, "write dropped: the session is closed");
        }
    }

    /// Runs queued writes one at a time, in submission order.
    async fn write_loop(
        self: Arc<Self>,
        mut queue: tokio::sync::mpsc::UnboundedReceiver<WriteRequest>,
    ) {
        loop {
            let request = tokio::select! {
                _ = self.root_cancel.cancelled() => return,
                request = queue.recv() => match request {
                    Some(request) => request,
                    None => return,
                },
            };
            self.run_write(request).await;
        }
    }

    async fn run_write(self: &Arc<Self>, request: WriteRequest) {
        let WriteRequest { op, after, run } = request;
        let Some(info) = self.repo_info() else {
            tracing::debug!(op, "write dropped: no repository is open");
            return;
        };
        let cancel = self.root_cancel.clone();
        self.sink.event(SessionEvent::WriteStarted { op });
        let result = run(self.executor.clone(), info, cancel).await;
        let rebuild_graph = match result {
            Ok(()) => {
                self.sink
                    .event(SessionEvent::WriteFinished { op, error: None });
                after == AfterWrite::Graph
            }
            Err(error) if error.is_cancelled() => return,
            Err(error) => {
                tracing::warn!(op, %error, "write failed");
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: Some(error.to_string()),
                });
                // A half-finished command still changed the repository
                // (conflicted merge, interrupted rebase, partial apply),
                // but it did not move history the way it meant to.
                false
            }
        };

        // Settle the working tree before touching the graph: the WIP row
        // exists only while the tree is dirty, so rebuilding first and then
        // reacting to the status would rebuild twice for one write.
        let wip_flipped = self.publish_status().await;
        if rebuild_graph || wip_flipped {
            // Off-screen rebuild: the pane keeps showing the old graph
            // until the finished one swaps in (or nothing changed and
            // nothing repaints — the auto-fetch common case).
            self.refresh_log();
        }
        self.refresh_side_snapshots();
        if after == AfterWrite::Author {
            self.refresh_author();
        }
    }

    /// `git add` for whole files.
    pub fn stage_paths(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "stage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::stage_paths(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// `git add -A` for the whole work tree.
    pub fn stage_all(self: &Arc<Self>) {
        self.write(
            "stage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::stage_all(&exec, &repo.workdir, &cancel).await
            },
        );
    }

    /// Empties the index back to HEAD.
    pub fn unstage_all(self: &Arc<Self>) {
        self.write(
            "unstage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::unstage_all(&exec, &repo.workdir, &cancel).await
            },
        );
    }

    /// Removes whole files from the index.
    pub fn unstage_paths(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "unstage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::unstage_paths(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Throws away unstaged modifications of tracked files.
    pub fn discard_paths(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "discard",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::discard_worktree(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Deletes untracked files.
    pub fn remove_untracked(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "clean",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::remove_untracked(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Stages or unstages part of one file's diff (hunk / line level).
    pub fn apply_partial(self: &Arc<Self>, target: DiffTarget, selects: Vec<HunkSelect>) {
        self.write(
            "stage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::apply_partial(&exec, &repo, &target, &selects, &cancel).await
            },
        );
    }

    /// Commits the index (or amends HEAD).
    pub fn commit(self: &Arc<Self>, message: String, options: CommitOptions) {
        self.write(
            "commit",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                commit::commit(&exec, &repo, &message, options, &cancel)
                    .await
                    .map(drop)
            },
        );
    }

    /// Moves HEAD, taking uncommitted work along as far as git will carry
    /// it (デザイン規約 §未コミット変更がある状態での移動).
    ///
    /// The everyday case needs no question asked: git carries the changes
    /// wherever they do not stand in the way. Where they do, it refuses
    /// and touches nothing — that refusal becomes
    /// [`SessionEvent::MoveBlocked`], and the answer comes back as
    /// [`RepoSession::checkout_stashing`] or
    /// [`RepoSession::checkout_merging`].
    pub fn checkout(self: &Arc<Self>, target: CheckoutTarget) {
        let session = Arc::clone(self);
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let outcome =
                    branch::checkout(&exec, &repo.workdir, &target, branch::Carry::AsIs, &cancel)
                        .await?;
                session.report_move(outcome);
                Ok(())
            },
        );
    }

    /// Stashes the working tree, then moves HEAD — "leave my changes here".
    ///
    /// One job rather than two queued ones. If the stash fails there is
    /// nothing left behind, and switching regardless would carry the
    /// changes to the other branch — the opposite of what was asked.
    ///
    /// The stash keeps git's own message ("WIP on `<branch>`: …"), which
    /// already names where the changes came from.
    pub fn checkout_stashing(self: &Arc<Self>, target: CheckoutTarget) {
        let session = Arc::clone(self);
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let options = stash::PushOptions {
                    include_untracked: true,
                    keep_index: false,
                    staged_only: false,
                };
                stash::push(&exec, &repo.workdir, "", options, &[], &cancel).await?;
                let outcome =
                    branch::checkout(&exec, &repo.workdir, &target, branch::Carry::AsIs, &cancel)
                        .await?;
                session.report_move(outcome);
                Ok(())
            },
        );
    }

    /// Merges the working tree into the target while moving — "bring my
    /// changes and let me sort out the overlap".
    ///
    /// The index is emptied first because git demands it here: with
    /// anything staged it refuses this path outright, colliding file or
    /// not. Nothing is lost by that — the staged content is the content in
    /// the working tree, and what the merge produces would land unstaged
    /// anyway — but the split between staged and unstaged does not survive.
    ///
    /// A conflicted result is still a *successful* move: git leaves the
    /// markers in place, exits zero, and the conflict shows up in the next
    /// status refresh. There is no merge to abort afterwards, which is
    /// exactly why the choice is offered before this runs and not after.
    pub fn checkout_merging(self: &Arc<Self>, target: CheckoutTarget) {
        let session = Arc::clone(self);
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stage::unstage_all(&exec, &repo.workdir, &cancel).await?;
                let outcome = branch::checkout(
                    &exec,
                    &repo.workdir,
                    &target,
                    branch::Carry::Merging,
                    &cancel,
                )
                .await?;
                session.report_move(outcome);
                Ok(())
            },
        );
    }

    /// Passes a refused move on to the UI. A move that happened says
    /// nothing extra: the refresh that follows every write shows it.
    fn report_move(&self, outcome: branch::CheckoutOutcome) {
        if let branch::CheckoutOutcome::Blocked(block) = outcome {
            tracing::info!(?block, "move refused: uncommitted work in the way");
            self.sink.event(SessionEvent::MoveBlocked { block });
        }
    }

    /// Creates a branch, optionally switching to it.
    pub fn create_branch(
        self: &Arc<Self>,
        name: String,
        start_point: Option<String>,
        switch_to: bool,
    ) {
        self.write(
            "branch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::create(
                    &exec,
                    &repo.workdir,
                    &name,
                    start_point.as_deref(),
                    switch_to,
                    &cancel,
                )
                .await
            },
        );
    }

    pub fn delete_branch(self: &Arc<Self>, name: String, force: bool) {
        self.write(
            "branch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::delete(&exec, &repo.workdir, &name, force, &cancel).await
            },
        );
    }

    pub fn rename_branch(self: &Arc<Self>, from: String, to: String, force: bool) {
        self.write(
            "branch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::rename(&exec, &repo.workdir, &from, &to, force, &cancel).await
            },
        );
    }

    /// `git stash push`.
    pub fn stash_push(self: &Arc<Self>, message: String, options: stash::PushOptions) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::push(&exec, &repo.workdir, &message, options, &[], &cancel).await
            },
        );
    }

    /// `git stash pop <selector>` (drops the stash on success).
    pub fn stash_pop(self: &Arc<Self>, selector: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::pop(&exec, &repo.workdir, &selector, &cancel).await
            },
        );
    }

    /// `git stash apply <selector>` (keeps the stash).
    pub fn stash_apply(self: &Arc<Self>, selector: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::apply(&exec, &repo.workdir, &selector, &cancel).await
            },
        );
    }

    /// `git stash drop <selector>`.
    pub fn stash_drop(self: &Arc<Self>, selector: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::drop(&exec, &repo.workdir, &selector, &cancel).await
            },
        );
    }

    /// `git fetch --prune`; `None` fetches every remote.
    pub fn fetch(self: &Arc<Self>, remote: Option<String>) {
        let timeout = self.network_timeout();
        self.write(
            "fetch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::fetch(&exec, &repo.workdir, remote.as_deref(), timeout, &cancel).await
            },
        );
    }

    /// `git push` for one branch.
    pub fn push(self: &Arc<Self>, spec: remote::PushSpec) {
        let timeout = self.network_timeout();
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await
            },
        );
    }

    /// Pushes the branch that is checked out to wherever it belongs.
    ///
    /// Resolving the target is part of the job rather than something the
    /// UI works out: which remote a branch tracks lives in configuration,
    /// and a name like `origin/main` cannot be split back apart reliably.
    pub fn push_current(self: &Arc<Self>, fallback_remote: String, force: remote::PushForce) {
        let timeout = self.network_timeout();
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let spec = remote::plan_current_push(
                    &exec,
                    &repo.workdir,
                    &fallback_remote,
                    force,
                    &cancel,
                )
                .await?;
                remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await
            },
        );
    }

    /// `git push <remote> --delete <branch>`.
    pub fn delete_remote_branch(self: &Arc<Self>, remote_name: String, branch_name: String) {
        let timeout = self.network_timeout();
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::delete_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &branch_name,
                    timeout,
                    &cancel,
                )
                .await
            },
        );
    }

    /// `git merge <rev>`.
    pub fn merge(self: &Arc<Self>, rev: String, options: integrate::MergeOptions) {
        self.write(
            "merge",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                integrate::merge(&exec, &repo.workdir, &rev, &options, &cancel).await
            },
        );
    }

    /// `git rebase <upstream>`.
    pub fn rebase(self: &Arc<Self>, upstream: String, options: integrate::RebaseOptions) {
        self.write(
            "rebase",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                integrate::rebase(&exec, &repo.workdir, &upstream, &options, &cancel).await
            },
        );
    }

    /// `git rebase --interactive` with a plan assembled in the UI. The todo
    /// editor is the helper binary shipped beside the application.
    pub fn rebase_interactive(
        self: &Arc<Self>,
        upstream: String,
        steps: Vec<sequencer::RebaseStep>,
        options: integrate::RebaseOptions,
    ) {
        self.write(
            "rebase",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let helper = sequencer::helper_path().map_err(|source| GitError::Io {
                    command: "git rebase --interactive".to_string(),
                    source,
                })?;
                sequencer::rebase_interactive(
                    &exec, &repo, &upstream, &steps, &options, &helper, &cancel,
                )
                .await
            },
        );
    }

    /// Folds one commit into its parent.
    pub fn squash_into_parent(self: &Arc<Self>, oid: String) {
        self.write(
            "squash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let plan = sequencer::plan_edit(
                    &exec,
                    &repo.workdir,
                    &oid,
                    sequencer::Edit::SquashIntoParent,
                    &cancel,
                )
                .await?;
                run_plan(&exec, &repo, &plan, &cancel).await
            },
        );
    }

    /// Replaces one commit's message.
    ///
    /// The newest commit is amended instead of replayed: an amend touches
    /// nothing else, while a rebase would rewrite every commit after it.
    pub fn reword(self: &Arc<Self>, oid: String, message: String) {
        self.write(
            "reword",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let head = commit::head_oid(&exec, &repo.workdir, &cancel).await?;
                if head.to_hex() == oid {
                    let options = CommitOptions {
                        amend: true,
                        ..Default::default()
                    };
                    return commit::commit(&exec, &repo, &message, options, &cancel)
                        .await
                        .map(drop);
                }
                let plan = sequencer::plan_edit(
                    &exec,
                    &repo.workdir,
                    &oid,
                    sequencer::Edit::Reword(message),
                    &cancel,
                )
                .await?;
                run_plan(&exec, &repo, &plan, &cancel).await
            },
        );
    }

    /// `git cherry-pick <revs>`.
    pub fn cherry_pick(self: &Arc<Self>, revs: Vec<String>) {
        self.write(
            "cherry-pick",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                integrate::cherry_pick(&exec, &repo.workdir, &revs, &cancel).await
            },
        );
    }

    /// `git revert <revs>`.
    pub fn revert(self: &Arc<Self>, revs: Vec<String>) {
        self.write(
            "revert",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                integrate::revert(&exec, &repo.workdir, &revs, &cancel).await
            },
        );
    }

    /// Continues / aborts / skips whatever operation is in progress.
    pub fn resolve_current(self: &Arc<Self>, continuation: integrate::Continuation) {
        self.write(
            "resolve",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                integrate::resolve_current(&exec, &repo.workdir, continuation, &cancel)
                    .await
                    .map(drop)
            },
        );
    }

    /// Resolves conflicted paths by taking one side wholesale.
    pub fn take_side(self: &Arc<Self>, paths: Vec<String>, side: conflict::Side) {
        self.write(
            "resolve",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::take_side(&exec, &repo.workdir, &paths, side, &cancel).await
            },
        );
    }

    /// Hands conflicted paths to `git mergetool` (empty = all of them).
    ///
    /// Runs under the write lock like any other write, which means it holds
    /// the lock for as long as the user keeps the tool open.
    pub fn mergetool(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "mergetool",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::mergetool(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Asks how much of `range` a remote already has, so the UI can warn
    /// before rewriting published history. A read, not a write.
    pub fn check_publish(self: &Arc<Self>, range: String) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match publish::state_of(&s.executor, &workdir, &range, &cancel).await {
                Ok(state) => s.sink.event(SessionEvent::PublishChecked { range, state }),
                Err(e) => s.fail("publish", e),
            }
        });
    }

    /// Asks whether HEAD can reach `oid`, so the UI can tell which commits
    /// a rewrite may start from. A read, not a write.
    pub fn check_in_history(self: &Arc<Self>, oid: Oid) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match commit::is_in_head_history(&s.executor, &workdir, &oid, &cancel).await {
                Ok(in_history) => s.sink.event(SessionEvent::InHistoryChecked {
                    oid: oid.to_hex(),
                    in_history,
                }),
                Err(e) => s.fail("history", e),
            }
        });
    }

    /// Loads full details of one commit (metadata + changed files).
    pub fn load_details(self: &Arc<Self>, oid: Oid) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match details::commit_details(&s.executor, &workdir, &oid, &cancel).await {
                Ok(details) => s.sink.event(SessionEvent::DetailsLoaded { details }),
                Err(e) => s.fail("details", e),
            }
        });
    }

    /// Loads a unified diff for one file.
    pub fn load_diff(self: &Arc<Self>, target: DiffTarget) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match details::file_diff(&s.executor, &workdir, &target, &cancel).await {
                Ok(patches) => {
                    let is_binary = patches.iter().any(|p| p.is_binary);
                    let preview =
                        preview::file_preview(&s.executor, &workdir, &target, is_binary, &cancel)
                            .await;
                    s.sink.event(SessionEvent::DiffLoaded {
                        target,
                        patches,
                        preview,
                    });
                }
                Err(e) => s.fail("diff", e),
            }
        });
    }

    // --- internals ------------------------------------------------------

    fn fail(&self, op: &'static str, error: GitError) {
        if error.is_cancelled() {
            return;
        }
        tracing::warn!(op, error = %error, "session operation failed");
        self.sink.event(SessionEvent::OpFailed { op, error });
    }

    fn set_info(&self, info: RepoInfo) {
        if let Ok(mut guard) = self.info.lock() {
            *guard = Some(info);
        }
    }

    fn lock_info(&self) -> std::sync::MutexGuard<'_, Option<RepoInfo>> {
        // A poisoned lock only happens if a holder panicked; the data is a
        // plain snapshot, safe to keep using.
        match self.info.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    fn lock_shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        match self.shared.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    async fn stream_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        generation: u64,
        options: LogOptions,
        cancel: &CancellationToken,
    ) -> Result<LogTotals, GitError> {
        // An unborn HEAD has nothing to log.
        let head = refs::head_state(&self.executor, workdir, cancel).await?;
        if head.oid.is_none() {
            return Ok(LogTotals::default());
        }

        // Stashes are part of the graph: their oids join the walk and the
        // synthetic index/untracked parents are sifted out below.
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        let mut cmd = GitCommand::new()
            .cwd(workdir)
            .args(["log", "-z", "--topo-order", LOG_FORMAT_ARG])
            .args(["HEAD", "--branches", "--remotes"])
            .no_timeout();
        if options.include_tags {
            cmd = cmd.arg("--tags");
        }
        if let Some(limit) = options.limit {
            cmd = cmd.arg(format!("--max-count={limit}"));
        }
        if !stash_refs.is_empty() {
            // A stash may vanish between the listing and the walk.
            cmd = cmd.arg("--ignore-missing");
            for oid in stash_refs.keys() {
                cmd = cmd.arg(oid.to_hex());
            }
        }

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut stash_skip: std::collections::HashSet<Oid> = std::collections::HashSet::new();
        let mut first_sent = false;
        let mut parse_error: Option<String> = None;
        let mut totals = LogTotals::default();

        // Dirty working tree: prepend the synthetic WIP row so the current
        // chain owns lane 0 from the very first paint.
        if self.wip_dirty.load(Ordering::SeqCst)
            && let Some(head_oid) = head.oid
        {
            self.emit_wip_row(generation, &head_oid);
            totals.shown += 1;
        }

        let result = self
            .executor
            .run_streaming(cmd, cancel, &mut |bytes| {
                if parse_error.is_some() {
                    return;
                }
                if let Err(e) = parser.feed(bytes, &mut pending) {
                    parse_error = Some(e.to_string());
                    cancel.cancel();
                    return;
                }
                let threshold = if first_sent {
                    CHUNK_ROWS
                } else {
                    FIRST_CHUNK_ROWS
                };
                if pending.len() >= threshold {
                    let batch = std::mem::take(&mut pending);
                    totals.walked += batch.len() as u32;
                    let mut items = Vec::with_capacity(batch.len());
                    sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
                    totals.shown += items.len() as u32;
                    first_sent = true;
                    self.emit_rows(generation, &items, parser.pool());
                }
            })
            .await;

        match result {
            Ok(_) => {}
            Err(e) => {
                // A self-inflicted cancel means the parser hit a fatal error.
                if let Some(msg) = parse_error {
                    return Err(GitError::UnexpectedOutput {
                        command: "git log".to_string(),
                        message: msg,
                    });
                }
                return Err(e);
            }
        }
        if let Err(e) = parser.finish() {
            return Err(GitError::UnexpectedOutput {
                command: "git log".to_string(),
                message: e.to_string(),
            });
        }
        if !pending.is_empty() {
            let batch = std::mem::take(&mut pending);
            totals.walked += batch.len() as u32;
            let mut items = Vec::with_capacity(batch.len());
            sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
            totals.shown += items.len() as u32;
            self.emit_rows(generation, &items, parser.pool());
        }
        Ok(totals)
    }

    /// Buffered variant of [`RepoSession::stream_log`]: rows accumulate
    /// into the caller's builder/vec without touching shared state or the
    /// sink (used by the tag-inclusive swap pass). Returns the number of
    /// commits the walk emitted (what `--max-count` limits — the shown
    /// row count is `out.len()`).
    async fn collect_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        builder: &mut GraphBuilder,
        out: &mut Vec<LogRow>,
    ) -> Result<u32, GitError> {
        // An unborn HEAD has nothing to log.
        let head = refs::head_state(&self.executor, workdir, cancel).await?;
        if head.oid.is_none() {
            return Ok(0);
        }

        // Dirty working tree: prepend the synthetic WIP row (mirrors
        // stream_log).
        if self.wip_dirty.load(Ordering::SeqCst)
            && let Some(head_oid) = head.oid
        {
            out.push(wip_row(&head_oid, builder));
        }

        // Stashes join the walk here too (see stream_log).
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        let mut cmd = GitCommand::new()
            .cwd(workdir)
            .args(["log", "-z", "--topo-order", LOG_FORMAT_ARG])
            .args(["HEAD", "--branches", "--remotes"])
            .no_timeout();
        if options.include_tags {
            cmd = cmd.arg("--tags");
        }
        if let Some(limit) = options.limit {
            cmd = cmd.arg(format!("--max-count={limit}"));
        }
        if !stash_refs.is_empty() {
            cmd = cmd.arg("--ignore-missing");
            for oid in stash_refs.keys() {
                cmd = cmd.arg(oid.to_hex());
            }
        }

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut stash_skip: std::collections::HashSet<Oid> = std::collections::HashSet::new();
        let mut parse_error: Option<String> = None;
        let mut walked: u32 = 0;

        let result = self
            .executor
            .run_streaming(cmd, cancel, &mut |bytes| {
                if parse_error.is_some() {
                    return;
                }
                if let Err(e) = parser.feed(bytes, &mut pending) {
                    parse_error = Some(e.to_string());
                    cancel.cancel();
                    return;
                }
                let batch = std::mem::take(&mut pending);
                walked += batch.len() as u32;
                let mut items = Vec::with_capacity(batch.len());
                sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
                for item in items {
                    let mut row =
                        make_row(&item.meta, parser.pool(), builder, item.stash_ref.is_some());
                    if let Some(r) = item.stash_ref {
                        row.stash_ref = r;
                    }
                    out.push(row);
                }
            })
            .await;
        match result {
            Ok(_) => {}
            Err(e) => {
                if let Some(msg) = parse_error {
                    return Err(GitError::UnexpectedOutput {
                        command: "git log".to_string(),
                        message: msg,
                    });
                }
                return Err(e);
            }
        }
        if let Err(e) = parser.finish() {
            return Err(GitError::UnexpectedOutput {
                command: "git log".to_string(),
                message: e.to_string(),
            });
        }
        let batch = std::mem::take(&mut pending);
        walked += batch.len() as u32;
        let mut items = Vec::with_capacity(batch.len());
        sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
        for item in items {
            let mut row = make_row(&item.meta, parser.pool(), builder, item.stash_ref.is_some());
            if let Some(r) = item.stash_ref {
                row.stash_ref = r;
            }
            out.push(row);
        }
        Ok(walked)
    }

    /// Sends the synthetic WIP row (dirty working tree) as its own chunk.
    fn emit_wip_row(&self, generation: u64, head: &Oid) {
        let mut guard = self.lock_shared();
        if self.log_gen.load(Ordering::SeqCst) != generation {
            return;
        }
        let row = wip_row(head, &mut guard.builder);
        guard.sent_rows.push(row.clone());
        self.sink.event(SessionEvent::LogChunk {
            generation,
            rows: vec![row],
        });
    }

    /// Builds graph rows for a batch and sends them (holding the shared
    /// lock so generations cannot interleave).
    fn emit_rows(&self, generation: u64, batch: &[StreamItem], pool: &crate::model::StrPool) {
        let mut guard = self.lock_shared();
        if self.log_gen.load(Ordering::SeqCst) != generation {
            return;
        }
        let shared = &mut *guard;
        let mut rows = Vec::with_capacity(batch.len());
        for item in batch {
            let mut row = make_row(
                &item.meta,
                pool,
                &mut shared.builder,
                item.stash_ref.is_some(),
            );
            if let Some(r) = &item.stash_ref {
                row.stash_ref = r.clone();
            }
            if let Some(labels) = shared.label_map.get(&item.meta.oid) {
                row.labels = labels.clone();
                shared.applied.insert(row.row, labels.clone());
            }
            rows.push(row);
        }
        shared.sent_rows.extend(rows.iter().cloned());
        self.sink.event(SessionEvent::LogChunk { generation, rows });
    }

    /// Installs a new refs snapshot into the label join and returns the
    /// rows whose chips changed.
    fn apply_refs(&self, refs: &[RefEntry], head: &HeadState) -> Vec<(u32, Vec<RefLabel>)> {
        let label_map = build_label_map(refs, head);
        let mut shared = self.lock_shared();
        shared.label_map = label_map;

        let mut fresh: HashMap<u32, Vec<RefLabel>> = HashMap::new();
        for (oid, labels) in &shared.label_map {
            if let Some(row) = shared.builder.row_of(oid) {
                fresh.insert(row, labels.clone());
            }
        }
        let mut changed: Vec<(u32, Vec<RefLabel>)> = Vec::new();
        for (row, labels) in &fresh {
            if shared.applied.get(row) != Some(labels) {
                changed.push((*row, labels.clone()));
            }
        }
        for row in shared.applied.keys() {
            if !fresh.contains_key(row) {
                changed.push((*row, Vec::new()));
            }
        }
        shared.applied = fresh;
        changed.sort_by_key(|(row, _)| *row);
        // Mirror the chip change into the delivered-rows record, or the
        // next background rebuild would see a phantom difference and swap
        // an identical graph.
        for (row, labels) in &changed {
            if let Some(sent) = shared.sent_rows.get_mut(*row as usize) {
                sent.labels = labels.clone();
            }
        }
        changed
    }
}

impl Drop for RepoSession {
    fn drop(&mut self) {
        self.root_cancel.cancel();
    }
}

/// Builds the synthetic row for uncommitted changes: zero id, no author,
/// one dashed edge running down to HEAD. The UI recognizes the all-zero
/// id and renders the dashed empty node and the WIP subject.
/// Replays a one-commit edit plan through `git rebase --interactive`.
async fn run_plan(
    executor: &GitExecutor,
    repo: &RepoInfo,
    plan: &sequencer::EditPlan,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let helper = sequencer::helper_path().map_err(|source| GitError::Io {
        command: "git rebase --interactive".to_string(),
        source,
    })?;
    sequencer::rebase_interactive(
        executor,
        repo,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper,
        cancel,
    )
    .await
}

fn wip_row(head: &Oid, builder: &mut GraphBuilder) -> LogRow {
    let zero = Oid::zero_like(head);
    let g = builder.push_virtual(&zero, head);
    LogRow {
        row: g.row,
        oid_hex: zero.to_hex(),
        short_sha: zero.short_hex(8),
        author: String::new(),
        time: 0,
        subject: String::new(),
        node_lane: g.node_lane,
        node_color: g.node_color,
        width: g.width,
        segments: g.segments,
        labels: Vec::new(),
        stash_ref: String::new(),
    }
}

/// Row counts of one completed log pass. They differ in both directions:
/// the synthetic WIP row is shown but never walked, and a stash's
/// synthetic index/untracked parents are walked but never shown.
#[derive(Clone, Copy, Default)]
struct LogTotals {
    /// Rows delivered to the UI.
    shown: u32,
    /// Commits the walk emitted — what `--max-count` limits, so this is
    /// what decides `truncated`.
    walked: u32,
}

/// One sifted stream entry (stash rows carry their reflog selector).
struct StreamItem {
    meta: CommitMeta,
    stash_ref: Option<String>,
}

/// Filters a parsed batch for display: stash commits keep only their
/// first-parent edge (the base commit), and their synthetic index /
/// untracked parent commits are recorded and dropped when they arrive
/// later (topo order guarantees the stash row streams first).
fn sift_batch(
    batch: Vec<CommitMeta>,
    stash_refs: &HashMap<Oid, String>,
    skip: &mut std::collections::HashSet<Oid>,
    out: &mut Vec<StreamItem>,
) {
    for mut meta in batch {
        if skip.contains(&meta.oid) {
            continue;
        }
        let stash_ref = stash_refs.get(&meta.oid).cloned();
        if stash_ref.is_some() && meta.parents.len() > 1 {
            for extra in &meta.parents[1..] {
                skip.insert(*extra);
            }
            meta.parents = Box::from(&meta.parents[..1]);
        }
        out.push(StreamItem { meta, stash_ref });
    }
}

/// Builds one display row from a commit (labels attached by the caller).
/// `dashed_edge` draws the first-parent edge dashed (stash rows).
fn make_row(
    commit: &CommitMeta,
    pool: &crate::model::StrPool,
    builder: &mut GraphBuilder,
    dashed_edge: bool,
) -> LogRow {
    let g = builder.push_with_edge_style(commit, dashed_edge);
    LogRow {
        row: g.row,
        oid_hex: commit.oid.to_hex(),
        short_sha: commit.oid.short_hex(8),
        author: pool.get(commit.author).to_string(),
        time: commit.time,
        subject: commit.subject.to_string(),
        node_lane: g.node_lane,
        node_color: g.node_color,
        width: g.width,
        segments: g.segments,
        labels: Vec::new(),
        stash_ref: String::new(),
    }
}

/// Builds the per-commit label chips from a refs listing.
fn build_label_map(refs: &[RefEntry], head: &HeadState) -> HashMap<Oid, Vec<RefLabel>> {
    let with_remote = refs::branches_with_remote(refs);
    let mut map: HashMap<Oid, Vec<RefLabel>> = HashMap::new();
    for r in refs {
        let kind = match r.kind {
            RefKind::LocalBranch => LabelKind::LocalBranch,
            RefKind::RemoteBranch => LabelKind::RemoteBranch,
            RefKind::Tag => LabelKind::Tag,
        };
        map.entry(r.commit_oid()).or_default().push(RefLabel {
            text: r.short.clone(),
            kind,
            has_remote: r.kind == RefKind::LocalBranch && with_remote.contains(&r.name),
            is_head: r.is_head,
        });
    }
    if head.detached
        && let Some(oid) = head.oid
    {
        map.entry(oid).or_default().push(RefLabel {
            text: "HEAD".to_string(),
            kind: LabelKind::Head,
            has_remote: false,
            is_head: true,
        });
    }
    for labels in map.values_mut() {
        labels.sort_by(|a, b| {
            (!a.is_head, a.kind, a.text.as_str()).cmp(&(!b.is_head, b.kind, b.text.as_str()))
        });
    }
    map
}

/// Builds the sorted sidebar snapshot.
fn build_snapshot(refs: &[RefEntry], head: &HeadState) -> RefsSnapshot {
    let with_remote = refs::branches_with_remote(refs);
    let mut snapshot = RefsSnapshot {
        head: Some(head.clone()),
        ..Default::default()
    };
    for r in refs {
        match r.kind {
            RefKind::LocalBranch => snapshot.locals.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid_hex: r.commit_oid().to_hex(),
                has_remote: with_remote.contains(&r.name),
                is_head: r.is_head,
            }),
            RefKind::RemoteBranch => snapshot.remotes.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid_hex: r.commit_oid().to_hex(),
                has_remote: true,
                is_head: false,
            }),
            RefKind::Tag => snapshot.tags.push(TagItem {
                short: r.short.clone(),
                oid_hex: r.commit_oid().to_hex(),
                annotated: r.peeled.is_some(),
                created_unix: r.created_unix,
            }),
        }
    }
    snapshot.locals.sort_by(|a, b| a.short.cmp(&b.short));
    snapshot.remotes.sort_by(|a, b| a.short.cmp(&b.short));
    // Tags newest-first (product decision), name as the tie-breaker.
    snapshot.tags.sort_by(|a, b| {
        b.created_unix
            .cmp(&a.created_unix)
            .then(a.short.cmp(&b.short))
    });
    snapshot
}
