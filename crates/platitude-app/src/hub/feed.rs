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
}

/// All feeds of one tab. Each feed is independently `Arc`-shared with the
/// QML object that consumes it (one consumer per feed; status fans out to
/// two consumers via a second feed).
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
    /// Status list consumer (the `worktree` NavSectionModel — the changed
    /// files the right pane's WIP view lists).
    pub status_nav: Arc<Feed<StatusMsg>>,
    pub stash: Arc<Feed<Vec<StashEntry>>>,
    pub worktrees: Arc<Feed<Vec<platitude_core::worktrees::WorktreeEntry>>>,
    pub details: Arc<Feed<platitude_core::details::CommitDetails>>,
    pub diff: Arc<Feed<DiffMsg>>,
    pub commands: Arc<Feed<CommandMsg>>,
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
