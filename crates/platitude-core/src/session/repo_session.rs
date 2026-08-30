//! The session handle itself: what one open repository holds, and what
//! it cancels on the way out (the opening sequence is [`super::open`]).

use super::*;

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
    pub(super) details_read: Mutex<DetailsRead>,
    /// One interactive-rebase plan ask at a time (`ask_rebase_plan`).
    pub(super) plan_read: Mutex<PlanRead>,
    /// The graph passes that could still walk, whether or not the stream
    /// is still theirs (see [`GraphPasses`]).
    pub(super) graph_passes: Arc<GraphPasses>,
    /// What the next graph pass to reach a given step runs there
    /// ([`RepoSession::run_inside_next_pass`]). Empty in the application,
    /// which never leaves anything here: the only caller is a test ending
    /// a pass the one way nothing else can.
    pub(super) pass_step: Mutex<Option<PassStepHook>>,
    /// What every graph pass reaching a given step fails with instead
    /// of walking ([`RepoSession::fail_every_pass`]). Empty except while a
    /// caller is driving the failure on purpose.
    pub(super) pass_fault: Mutex<Option<(PassStep, String)>>,
    /// Whether the graph on screen has been left behind the repository:
    /// a rebuild that would have replaced it did not land, so every row
    /// standing there is real and none of them is current.
    ///
    /// Held rather than derived because the consumer is told on the turn
    /// only ([`SessionEvent::LogStale`]) — the pass that finds nothing
    /// changed is the quiet one, and it has to be able to say "current
    /// again" without every quiet pass saying anything.
    pub(super) graph_stale: std::sync::atomic::AtomicBool,
    /// Which diff read is the current one. Bumped by every
    /// [`RepoSession::load_diff`], and read again just before the colours
    /// for that diff would be worked out: a reader going down a commit's
    /// file list starts a read per row, and colouring costs enough (see
    /// [`SessionEvent::DiffColoured`]) that the ones nobody is waiting for
    /// any more are worth not doing at all. The rows are unaffected —
    /// those are cheap, and a stale one is dropped by the pane on arrival.
    pub(super) diff_epoch: AtomicU64,
    /// The diff last published and the fingerprint of the bytes it was
    /// read from — what [`RepoSession::refresh_diff`] compares against, so
    /// a poll tick over a file nobody has touched sends nothing at all.
    ///
    /// One slot, because one pane asks: `load_diff` is reached from the
    /// diff view alone. A read for some other target therefore reads as
    /// "not the one held", which is the safe direction — it publishes.
    ///
    /// Written where the event goes out rather than where the bytes are
    /// read: recording a fingerprint the pane never received would leave
    /// it stale for as long as the file stayed that way.
    pub(super) last_diff: Mutex<Option<(DiffTarget, u64)>>,
    /// Lexer states remembered down the file the last diff's colours
    /// were read against (`highlight::LexCache`) — what lets the re-read
    /// after every partial stage start near its hunks instead of at
    /// line 1. Self-invalidating: the cache carries the source text's
    /// hash and is dropped by the reader when the text has changed.
    pub(super) lex_cache: Mutex<Option<crate::highlight::LexCache>>,
    /// Dirty working tree — one of the two halves that put a synthetic WIP
    /// row in front of the log stream (the other is below).
    pub(super) wip_dirty: std::sync::atomic::AtomicBool,
    /// What a standing merge is bringing in (`MERGE_HEAD`), empty the rest
    /// of the time: the WIP row leashes these as well as HEAD, so it draws
    /// the fork the merge commit is about to have. Kept beside
    /// [`RepoSession::wip_dirty`] because the two decide the same row —
    /// either one makes it (a merge resolved as ours has a merge commit to
    /// write with nothing dirty left to show for it), and either moving is
    /// a graph to rebuild.
    pub(super) merge_incoming: Mutex<Vec<Oid>>,
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
    /// What remotes are configured, and which of them a push goes to. Read
    /// on every refs listing before this, which is a process per poll tick
    /// for an answer that only a write moves.
    ///
    /// The two travel together because one file holds both and one stat of
    /// it drops both — two cells would be two things to remember to
    /// forget.
    pub(super) remotes: Derived<remote::Remotes>,
    /// How the config file looked when what is read out of it was last
    /// read (see [`RepoSession::forget_what_the_config_decides`]). `None`
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
    /// The branches other working copies have checked out, as the last
    /// worktree read left them. Read by the ref joins so the sidebar rows
    /// and the graph chips get one answer between them; shared for the
    /// reason the tag index is, since every refs read wants it and none
    /// of them changes it.
    pub(super) worktree_holders: Mutex<Arc<super::joins::WorktreeHolders>>,
    /// Bumped when that set became a different one. **Nothing else in the
    /// refs key would notice** — taking or giving back a working copy
    /// moves no ref, so without this the mark would wait for an unrelated
    /// ref to move.
    pub(super) worktree_gen: AtomicU64,
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
        *relock(&self.network_timeout)
    }

    /// Raises or lowers the fetch / push time budget. Zero is ignored — a
    /// network command must always have a backstop.
    pub fn set_network_timeout(&self, timeout: std::time::Duration) {
        if timeout.is_zero() {
            return;
        }
        *relock(&self.network_timeout) = timeout;
    }

    /// Moves what the command log keeps — whether the reads this session
    /// makes on its own (polling, refreshes, details) are in it too.
    ///
    /// Applies to commands spawned from here on, not retroactively, and
    /// not to an opening already under way: where a session *starts* is
    /// [`RepoSession::open_recording`]'s to say (see [`Recording`]).
    pub fn set_recording(&self, recording: Recording) {
        self.commands.set_recording(recording);
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
        *self.lock_info() = Some(info);
    }

    pub(super) fn lock_info(&self) -> std::sync::MutexGuard<'_, Option<RepoInfo>> {
        relock(&self.info)
    }

    pub(super) fn lock_shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        relock(&self.shared)
    }
}

impl Drop for RepoSession {
    fn drop(&mut self) {
        self.root_cancel.cancel();
    }
}
