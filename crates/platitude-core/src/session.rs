//! RepoSession: one open repository = one session (実装計画 §2.3).
//!
//! Owns all git activity for a repository: the streaming log → graph
//! pipeline, parallel snapshot refreshes (refs / status / stash) and
//! on-demand queries (details, diffs). Everything runs on a tokio runtime;
//! results are pushed to the UI through a [`SessionSink`], which must be
//! cheap and non-blocking (the app bridge posts queued invocations to the
//! Qt main thread).
//!
//! Reads run concurrently; writes take a session-wide lock so two commands
//! can never touch one repository's index or refs at the same time
//! (実装計画 §2.3). Every write refreshes afterwards — including a failed
//! one, because a command that stops halfway (a conflicted merge, an
//! interrupted rebase) has still changed the repository.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio_util::sync::CancellationToken;

use crate::branch::{self, CheckoutTarget};
use crate::commit::{self, CommitOptions};
use crate::details::{self, CommitDetails, DiffTarget};
use crate::error::GitError;
use crate::graph::{GraphBuilder, Segment};
use crate::model::CommitMeta;
use crate::oid::Oid;
use crate::opstate::{self, OpState};
use crate::parse::diff::FilePatch;
use crate::parse::log::{LOG_FORMAT_ARG, LogParser};
use crate::patch::HunkSelect;
use crate::process::{GitCommand, GitExecutor};
use crate::refs::{self, HeadState, RefEntry, RefKind};
use crate::remote;
use crate::repo::{self, RepoInfo};
use crate::stage;
use crate::stash::{self, StashEntry};
use crate::status::{self, WorkTreeStatus};

/// First chunk is small so the first paint happens as early as possible.
const FIRST_CHUNK_ROWS: usize = 512;
const CHUNK_ROWS: usize = 4096;

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
    },
    /// A background refresh/query failed (op is a stable identifier).
    OpFailed {
        op: &'static str,
        error: GitError,
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
}

/// Receives session events; implementations must be non-blocking.
pub trait SessionSink: Send + Sync + 'static {
    fn event(&self, event: SessionEvent);
}

/// Shared mutable state between the log task and refs joins.
#[derive(Default)]
struct Shared {
    builder: GraphBuilder,
    /// Label chips per commit id, derived from the last refs snapshot.
    label_map: HashMap<Oid, Vec<RefLabel>>,
    /// Labels currently shown per row (for diffing on refs refresh).
    applied: HashMap<u32, Vec<RefLabel>>,
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
    /// Serializes write commands within this repository (held across the
    /// subprocess, hence tokio's mutex rather than std's).
    write_lock: tokio::sync::Mutex<()>,
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
            write_lock: tokio::sync::Mutex::new(()),
            refs_gate: OpGate::default(),
            status_gate: OpGate::default(),
            stash_gate: OpGate::default(),
            worktrees_gate: OpGate::default(),
        });

        let s = Arc::clone(&session);
        runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match repo::open(&s.executor, &path, &cancel).await {
                Ok(info) => {
                    s.set_info(info.clone());
                    s.sink.event(SessionEvent::Opened { info });
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

    /// Cancels everything this session is doing. Idempotent.
    pub fn close(&self) {
        self.root_cancel.cancel();
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
        }
        self.sink.event(SessionEvent::LogStarted { generation });
        let started = Instant::now();
        match self.stream_log(workdir, generation, options, cancel).await {
            Ok(total) => {
                self.sink.event(SessionEvent::LogFinished {
                    generation,
                    total,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    truncated: options.limit.is_some_and(|n| total >= n),
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
        match result {
            Ok(()) => {}
            Err(error) => {
                // The fast pass is already on screen; report quietly.
                if !matches!(error, GitError::Cancelled { .. }) {
                    self.fail("log", error);
                }
                return;
            }
        }

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
            shared.builder = builder;
            shared.applied = applied;
        }
        self.sink.event(SessionEvent::LogStarted { generation });
        self.sink.event(SessionEvent::LogChunk { generation, rows });
        self.sink.event(SessionEvent::LogFinished {
            generation,
            total,
            elapsed_ms: started.elapsed().as_millis() as u64,
            truncated: options.limit.is_some_and(|n| total >= n),
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
            match (refs, head) {
                (Ok(refs), Ok(head)) => {
                    if !s.refs_gate.is_current(op_gen) {
                        return;
                    }
                    let snapshot = build_snapshot(&refs, &head);
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
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        let op_gen = self.status_gate.begin();
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            let status = status::load(&s.executor, &workdir, &cancel).await;
            let op = opstate::detect(&s.executor, &workdir, &cancel).await;
            match (status, op) {
                (Ok(status), Ok(op_state)) => {
                    if s.status_gate.is_current(op_gen) {
                        s.update_wip(status.is_dirty());
                        s.sink
                            .event(SessionEvent::StatusLoaded { status, op_state });
                    }
                }
                (Err(e), _) | (_, Err(e)) => s.fail("status", e),
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
        Fut: std::future::Future<Output = Result<(), GitError>> + Send,
    {
        let Some(info) = self.repo_info() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            let result = {
                let guard = s.write_lock.lock().await;
                s.sink.event(SessionEvent::WriteStarted { op });
                let result = task(s.executor.clone(), info, cancel).await;
                drop(guard);
                result
            };
            match result {
                Ok(()) => {
                    s.sink
                        .event(SessionEvent::WriteFinished { op, error: None });
                    if after == AfterWrite::Graph {
                        s.restart_log();
                    }
                    s.refresh_quick();
                }
                Err(error) if error.is_cancelled() => {}
                Err(error) => {
                    tracing::warn!(op, %error, "write failed");
                    s.sink.event(SessionEvent::WriteFinished {
                        op,
                        error: Some(error.to_string()),
                    });
                    // A half-finished command still changed the repository
                    // (conflicted merge, interrupted rebase, partial apply).
                    s.refresh_quick();
                }
            }
        });
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

    /// Moves HEAD.
    pub fn checkout(self: &Arc<Self>, target: CheckoutTarget) {
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::checkout(&exec, &repo.workdir, &target, &cancel).await
            },
        );
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
        self.write(
            "fetch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::fetch(&exec, &repo.workdir, remote.as_deref(), &cancel).await
            },
        );
    }

    /// `git push` for one branch.
    pub fn push(self: &Arc<Self>, spec: remote::PushSpec) {
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::push(&exec, &repo.workdir, &spec, &cancel).await
            },
        );
    }

    /// `git push <remote> --delete <branch>`.
    pub fn delete_remote_branch(self: &Arc<Self>, remote_name: String, branch_name: String) {
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::delete_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &branch_name,
                    &cancel,
                )
                .await
            },
        );
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
                Ok(patches) => s.sink.event(SessionEvent::DiffLoaded { target, patches }),
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
    ) -> Result<u32, GitError> {
        // An unborn HEAD has nothing to log.
        let head = refs::head_state(&self.executor, workdir, cancel).await?;
        if head.oid.is_none() {
            return Ok(0);
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
        let mut total: u32 = 0;

        // Dirty working tree: prepend the synthetic WIP row so the current
        // chain owns lane 0 from the very first paint.
        if self.wip_dirty.load(Ordering::SeqCst)
            && let Some(head_oid) = head.oid
        {
            self.emit_wip_row(generation, &head_oid);
            total += 1;
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
                    let mut items = Vec::with_capacity(batch.len());
                    sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
                    total += items.len() as u32;
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
            let mut items = Vec::with_capacity(batch.len());
            sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
            total += items.len() as u32;
            self.emit_rows(generation, &items, parser.pool());
        }
        Ok(total)
    }

    /// Buffered variant of [`RepoSession::stream_log`]: rows accumulate
    /// into the caller's builder/vec without touching shared state or the
    /// sink (used by the tag-inclusive swap pass).
    async fn collect_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        builder: &mut GraphBuilder,
        out: &mut Vec<LogRow>,
    ) -> Result<(), GitError> {
        // An unborn HEAD has nothing to log.
        let head = refs::head_state(&self.executor, workdir, cancel).await?;
        if head.oid.is_none() {
            return Ok(());
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
        let mut items = Vec::with_capacity(batch.len());
        sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
        for item in items {
            let mut row = make_row(&item.meta, parser.pool(), builder, item.stash_ref.is_some());
            if let Some(r) = item.stash_ref {
                row.stash_ref = r;
            }
            out.push(row);
        }
        Ok(())
    }

    /// Records the working-tree dirtiness; an edge restarts the log so
    /// the synthetic WIP row appears/disappears with proper lanes.
    fn update_wip(self: &Arc<Self>, dirty: bool) {
        if self.wip_dirty.swap(dirty, Ordering::SeqCst) != dirty {
            self.restart_log();
        }
    }

    /// Sends the synthetic WIP row (dirty working tree) as its own chunk.
    fn emit_wip_row(&self, generation: u64, head: &Oid) {
        let row = {
            let mut guard = self.lock_shared();
            if self.log_gen.load(Ordering::SeqCst) != generation {
                return;
            }
            wip_row(head, &mut guard.builder)
        };
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
