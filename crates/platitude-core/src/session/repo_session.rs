//! The session handle itself: what one open repository holds, and what
//! it cancels on the way out (the opening sequence is [`super::open`]).

use super::*;

pub struct RepoSession {
    /// Reads and refreshes: recorded in the command log only while
    /// background recording is on.
    pub(super) executor: GitExecutor,
    /// The reads nobody is waiting on — the other copies' status, the
    /// walk behind a chip, the remote tags — served last and kept out of
    /// the click's reserve (`process::Priority::Background`); off the log
    /// like `executor`. The poll's reads stay on `executor`: the front
    /// page is what somebody is looking at.
    pub(super) exec_background: GitExecutor,
    /// The queue's handle: everything run through it is something the
    /// user asked for, and is always recorded.
    pub(super) exec_user: GitExecutor,
    /// The fetches nobody asked for (the interval's and the opening's).
    /// Off the log unless git says no, so the panel a failure raises
    /// holds the command that raised it
    /// (デザイン規約 §git が言ったことを読む場所).
    pub(super) exec_unasked_fetch: GitExecutor,
    pub(super) commands: Arc<CommandFeed>,
    pub(super) runtime: tokio::runtime::Handle,
    pub(super) sink: Arc<dyn SessionSink>,
    /// Cancelled when the session closes; every op but a local write
    /// derives from it ([`Lane::Local`]).
    pub(super) root_cancel: CancellationToken,
    pub(super) info: Mutex<Option<RepoInfo>>,
    pub(super) shared: Arc<Mutex<Shared>>,
    pub(super) log_options: Mutex<LogOptions>,
    pub(super) log_gen: AtomicU64,
    pub(super) log_cancel: Mutex<Option<CancellationToken>>,
    /// One commit-details read at a time, numbered (`load_details`).
    pub(super) details_read: Latest,
    /// One interactive-rebase plan ask at a time (`ask_rebase_plan`).
    pub(super) plan_read: Latest,
    /// The write the queue is serving, from before git runs it until the
    /// reads it invalidated have published; `None` between requests. Read
    /// by the poll, which stays out mid-operation unless the write replays
    /// ([`OperationKind::replays_history`]), and by a task under the queue
    /// that has to say which write it is (`note_landing`).
    pub(super) write_running: Mutex<Option<Operation>>,
    /// The graph passes that could still walk, whether or not the stream
    /// is still theirs (see [`GraphPasses`]).
    pub(super) graph_passes: Arc<GraphPasses>,
    /// What the outside was let into the graph passes with when this
    /// session was opened ([`PassHooks`]) — `None` in the application as
    /// shipped.
    pub(super) pass_hooks: Option<Arc<dyn PassHooks>>,
    /// What this session's opening does about a graph already on screen
    /// ([`FirstPass`]) — read once, by the opening itself.
    pub(super) first_pass: FirstPass,
    /// Whether the graph on screen has been left behind the repository: a
    /// rebuild that would have replaced it did not land.
    ///
    /// Held because the consumer is told only on the turn
    /// ([`SessionEvent::LogStale`]), so a quiet pass can say "current
    /// again" without every quiet pass saying something.
    pub(super) graph_stale: std::sync::atomic::AtomicBool,
    /// Which diff read is the current one: bumped by every
    /// [`RepoSession::load_diff`] and checked wherever a read would hand
    /// something over. Bumping it cancels the passed read's git.
    ///
    /// A passed read hands over nothing, its rows included: the
    /// fingerprint riding with them is what the next partial stage is
    /// refused against (`stage::refusal::verify_fingerprint`), so an older
    /// read landing last leaves the pane unable to stage (`publish_diff`).
    pub(super) diff_epoch: DiffEpoch,
    /// The re-reads of the pane's diff that are out, by file: a tick or a
    /// focus asking while one is out shares the one after it
    /// ([`RepoSession::refresh_diff`]).
    pub(super) rereads: Rereads,
    /// The diff last published and the fingerprint of the bytes it was
    /// read from — what [`RepoSession::refresh_diff`] compares against, so
    /// a poll tick over an untouched file sends nothing.
    ///
    /// One slot, because one pane asks; a read for another target reads as
    /// "not the one held" and publishes. The working copy is part of the
    /// key: the same path in another copy is another file
    /// (`RepoSession::load_carried_diff`).
    ///
    /// Written where the event goes out: a fingerprint the pane never
    /// received would leave it stale for as long as the file stayed so.
    pub(super) last_diff: Mutex<Option<(PathBuf, DiffTarget, u64)>>,
    /// Lexer states down the file the last diff's colours were read against
    /// (`highlight::LexCache`), so the re-read after a partial stage starts
    /// near its hunks. Self-invalidating by the source text's hash.
    pub(super) lex_cache: Mutex<Option<crate::highlight::LexCache>>,
    /// The files a picture's blob sides are written to so the pane can
    /// name them by URL (`preview::PreviewFiles`). Numbered by the diff
    /// epoch above; each read published sweeps the reads before it, the
    /// pane closing sweeps them all ([`RepoSession::release_preview`]),
    /// and the close removes the directory.
    pub(super) preview_files: preview::PreviewFiles,
    /// Where the repository stands, as the reads left it — HEAD and what
    /// derives from it, the dirty tree, the merge's sides, the rebase stop
    /// and the merge tool; the one record of each. Its stamps keep a read
    /// that looked before a write from overwriting the write's own read
    /// (`session::standing`).
    pub(super) standing: Standing,
    /// Set by [`RepoSession::ask_merge_tool`] to have the next status read
    /// name the merge tool even with nothing conflicted. Cleared by that
    /// read, so the common case does not pay the spawns on every poll.
    pub(super) merge_tool_wanted: std::sync::atomic::AtomicBool,
    /// One signature verification at a time: a selection that moves on
    /// cancels the gpg or ssh-keygen run the last one started
    /// (`session::latest`).
    pub(super) signature_read: Latest,
    /// The discard log's picked entry, which the walk takes one tip more
    /// for (`session::shown_discard`); `None` while nothing is picked.
    pub(super) shown_discard: Mutex<Option<Arc<ShownDiscard>>>,
    /// One remote-branch check at a time — a name being typed asks per
    /// settled keystroke, and each ask is a network round trip.
    pub(super) remote_branch_read: Latest,
    /// One check of whether a remote already has HEAD's commit at a time,
    /// for the rare pass that could not read it off its rows
    /// ([`RepoSession::settle_head_published`]).
    pub(super) head_published_read: Latest,
    /// One read of another working copy's files at a time
    /// ([`RepoSession::read_carried_status`]), which also orders the
    /// answers: reads of different trees finish in any order, and the pane
    /// would show whichever finished last — the copy the reader stepped off.
    pub(super) carried_read: Latest,
    /// Line-ending baselines already sampled, keyed by (directory,
    /// extension) — what a house style is scoped to.
    ///
    /// `None` is a cached "unknown": it cost the same reads as knowing.
    /// Emptied by a write that reaches the tree or the history and when refs
    /// move, since either can bring a new `.gitattributes` or new neighbours
    /// (a fetch, a config write, a delete or a copy taken away cannot).
    pub(super) eol_baselines: Mutex<HashMap<(String, String), Option<crate::eol::Baseline>>>,
    /// Whether git normalises line endings here (`core.autocrlf`).
    pub(super) eol_normalises: Derived<bool>,
    /// What remotes are configured, and which of them a push goes to —
    /// cached, since only a write moves them. One cell because one file
    /// holds both and one stat of it drops both.
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
    /// What Git LFS asks of the pending files where git cannot run it,
    /// repeated by every status read until it is read again
    /// (`RepoSession::lfs_needs`).
    pub(super) lfs_held: Mutex<super::lfs::LfsNeeds>,
    /// Set when that reading could not be had, so the next tick reads again.
    pub(super) lfs_unsettled: std::sync::atomic::AtomicBool,
    /// Whether git runs Git LFS here — kept once it does, dropped when it
    /// does not, so only a yes saves the next asking.
    pub(super) lfs_runs: Derived<bool>,
    /// Fingerprint of the last status read, so a tick that finds the same
    /// files in the same states reads no diffs at all.
    pub(super) status_key: Mutex<Option<u64>>,
    /// Fingerprint of the last refs read (see `joins::refs_keys`), so a
    /// refresh can tell an external commit / fetch / switch from a quiet
    /// re-read.
    /// `None` until the first read: opening already streams the graph.
    pub(super) refs_key: Mutex<Option<u64>>,
    /// The same for everything the joins read, which is more than the
    /// listing: `refs_key` moving means the history is walked again, this
    /// moving means only that the snapshot and the chips are rebuilt.
    pub(super) join_key: Mutex<Option<u64>>,
    /// The snapshot last published. A read that finds nothing moved hands
    /// this one out again, so the sidebar can tell "the same" by pointer
    /// instead of rebuilding every section on the Qt thread.
    pub(super) last_snapshot: Mutex<Option<Arc<RefsSnapshot>>>,
    /// Local writes handed to the queue and not yet done with — the ones
    /// a close waits out ([`Lane::Local`]). What the application's quit
    /// gate reads: zero on every open session is the moment the window
    /// may go.
    pub(super) local_writes: std::sync::atomic::AtomicUsize,
    /// The write loop's own task. Handed to the application on the way
    /// out ([`RepoSession::take_write_join`]): dropping the runtime drops
    /// the loop mid-write, and `kill_on_drop` then ends git itself — so a
    /// shutdown joins this first.
    pub(super) write_join: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// One permit, held by a running poll: a tick that arrives while the
    /// previous one is still reading is dropped.
    pub(super) poll_slot: Arc<tokio::sync::Semaphore>,
    /// A read this session owes the working tree: a write landed in it
    /// while this session was already reading, so the read holding the
    /// slot began before that write and cannot answer for it
    /// ([`RepoSession::read_again`]). Remembered, because a refused clock
    /// tick comes round again and a write's news does not.
    pub(super) read_owed: std::sync::atomic::AtomicBool,
    /// The poll's permit, for the other copies' own tick: a pass carries a
    /// `status` per copy and on a big tree can outlast its interval.
    pub(super) carried_slot: Arc<tokio::sync::Semaphore>,
    /// Whether the other copies are read at all (`set_copies_read` —
    /// the settings' "never", which holds for the page's tick, an
    /// opening and a focus fire alike), what the last pass left and the
    /// pass in flight, under one lock (`carried::Copies`).
    pub(super) copies: super::carried::Copies,
    /// One pass in flight per snapshot, for the reads asked for from more
    /// than one place at once ([`ReadFlight`]).
    pub(super) refs_read: ReadFlight<Reread>,
    pub(super) status_read: ReadFlight<Reread>,
    pub(super) stash_read: ReadFlight<StashRead>,
    /// The stash listings so far, for whether one moved and whether a
    /// list handed on is still the newest (`RepoSession::stashes_standing`).
    pub(super) stash_listings: Mutex<StashListings>,
    pub(super) worktrees_read: ReadFlight<WorktreeRead>,
    /// Submission end of the write queue (`session::write`).
    pub(super) write_tx: tokio::sync::mpsc::UnboundedSender<WriteRequest>,
    /// The order this working tree's local writes run in, shared with
    /// every other session on the same tree (`session::write_order`).
    /// `None` until the repository is open.
    pub(super) write_order: Mutex<Option<Arc<WriteOrder>>>,
    /// Held while a write takes its place in that order and goes into the
    /// queue above, so the two can never disagree about which write came
    /// first ([`RepoSession::write`] says what disagreeing would cost).
    /// Nothing else about a write is under it.
    pub(super) accepting: Mutex<()>,
    /// The deletes out now, whose commits the graph stands in for
    /// (`session::leaving`). The session's own, not the graph's record: a
    /// record handed to the next session (`DrawnGraph`) must not carry
    /// deletes whose answers only this one hears. Taken after the graph's
    /// lock where both are held.
    pub(super) leaving: Mutex<Vec<Leaving>>,
    /// Set by the close, before it gives anything back
    /// ([`RepoSession::keeps_what_it_reads`]). Reads already in flight
    /// answer after it, and what they answer with is dropped.
    pub(super) released: std::sync::atomic::AtomicBool,
    /// Time budget for fetch / push. Persisted as the settings key
    /// `network_timeout_secs`; only the settings dialog's input field is
    /// missing (実装計画 §7).
    pub(super) network_timeout: Mutex<std::time::Duration>,
    /// What the remotes last advertised under `refs/tags/`, merged into
    /// the shape the joins read. Empty until a fetch has been through:
    /// asking costs the network, so it rides the one command the user
    /// already meant to spend it on. Shared because every refs read wants
    /// it and none of them changes it.
    pub(super) remote_tag_index: Mutex<Arc<RemoteTagIndex>>,
    /// Bumped whenever the index above became different readings, so the
    /// join key can cover it without walking the entries.
    pub(super) remote_tag_gen: AtomicU64,
    /// The branches other working copies have checked out, as the last
    /// worktree read left them. Read by the ref joins so the sidebar rows
    /// and the graph chips get one answer between them; shared as the tag
    /// index is.
    pub(super) worktree_holders: Mutex<Arc<super::joins::WorktreeHolders>>,
    /// Where each other working copy's HEAD stood when the listing last
    /// named it, by the key its path is compared on (`joins::same_path_key`).
    ///
    /// The listing runs on the page's tick and learns that a copy has
    /// committed long before that copy's own `status` reading does
    /// (`session::carried`); a row drawn from the older reading would
    /// stand on a commit the copy has left (`relay::Standing`).
    ///
    /// Kept apart from [`Self::worktree_holders`], which decides whether
    /// the refs are read and the graph walked again: folded in there, every
    /// commit in a neighbouring copy would spend a full refs listing to say
    /// nothing (`joins::note_worktree_holders`).
    pub(super) copy_heads: Mutex<Arc<std::collections::HashMap<String, Oid>>>,
    /// Bumped when the worktree holders became a different set: taking or
    /// giving back a working copy moves no ref, so nothing else in the join
    /// key would notice.
    pub(super) worktree_gen: AtomicU64,
    /// One permit for the background remote-tags read, so a second
    /// permission-granting call cannot stack another on top of it — and
    /// the word that it was let go, for the asks that call stacked
    /// nothing for (`auto_fetch::RemoteTagSlot`).
    pub(super) remote_tags_slot: super::auto_fetch::RemoteTagSlot,
    /// One permit: the walk is the only part of a refresh that scales
    /// with the history, and a tick arriving mid-walk is dropped.
    pub(super) head_reach_slot: Arc<tokio::sync::Semaphore>,
    /// One merge-tool candidate read at a time: opening settings twice in
    /// a row starts one slow `mergetool --tool-help`
    /// (ci/baseline/code-costs-windows-x64.md).
    pub(super) merge_tools_slot: Arc<tokio::sync::Semaphore>,
    /// The running auto-fetch timer, if any.
    pub(super) auto_fetch: Mutex<Option<AutoFetch>>,
    /// The interval the timer was last *asked* for, kept while it is
    /// suspended so there is something to put back.
    pub(super) auto_fetch_interval: Mutex<Option<std::time::Duration>>,
    /// One permit: an auto fetch that is still queued or running holds it,
    /// so a tick that arrives meanwhile is skipped. A permit moved into a
    /// dropped request is released with it.
    pub(super) auto_fetch_slot: Arc<tokio::sync::Semaphore>,
    /// Where the opening's own fetch stands (see [`OpenFetchState`]).
    pub(super) open_fetch: Mutex<OpenFetchState>,
    /// The opening's completion boundary: true once the open has settled,
    /// with a working tree or without one — what tells `workdir()`'s
    /// *not yet* from *never* ([`RepoSession::workdir_when_open`]).
    pub(super) opened: tokio::sync::watch::Sender<bool>,
}

impl RepoSession {
    /// Workdir of the opened repository (None until `Opened`).
    pub fn workdir(&self) -> Option<PathBuf> {
        self.lock_info().as_ref().map(|i| i.workdir.clone())
    }

    /// The working tree, waiting out an opening that has not settled;
    /// `None` only once it settled with none (it failed, or the session is
    /// going away) — an answer the caller can publish.
    ///
    /// For the reads a screen marks itself loading for: asked inside the
    /// opening, a read that gave up on [`Self::workdir`]'s `None` would
    /// leave the indicator turning with nothing to put it down.
    /// Best-effort reads keep using `workdir` and give up; a poll or a
    /// later click asks again (`auto_fetch`, `head_reach`).
    pub(super) async fn workdir_when_open(&self) -> Option<PathBuf> {
        // Subscribed before the look, so an opening that settles between
        // the two is a change this still sees.
        let mut settled = self.opened.subscribe();
        loop {
            if let Some(workdir) = self.workdir() {
                return Some(workdir);
            }
            if *settled.borrow_and_update() {
                return None;
            }
            if settled.changed().await.is_err() {
                return None;
            }
        }
    }

    /// Config file of the opened repository (None until `Opened`) — an
    /// accessor of its own because every poll tick reads it.
    pub(super) fn config_path(&self) -> Option<PathBuf> {
        self.lock_info().as_ref().map(|i| i.config_path.clone())
    }

    /// Git directory of the opened repository (None until `Opened`) — its
    /// own accessor too: the replay counter reads it several times a
    /// second ([`Self::refresh_op_progress`]).
    pub(super) fn git_dir(&self) -> Option<PathBuf> {
        self.lock_info().as_ref().map(|i| i.git_dir.clone())
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
    /// Applies to commands spawned from here on: where a session
    /// *starts* is [`RepoSession::open_recording`]'s to say (see
    /// [`Recording`]).
    pub fn set_recording(&self, recording: Recording) {
        self.commands.set_recording(recording);
    }

    /// Cancels everything this session is doing — except the local
    /// writes already asked for, running and queued alike, which run to
    /// completion on tokens of their own ([`Lane::Local`]). Intake stops,
    /// and the network-paced requests in the tail die on this cancel.
    /// Idempotent.
    ///
    /// Those writes keep their places in the working tree's order
    /// (`session::write_order`): a session opened over this one queues
    /// behind them, and giving them back would let it overtake. What is
    /// given up is the reading half — off the list of pages the tree
    /// reports to, and what it had drawn let go.
    pub fn close(&self) {
        self.root_cancel.cancel();
        self.preview_files.remove_all();
        if let Some(order) = self.write_order() {
            order.leave(self);
        }
        self.forget_the_screens_copy();
    }

    /// This working tree's write order, once the repository is open.
    pub(super) fn write_order(&self) -> Option<Arc<WriteOrder>> {
        relock(&self.write_order).clone()
    }

    /// Puts this session on its working tree's order, and on the list of
    /// pages told when a write lands in that tree.
    ///
    /// Before the `Opened` event on purpose: that event is what lets the
    /// consumer ask for a write, and a write accepted with no order
    /// installed would take no place in the tree's queue.
    pub(super) fn join_write_order(self: &Arc<Self>, info: &RepoInfo) {
        let order = write_order::of(&info.git_dir);
        order.join(self);
        *relock(&self.write_order) = Some(order);
    }

    /// Whether what this session's reads answer with is still kept.
    ///
    /// Read inside the lock of whatever is about to be stored, and false
    /// from before the close gives anything back, so a read already in
    /// flight either lands ahead of the release — and is cleared by it —
    /// or finds the door shut. Without that the pass walking when the tab
    /// went would write its chips back into the `Shared` just emptied.
    /// Loads need no door: a released session's copy is empty, which is
    /// the true answer.
    pub(super) fn keeps_what_it_reads(&self) -> bool {
        !self.released.load(Ordering::SeqCst)
    }

    /// Where a read's answer about the graph is kept — `None` once this
    /// session has let go of it ([`Self::keeps_what_it_reads`]).
    pub(super) fn store_shared(&self) -> Option<std::sync::MutexGuard<'_, Shared>> {
        let shared = self.lock_shared();
        self.keeps_what_it_reads().then_some(shared)
    }

    /// Lets go of what this session read for a page that is gone: a local
    /// write let run on keeps a closed session alive
    /// (`Hub::park_writes_of`), and these are the largest things in the
    /// process. Nothing reads them after: a closed session makes no reads
    /// behind its writes. Replaced, every one of them: `clear()` keeps a
    /// collection's buckets, which is most of what a full one costs.
    fn forget_the_screens_copy(&self) {
        // Before anything is given back, so no store can read an open
        // door and land after the release ([`Self::keeps_what_it_reads`]).
        self.released.store(true, Ordering::SeqCst);
        *self.lock_shared() = Shared::default();
        *relock(&self.last_snapshot) = None;
        *relock(&self.lex_cache) = None;
        *relock(&self.remote_tag_index) = Arc::new(RemoteTagIndex::default());
        *relock(&self.eol_baselines) = HashMap::new();
    }

    /// The pane closed: the picture files the last diff read wrote are
    /// nobody's to look at any more. The next read sweeps them too, so
    /// this only matters for a pane that is not reading again.
    pub fn release_preview(&self) {
        self.preview_files.release();
    }

    /// How many local writes are queued or running — the ones
    /// [`RepoSession::close`] waits out (network writes die with the
    /// session). Zero is the moment nothing here would outlast a
    /// shutdown.
    pub fn local_writes_pending(&self) -> usize {
        self.local_writes.load(Ordering::SeqCst)
    }

    /// The write loop's task, for the one caller that has to outwait it:
    /// the application's shutdown joins it after [`RepoSession::close`]
    /// so a local write in flight is not dropped with the runtime.
    /// `None` after the first take.
    pub fn take_write_join(&self) -> Option<tokio::task::JoinHandle<()>> {
        relock(&self.write_join).take()
    }

    // --- internals ------------------------------------------------------

    /// Runs one on-demand read in the background and answers with the
    /// event it produced, or puts `op` on the error surface.
    ///
    /// Reads that answer otherwise than exactly one event or failure — a
    /// missing HEAD that is a state, a refusal that *is* the answer, a
    /// read that claims an epoch before it starts — stay out: folding them
    /// in would change what they report.
    pub(super) fn spawn_read<F, Fut>(self: &Arc<Self>, op: &'static str, read: F)
    where
        F: FnOnce(Arc<Self>, PathBuf, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<SessionEvent, GitError>> + Send,
    {
        let cancel = self.root_cancel.clone();
        self.spawn_read_under(op, cancel, read);
    }

    /// [`Self::spawn_read`] for a read that answers one question at a
    /// time: `latest` cancels the ask still out, so the read this starts
    /// is the only one of its kind running ([`Latest`]). A cancelled read
    /// reports nothing — the ask that displaced it is the one being
    /// waited on.
    pub(super) fn spawn_read_latest<F, Fut>(
        self: &Arc<Self>,
        op: &'static str,
        latest: &Latest,
        read: F,
    ) where
        F: FnOnce(Arc<Self>, PathBuf, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<SessionEvent, GitError>> + Send,
    {
        let cancel = latest.begin(&self.root_cancel);
        self.spawn_read_under(op, cancel, read);
    }

    fn spawn_read_under<F, Fut>(
        self: &Arc<Self>,
        op: &'static str,
        cancel: CancellationToken,
        read: F,
    ) where
        F: FnOnce(Arc<Self>, PathBuf, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<SessionEvent, GitError>> + Send,
    {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
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
