//! The queue itself: one per consumer, and the set of them a tab owns.

use super::*;

/// A queue whose consumer is one QML object on the Qt main thread.
pub struct Feed<T> {
    state: Mutex<FeedState<T>>,
}

struct FeedState<T> {
    queue: VecDeque<T>,
    latest: Option<u64>,
    invoker: Option<QmlMethodInvoker>,
}

impl<T> Default for Feed<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(FeedState {
                queue: VecDeque::new(),
                latest: None,
                invoker: None,
            }),
        }
    }
}

impl<T> Feed<T> {
    fn lock(&self) -> MutexGuard<'_, FeedState<T>> {
        match self.state.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// Appends a message and wakes the attached consumer.
    pub fn push(&self, item: T) {
        let guard = {
            let mut s = self.lock();
            s.queue.push_back(item);
            s
        };
        Self::wake(guard);
    }

    /// Replaces any queued messages (snapshot semantics) and wakes.
    pub fn push_replace(&self, item: T) {
        let guard = {
            let mut s = self.lock();
            s.queue.clear();
            s.queue.push_back(item);
            s
        };
        Self::wake(guard);
    }

    /// Replaces the queued messages of this one's kind — its enum variant
    /// — and keeps the rest: snapshot semantics for a feed that carries
    /// more than one kind (a newer status supersedes an undrained one; the
    /// HEAD report beside it stays).
    pub fn push_coalescing(&self, item: T) {
        let kind = std::mem::discriminant(&item);
        let guard = {
            let mut s = self.lock();
            s.queue
                .retain(|queued| std::mem::discriminant(queued) != kind);
            s.queue.push_back(item);
            s
        };
        Self::wake(guard);
    }

    /// One pending answer, ordered by request. The watermark outlives a
    /// drain, so a late duplicate neither wakes the consumer nor stays
    /// queued; [`Feed::clear_queued`] resets it for a new session.
    pub fn push_latest(&self, generation: u64, item: T) {
        let mut guard = self.lock();
        if guard.latest.is_some_and(|latest| generation <= latest) {
            return;
        }
        guard.latest = Some(generation);
        guard.queue.clear();
        guard.queue.push_back(item);
        Self::wake(guard);
    }

    fn wake(guard: MutexGuard<'_, FeedState<T>>) {
        // Invoking while holding the lock is fine: it only posts a queued
        // call to the Qt event loop and never runs the slot synchronously.
        if let Some(inv) = &guard.invoker {
            inv.invoke_method("drain");
        }
    }

    /// Registers the consumer (Qt main thread) and wakes it if messages
    /// are already waiting.
    pub fn attach(&self, invoker: QmlMethodInvoker) {
        let mut s = self.lock();
        let has_pending = !s.queue.is_empty();
        if has_pending {
            invoker.invoke_method("drain");
        }
        s.invoker = Some(invoker);
    }

    /// Takes everything queued (called from the consumer's `drain` slot).
    pub fn drain(&self) -> Vec<T> {
        self.lock().queue.drain(..).collect()
    }

    /// Messages waiting for their consumer. Only the memory report reads
    /// it: a feed nobody drains keeps whole snapshots alive.
    pub fn depth(&self) -> usize {
        self.lock().queue.len()
    }

    /// Drops what is queued and lets go of the consumer — what a tab does
    /// on its way off the front (`Hub::release_tab`). Both halves matter:
    /// a queued refs snapshot is the very memory the release is for, and
    /// the invoker names a QML object about to be destroyed (the page that
    /// comes back attaches its own).
    pub fn release(&self) {
        let mut s = self.lock();
        s.queue.clear();
        s.invoker = None;
    }

    /// Drops what is queued and keeps the consumer.
    ///
    /// Cancellation is cooperative, so a task can push after
    /// [`Feed::release`], and a page opening the repository again would
    /// read that as the new session's. Worst in the graph: its generations
    /// restart with the model, so one stale `Started` leaves it on a number
    /// the new stream never reaches and none of its chunks is drawn.
    pub fn clear_queued(&self) {
        let mut guard = self.lock();
        guard.queue.clear();
        guard.latest = None;
    }
}

/// All feeds of one tab. Each feed is independently `Arc`-shared with the
/// QML object that consumes it (one consumer per feed; status fans out to
/// the headline and to one list per bucket run).
#[derive(Default)]
pub struct Feeds {
    pub tab: Arc<Feed<TabMsg>>,
    pub graph: Arc<Feed<GraphMsg>>,
    /// Refs fan out to one feed per sidebar section (one consumer each).
    /// Shared: a deep copy per section would duplicate tens of thousands
    /// of strings. The branches section's also carries HEAD.
    pub refs_branches: Arc<Feed<RefsMsg>>,
    pub refs_remotes: Arc<Feed<RefsMsg>>,
    pub refs_tags: Arc<Feed<RefsMsg>>,
    /// Status headline consumer (WorkingTreeModel): where the tree stands
    /// — HEAD and what is derived from it — and the last status's
    /// counts, in the order the session said them.
    pub status: Arc<Feed<StateMsg>>,
    /// Status list consumers — one `files` NavSectionModel per bucket
    /// run of the changed files the right pane's WIP view lists. A feed is
    /// drained by whoever gets there first, so each run needs its own.
    pub status_nav_conflicts: Arc<Feed<StatusMsg>>,
    pub status_nav_unstaged: Arc<Feed<StatusMsg>>,
    pub status_nav_staged: Arc<Feed<StatusMsg>>,
    /// The list of another worktree the pane is showing (`CarriedStatusMsg`).
    /// One list where this window's own tree has three — nothing here can
    /// move that worktree's index — and a feed of its own: shared, this
    /// window's status tick would take the other's rows off the screen.
    pub carried_nav: Arc<Feed<CarriedStatusMsg>>,
    pub stash: Arc<Feed<super::StashList>>,
    pub worktrees: Arc<Feed<super::WorktreeList>>,
    pub details: Arc<Feed<DetailsMsg>>,
    pub diff: Arc<Feed<DiffMsg>>,
    pub commands: Arc<Feed<CommandMsg>>,
    /// The interactive-rebase screen's plan rows (one consumer, the plan
    /// model; one pending answer, ordered by the ask the way the details
    /// feed is — `Feed::push_latest`).
    pub plan: Arc<Feed<PlanMsg>>,
    /// How far a running replay has got. Same consumer as `status`, a feed
    /// of its own because it is asked several times a second: through the
    /// status snapshot it would be a whole `git status` per tick
    /// (`RepoSession::refresh_op_progress`).
    pub op_progress: Arc<Feed<OpProgressMsg>>,
}

/// One feed, seen without its message type, so a tab's feeds fit one
/// plain array ([`Feeds::each`]).
pub trait FeedOps {
    fn release(&self);
    fn clear_queued(&self);
    fn depth(&self) -> usize;
}

impl<T> FeedOps for Feed<T> {
    fn release(&self) {
        Feed::release(self);
    }
    fn clear_queued(&self) {
        Feed::clear_queued(self);
    }
    fn depth(&self) -> usize {
        Feed::depth(self)
    }
}

/// The command log's name in [`Feeds::each`] — spelled once:
/// [`Feeds::clear_queued_reads`] skips it by name, and a rename that
/// missed that walk would empty the log's queue in silence.
const COMMANDS: &str = "commands";

impl Feeds {
    /// Every feed with its report name — the one list of feeds, which the
    /// walks below and the depth report ([`crate::hub::Hub::feed_depths`])
    /// iterate. A feed left out is a queue that goes on holding a
    /// repository nobody is reading.
    pub fn each(&self) -> [(&'static str, &dyn FeedOps); 17] {
        [
            ("tab", &*self.tab),
            ("graph", &*self.graph),
            ("refs-branches", &*self.refs_branches),
            ("refs-remotes", &*self.refs_remotes),
            ("refs-tags", &*self.refs_tags),
            ("status", &*self.status),
            ("status-nav-conflicts", &*self.status_nav_conflicts),
            ("status-nav-unstaged", &*self.status_nav_unstaged),
            ("status-nav-staged", &*self.status_nav_staged),
            ("carried-nav", &*self.carried_nav),
            ("stash", &*self.stash),
            ("worktrees", &*self.worktrees),
            ("details", &*self.details),
            ("diff", &*self.diff),
            (COMMANDS, &*self.commands),
            ("plan", &*self.plan),
            ("op-progress", &*self.op_progress),
        ]
    }

    /// Empties every queue and forgets every consumer ([`Feed::release`]).
    pub fn release_all(&self) {
        for (_, feed) in self.each() {
            feed.release();
        }
    }

    /// Empties every queue but the command log's and keeps the consumers
    /// ([`Feed::clear_queued`]) — for a session opening over a page that
    /// is already standing (`Hub::open_session`).
    ///
    /// The log is the one feed that outlives the session: a command
    /// message names the session that ran it (`CommandMsg`) and belongs to
    /// rows staying on screen. A feed added later is emptied unless it is
    /// spelled out here.
    pub fn clear_queued_reads(&self) {
        for (name, feed) in self.each() {
            if name != COMMANDS {
                feed.clear_queued();
            }
        }
    }
}

/// Hands `invoker` to `feed` and gives the caller its own handle on it.
/// By value: an invoker is not `Clone` (.claude/rules/app-ui.md
/// §Qt Bridges・QML の不変条件).
pub fn attached<T>(feed: &Arc<Feed<T>>, invoker: QmlMethodInvoker) -> Arc<Feed<T>> {
    let feed = Arc::clone(feed);
    feed.attach(invoker);
    feed
}

/// The same for one feed of a tab, picked out of its [`Feeds`]. `None`
/// when the tab is gone — a slot that arrives for one has nothing to
/// attach to.
pub fn attach_feed<T>(
    tab_id: i32,
    pick: impl FnOnce(&Feeds) -> &Arc<Feed<T>>,
    invoker: QmlMethodInvoker,
) -> Option<Arc<Feed<T>>> {
    let feeds = Hub::with(|hub| hub.feeds(tab_id))??;
    Some(attached(pick(&feeds), invoker))
}
