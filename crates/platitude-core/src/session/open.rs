//! Opening a repository: the constructor and the background opening
//! sequence it starts.

use super::*;
use crate::repo;

/// What the opening's first graph pass does about whatever is already
/// drawn. Fixed at creation: the opening spawns that pass itself, so
/// flipping it later would race it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FirstPass {
    /// Nothing is on screen: stream in chunks, drawing the first rows as
    /// they arrive.
    #[default]
    Streamed,
    /// The consumer already shows this repository's graph, read from
    /// another working copy that shares its refs and objects
    /// (`Hub::restand_tab`): build off-screen and swap in whole, so the
    /// reader keeps their place.
    Swapped,
}

/// The record of the graph a consumer is showing — the row prints and the
/// chips they wear — moved from the session that drew it to the one taking
/// its place ([`RepoSession::take_drawn_graph`];
/// rules-refs/core.md「画面に出ているグラフの記録はセッションより長生きする」).
/// Without it the next session calls every picture new and redraws. The
/// rows are not in it: the walk still runs.
pub struct DrawnGraph(Shared);

/// What an opening is settled with, all decided before the first git
/// command is spawned.
struct Opening {
    recording: Recording,
    first_pass: FirstPass,
    drawn: Option<DrawnGraph>,
}

impl RepoSession {
    /// Creates the session and starts opening `path` in the background.
    /// The command log keeps only what the user asks for
    /// ([`RepoSession::open_recording`] keeps the opening's own reads too).
    /// `pass_hooks` lets the outside into the graph passes ([`PassHooks`]);
    /// `None` for a session nobody drives.
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

    /// [`RepoSession::open`] for a consumer already showing this
    /// repository's graph from another working copy
    /// ([`FirstPass::Swapped`]); `drawn` comes from the session being
    /// closed ([`DrawnGraph`]).
    pub fn open_standing_in(
        executor: GitExecutor,
        runtime: tokio::runtime::Handle,
        path: PathBuf,
        sink: Arc<dyn SessionSink>,
        pass_hooks: Option<Arc<dyn PassHooks>>,
        drawn: Option<DrawnGraph>,
    ) -> Arc<Self> {
        Self::open_as(
            executor,
            runtime,
            path,
            sink,
            pass_hooks,
            Opening {
                recording: Recording::UserOnly,
                first_pass: FirstPass::Swapped,
                drawn,
            },
        )
    }

    /// Takes the record of the graph on screen out of this session, for
    /// the one taking its place ([`DrawnGraph`]). Call before
    /// `RepoSession::close`, which throws it away.
    pub fn take_drawn_graph(&self) -> DrawnGraph {
        DrawnGraph(std::mem::take(&mut *self.lock_shared()))
    }

    /// [`RepoSession::open`], with what the command log keeps fixed before
    /// the first git command is spawned (see [`Recording`]).
    pub fn open_recording(
        executor: GitExecutor,
        runtime: tokio::runtime::Handle,
        path: PathBuf,
        sink: Arc<dyn SessionSink>,
        pass_hooks: Option<Arc<dyn PassHooks>>,
        recording: Recording,
    ) -> Arc<Self> {
        Self::open_as(
            executor,
            runtime,
            path,
            sink,
            pass_hooks,
            Opening {
                recording,
                first_pass: FirstPass::Streamed,
                drawn: None,
            },
        )
    }

    /// The constructor the doors above share ([`Opening`]).
    fn open_as(
        executor: GitExecutor,
        runtime: tokio::runtime::Handle,
        path: PathBuf,
        sink: Arc<dyn SessionSink>,
        pass_hooks: Option<Arc<dyn PassHooks>>,
        opening: Opening,
    ) -> Arc<Self> {
        let Opening {
            recording,
            first_pass,
            drawn,
        } = opening;
        // The pass number comes with a kept graph: a counter starting over
        // would have this session's first pass refused as already seen
        // (`GraphModel::take_chunk`).
        let shared = drawn.map_or_else(Shared::default, |carried| carried.0);
        let passes = shared.generation;
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
            shared: Arc::new(Mutex::new(shared)),
            log_options: Mutex::new(LogOptions::default()),
            log_gen: AtomicU64::new(passes),
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
            first_pass,
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
            lfs_needed: std::sync::atomic::AtomicUsize::new(0),
            lfs_unsettled: std::sync::atomic::AtomicBool::new(false),
            lfs_runs: Derived::default(),
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
            leaving: Mutex::new(Vec::new()),
            released: std::sync::atomic::AtomicBool::new(false),
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            config_stamp: Mutex::new(None),
            remote_tag_index: Mutex::new(Arc::new(RemoteTagIndex::default())),
            remote_tag_gen: AtomicU64::new(0),
            worktree_holders: Mutex::new(Arc::new(super::joins::WorktreeHolders::default())),
            copy_heads: Mutex::new(Arc::new(std::collections::HashMap::new())),
            worktree_gen: AtomicU64::new(0),
            remote_tags_slot: super::auto_fetch::RemoteTagSlot::default(),
            head_reach_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            merge_tools_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            auto_fetch: Mutex::new(None),
            auto_fetch_interval: Mutex::new(None),
            auto_fetch_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            open_fetch: Mutex::new(OpenFetchState::Unasked),
            opened: tokio::sync::watch::channel(false).0,
        });
        // Kept for the shutdown to join (`RepoSession::take_write_join`).
        let write_loop = runtime.spawn(Arc::clone(&session).write_loop(write_rx));
        *relock(&session.write_join) = Some(write_loop);

        runtime.spawn(settle(Arc::clone(&session), path));
        session
    }
}

/// The opening itself, ending on every road — the cancelled one included —
/// with the word that the opening is over. That word is a completion
/// boundary: before it, `workdir()` answering `None` means *not yet*;
/// after it, *never* (`RepoSession::workdir_when_open`).
async fn settle(s: Arc<RepoSession>, path: PathBuf) {
    let cancel = s.root_cancel.clone();
    match repo::open(&s.executor, &path, &cancel).await {
        Ok(info) => {
            let workdir = info.workdir.clone();
            s.set_info(info.clone());
            // Before the event that lets a write be asked for: a write
            // accepted before the order is installed would take no place
            // in it (`session::write_order`).
            s.join_write_order(&info);
            s.sink.event(SessionEvent::Opened { info });
            // First, so the UI can ask for a missing identity before the
            // first commit fails on it.
            s.refresh_author();
            // The one place the two openings differ ([`FirstPass`]).
            match s.first_pass {
                FirstPass::Streamed => s.restart_log(),
                FirstPass::Swapped => s.refresh_log(),
            }
            s.refresh_quick();
            // The fetch decision is spawned beside the reads, not awaited
            // before them (rules-refs/core.md「開いたら 1 回 fetch する」).
            let opening = Arc::clone(&s);
            s.runtime.spawn(async move {
                let fetching = opening.take_open_fetch(&workdir).await;
                // A fetch reads the remotes' tags on its way out, so only
                // an opening without one asks. Asked here, not by
                // `set_auto_fetch` (whose interval grants the permission):
                // that runs before there is a workdir.
                if !fetching {
                    opening.catch_up_remote_tags();
                }
            });
        }
        Err(error) => {
            if !error.is_cancelled() {
                s.sink.event(SessionEvent::OpenFailed { path, error });
            }
        }
    }
    publish_opened(&s.opened);
}

/// Says the opening is over — to whoever asks, now or later.
///
/// Not `send`: a `watch` sender with no receivers refuses the send and
/// keeps the old value, and this channel has a receiver only while a read
/// is inside [`RepoSession::workdir_when_open`] — so a later read would
/// wait for a word already said.
fn publish_opened(opened: &tokio::sync::watch::Sender<bool>) {
    opened.send_replace(true);
}

#[cfg(test)]
mod tests {
    /// Pins `send_replace`: with nothing subscribed, `send` keeps the old
    /// value, and every road out of an opening can get here with no reader.
    #[test]
    fn a_completion_nobody_waited_for_is_still_kept() {
        let opened = tokio::sync::watch::channel(false).0;
        super::publish_opened(&opened);
        assert!(
            *opened.subscribe().borrow(),
            "the opening settled, and a reader that came afterwards was told it had not"
        );
    }
}
