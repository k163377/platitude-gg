//! The session handle itself: what one open repository holds, and what
//! it cancels on the way out (the opening sequence is [`super::open`]).

use super::*;

pub struct RepoSession {
    /// Reads and refreshes: recorded in the command log only while
    /// background recording is on.
    pub(super) executor: GitExecutor,
    /// The reads nobody is waiting on — the other copies' status, the
    /// walk behind a chip, the remote tags — on the handle the slots
    /// serve last and keep out of the click's reserve
    /// (`process::Priority::Background`). Off the log like `executor`.
    /// **Not the poll's own reads**: what the front page shows is what
    /// somebody is looking at, and those reads are single-flight already
    /// (`ReadFlight`), so a slot is all they need.
    pub(super) exec_background: GitExecutor,
    /// The queue's handle: everything run through it is something the
    /// user asked for, and is always recorded.
    pub(super) exec_user: GitExecutor,
    /// The reaches for the network nobody asked for — the interval's
    /// fetch and the one an opening fires. Off the log while they land,
    /// and a row when git says no: the panel that a fetch failure raises
    /// has to hold the command that raised it
    /// (デザイン規約 §git が言ったことを読む場所).
    pub(super) exec_unasked_fetch: GitExecutor,
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
    /// One commit-details read at a time, numbered (`load_details`).
    pub(super) details_read: Latest,
    /// One interactive-rebase plan ask at a time (`ask_rebase_plan`).
    pub(super) plan_read: Latest,
    /// The write the queue is serving, held from before git runs it
    /// until the reads it invalidated have published — the whole request
    /// — and `None` between requests. Two readers: the poll, which stays
    /// out of a repository mid-operation unless the write is one that
    /// replays ([`OperationKind::replays_history`]), and a task running
    /// under the queue that has to say which write it is
    /// (`note_landing`).
    pub(super) write_running: Mutex<Option<Operation>>,
    /// The graph passes that could still walk, whether or not the stream
    /// is still theirs (see [`GraphPasses`]).
    pub(super) graph_passes: Arc<GraphPasses>,
    /// What the outside was let into the graph passes with when this
    /// session was opened ([`PassHooks`]) — `None` in the application as
    /// shipped, where no pass asks anything and nothing is held for it.
    pub(super) pass_hooks: Option<Arc<dyn PassHooks>>,
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
    /// [`RepoSession::load_diff`], and read again at every point a read
    /// would hand something over: a reader going down a commit's file
    /// list starts a read per row, and two reads of one file overlap
    /// wherever a write answers and the status behind it lands.
    ///
    /// **A read another has passed hands over nothing** — not its
    /// colours, which cost enough (see [`SessionEvent::DiffColoured`])
    /// that the ones nobody is waiting for are worth not doing at all,
    /// and not its rows either: those are cheap, but the fingerprint that
    /// rides with them is what the next partial stage is refused against
    /// (`stage::refusal::verify_fingerprint`), so an older read landing
    /// last leaves the pane unable to stage anything (`publish_diff`).
    pub(super) diff_epoch: AtomicU64,
    /// The diff last published and the fingerprint of the bytes it was
    /// read from — what [`RepoSession::refresh_diff`] compares against, so
    /// a poll tick over a file nobody has touched sends nothing at all.
    ///
    /// One slot, because one pane asks: `load_diff` is reached from the
    /// diff view alone. A read for some other target therefore reads as
    /// "not the one held", which is the safe direction — it publishes.
    ///
    /// **The copy it was read from is part of the key.** The same path in
    /// another working copy is another file, and the pane reads those too
    /// (`RepoSession::load_carried_diff`); keyed by target alone, a poll
    /// of this tree would match a fingerprint taken from somebody else's
    /// bytes and call the file unmoved.
    ///
    /// Written where the event goes out rather than where the bytes are
    /// read: recording a fingerprint the pane never received would leave
    /// it stale for as long as the file stayed that way.
    pub(super) last_diff: Mutex<Option<(PathBuf, DiffTarget, u64)>>,
    /// Lexer states remembered down the file the last diff's colours
    /// were read against (`highlight::LexCache`) — what lets the re-read
    /// after every partial stage start near its hunks instead of at
    /// line 1. Self-invalidating: the cache carries the source text's
    /// hash and is dropped by the reader when the text has changed.
    pub(super) lex_cache: Mutex<Option<crate::highlight::LexCache>>,
    /// The files a picture's blob sides are written to so the pane can
    /// name them by URL (`preview::PreviewFiles`). Numbered by the diff
    /// epoch above; each read published sweeps the reads before it, the
    /// pane closing sweeps them all ([`RepoSession::release_preview`]),
    /// and the close removes the directory.
    pub(super) preview_files: preview::PreviewFiles,
    /// Where the repository stands, as the reads left it — HEAD and
    /// everything derived from it, the dirty tree, the standing merge's
    /// sides, the rebase stop and the merge tool. **The one record of
    /// each**: every read reports into it and every reader asks it, and
    /// the stamps it orders the reports by are what keep a read that
    /// looked before a write from overwriting the write's own read
    /// (`session::standing`).
    pub(super) standing: Standing,
    /// Set by [`RepoSession::ask_merge_tool`] to have the next status read
    /// name the merge tool even with nothing conflicted. Cleared by that
    /// read: two `git config` spawns on every poll of every open tab is
    /// not a price the common case should pay for a settings field.
    pub(super) merge_tool_wanted: std::sync::atomic::AtomicBool,
    /// One signature verification at a time: a selection that moves on
    /// cancels the gpg or ssh-keygen run the last one started, which is
    /// the slowest read a click can start (`session::latest`).
    pub(super) signature_read: Latest,
    /// One remote-branch check at a time — a name being typed asks per
    /// settled keystroke, and each ask is a network round trip.
    pub(super) remote_branch_read: Latest,
    /// One check of whether a remote already has HEAD's commit at a time,
    /// for the rare pass that could not read it off its rows
    /// ([`RepoSession::settle_head_published`]).
    pub(super) head_published_read: Latest,
    /// One read of another working copy's files at a time: the pane shows
    /// one copy, and a reader walking down the rows would otherwise leave
    /// a whole `status` running behind each row they passed
    /// ([`RepoSession::read_carried_status`]).
    ///
    /// It is also what settles the answers against each other. These are
    /// reads of different trees, so they take different times, and the
    /// pane holding whichever finished last would be showing the copy the
    /// reader stepped off.
    pub(super) carried_read: Latest,
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
    /// Local writes handed to the queue and not yet done with — the ones
    /// a close waits out rather than kills ([`Lane::Local`]; the other
    /// lanes die with the session). What the application's quit gate
    /// reads: zero on every open session is the moment the window may
    /// go.
    pub(super) local_writes: std::sync::atomic::AtomicUsize,
    /// The write loop's own task. Handed to the application on the way
    /// out ([`RepoSession::take_write_join`]): dropping the runtime drops
    /// the loop mid-write, and `kill_on_drop` then ends git itself — so a
    /// shutdown joins this first.
    pub(super) write_join: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// One permit, held by a running poll: a tick that arrives while the
    /// previous one is still reading is dropped rather than queued.
    pub(super) poll_slot: Arc<tokio::sync::Semaphore>,
    /// A read this session owes the working tree: a write landed in it
    /// while this session was already reading, so the read holding the
    /// slot began before that write and cannot answer for it
    /// ([`RepoSession::read_again`]). Remembered rather than dropped,
    /// because a clock tick that is refused comes round again and a
    /// write's news does not.
    pub(super) read_owed: std::sync::atomic::AtomicBool,
    /// The same as the poll's permit for the other copies' own tick, which
    /// is slower and carries a `status` per copy — on a big tree a pass
    /// can outlast its interval, and stacking them is what the cap is
    /// there to prevent.
    pub(super) carried_slot: Arc<tokio::sync::Semaphore>,
    /// Whether the other copies are read at all (`set_copies_read` —
    /// the settings' "never", which has to hold for the reads an opening
    /// and a focus fire, not only for the page's tick), what the last
    /// pass left — a row each, drawn where their HEAD lands — and the
    /// pass in flight, under one lock (`carried::Copies`).
    pub(super) copies: super::carried::Copies,
    /// One in flight per snapshot, for the reads a repository can be asked
    /// for from more than one place at once (see [`ReadFlight`]). Each
    /// answers its callers with what they act on: whether the refs or
    /// the WIP row moved, whether the stash listing published, what the
    /// worktree listing found moved.
    pub(super) refs_read: ReadFlight<Reread>,
    pub(super) status_read: ReadFlight<Reread>,
    pub(super) stash_read: ReadFlight,
    pub(super) worktrees_read: ReadFlight<WorktreeRead>,
    /// Submission end of the write queue (see the module docs).
    pub(super) write_tx: tokio::sync::mpsc::UnboundedSender<WriteRequest>,
    /// The order this working tree's local writes run in, shared with
    /// every other session on the same tree (`session::write_order`).
    /// `None` until the repository is open, because which tree it is is
    /// not known before that — and nothing can be asked for either.
    pub(super) write_order: Mutex<Option<Arc<WriteOrder>>>,
    /// Held while a write takes its place in that order and goes into the
    /// queue above, so the two can never disagree about which write came
    /// first ([`RepoSession::write`] says what disagreeing would cost).
    /// Nothing else about a write is under it.
    pub(super) accepting: Mutex<()>,
    /// Set by the close, before it gives anything back
    /// ([`RepoSession::keeps_what_it_reads`]). Reads already in flight
    /// answer after it, and what they answer with must not be kept.
    pub(super) released: std::sync::atomic::AtomicBool,
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
    /// Where each other working copy's HEAD stood when the listing last
    /// named it, by the key its path is compared on (`joins::same_path_key`).
    ///
    /// **The cheap half of what a copy's row is drawn from.** The row's
    /// place and its tallies come from a `status` of that copy on a tick
    /// of its own — most of a second each, so a slow one
    /// (`session::carried`) — while the listing that names every copy's
    /// HEAD is one process of twenty-odd milliseconds on the page's own
    /// tick. So this is how a window learns that a copy has committed
    /// long before it learns what the copy is now carrying, and the row
    /// drawn from the older reading would stand on a commit that copy
    /// has left (`relay::Standing`).
    ///
    /// **Kept apart from [`Self::worktree_holders`] on purpose**: that
    /// set decides whether the refs are read and the graph walked again,
    /// and a commit in a neighbouring copy moves no row of ours — folded
    /// in there, every one of them would spend a listing of tens of
    /// thousands of refs to say nothing (`joins::note_worktree_holders`).
    pub(super) copy_heads: Mutex<Arc<std::collections::HashMap<String, Oid>>>,
    /// Bumped when that set became a different one. **Nothing else in the
    /// refs key would notice** — taking or giving back a working copy
    /// moves no ref, so without this the mark would wait for an unrelated
    /// ref to move.
    pub(super) worktree_gen: AtomicU64,
    /// One permit for the background read of the above, so a second
    /// permission-granting call cannot stack another on top of it — and
    /// the word that it was let go, for the asks that call stacked
    /// nothing for (`auto_fetch::RemoteTagSlot`).
    pub(super) remote_tags_slot: super::auto_fetch::RemoteTagSlot,
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

    /// Git directory of the opened repository (None until `Opened`).
    ///
    /// Same reason as the line above, more so: the running replay's counter
    /// is read off this several times a second and nothing else about it is
    /// wanted ([`Self::refresh_op_progress`]).
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
    /// Applies to commands spawned from here on, not retroactively, and
    /// not to an opening already under way: where a session *starts* is
    /// [`RepoSession::open_recording`]'s to say (see [`Recording`]).
    pub fn set_recording(&self, recording: Recording) {
        self.commands.set_recording(recording);
    }

    /// Cancels everything this session is doing — except the local
    /// writes already asked for, running and queued alike, which run to
    /// completion on tokens of their own ([`Lane::Local`]):
    /// killing git mid-write loses what the user asked for, dropping a
    /// queued request loses it silently, and a local git is only ever
    /// slow in proportion to the work. What stops is intake — nothing
    /// sent after the close is accepted — and the network-paced requests
    /// in the tail, which die on this cancel as always. Idempotent.
    ///
    /// **The places those writes hold in the working tree's order are
    /// not given up** (`session::write_order`): they are what a session
    /// opened over this one queues behind, and giving them back here
    /// would be the overtaking this close is not allowed to cause. What
    /// is given up is the reading half — this session is off the list of
    /// pages the tree reports to, and lets go of what it had drawn.
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

    /// Lets go of what this session read for a page that is gone.
    ///
    /// A close is not the end of the session: a local write let run on
    /// holds it alive to the last (`Hub::park_writes_of`), and on a
    /// repository the size of the budget's these are the largest things
    /// in the process — the drawn graph, and the refs snapshot and remote
    /// tag index behind its chips. Nothing left here reads them: a closed
    /// session makes no reads behind its writes, and what they leave is
    /// read by whichever session holds the tree next.
    /// Whether what this session's reads answer with is still kept.
    ///
    /// **Read inside the lock of whatever is about to be stored**, and
    /// false from before the close gives anything back, so a read already
    /// in flight either lands ahead of the release — and is cleared by it
    /// — or finds the door shut. Without that the pass that was walking
    /// when the tab went would write its chips back into the `Shared`
    /// just emptied, and the memory would be held for as long as the
    /// write keeping the session alive takes.
    ///
    /// Reading needs no door: a released session's copy is empty, which
    /// is the true answer.
    pub(super) fn keeps_what_it_reads(&self) -> bool {
        !self.released.load(Ordering::SeqCst)
    }

    /// Where a read's answer about the graph is kept — `None` once this
    /// session has let go of it ([`Self::keeps_what_it_reads`]).
    pub(super) fn store_shared(&self) -> Option<std::sync::MutexGuard<'_, Shared>> {
        let shared = self.lock_shared();
        self.keeps_what_it_reads().then_some(shared)
    }

    /// Replaced rather than emptied, every one of them: `clear()` keeps a
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
    /// [`RepoSession::close`] waits out rather than kills (network
    /// writes die with the session instead). Zero is the moment nothing
    /// here would outlast a shutdown.
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
