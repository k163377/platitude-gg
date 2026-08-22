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
    pub details: Arc<Feed<platitude_core::details::CommitDetails>>,
    pub diff: Arc<Feed<DiffMsg>>,
    pub commands: Arc<Feed<CommandMsg>>,
}

/// Expands `$with!(feeds, field, "name")` once per feed a tab has.
///
/// The list of feeds is written once, here. There is nothing to iterate —
/// each feed carries a different message type — so every walk over "all
/// of a tab's feeds" expands from this list through a small local adapter
/// macro: the no-argument methods below, and the named depth report
/// ([`crate::hub::Hub::feed_depths`]). A hand-kept second copy of the
/// list is free to disagree about what a tab holds, and a feed left out
/// of one is a queue that goes on holding a repository nobody is reading
/// — or one the memory report cannot name, which is exactly the queue
/// that report exists to catch.
macro_rules! for_every_feed {
    ($with:ident, $feeds:expr) => {
        $with!($feeds, tab, "tab");
        $with!($feeds, graph, "graph");
        $with!($feeds, refs_branches, "refs-branches");
        $with!($feeds, refs_remotes, "refs-remotes");
        $with!($feeds, refs_tags, "refs-tags");
        $with!($feeds, status, "status");
        $with!($feeds, status_nav_conflicts, "status-nav-conflicts");
        $with!($feeds, status_nav_unstaged, "status-nav-unstaged");
        $with!($feeds, status_nav_staged, "status-nav-staged");
        $with!($feeds, stash, "stash");
        $with!($feeds, worktrees, "worktrees");
        $with!($feeds, details, "details");
        $with!($feeds, diff, "diff");
        $with!($feeds, commands, "commands");
    };
}
pub(super) use for_every_feed;

impl Feeds {
    /// Empties every queue and forgets every consumer ([`Feed::release`]).
    pub fn release_all(&self) {
        macro_rules! call {
            ($feeds:expr, $field:ident, $name:literal) => {
                $feeds.$field.release();
            };
        }
        for_every_feed!(call, self);
    }

    /// Empties every queue and keeps the consumers ([`Feed::clear_queued`]).
    pub fn clear_queued_all(&self) {
        macro_rules! call {
            ($feeds:expr, $field:ident, $name:literal) => {
                $feeds.$field.clear_queued();
            };
        }
        for_every_feed!(call, self);
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
