//! The session handle itself: what one open repository holds, how it is
//! opened, and what it cancels on the way out.

use super::*;
use crate::repo;

pub struct RepoSession {
    /// Reads and refreshes: recorded in the command log only while
    /// background recording is on.
    pub(super) executor: GitExecutor,
    /// The queue's handle: everything run through it is something the
    /// user asked for, and is always recorded.
    pub(super) exec_user: GitExecutor,
    pub(super) commands: Arc<CommandFeed>,
    pub(super) runtime: tokio::runtime::Handle,
    pub(super) sink: Arc<dyn SessionSink>,
    /// Cancelled when the session closes; all ops derive from it.
    pub(super) root_cancel: CancellationToken,
    pub(super) info: Mutex<Option<RepoInfo>>,
    pub(super) shared: Arc<Mutex<Shared>>,
    pub(super) log_options: Mutex<LogOptions>,
    pub(super) log_gen: AtomicU64,
    pub(super) log_cancel: Mutex<Option<CancellationToken>>,
    /// Which diff read is the current one. Bumped by every
    /// [`RepoSession::load_diff`], and read again just before the colours
    /// for that diff would be worked out: a reader going down a commit's
    /// file list starts a read per row, and colouring costs enough (see
    /// [`SessionEvent::DiffColoured`]) that the ones nobody is waiting for
    /// any more are worth not doing at all. The rows are unaffected —
    /// those are cheap, and a stale one is dropped by the pane on arrival.
    pub(super) diff_epoch: AtomicU64,
    /// Dirty working tree → the log stream prepends a synthetic WIP row.
    pub(super) wip_dirty: std::sync::atomic::AtomicBool,
    /// Set by [`RepoSession::ask_merge_tool`] to have the next status read
    /// name the merge tool even with nothing conflicted. Cleared by that
    /// read: two `git config` spawns on every poll of every open tab is
    /// not a price the common case should pay for a settings field.
    pub(super) merge_tool_wanted: std::sync::atomic::AtomicBool,
    /// The last answer, repeated by refreshes that did not read it.
    pub(super) merge_tool_seen: Mutex<String>,
    /// Line-ending baselines already sampled, keyed by (directory,
    /// extension) — what a house style is scoped to, and what makes the
    /// second file opened in a directory cost nothing.
    ///
    /// `None` is a cached "unknown", which is worth keeping: not knowing
    /// costs the same reads as knowing. Emptied whenever a write lands or
    /// refs move, since either can bring a new `.gitattributes` or change
    /// what the neighbours look like.
    pub(super) eol_baselines: Mutex<HashMap<(String, String), Option<crate::eol::Baseline>>>,
    /// Whether git normalises line endings here (`core.autocrlf`).
    pub(super) eol_normalises: Derived<bool>,
    /// What remotes are configured. Read on every refs listing before
    /// this, which is a process per poll tick for an answer that only a
    /// write moves.
    pub(super) remotes: Derived<Vec<remote::Remote>>,
    /// How the config file looked when the remotes above were last read
    /// (see [`RepoSession::forget_remotes_if_config_moved`]). `None`
    /// until the first look.
    pub(super) config_stamp: Mutex<Option<ConfigStamp>>,
    /// Pending paths whose change has something to say about line endings,
    /// repeated by every status read until something asks for them again.
    pub(super) eol_marks: Mutex<Arc<Vec<EolMark>>>,
    /// Set when the marks are worth re-reading whatever status says: a
    /// write landed, or the tab has only just opened.
    pub(super) eol_marks_stale: std::sync::atomic::AtomicBool,
    /// Fingerprint of the last status read, so a tick that finds the same
    /// files in the same states reads no diffs at all.
    pub(super) status_key: Mutex<Option<u64>>,
    /// Fingerprint of the last refs read (see [`refs_key`]), so a refresh
    /// can tell an external commit / fetch / switch from a quiet re-read.
    /// `None` until the first read: opening already streams the graph.
    pub(super) refs_key: Mutex<Option<u64>>,
    /// The same for everything the joins read, which is more than the
    /// listing: `refs_key` moving means the history is walked again, this
    /// moving means only that the snapshot and the chips are rebuilt.
    pub(super) join_key: Mutex<Option<u64>>,
    /// The snapshot last published. A read that finds nothing moved hands
    /// this one out again rather than an equal copy, so the sidebar can
    /// tell "the same" from "equal" by pointer — a repository with tens of
    /// thousands of tags must not rebuild every section, on the Qt thread,
    /// to discover that a poll tick changed nothing.
    pub(super) last_snapshot: Mutex<Option<Arc<RefsSnapshot>>>,
    /// Set while the write queue runs a request, so the poll can stay out
    /// of a repository that is mid-operation.
    pub(super) write_busy: std::sync::atomic::AtomicBool,
    /// One permit, held by a running poll: a tick that arrives while the
    /// previous one is still reading is dropped rather than queued.
    pub(super) poll_slot: Arc<tokio::sync::Semaphore>,
    /// One in flight per snapshot, for the reads a repository can be asked
    /// for from more than one place at once (see [`ReadSlot`]).
    pub(super) refs_read: ReadSlot,
    pub(super) status_read: ReadSlot,
    pub(super) stash_read: ReadSlot,
    pub(super) worktrees_read: ReadSlot,
    /// Submission end of the write queue (see the module docs).
    pub(super) write_tx: tokio::sync::mpsc::UnboundedSender<WriteRequest>,
    /// Time budget for fetch / push. Persisted as the settings key
    /// `network_timeout_secs`; only the settings dialog's input field is
    /// missing (実装計画 §7).
    pub(super) network_timeout: Mutex<std::time::Duration>,
    /// What the remotes last advertised under `refs/tags/`, merged into
    /// the shape the joins read. Empty until a fetch has been through:
    /// asking costs the network, so it rides the one command the user
    /// already meant to spend it on, and before that every tag reads as
    /// one this repository alone has. Rebuilt only
    /// when a remote has spoken. Shared because every refs read wants it
    /// and none of them changes it: merging tens of thousands of names on
    /// every poll tick copies the whole tag list for nothing
    /// (`JetBrains/kotlin`: 45,782 names).
    pub(super) remote_tag_index: Mutex<Arc<RemoteTagIndex>>,
    /// Bumped whenever the index above became different readings, so the
    /// refs key can cover it without walking 45,909 entries.
    pub(super) remote_tag_gen: AtomicU64,
    /// One permit for the background read of the above, so a second
    /// permission-granting call cannot stack another on top of it.
    pub(super) remote_tags_slot: Arc<tokio::sync::Semaphore>,
    /// What the last refs read saw of the branch tip, so the walk behind
    /// [`reachable`] can be started without reading the listing again.
    pub(super) head_hold: Mutex<Option<HeadHold>>,
    /// The same read's answer to "what commit is HEAD on", kept where the
    /// graph walk can reach it: `None` until a refs read has landed,
    /// `Some(None)` for a branch with no commits yet.
    ///
    /// Two levels because the walk has to tell "nobody has looked" from
    /// "looked, and there is nothing there" — they take opposite actions.
    /// Separate from [`Self::head_hold`], whose own `None` already means
    /// the second of those.
    pub(super) head_tip: Mutex<Option<Option<Oid>>>,
    /// The last answer sent, so a re-check landing on the same one says
    /// nothing.
    pub(super) head_reach_seen: Mutex<Option<bool>>,
    /// One permit: the walk is the only part of a refresh that scales with
    /// the history rather than the refs, and a tick arriving mid-walk is
    /// dropped rather than stacked.
    pub(super) head_reach_slot: Arc<tokio::sync::Semaphore>,
    /// One merge-tool candidate read at a time. Opening settings twice in
    /// a row must not start a second eight-second walk of the registry.
    pub(super) merge_tools_slot: Arc<tokio::sync::Semaphore>,
    /// The running auto-fetch timer, if any.
    pub(super) auto_fetch: Mutex<Option<AutoFetch>>,
    /// The interval the timer was last *asked* for, kept while it is
    /// suspended so there is something to put back.
    pub(super) auto_fetch_interval: Mutex<Option<std::time::Duration>>,
    /// One permit: an auto fetch that is still queued or running holds it,
    /// so a tick that arrives meanwhile is skipped instead of stacking up.
    /// A permit moved into a dropped request is released with it.
    pub(super) auto_fetch_slot: Arc<tokio::sync::Semaphore>,
    /// Where the opening's own fetch stands (see [`OpenFetchState`]).
    pub(super) open_fetch: Mutex<OpenFetchState>,
    pub(super) refs_gate: OpGate,
    pub(super) status_gate: OpGate,
    pub(super) stash_gate: OpGate,
    pub(super) worktrees_gate: OpGate,
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
            diff_epoch: AtomicU64::new(0),
            log_cancel: Mutex::new(None),
            wip_dirty: std::sync::atomic::AtomicBool::new(false),
            merge_tool_wanted: std::sync::atomic::AtomicBool::new(false),
            merge_tool_seen: Mutex::new(String::new()),
            eol_baselines: Mutex::new(HashMap::new()),
            eol_normalises: Derived::default(),
            remotes: Derived::default(),
            eol_marks: Mutex::new(Arc::new(Vec::new())),
            eol_marks_stale: std::sync::atomic::AtomicBool::new(true),
            status_key: Mutex::new(None),
            refs_key: Mutex::new(None),
            join_key: Mutex::new(None),
            last_snapshot: Mutex::new(None),
            write_busy: std::sync::atomic::AtomicBool::new(false),
            poll_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            refs_read: ReadSlot::default(),
            status_read: ReadSlot::default(),
            stash_read: ReadSlot::default(),
            worktrees_read: ReadSlot::default(),
            write_tx,
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            config_stamp: Mutex::new(None),
            remote_tag_index: Mutex::new(Arc::new(RemoteTagIndex::default())),
            remote_tag_gen: AtomicU64::new(0),
            remote_tags_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            head_hold: Mutex::new(None),
            head_tip: Mutex::new(None),
            head_reach_seen: Mutex::new(None),
            head_reach_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            merge_tools_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            auto_fetch: Mutex::new(None),
            auto_fetch_interval: Mutex::new(None),
            auto_fetch_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            open_fetch: Mutex::new(OpenFetchState::Unasked),
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
                    let workdir = info.workdir.clone();
                    s.set_info(info.clone());
                    s.sink.event(SessionEvent::Opened { info });
                    // The network before the reads: a round trip is the
                    // longest thing an opening starts, and starting it
                    // first is what lets the reads below run inside it
                    // rather than in front of it. It holds nothing up —
                    // the reads do not wait on the write queue, and what
                    // the fetch brings down is published by its own
                    // refresh rather than read a second time
                    // (`AfterWrite::Graph`).
                    let fetching = s.take_open_fetch(&workdir).await;
                    // Before anything else: a missing identity turns the
                    // first commit into a wall of git text, and the UI can
                    // ask for one instead.
                    s.refresh_author();
                    s.restart_log();
                    s.refresh_quick();
                    // A fetch reads what the remotes carry under
                    // `refs/tags/` on its way out, so only an opening
                    // without one has anything left to ask. Not from
                    // `set_auto_fetch`, which the application calls the
                    // instant this session is handed over — there is no
                    // workdir to read from until the lines above, and the
                    // interval it installs is what grants permission to
                    // look at all.
                    if !fetching {
                        s.catch_up_remote_tags();
                    }
                }
                Err(error) => {
                    if !error.is_cancelled() {
                        s.sink.event(SessionEvent::OpenFailed { path, error });
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

    /// Config file of the opened repository (None until `Opened`).
    ///
    /// Its own path rather than the whole of [`RepoInfo`]: this is read on
    /// every poll tick.
    pub(super) fn config_path(&self) -> Option<PathBuf> {
        self.lock_info().as_ref().map(|i| i.config_path.clone())
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

    // --- internals ------------------------------------------------------

    /// Runs one on-demand read in the background and answers with the
    /// event it produced, or puts `op` on the error surface.
    ///
    /// The shape every such read has: nothing to do before the repository
    /// is open, one git command against the workdir under the session's
    /// cancellation, and exactly one of an event or a failure. Reads that
    /// answer differently — a missing HEAD that is a state rather than a
    /// failure, a refusal that *is* the answer, a read that claims an
    /// epoch before it starts — do not go through here, because folding
    /// them in would change what they report.
    pub(super) fn spawn_read<F, Fut>(self: &Arc<Self>, op: &'static str, read: F)
    where
        F: FnOnce(Arc<Self>, PathBuf, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<SessionEvent, GitError>> + Send,
    {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match read(Arc::clone(&s), workdir, cancel).await {
                Ok(event) => s.sink.event(event),
                Err(e) => s.fail(op, e),
            }
        });
    }

    pub(super) fn fail(&self, op: &'static str, error: GitError) {
        if error.is_cancelled() {
            return;
        }
        tracing::warn!(op, error = %error, "session operation failed");
        self.sink.event(SessionEvent::OpFailed { op, error });
    }

    pub(super) fn set_info(&self, info: RepoInfo) {
        if let Ok(mut guard) = self.info.lock() {
            *guard = Some(info);
        }
    }

    pub(super) fn lock_info(&self) -> std::sync::MutexGuard<'_, Option<RepoInfo>> {
        // A poisoned lock only happens if a holder panicked; the data is a
        // plain snapshot, safe to keep using.
        match self.info.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    pub(super) fn lock_shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        match self.shared.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// What this session is holding on to, part by part.
    ///
    /// Everything named here outlives the operation that filled it: it is
    /// still there when the window is idle, which is what the memory budget
    /// is about. `refs-snapshot` is an `Arc` the sidebar models hold too —
    /// counted in full on both sides, and the app's report says so.
    pub fn heap_report(&self) -> Vec<crate::mem::Part> {
        use crate::mem::{Footprint as _, Part};
        let shared = self.lock_shared();
        let mut parts = vec![
            Part::of("sent-rows", &shared.sent_rows),
            Part::new(
                "label-map",
                shared.label_map.heap_bytes(),
                shared.label_map.len(),
            ),
            Part::new("applied", shared.applied.heap_bytes(), shared.applied.len()),
            Part::new(
                "graph-builder",
                shared.builder.heap_bytes(),
                shared.builder.tracked_oids(),
            ),
        ];
        drop(shared);

        let snapshot = match self.last_snapshot.lock() {
            Ok(g) => g.clone(),
            Err(e) => e.into_inner().clone(),
        };
        let (snap_bytes, snap_refs) = match &snapshot {
            Some(s) => (
                s.heap_bytes(),
                s.locals.len() + s.remotes.len() + s.tags.len(),
            ),
            None => (0, 0),
        };
        parts.push(Part::new("refs-snapshot", snap_bytes, snap_refs));

        let index = self.remote_tag_index();
        parts.push(Part::new(
            "remote-tag-index",
            index.heap_bytes(),
            index.len(),
        ));

        let marks = match self.eol_marks.lock() {
            Ok(g) => Arc::clone(&g),
            Err(e) => Arc::clone(&e.into_inner()),
        };
        parts.push(Part::new("eol-marks", marks.heap_bytes(), marks.len()));

        let (base_bytes, base_count) = match self.eol_baselines.lock() {
            Ok(g) => (g.heap_bytes(), g.len()),
            Err(e) => {
                let g = e.into_inner();
                (g.heap_bytes(), g.len())
            }
        };
        parts.push(Part::new("eol-baselines", base_bytes, base_count));
        parts
    }
}

impl Drop for RepoSession {
    fn drop(&mut self) {
        self.root_cancel.cancel();
    }
}
