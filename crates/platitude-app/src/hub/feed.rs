//! The queue itself: one per consumer, and the set of them a tab owns.

use super::*;

/// A queue whose consumer is one QML object on the Qt main thread.
pub struct Feed<T> {
    state: Mutex<FeedState<T>>,
}

struct FeedState<T> {
    queue: VecDeque<T>,
    invoker: Option<QmlMethodInvoker>,
}

impl<T> Default for Feed<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(FeedState {
                queue: VecDeque::new(),
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
    /// on its way off the front (`Hub::release_tab`).
    ///
    /// Both halves matter. The queue is where a snapshot of fifty
    /// thousand refs sits waiting, and holding one for a page that has
    /// been taken down is the very memory the release is for. The invoker
    /// names a QML object that is about to be destroyed, so a message
    /// racing the release would be waking something that is no longer
    /// there; the page that comes back attaches its own.
    pub fn release(&self) {
        let mut s = self.lock();
        s.queue.clear();
        s.invoker = None;
    }

    /// Drops what is queued and keeps the consumer.
    ///
    /// For the other end of a release: cancellation is cooperative, so a
    /// task that had already worked out an answer can push it after
    /// [`Feed::release`] has run, and it would then be waiting here for a
    /// page that opens the repository *again* — a message about a session
    /// that no longer exists, read as though it were about the new one.
    /// The graph is where that goes worst: its generations restart with
    /// the model, so one stale `Started` leaves it holding a number the
    /// new stream never reaches and no chunk it sends is ever drawn.
    pub fn clear_queued(&self) {
        self.lock().queue.clear();
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
    /// Shared, not copied: a deep copy per section duplicates tens of
    /// thousands of strings.
    pub refs_branches: Arc<Feed<Arc<RefsSnapshot>>>,
    pub refs_remotes: Arc<Feed<Arc<RefsSnapshot>>>,
    pub refs_tags: Arc<Feed<Arc<RefsSnapshot>>>,
    /// Status headline consumer (WorkTreeModel: header props/counts).
    pub status: Arc<Feed<StatusMsg>>,
    /// Status list consumers — one `worktree` NavSectionModel per bucket
    /// run of the changed files the right pane's WIP view lists. Each run
    /// is a list of its own with a share of the pane of its own, and a feed
    /// is drained by whoever gets there first, so each run needs a feed of
    /// its own (the refs fan out per sidebar section the same way).
    pub status_nav_conflicts: Arc<Feed<StatusMsg>>,
    pub status_nav_unstaged: Arc<Feed<StatusMsg>>,
    pub status_nav_staged: Arc<Feed<StatusMsg>>,
    pub stash: Arc<Feed<Vec<StashEntry>>>,
    pub worktrees: Arc<Feed<Vec<platitude_core::worktrees::WorktreeEntry>>>,
    pub details: Arc<Feed<DetailsMsg>>,
    pub diff: Arc<Feed<DiffMsg>>,
    pub commands: Arc<Feed<CommandMsg>>,
}

/// One feed, seen without its message type — the three questions every
/// walk over "all of a tab's feeds" asks. None of [`Feed`]'s answers here
/// depend on `T`, which is what lets the list of feeds live in one plain
/// array ([`Feeds::each`]) instead of a macro per walk.
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

impl Feeds {
    /// Every feed with its report name. **The list of feeds is written
    /// once, here** — the walks below and the depth report
    /// ([`crate::hub::Hub::feed_depths`]) all iterate this. A hand-kept
    /// second copy is free to disagree about what a tab holds, and a feed
    /// left out of one is a queue that goes on holding a repository
    /// nobody is reading — or one the memory report cannot name, which is
    /// exactly the queue that report exists to catch.
    pub fn each(&self) -> [(&'static str, &dyn FeedOps); 14] {
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
            ("stash", &*self.stash),
            ("worktrees", &*self.worktrees),
            ("details", &*self.details),
            ("diff", &*self.diff),
            ("commands", &*self.commands),
        ]
    }

    /// Empties every queue and forgets every consumer ([`Feed::release`]).
    pub fn release_all(&self) {
        for (_, feed) in self.each() {
            feed.release();
        }
    }

    /// Empties every queue and keeps the consumers ([`Feed::clear_queued`]).
    pub fn clear_queued_all(&self) {
        for (_, feed) in self.each() {
            feed.clear_queued();
        }
    }
}

/// Hands `invoker` to `feed` and gives the caller its own handle on it.
///
/// The invoker is passed by value because it has to be: `QmlMethodInvoker`
/// is `Send` but not `Clone`, so a consumer takes its own from
/// `get_qml_method_invoker()` rather than sharing one
/// (.claude/rules/app-ui.md "Qt Bridges の要点").
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
