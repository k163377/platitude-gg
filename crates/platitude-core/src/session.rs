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

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
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
use crate::process::{CommandEnd, GitCommand, GitExecutor};
use crate::publish;
use crate::refs::{self, HeadState, RefEntry, RefKind};
use crate::remote;
use crate::repo::{self, RepoInfo};
use crate::sequencer;
use crate::stage;
use crate::stash::{self, StashEntry};
use crate::status::{self, WorkTreeStatus};
use crate::tag;

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
    /// Cloud badge: this name is on a remote as well. The PR dimension is
    /// wired in Phase 4.
    pub has_remote: bool,
    /// True when HEAD is on this branch (bold chip).
    pub is_head: bool,
    /// Whether this repository holds the ref — the whereabouts the chip
    /// writes in the name's colour (デザイン規約 §ref の種別). False for a
    /// remote branch, and for a tag that is only over there or that points
    /// somewhere this one does not.
    pub here: bool,
    /// Whose reading this is, when it is not this repository's: the remote
    /// names carrying the tag, comma-separated. Empty for everything else.
    ///
    /// A remote branch says it in its own name (`origin/main`); a tag has
    /// no such namespace to say it in, and a drifted one puts the same
    /// bare name on two rows. The hover card is where those two meet, and
    /// this is what tells them apart there.
    pub remote: String,
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

/// What the remotes last said they carry under `refs/tags/`: tag name →
/// every commit some remote has it on, and what is known about it there.
///
/// Two commits under one name means the remotes disagree, which reads on
/// screen exactly like a tag that drifted from the one here — the name
/// standing on more than one row.
type RemoteTagIndex = BTreeMap<String, BTreeMap<Oid, RemoteTagPlace>>;

/// One reading of a tag: what the remotes holding it there call themselves,
/// and whether it is annotated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RemoteTagPlace {
    annotated: bool,
    /// Sorted, and more than one when several remotes agree on the commit.
    remotes: Vec<String>,
}

/// The same thing before it is merged, kept per remote so one that could
/// not be reached keeps its last answer instead of dropping every badge it
/// accounted for. `refs/remotes/` does this for branches; a tag has no such
/// local record, so the session holds it.
type RemoteTagsByRemote = BTreeMap<String, Vec<remote::RemoteTag>>;

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
    /// For a local branch, the remote branch it speaks for (`origin/main`),
    /// wherever the two stand — the one its badge is about, and the one a
    /// rename offers to carry over. Empty when it speaks for none.
    pub upstream: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagItem {
    pub short: String,
    /// Peeled commit id (what the graph row is keyed on).
    pub oid_hex: String,
    pub annotated: bool,
    /// Creator date (unix seconds); the sidebar sorts tags newest-first.
    /// Zero for a tag only a remote has: an advertisement carries the name
    /// and the commit, and no date to sort by.
    pub created_unix: i64,
    /// A remote carries this name too (cloud badge).
    pub has_remote: bool,
    /// Whether this repository holds the tag. False lists a name only a
    /// remote has — the sidebar is where it can be read at all, since no
    /// local ref puts it on a graph row.
    pub here: bool,
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
    /// Answer to [`RepoSession::check_signature`].
    SignatureChecked {
        oid: String,
        signature: identity::Signature,
    },
    /// Answer to [`RepoSession::load_head_commit`] — what an amend starts
    /// from: HEAD's message, and whose commit it is about to replace. All
    /// empty on an unborn branch.
    HeadCommitLoaded {
        head: commit::HeadCommit,
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
        /// Fingerprint of the bytes `patches` was parsed from. Hunk/line
        /// selections carry it back, so a partial write can refuse a diff
        /// that drifted under the selection (`stage::apply_partial`).
        fingerprint: u64,
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
    /// A branch move would leave commits unreachable, so it was not made.
    /// Nothing changed; the UI asks before running it for real
    /// ([`RepoSession::checkout`] with [`CheckoutTarget::ForceCreate`]).
    MoveNeedsAsk {
        local: String,
        start: String,
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
    /// A git subprocess was spawned (command log). Only what the user
    /// asked for, unless background reads were switched on.
    CommandStarted {
        id: u64,
        /// The command as a log line shows it.
        display: String,
        /// The same command with the always-applied configuration and
        /// environment spelled out, for copying.
        full: String,
        /// Wall clock at the spawn, milliseconds since the epoch.
        at_ms: i64,
    },
    /// The command with this id ended.
    CommandFinished {
        id: u64,
        end: CommandEnd,
        elapsed_ms: u64,
        /// git's own output on the way out (stderr), or the reason it
        /// never ran. Empty when it said nothing.
        message: String,
    },
}

/// Turns invocations into session events, so the command log travels the
/// same path as everything else the UI shows.
struct CommandFeed {
    sink: Arc<dyn SessionSink>,
    next_id: AtomicU64,
    /// Off by default: a poll tick runs five commands and would bury the
    /// operations the user actually performed.
    record_background: std::sync::atomic::AtomicBool,
}

/// Id of a command that is not being recorded; its end is dropped too.
const UNRECORDED: u64 = 0;

impl CommandFeed {
    fn new(sink: Arc<dyn SessionSink>) -> Self {
        Self {
            sink,
            next_id: AtomicU64::new(UNRECORDED + 1),
            record_background: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl crate::process::CommandObserver for CommandFeed {
    fn records(&self, user: bool) -> bool {
        user || self.record_background.load(Ordering::Relaxed)
    }

    fn started(&self, display: &str, full: &str, user: bool) -> u64 {
        if !self.records(user) {
            return UNRECORDED;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default();
        self.sink.event(SessionEvent::CommandStarted {
            id,
            display: display.to_string(),
            full: full.to_string(),
            at_ms,
        });
        id
    }

    fn finished(&self, id: u64, end: CommandEnd, elapsed_ms: u64, message: &str) {
        if id == UNRECORDED {
            return;
        }
        self.sink.event(SessionEvent::CommandFinished {
            id,
            end,
            elapsed_ms,
            message: message.to_string(),
        });
    }
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

/// One tick handed to the timer by hand rather than by the clock; the
/// timer answers on it once it has acted (see [`AutoFetchTicker`]).
type AutoFetchTick = tokio::sync::oneshot::Sender<()>;

/// The auto-fetch timer that is running: what stops it, and the way in for
/// a tick that does not come from the clock.
struct AutoFetch {
    cancel: CancellationToken,
    ticks: tokio::sync::mpsc::UnboundedSender<AutoFetchTick>,
}

/// Steps one auto-fetch timer by hand, in place of waiting out its
/// interval.
///
/// Handed out by [`RepoSession::auto_fetch_ticker`] and bound to the timer
/// that was running when it was taken, so a tick reports back whether that
/// timer is still there to take it. That is what makes "this timer fetches
/// nothing any more" something to wait for rather than a wall-clock margin
/// to guess at, which is all the tests have to go on otherwise: a fetch
/// queued a moment before the stop can start much later on a loaded
/// machine, and no length of quiet proves the next one is not coming. The
/// app only ever sets an interval.
pub struct AutoFetchTicker(tokio::sync::mpsc::UnboundedSender<AutoFetchTick>);

impl AutoFetchTicker {
    /// Fires one tick and resolves once the timer has acted on it: `true`
    /// when it took the tick, `false` once that timer has stopped — turned
    /// off, replaced by another interval, or gone with the session. A
    /// stopped timer never takes a tick, not even one that raced its own
    /// cancellation, so `false` is the last word on it.
    pub async fn tick(&self) -> bool {
        let (ack, taken) = tokio::sync::oneshot::channel();
        if self.0.send(ack).is_err() {
            return false;
        }
        taken.await.is_ok()
    }
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
    /// Reads and refreshes: recorded in the command log only while
    /// background recording is on.
    executor: GitExecutor,
    /// The queue's handle: everything run through it is something the
    /// user asked for, and is always recorded.
    exec_user: GitExecutor,
    commands: Arc<CommandFeed>,
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
    /// Fingerprint of the last refs read (see [`refs_key`]), so a refresh
    /// can tell an external commit / fetch / switch from a quiet re-read.
    /// `None` until the first read: opening already streams the graph.
    refs_key: Mutex<Option<u64>>,
    /// Set while the write queue runs a request, so the poll can stay out
    /// of a repository that is mid-operation.
    write_busy: std::sync::atomic::AtomicBool,
    /// One permit, held by a running poll: a tick that arrives while the
    /// previous one is still reading is dropped rather than queued.
    poll_slot: Arc<tokio::sync::Semaphore>,
    /// Submission end of the write queue (see the module docs).
    write_tx: tokio::sync::mpsc::UnboundedSender<WriteRequest>,
    /// Time budget for fetch / push (settings, Phase 4, persist this).
    network_timeout: Mutex<std::time::Duration>,
    /// What each remote last advertised under `refs/tags/`. Empty until a
    /// fetch has been through: asking costs the network, so it rides the
    /// one command the user already meant to spend it on, and before that
    /// every tag reads as one this repository alone has.
    remote_tags: Mutex<RemoteTagsByRemote>,
    /// One permit for the background read of the above, so a second
    /// permission-granting call cannot stack another on top of it.
    remote_tags_slot: Arc<tokio::sync::Semaphore>,
    /// The running auto-fetch timer, if any.
    auto_fetch: Mutex<Option<AutoFetch>>,
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
        let commands = Arc::new(CommandFeed::new(Arc::clone(&sink)));
        let observer: Arc<dyn crate::process::CommandObserver> = Arc::clone(&commands) as _;
        let session = Arc::new(Self {
            executor: executor.observed(Arc::clone(&observer), false),
            exec_user: executor.observed(observer, true),
            commands,
            runtime: runtime.clone(),
            sink,
            root_cancel: CancellationToken::new(),
            info: Mutex::new(None),
            shared: Arc::new(Mutex::new(Shared::default())),
            log_options: Mutex::new(LogOptions::default()),
            log_gen: AtomicU64::new(0),
            log_cancel: Mutex::new(None),
            wip_dirty: std::sync::atomic::AtomicBool::new(false),
            refs_key: Mutex::new(None),
            write_busy: std::sync::atomic::AtomicBool::new(false),
            poll_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            write_tx,
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            remote_tags: Mutex::new(RemoteTagsByRemote::new()),
            remote_tags_slot: Arc::new(tokio::sync::Semaphore::new(1)),
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
                    // Not from `set_auto_fetch`, which the application
                    // calls the instant this session is handed over —
                    // there is no workdir to read from until the line
                    // above, and the interval it installs is what grants
                    // permission to look at all.
                    s.catch_up_remote_tags();
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

    /// Whether the command log also records the reads this session makes
    /// on its own (polling, refreshes, details). Off by default; it
    /// applies to commands spawned from here on, not retroactively.
    pub fn set_record_background(&self, on: bool) {
        self.commands.record_background.store(on, Ordering::Relaxed);
    }

    /// Cancels everything this session is doing. Idempotent.
    pub fn close(&self) {
        self.root_cancel.cancel();
    }

    /// Starts, restarts or stops the periodic `git fetch --prune`.
    ///
    /// `None` (or zero) turns it off, and nothing is queued once it has
    /// returned (a fetch already in the write queue still runs). Only one
    /// fetch is ever outstanding: on a slow link or a repository whose
    /// credential helper is taking its time, a tick that finds the previous
    /// fetch unfinished is skipped rather than queued behind it.
    ///
    /// The clock is not the only way in: [`Self::auto_fetch_ticker`] steps
    /// the timer this starts.
    pub fn set_auto_fetch(self: &Arc<Self>, interval: Option<std::time::Duration>) {
        let mut guard = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(previous) = guard.take() {
            previous.cancel.cancel();
        }
        let Some(interval) = interval.filter(|i| !i.is_zero()) else {
            return;
        };
        let cancel = self.root_cancel.child_token();
        let (ticks, mut by_hand) = tokio::sync::mpsc::unbounded_channel();
        *guard = Some(AutoFetch {
            cancel: cancel.clone(),
            ticks,
        });
        drop(guard);

        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // tokio fires the first tick immediately; opening the
            // repository has just read it, so wait out a full interval.
            ticker.tick().await;
            loop {
                tokio::select! {
                    // Biased: a stop that arrives while ticks are already
                    // overdue (a starved timer catches up in a burst) wins
                    // over them instead of being picked at random.
                    biased;
                    _ = cancel.cancelled() => return,
                    _ = ticker.tick() => {
                        if !s.auto_fetch_tick(&cancel) {
                            return;
                        }
                    }
                    Some(ack) = by_hand.recv() => {
                        // Answered only once the tick has been acted on,
                        // and never by a timer that has been stopped.
                        if !s.auto_fetch_tick(&cancel) {
                            return;
                        }
                        if ack.send(()).is_err() {
                            tracing::debug!("auto fetch tick: nobody waiting for it");
                        }
                    }
                }
            }
        });
        self.catch_up_remote_tags();
    }

    /// Reads what the remotes carry under `refs/tags/` without waiting for
    /// a fetch, when two things are true: automatic fetching is on, and the
    /// graph is showing tags.
    ///
    /// The first is the permission — a repository whose owner turned the
    /// timer off has said not to reach the network unasked, and a badge is
    /// not the thing to break that for. The second is the priority: with
    /// tags out of the walk the chips that carry this reading are not on
    /// screen, and the timer will fill it in within the interval anyway.
    ///
    /// Deliberately **not** on the write queue. A request there sets
    /// `write_busy`, which holds the poll out and puts every later write
    /// behind this one — far too much to spend on a badge. It runs as a
    /// plain background read instead, on the handle that keeps it out of
    /// the command log, and only republishes if the answer moved.
    ///
    /// Entered twice: once the repository is open (the interval is already
    /// installed by then — the application sets it the moment the session
    /// is handed over), and from [`Self::set_auto_fetch`] afterwards, so
    /// granting the permission in settings is itself a reason to look.
    fn catch_up_remote_tags(self: &Arc<Self>) {
        let interval_is_on = match self.auto_fetch.lock() {
            Ok(g) => g.is_some(),
            Err(e) => e.into_inner().is_some(),
        };
        if !interval_is_on || !self.log_options().include_tags {
            return;
        }
        let Ok(permit) = Arc::clone(&self.remote_tags_slot).try_acquire_owned() else {
            tracing::debug!("remote tags: the previous read has not finished");
            return;
        };
        let s = Arc::clone(self);
        let timeout = self.network_timeout();
        self.runtime.spawn(async move {
            let _permit = permit;
            let Some(workdir) = s.workdir() else {
                return;
            };
            let cancel = s.root_cancel.clone();
            let before = s.remote_tag_index();
            s.read_remote_tags(&s.executor, &workdir, None, timeout, &cancel)
                .await;
            if s.remote_tag_index() != before {
                s.refresh_refs();
            }
        });
    }

    /// Handle for stepping the running timer, in place of waiting out its
    /// interval — see [`AutoFetchTicker`]. `None` while auto fetch is off.
    pub fn auto_fetch_ticker(&self) -> Option<AutoFetchTicker> {
        let guard = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        guard.as_ref().map(|a| AutoFetchTicker(a.ticks.clone()))
    }

    /// Queues one automatic fetch, unless the previous one is still going.
    ///
    /// Reported under its own op name: a laptop that is simply offline must
    /// not put a fresh error banner on screen every interval.
    ///
    /// Answers whether the timer is still running. Its token is read under
    /// the lock a stop cancels it under, so the two cannot interleave: a
    /// tick either has its fetch in the write queue before `set_auto_fetch`
    /// returns, or sees the stop and queues nothing. Turning it off leaves
    /// nothing still to come.
    fn auto_fetch_tick(self: &Arc<Self>, cancel: &CancellationToken) -> bool {
        let _stop = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if cancel.is_cancelled() {
            return false;
        }
        let Ok(permit) = Arc::clone(&self.auto_fetch_slot).try_acquire_owned() else {
            tracing::debug!("auto fetch skipped: the previous one has not finished");
            return true;
        };
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            AUTO_FETCH_OP,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let _permit = permit;
                s.fetch_and_read_tags(&exec, &repo.workdir, None, timeout, &cancel)
                    .await
            },
        );
        true
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

    /// The periodic re-read that runs while the repository is on screen:
    /// refs and status only. Stashes and worktrees ride the focus and
    /// post-write refreshes instead — two more processes every tick to
    /// catch what a poll practically never sees move on its own.
    ///
    /// Skipped while a write runs (that repository is mid-operation, and
    /// the write refreshes when it lands) and while the previous poll is
    /// still going, so a slow repository polls less often instead of
    /// stacking reads up.
    pub fn refresh_poll(self: &Arc<Self>) {
        if self.write_busy.load(Ordering::SeqCst) {
            tracing::trace!("poll skipped: a write is running");
            return;
        }
        let Ok(permit) = Arc::clone(&self.poll_slot).try_acquire_owned() else {
            tracing::trace!("poll skipped: the previous one has not finished");
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            // Both reads can call for a rebuild, but the graph is one
            // picture: an external commit moves a ref *and* cleans the
            // tree, and walking twice would throw one pass away.
            let (refs_moved, wip_flipped) = tokio::join!(s.publish_refs(), s.publish_status());
            if refs_moved || wip_flipped {
                s.refresh_log();
            }
        });
    }

    pub fn refresh_refs(self: &Arc<Self>) {
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            if s.publish_refs().await {
                s.refresh_log();
            }
        });
    }

    /// Reads refs and HEAD, publishes the snapshot and the label diff, and
    /// reports whether the ref layout moved since the last read.
    ///
    /// Chips alone are applied without rebuilding, but a moved ref means
    /// commits the graph has never seen (an external commit, a fetch, a
    /// switch), and those only appear if the walk runs again.
    ///
    /// Does not rebuild the graph itself: a caller that reads status in the
    /// same pass rebuilds once for both (see [`RepoSession::refresh_poll`]).
    async fn publish_refs(self: &Arc<Self>) -> bool {
        let Some(workdir) = self.workdir() else {
            return false;
        };
        let op_gen = self.refs_gate.begin();
        let cancel = self.root_cancel.clone();
        let refs = refs::load(&self.executor, &workdir, &cancel).await;
        let head = refs::head_state(&self.executor, &workdir, &cancel).await;
        // A repository with no remotes is normal, and so is a failure
        // to read the list; neither is a reason to lose the refs.
        let remotes = remote::list(&self.executor, &workdir, &cancel)
            .await
            .unwrap_or_default();
        match (refs, head) {
            (Ok(refs), Ok(head)) => {
                if !self.refs_gate.is_current(op_gen) {
                    return false;
                }
                let key = refs_key(&refs, &head);
                let previous = self
                    .refs_key
                    .lock()
                    .map(|mut slot| slot.replace(key))
                    .unwrap_or_default();
                let remote_tags = self.remote_tag_index();
                let mut snapshot = build_snapshot(&refs, &head, &remote_tags);
                snapshot.remote_names = remotes.into_iter().map(|r| r.name).collect();
                let label_updates = self.apply_refs(&refs, &head);
                self.sink.event(SessionEvent::RefsLoaded { snapshot });
                if !label_updates.is_empty() {
                    self.sink.event(SessionEvent::LabelsChanged {
                        rows: label_updates,
                    });
                }
                previous.is_some_and(|previous| previous != key)
            }
            (Err(e), _) | (_, Err(e)) => {
                self.fail("refs", e);
                false
            }
        }
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

    /// Reads HEAD's message and author so an amend can start from them.
    ///
    /// On demand rather than with every refresh: only the amend path wants
    /// them, and a repository refresh already runs several commands.
    pub fn load_head_commit(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            // An unborn branch has no HEAD to amend; that is a state, not a
            // failure worth an error banner.
            let head = commit::head_commit(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
            s.sink.event(SessionEvent::HeadCommitLoaded { head });
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
            // Set around the whole request, refreshes included, so the
            // poll keeps out until the write's own refresh has landed.
            self.write_busy.store(true, Ordering::SeqCst);
            self.run_write(request).await;
            self.write_busy.store(false, Ordering::SeqCst);
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
        // Auto fetch travels this queue too, but nobody asked for it: it
        // stays out of the command log unless background reads are on,
        // and so cannot make an offline laptop raise the panel every
        // interval.
        let exec = if op == AUTO_FETCH_OP {
            self.executor.clone()
        } else {
            self.exec_user.clone()
        };
        let result = run(exec, info, cancel).await;
        let rebuild_graph = match result {
            Ok(()) => {
                self.sink
                    .event(SessionEvent::WriteFinished { op, error: None });
                after == AfterWrite::Graph
            }
            // Cancelled means the session is closing, but the event pair
            // must still balance: the UI counts Started/Finished to know
            // whether a write is in flight, and an unmatched start would
            // pin that count for good.
            Err(error) if error.is_cancelled() => {
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: Some(error.to_string()),
                });
                return;
            }
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

        // Settle the working tree and the refs before touching the graph:
        // the WIP row exists only while the tree is dirty and a write that
        // lands a commit moves a ref, so rebuilding first and then reacting
        // to either would walk the whole history twice for one write.
        let (wip_flipped, refs_moved) = tokio::join!(self.publish_status(), self.publish_refs());
        if rebuild_graph || wip_flipped || refs_moved {
            // Off-screen rebuild: the pane keeps showing the old graph
            // until the finished one swaps in (or nothing changed and
            // nothing repaints — the auto-fetch common case).
            self.refresh_log();
        }
        self.refresh_stashes();
        self.refresh_worktrees();
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

    /// `git add --all` for the whole work tree, untracked included.
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

    /// Throws away both sides of a path at once, back to HEAD: what is
    /// staged and what is on disk. A rename must be given both of its
    /// names (see [`stage::discard_to_head`]).
    pub fn discard_paths_to_head(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "discard",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::discard_to_head(&exec, &repo.workdir, &paths, &cancel).await
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

    /// Throws away part of one file's unstaged diff (hunk / line level).
    /// The index keeps what is staged (see [`stage::discard_partial`]).
    pub fn discard_partial(
        self: &Arc<Self>,
        target: DiffTarget,
        selects: Vec<HunkSelect>,
        seen: u64,
    ) {
        self.write(
            "discard",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::discard_partial(&exec, &repo, &target, &selects, seen, &cancel).await
            },
        );
    }

    /// Stages or unstages part of one file's diff (hunk / line level).
    /// `seen` is the fingerprint the selection's diff arrived with
    /// ([`SessionEvent::DiffLoaded`]).
    pub fn apply_partial(
        self: &Arc<Self>,
        target: DiffTarget,
        selects: Vec<HunkSelect>,
        seen: u64,
    ) {
        self.write(
            "stage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::apply_partial(&exec, &repo, &target, &selects, seen, &cancel).await
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
                let outcome = branch::checkout(&exec, &repo.workdir, &target, &cancel).await?;
                session.report_move(outcome);
                Ok(())
            },
        );
    }

    /// Moves a local branch onto `start` and lands on it, asking first only
    /// when there is something to ask about.
    ///
    /// Landing on a branch that has fallen behind is the everyday case and
    /// loses nothing: every commit it has is already reachable from where
    /// it is going, so the move is a fast-forward and simply happens. Only
    /// where the branch holds commits `start` does not — the case the
    /// question's own words describe — does this stop and emit
    /// [`SessionEvent::MoveNeedsAsk`] without touching anything.
    ///
    /// The check is a read, so a refusal here has nothing to undo.
    pub fn checkout_moving_branch(self: &Arc<Self>, local: String, start: String) {
        let session = Arc::clone(self);
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                if !branch::is_merged_into(&exec, &repo.workdir, &local, &start, &cancel).await? {
                    session
                        .sink
                        .event(SessionEvent::MoveNeedsAsk { local, start });
                    return Ok(());
                }
                let target = CheckoutTarget::ForceCreate { local, start };
                let outcome = branch::checkout(&exec, &repo.workdir, &target, &cancel).await?;
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
    ///
    /// A move refused after all leaves neither half standing: the entry was
    /// only ever the room the switch needed, so it goes back rather than
    /// parking the work on the branch it never left.
    pub fn checkout_stashing(self: &Arc<Self>, target: CheckoutTarget) {
        let session = Arc::clone(self);
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let stashed = stash_everything(&exec, &repo, &cancel).await?;
                let outcome = match branch::checkout(&exec, &repo.workdir, &target, &cancel).await {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        if stashed {
                            pop_back_after_failed_switch(&exec, &repo, &cancel).await;
                        }
                        return Err(error);
                    }
                };
                if stashed && matches!(outcome, branch::CheckoutOutcome::Blocked(_)) {
                    stash::pop(&exec, &repo.workdir, STASH_TOP, &cancel).await?;
                }
                session.report_move(outcome);
                Ok(())
            },
        );
    }

    /// Carries the working tree across the move — "bring my changes and
    /// let me sort out the overlap".
    ///
    /// Stash, move, put back: the restore is a merge, so the changes land
    /// on top of what the target has and only the parts git cannot combine
    /// need settling. Going through the stash rather than `switch --merge`
    /// buys two things that flag cannot give — **the staged/unstaged split
    /// survives** (`--index`), and a conflict **keeps the stash entry**, so
    /// the work still exists somewhere other than a marked-up file.
    ///
    /// A conflicting restore exits non-zero while having done exactly what
    /// was asked, so the exit code alone cannot judge it: the working tree
    /// decides. Unmerged paths mean the merge landed and is waiting to be
    /// settled; a clean tree means the restore did nothing, and then the
    /// split has to be given up on (see below) or git's message goes
    /// through.
    pub fn checkout_merging(self: &Arc<Self>, target: CheckoutTarget) {
        let session = Arc::clone(self);
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                // With nothing of ours stashed there is nothing to bring
                // across, and the move alone is the whole job.
                if !stash_everything(&exec, &repo, &cancel).await? {
                    let outcome = branch::checkout(&exec, &repo.workdir, &target, &cancel).await?;
                    session.report_move(outcome);
                    return Ok(());
                }
                let outcome = match branch::checkout(&exec, &repo.workdir, &target, &cancel).await {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        pop_back_after_failed_switch(&exec, &repo, &cancel).await;
                        return Err(error);
                    }
                };
                if let branch::CheckoutOutcome::Blocked(_) = outcome {
                    // Nothing should stand in the way of a tree that was
                    // just emptied; put the work back rather than leave it
                    // stashed behind a refusal.
                    stash::pop(&exec, &repo.workdir, STASH_TOP, &cancel).await?;
                    session.report_move(outcome);
                    return Ok(());
                }

                let kept_index =
                    stash::pop_with_index(&exec, &repo.workdir, STASH_TOP, &cancel).await;
                if kept_index.is_ok() || conflicts_now(&exec, &repo, &cancel).await? {
                    return Ok(());
                }
                // git refuses `--index` outright when the staged half is
                // what collides ("conflicts in index. Try without
                // --index.") and leaves everything where it was. Its own
                // advice is the fallback: restore without the index, which
                // brings the changes across merged and gives up only on
                // the staged/unstaged split.
                match stash::pop(&exec, &repo.workdir, STASH_TOP, &cancel).await {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        if conflicts_now(&exec, &repo, &cancel).await? {
                            Ok(())
                        } else {
                            Err(error)
                        }
                    }
                }
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

    /// Moves the current branch to `rev`, carrying the index and the
    /// working tree as far as `mode` says. `ResetMode::Hard` destroys
    /// uncommitted work, so the UI asks before sending that one.
    pub fn reset(self: &Arc<Self>, rev: String, mode: branch::ResetMode) {
        self.write(
            "reset",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::reset(&exec, &repo.workdir, &rev, mode, &cancel).await
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

    /// Renames a tag: a new name on the same object, then the old name
    /// dropped (git has no rename of its own — see [`crate::tag`]).
    pub fn rename_tag(self: &Arc<Self>, from: String, to: String) {
        self.write(
            "tag",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::rename(&exec, &repo.workdir, &from, &to, &cancel).await
            },
        );
    }

    /// Deletes a tag. Destructive in one way only: what it marked may
    /// have nothing else reaching it, so the UI asks first.
    pub fn delete_tag(self: &Arc<Self>, name: String) {
        self.write(
            "tag",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::delete(&exec, &repo.workdir, &name, &cancel).await
            },
        );
    }

    /// Renames a stash entry — stored again under the new label, old entry
    /// dropped (git has no rename for one — see [`crate::stash::rename`]).
    pub fn rename_stash(self: &Arc<Self>, selector: String, message: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::rename(&exec, &repo.workdir, &selector, &message, &cancel).await
            },
        );
    }

    /// `git stash push`, over the whole working tree or only `paths`.
    pub fn stash_push(
        self: &Arc<Self>,
        message: String,
        options: stash::PushOptions,
        paths: Vec<String>,
    ) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::push(&exec, &repo.workdir, &message, options, &paths, &cancel).await
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
        let s = Arc::clone(self);
        self.write(
            "fetch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                s.fetch_and_read_tags(&exec, &repo.workdir, remote.as_deref(), timeout, &cancel)
                    .await
            },
        );
    }

    /// The fetch, and then the one thing it cannot leave behind: where the
    /// remotes keep their tags.
    ///
    /// A fetched tag lands in `refs/tags/` beside the ones made here, so
    /// afterwards nothing local says which is which. Asking costs a second
    /// round trip, and this is where it belongs — the user has already
    /// agreed to reach the network, and a poll never should.
    async fn fetch_and_read_tags(
        self: &Arc<Self>,
        exec: &GitExecutor,
        workdir: &Path,
        remote_name: Option<&str>,
        timeout: std::time::Duration,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        remote::fetch(exec, workdir, remote_name, timeout, cancel).await?;
        self.read_remote_tags(exec, workdir, remote_name, timeout, cancel)
            .await;
        Ok(())
    }

    /// Records what each remote advertises under `refs/tags/`.
    ///
    /// Reports nothing upwards. A badge is not worth failing a fetch that
    /// worked, and a remote that could not be reached keeps the answer it
    /// last gave instead of dropping every cloud it accounted for — which
    /// is what `refs/remotes/` does for branches on its own.
    async fn read_remote_tags(
        &self,
        exec: &GitExecutor,
        workdir: &Path,
        only: Option<&str>,
        timeout: std::time::Duration,
        cancel: &CancellationToken,
    ) {
        let remotes = match remote::list(exec, workdir, cancel).await {
            Ok(list) => list,
            Err(error) => {
                tracing::debug!(%error, "remote tags: the remotes could not be listed");
                return;
            }
        };
        // A remote that is no longer configured stops answering for names.
        self.lock_remote_tags()
            .retain(|name, _| remotes.iter().any(|r| r.name == *name));
        for r in remotes {
            if only.is_some_and(|wanted| wanted != r.name) {
                continue;
            }
            match remote::list_tags(exec, workdir, &r.name, timeout, cancel).await {
                Ok(tags) => {
                    self.lock_remote_tags().insert(r.name, tags);
                }
                Err(error) if error.is_cancelled() => return,
                Err(error) => {
                    tracing::debug!(remote = %r.name, %error, "remote tags: unreadable");
                }
            }
        }
    }

    fn lock_remote_tags(&self) -> std::sync::MutexGuard<'_, RemoteTagsByRemote> {
        match self.remote_tags.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// The per-remote answers merged into the index the join reads.
    fn remote_tag_index(&self) -> RemoteTagIndex {
        let mut index = RemoteTagIndex::new();
        for (remote, tags) in self.lock_remote_tags().iter() {
            for tag in tags {
                let place: &mut RemoteTagPlace = index
                    .entry(tag.name.clone())
                    .or_default()
                    .entry(tag.commit)
                    .or_default();
                place.annotated |= tag.annotated;
                place.remotes.push(remote.clone());
            }
        }
        index
    }

    /// `git push` for one branch.
    pub fn push(self: &Arc<Self>, spec: remote::PushSpec) {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let target = spec.remote.clone();
                let result = remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await;
                s.catch_up_after(&result, target);
                result
            },
        );
    }

    /// Fetches when a push was refused for knowing the remote only as it
    /// used to be, and does nothing otherwise.
    ///
    /// It is the one refusal with an answer: the fetch is what shows which
    /// commits the remote actually holds — the graph then draws both sides,
    /// so what an overwrite would remove can be seen rather than described —
    /// and it is also what re-arms the lease, which is pinned to a commit
    /// the remote has left and would be turned down again as it stands.
    ///
    /// Queued rather than run here, so it reports and refreshes like any
    /// other fetch. The push still fails: nothing is retried, and the next
    /// move is whoever is looking at it to make.
    fn catch_up_after(self: &Arc<Self>, result: &Result<(), GitError>, remote: String) {
        if matches!(result, Err(GitError::PushOutdated { .. })) {
            self.fetch(Some(remote));
        }
    }

    /// Pushes the branch that is checked out to wherever it belongs.
    ///
    /// Resolving the target is part of the job rather than something the
    /// UI works out: which remote a branch tracks lives in configuration,
    /// and a name like `origin/main` cannot be split back apart reliably.
    pub fn push_current(self: &Arc<Self>, fallback_remote: String, force: remote::PushForce) {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
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
                let target = spec.remote.clone();
                let result = remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await;
                s.catch_up_after(&result, target);
                result
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

    /// Renames a branch on a remote, which git does as a push and a delete
    /// (see [`remote::rename_remote_branch`]). The UI asks first: the old
    /// name is destroyed, not moved.
    pub fn rename_remote_branch(self: &Arc<Self>, remote_name: String, from: String, to: String) {
        let timeout = self.network_timeout();
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::rename_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &from,
                    &to,
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

    /// Asks whether `oid` carries a signature and what git makes of it.
    ///
    /// Kept out of [`Self::load_details`] on purpose: verifying runs gpg or
    /// ssh-keygen, and the details pane has a 100ms budget. The answer
    /// arrives on its own, after the commit is already on screen.
    pub fn check_signature(self: &Arc<Self>, oid: Oid) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match identity::verify_commit(&s.executor, &workdir, &oid.to_hex(), &cancel).await {
                Ok(signature) => s.sink.event(SessionEvent::SignatureChecked {
                    oid: oid.to_hex(),
                    signature,
                }),
                Err(e) => s.fail("signature", e),
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
            match details::file_diff_with_fingerprint(&s.executor, &workdir, &target, &cancel).await
            {
                Ok((patches, fingerprint)) => {
                    let is_binary = patches.iter().any(|p| p.is_binary);
                    let preview =
                        preview::file_preview(&s.executor, &workdir, &target, is_binary, &cancel)
                            .await;
                    s.sink.event(SessionEvent::DiffLoaded {
                        target,
                        patches,
                        preview,
                        fingerprint,
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
    /// sink (used by every offscreen rebuild — the tag-inclusive swap
    /// pass and `refresh_log`'s background refreshes). Returns the number
    /// of commits the walk emitted (what `--max-count` limits — the shown
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
        let label_map = build_label_map(refs, head, &self.remote_tag_index());
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

/// The entry a [`stash_everything`] just made, for the moves that put it
/// back.
const STASH_TOP: &str = "stash@{0}";

/// Stashes the whole working tree out of a move's way, and answers whether
/// an entry of ours was really made.
///
/// A clean tree stashes nothing while exiting 0 — the refusal that raised
/// the question can go stale when the tree is cleaned from a terminal in
/// between. With no entry of ours, [`STASH_TOP`] names somebody else's
/// work and must not be touched.
async fn stash_everything(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let options = stash::PushOptions {
        include_untracked: true,
        keep_index: false,
        staged_only: false,
    };
    let before = stash::tip(executor, &repo.workdir, cancel).await?;
    stash::push(executor, &repo.workdir, "", options, &[], cancel).await?;
    Ok(stash::tip(executor, &repo.workdir, cancel).await? != before)
}

/// Best-effort restore after a switch that failed outright (an `Err`,
/// not a `Blocked` refusal — a refusal git words in a way
/// `CheckoutBlock` does not know arrives here). The switch did nothing,
/// so the stash was only the room it needed: put the work back before
/// the caller surfaces the switch's own error. If even the pop fails,
/// that is logged and the entry stays in the stash list, where the work
/// is still recoverable — the switch error is the one worth showing.
async fn pop_back_after_failed_switch(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) {
    if let Err(error) = stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await {
        tracing::warn!(
            %error,
            "the switch failed and the stashed work could not be popped back; \
             it remains in the stash list"
        );
    }
}

/// Whether the working tree has unmerged paths right now.
///
/// What a restore leaves behind is the only honest answer to "did that
/// non-zero exit do anything": `git stash pop` reports a conflict and a
/// refusal the same way.
async fn conflicts_now(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    Ok(status::load(executor, &repo.workdir, cancel)
        .await?
        .has_conflicts())
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

/// Builds the per-commit label chips from a refs listing and what the
/// remotes carry under `refs/tags/`.
///
/// A branch and the remote it is about get one chip between them: the
/// cloud badge already says the remote is here, so the remote's own label
/// is dropped (see [`refs::remotes_folded_into_local`]).
///
/// Tags fold on the same terms, but only when both sides point at the same
/// commit. One that points elsewhere over there gets a label of its own on
/// the row it is really on, so the same name stands on two rows — the whole
/// of that signal, since a fetch never resolves the disagreement (measured:
/// `--prune` leaves the local tag silently) and it has to keep showing.
fn build_label_map(
    refs: &[RefEntry],
    head: &HeadState,
    remote_tags: &RemoteTagIndex,
) -> HashMap<Oid, Vec<RefLabel>> {
    let with_remote = refs::branches_with_remote(refs);
    let folded = refs::remotes_folded_into_local(refs);
    let mut map: HashMap<Oid, Vec<RefLabel>> = HashMap::new();
    for r in refs {
        if r.kind == RefKind::RemoteBranch && folded.contains(&r.name) {
            continue;
        }
        let kind = match r.kind {
            RefKind::LocalBranch => LabelKind::LocalBranch,
            RefKind::RemoteBranch => LabelKind::RemoteBranch,
            RefKind::Tag => LabelKind::Tag,
        };
        let has_remote = match r.kind {
            RefKind::LocalBranch => with_remote.contains(&r.name),
            RefKind::Tag => remote_tags.contains_key(&r.short),
            RefKind::RemoteBranch => false,
        };
        map.entry(r.commit_oid()).or_default().push(RefLabel {
            text: r.short.clone(),
            kind,
            has_remote,
            is_head: r.is_head,
            here: r.kind != RefKind::RemoteBranch,
            remote: String::new(),
        });
    }
    for (name, commits) in remote_tags {
        let local = refs
            .iter()
            .find(|r| r.kind == RefKind::Tag && r.short == *name)
            .map(RefEntry::commit_oid);
        for (oid, place) in commits {
            // Where the two agree there is one tag to speak of, and the
            // local label is already carrying its cloud.
            if local == Some(*oid) {
                continue;
            }
            map.entry(*oid).or_default().push(RefLabel {
                text: name.clone(),
                kind: LabelKind::Tag,
                has_remote: true,
                is_head: false,
                here: false,
                remote: place.remotes.join(", "),
            });
        }
    }
    if head.detached
        && let Some(oid) = head.oid
    {
        map.entry(oid).or_default().push(RefLabel {
            text: "HEAD".to_string(),
            kind: LabelKind::Head,
            has_remote: false,
            is_head: true,
            here: true,
            remote: String::new(),
        });
    }
    for labels in map.values_mut() {
        labels.sort_by(|a, b| {
            (!a.is_head, a.kind, a.text.as_str()).cmp(&(!b.is_head, b.kind, b.text.as_str()))
        });
    }
    map
}

/// Fingerprints where every ref points, so two reads can be compared
/// without keeping the listing around.
///
/// Only what moves the walk counts: a renamed upstream or a changed sort
/// date redraws chips through the label diff, and rebuilding for those
/// would repaint the graph over nothing. `git for-each-ref` lists in
/// refname order, so equal layouts hash equal.
fn refs_key(refs: &[RefEntry], head: &HeadState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for entry in refs {
        entry.name.hash(&mut hasher);
        entry.target.hash(&mut hasher);
        entry.peeled.hash(&mut hasher);
    }
    head.branch.hash(&mut hasher);
    head.oid.hash(&mut hasher);
    head.detached.hash(&mut hasher);
    hasher.finish()
}

/// Builds the sorted sidebar snapshot.
///
/// Tags a remote has and this repository does not are listed too: no local
/// ref puts them on a graph row, so the sidebar is the only place they can
/// be read at all. Where a name exists on both sides it is listed once —
/// the sidebar is a list of names to act on, and which commits the two
/// sides point at is what the graph rows are for.
fn build_snapshot(
    refs: &[RefEntry],
    head: &HeadState,
    remote_tags: &RemoteTagIndex,
) -> RefsSnapshot {
    let with_remote = refs::branches_with_remote(refs);
    let remotes: Vec<&RefEntry> = refs
        .iter()
        .filter(|r| r.kind == RefKind::RemoteBranch)
        .collect();
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
                upstream: refs::spoken_for_remote(r, &remotes)
                    .map(|u| u.short.clone())
                    .unwrap_or_default(),
            }),
            RefKind::RemoteBranch => snapshot.remotes.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid_hex: r.commit_oid().to_hex(),
                has_remote: true,
                is_head: false,
                upstream: String::new(),
            }),
            RefKind::Tag => snapshot.tags.push(TagItem {
                short: r.short.clone(),
                oid_hex: r.commit_oid().to_hex(),
                annotated: r.peeled.is_some(),
                created_unix: r.created_unix,
                has_remote: remote_tags.contains_key(&r.short),
                here: true,
            }),
        }
    }
    for (name, commits) in remote_tags {
        if snapshot.tags.iter().any(|t| t.short == *name) {
            continue;
        }
        // Remotes that disagree about a name still name one tag, and the
        // sidebar answers "does this name exist" rather than "where".
        let Some((oid, place)) = commits.iter().next() else {
            continue;
        };
        snapshot.tags.push(TagItem {
            short: name.clone(),
            oid_hex: oid.to_hex(),
            annotated: place.annotated,
            // An advertisement carries no date; these sort last, after
            // every tag whose creation this repository can see.
            created_unix: 0,
            has_remote: true,
            here: false,
        });
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
