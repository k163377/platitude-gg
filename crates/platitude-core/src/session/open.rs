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
        let watched = |kept: Kept| executor.observed(Arc::clone(&observer), kept);
        let session = Arc::new(Self {
            executor: watched(Kept::Unasked),
            exec_background: watched(Kept::Unasked).background(),
            exec_user: watched(Kept::Asked),
            // Nobody asked for these fetches, so they wait behind the
            // ones somebody did (`process::Priority::Background`).
            exec_unasked_fetch: watched(Kept::UnaskedUnlessItFails).background(),
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
            preview_files: preview::PreviewFiles::new(),
            log_cancel: Mutex::new(None),
            details_read: Latest::default(),
            plan_read: Latest::default(),
            write_running: Mutex::new(None),
            graph_passes: Arc::default(),
            pass_hooks,
            graph_stale: std::sync::atomic::AtomicBool::new(false),
            standing: Standing::default(),
            merge_tool_wanted: std::sync::atomic::AtomicBool::new(false),
            signature_read: Latest::default(),
            remote_branch_read: Latest::default(),
            head_published_read: Latest::default(),
            carried_read: Latest::default(),
            eol_baselines: Mutex::new(HashMap::new()),
            eol_normalises: Derived::default(),
            remotes: Derived::default(),
            eol_marks: Mutex::new(Arc::new(Vec::new())),
            eol_marks_stale: std::sync::atomic::AtomicBool::new(true),
            status_key: Mutex::new(None),
            refs_key: Mutex::new(None),
            join_key: Mutex::new(None),
            last_snapshot: Mutex::new(None),
            local_writes: std::sync::atomic::AtomicUsize::new(0),
            write_join: Mutex::new(None),
            poll_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            read_owed: std::sync::atomic::AtomicBool::new(false),
            carried_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            copies: super::carried::Copies::default(),
            refs_read: ReadFlight::default(),
            status_read: ReadFlight::default(),
            stash_read: ReadFlight::default(),
            worktrees_read: ReadFlight::default(),
            write_tx,
            write_order: Mutex::new(None),
            accepting: Mutex::new(()),
            released: std::sync::atomic::AtomicBool::new(false),
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            config_stamp: Mutex::new(None),
            remote_tag_index: Mutex::new(Arc::new(RemoteTagIndex::default())),
            remote_tag_gen: AtomicU64::new(0),
            worktree_holders: Mutex::new(Arc::new(super::joins::WorktreeHolders::default())),
            worktree_gen: AtomicU64::new(0),
            remote_tags_slot: super::auto_fetch::RemoteTagSlot::default(),
            head_reach_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            merge_tools_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            auto_fetch: Mutex::new(None),
            auto_fetch_interval: Mutex::new(None),
            auto_fetch_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            open_fetch: Mutex::new(OpenFetchState::Unasked),
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
                    // Before the event that lets a write be asked for:
                    // the tree this session shares its writes with is
                    // only known now, and a write accepted with no order
                    // installed would take no place in it
                    // (`session::write_order`).
                    s.join_write_order(&info);
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
