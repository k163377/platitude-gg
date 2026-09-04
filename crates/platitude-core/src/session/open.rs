//! Opening a repository: the constructor and the background opening
//! sequence it starts.

use super::*;
use crate::repo;

impl RepoSession {
    /// Creates the session and starts opening `path` in the background.
    /// On success everything loads: log stream, refs, status, stashes.
    ///
    /// The command log holds what the user asks for and nothing else;
    /// [`RepoSession::open_recording`] is the door for an opening whose
    /// own reads are to be kept as well.
    ///
    /// `pass_hooks` is what the graph passes let the outside in with
    /// ([`PassHooks`]) — `None` for a session nobody drives, which then
    /// holds nothing for it.
    pub fn open(
        executor: GitExecutor,
        runtime: tokio::runtime::Handle,
        path: PathBuf,
        sink: Arc<dyn SessionSink>,
        pass_hooks: Option<Arc<dyn PassHooks>>,
    ) -> Arc<Self> {
        Self::open_recording(
            executor,
            runtime,
            path,
            sink,
            pass_hooks,
            Recording::UserOnly,
        )
    }

    /// [`RepoSession::open`], with what the command log keeps decided
    /// before the first git command is spawned rather than after the
    /// handle comes back — which is the only way to be sure of how much
    /// of an opening it holds (see [`Recording`]).
    pub fn open_recording(
        executor: GitExecutor,
        runtime: tokio::runtime::Handle,
        path: PathBuf,
        sink: Arc<dyn SessionSink>,
        pass_hooks: Option<Arc<dyn PassHooks>>,
        recording: Recording,
    ) -> Arc<Self> {
        let (write_tx, write_rx) = tokio::sync::mpsc::unbounded_channel();
        let commands = Arc::new(CommandFeed::new(Arc::clone(&sink), recording));
        let observer: Arc<dyn crate::process::CommandObserver> = Arc::clone(&commands) as _;
        let session = Arc::new(Self {
            executor: executor.observed(Arc::clone(&observer), Kept::Unasked),
            exec_user: executor.observed(Arc::clone(&observer), Kept::Asked),
            exec_unasked_fetch: executor.observed(observer, Kept::UnaskedUnlessItFails),
            commands,
            runtime: runtime.clone(),
            sink,
            root_cancel: CancellationToken::new(),
            info: Mutex::new(None),
            shared: Arc::new(Mutex::new(Shared::default())),
            log_options: Mutex::new(LogOptions::default()),
            log_gen: AtomicU64::new(0),
            diff_epoch: AtomicU64::new(0),
            last_diff: Mutex::new(None),
            lex_cache: Mutex::new(None),
            log_cancel: Mutex::new(None),
            details_read: Mutex::new(DetailsRead::default()),
            plan_read: Mutex::new(PlanRead::default()),
            write_replays: std::sync::atomic::AtomicBool::new(false),
            graph_passes: Arc::default(),
            pass_hooks,
            graph_stale: std::sync::atomic::AtomicBool::new(false),
            wip_dirty: std::sync::atomic::AtomicBool::new(false),
            merge_incoming: Mutex::new(Vec::new()),
            merge_tool_wanted: std::sync::atomic::AtomicBool::new(false),
            merge_tool_seen: Mutex::new(String::new()),
            rebase_stop_seen: Mutex::new(integrate::RebaseStop::default()),
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
            local_writes: std::sync::atomic::AtomicUsize::new(0),
            write_join: Mutex::new(None),
            poll_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            refs_read: ReadFlight::default(),
            status_read: ReadFlight::default(),
            stash_read: ReadFlight::default(),
            worktrees_read: ReadFlight::default(),
            write_tx,
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            config_stamp: Mutex::new(None),
            remote_tag_index: Mutex::new(Arc::new(RemoteTagIndex::default())),
            remote_tag_gen: AtomicU64::new(0),
            worktree_holders: Mutex::new(Arc::new(super::joins::WorktreeHolders::default())),
            worktree_gen: AtomicU64::new(0),
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
        // The handle is kept, not dropped: the application's shutdown
        // joins the loop so a local write in flight ends before the
        // runtime does (`RepoSession::take_write_join`).
        let write_loop = runtime.spawn(Arc::clone(&session).write_loop(write_rx));
        *relock(&session.write_join) = Some(write_loop);

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
}
