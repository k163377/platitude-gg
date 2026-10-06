//! What one session holds between operations: the derived answers, the
//! gates and slots that keep concurrent reads from tripping over each
//! other, and the shapes the log and refresh tasks share.

use super::*;

/// How a config file looked, closely enough to tell "somebody wrote this"
/// from "nobody has touched it".
///
/// A missing file has a stamp of its own, so one that appears later reads
/// as the change it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfigStamp {
    modified: Option<std::time::SystemTime>,
    len: u64,
}

impl ConfigStamp {
    pub(super) fn of(path: &Path) -> Self {
        match std::fs::metadata(path) {
            Ok(meta) => Self {
                modified: meta.modified().ok(),
                len: meta.len(),
            },
            Err(_) => Self {
                modified: None,
                len: 0,
            },
        }
    }
}

/// One tick handed to the timer by hand; the timer answers on it once
/// it has acted (see [`AutoFetchTicker`]).
pub(super) type AutoFetchTick = tokio::sync::oneshot::Sender<()>;

/// The auto-fetch timer that is running: what stops it, and the way in for
/// a tick that does not come from the clock.
pub(super) struct AutoFetch {
    pub(super) cancel: CancellationToken,
    pub(super) ticks: tokio::sync::mpsc::UnboundedSender<AutoFetchTick>,
}

/// Where the fetch that opening a repository fires stands.
///
/// The app's ask (once it has handed over its settings) and the opening
/// can arrive in either order; under one lock, whichever arrives second
/// fires it, exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OpenFetchState {
    /// Nobody has asked for one.
    Unasked,
    /// Asked for before the repository was open; the opening redeems it.
    Held,
    /// Started, declined, or already spent — this session asks no more.
    Settled,
}

/// Steps one auto-fetch timer by hand, in place of waiting out its
/// interval.
///
/// Bound to the timer running when [`RepoSession::auto_fetch_ticker`]
/// handed it out, so a tick reports whether that timer is still there —
/// which makes "this timer fetches nothing any more" something to wait
/// for rather than a quiet to sit out. The app only ever sets an interval.
pub struct AutoFetchTicker {
    pub(super) ticks: tokio::sync::mpsc::UnboundedSender<AutoFetchTick>,
    /// The bound timer's stop, read directly: a cancelled task is polled
    /// on no deadline, so waiting for it to drop the channel can hang the
    /// caller under load.
    pub(super) stopped: CancellationToken,
}

impl AutoFetchTicker {
    /// Fires one tick and resolves once the timer has acted on it: `true`
    /// when it took the tick, `false` once that timer has stopped (turned
    /// off, replaced by another interval, or gone with the session). A
    /// stopped timer never takes a tick, not even one that raced its own
    /// cancellation, so `false` is the last word on it.
    pub async fn tick(&self) -> bool {
        let (ack, taken) = tokio::sync::oneshot::channel();
        if self.ticks.send(ack).is_err() {
            return false;
        }
        // Biased towards the answer: a tick acted on says so even when the
        // stop lands right behind it; the token answers for a stopped timer
        // (`stopped`).
        tokio::select! {
            biased;
            answered = taken => answered.is_ok(),
            () = self.stopped.cancelled() => false,
        }
    }
}

/// One write as the queue was asked for it — the header every event
/// about the write repeats, and what the queue reads to serve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Operation {
    /// Handed back at acceptance; every event about the write carries it.
    pub(super) id: OperationId,
    pub(super) kind: OperationKind,
    pub(super) lane: Lane,
    /// What the write invalidates — which reads follow it.
    pub(super) after: AfterWrite,
}

impl Operation {
    pub(super) fn new(kind: OperationKind, after: AfterWrite) -> Self {
        Self {
            id: OperationId::next(),
            kind,
            lane: kind.lane(),
            after,
        }
    }
}

/// What a snapshot read came back with, for the callers that act on it
/// — the answer the refs and status flights share ([`ReadFlight`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Reread {
    /// Nothing the caller acts on moved. Also what a read fenced by a
    /// write answers, having asked again for itself.
    #[default]
    Same,
    /// What the caller acts on moved: the WIP row, or the refs.
    Moved,
    /// The read failed and published nothing. The failure is on the error
    /// surface already ([`SessionEvent::OpFailed`]); this lets a write
    /// settling behind the read name it under its own id.
    Failed,
}

/// What one stash listing published — the flight's shared answer, so a
/// walk can take the stashes from the listing that answered it instead of
/// reading them again (`RepoSession::read_stashes`), while that listing
/// still stands (`RepoSession::stashes_standing`).
#[derive(Debug, Clone, Default)]
pub(super) struct StashRead {
    /// The list published; `None` is a failed read.
    pub(super) list: Option<Arc<Vec<StashEntry>>>,
    /// When it looked (`Standing::stamp`).
    pub(super) looked: u64,
    /// Which listing it was, counted over every reader ([`StashListings`]).
    pub(super) listing: u64,
    /// Whether it differs from the listing before it. A stash taken or
    /// dropped moves no ref the refs read lists, so this is the only word
    /// of it a walk gets.
    pub(super) moved: bool,
}

/// The stash listings so far: the last one's list, and how many.
#[derive(Default)]
pub(super) struct StashListings {
    last: Option<Arc<Vec<StashEntry>>>,
    count: u64,
}

impl StashListings {
    /// Records a listing, answering its number and whether it moved. The
    /// first moves nothing: the opening's walk reads the stashes itself.
    pub(super) fn record(&mut self, list: &Arc<Vec<StashEntry>>) -> (u64, bool) {
        self.count += 1;
        let moved = self.last.as_ref().is_some_and(|last| **last != **list);
        self.last = Some(Arc::clone(list));
        (self.count, moved)
    }

    /// The number of the newest listing.
    pub(super) fn newest(&self) -> u64 {
        self.count
    }
}

/// What the worktree listing came back with — the flight's shared answer,
/// so every caller answered by one pass acts on the same news and none
/// of them settles before the reads that news asks for.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct WorktreeRead {
    /// Whether the listing published; `false` is a failed read.
    pub(super) published: bool,
    /// What the listing found moved, and so which reads it asks for.
    pub(super) news: super::joins::WorktreeNews,
}

/// A queued write: what it is, where it stands in its worktree's
/// order, and what to run.
pub(super) struct WriteRequest {
    pub(super) operation: Operation,
    /// Taken at acceptance and held until the request is done — `None`
    /// for the lanes that take no place (`session::write_order`). A request
    /// the queue turns away gives it back by being dropped.
    pub(super) place: Option<super::write_order::Place>,
    /// Places in the orders of the other worktrees the write puts
    /// work into (a restore of work thrown away there), taken with `place`
    /// and held as long ([`RepoSession::write_into`]).
    pub(super) into: Vec<super::write_order::Place>,
    #[expect(
        clippy::type_complexity,
        reason = "a boxed async job needs its shape spelled out"
    )]
    pub(super) run: Box<
        dyn FnOnce(
                GitExecutor,
                RepoInfo,
                CancellationToken,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), GitError>> + Send>,
            > + Send,
    >,
}

/// What a pass reported under its graph: how far the walk got, and
/// whether the window cut it.
///
/// It does not follow from the rows (walked is not shown — `LogTotals`),
/// so two passes can draw the same graph and owe different footers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Footer {
    pub(super) walked: u32,
    pub(super) truncated: bool,
}

/// Shared mutable state between the log task and refs joins.
#[derive(Default)]
pub(super) struct Shared {
    pub(super) builder: GraphBuilder,
    /// Carries "a remote already has this" down the walk the `builder` is
    /// drawing; reset with it, since both are one pass's state over one
    /// window (`session::published::PublishMarks`).
    pub(super) publish_marks: PublishMarks,
    /// The graph the consumer is on, which `builder`, `applied` and
    /// `sent_rows` speak in. Not `log_gen`: that is bumped before a pass
    /// takes this lock and by passes that send nothing, while everything
    /// here still belongs to the walk that was shown.
    pub(super) generation: u64,
    /// Label chips per commit id, derived from the last refs snapshot.
    pub(super) label_map: LabelIndex,
    /// Labels currently shown per row (for diffing on refs refresh).
    pub(super) applied: HashMap<u32, Vec<RefLabel>>,
    /// A print of each row as delivered to the UI (chips included), so a
    /// background rebuild can tell "same picture" and skip the swap.
    /// Prints rather than rows: this grows with the window ([`RowPrint`]).
    pub(super) sent_rows: Vec<RowPrint>,
    /// The commit rows as the walk behind `sent_rows` gave them, before a
    /// delete that is out took any away — what the delete's stand-in lays
    /// out again (`session::leaving`). Written where `sent_rows` is, by
    /// the walk; laying out again leaves it be.
    pub(super) walked: Walked,
    /// The footer delivered with them, compared alongside: a window change
    /// can keep every row and still change the footer (two commits are cut
    /// by a window of two, not by three), and a rebuild that overtook the
    /// stream would otherwise swallow it. `None` until a pass has answered
    /// for this graph.
    pub(super) sent_footer: Option<Footer>,
}

impl Shared {
    /// Whether the consumer already shows `rows` under `footer` — rows and
    /// footer both: rows alone would call a widened window the same
    /// picture and leave the truncation notice standing
    /// (rules-refs/core.md「swap を省く判定は行とフッタの両方」).
    pub(super) fn shows(&self, rows: &[LogRow], footer: Footer) -> bool {
        self.sent_footer == Some(footer)
            && self.sent_rows.len() == rows.len()
            && self
                .sent_rows
                .iter()
                .zip(rows)
                .all(|(sent, fresh)| *sent == RowPrint::of(fresh))
    }
}

/// One fact read out of the repository and kept until something that
/// could have changed it happens.
///
/// Each is a single answer about the whole repository (does git normalise
/// line endings here, what remotes are configured). Invalidation stays in
/// the two event boundaries that own it: before a write's refresh, and
/// after an external ref move — which has already read the current remote
/// answer, so it drops line-ending context at once and remotes only for
/// the following read (see [`RepoSession::forget_derived`]).
///
/// Hand-written: these are cancellable async subprocess reads that expire
/// on an event, never on a clock — outside what `salsa` (pure sync
/// queries) or `moka` (age / size eviction) model.
pub(super) struct Derived<T> {
    state: Mutex<DerivedState<T>>,
    /// One repository read at a time (async: held across the subprocess).
    reading: tokio::sync::Mutex<()>,
}

struct DerivedState<T> {
    generation: u64,
    value: Option<T>,
}

impl<T> Default for Derived<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(DerivedState {
                generation: 0,
                value: None,
            }),
            reading: tokio::sync::Mutex::new(()),
        }
    }
}

impl<T: Clone> Derived<T> {
    /// Returns the current answer, or lets exactly one caller read it
    /// (single-flight: callers waiting behind the gate re-check the value).
    ///
    /// A read that returns into a newer generation reads once more, and
    /// only once: writes invalidate on their way out, so chasing until
    /// no write lands mid-read is unbounded with every waiter parked behind
    /// the gate. That second reading answers this call but stays out of
    /// the cache.
    pub(super) async fn get_or_try_init<E, F, Fut>(&self, mut read: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        if let Some(value) = self.lock_state().value.clone() {
            return Ok(value);
        }

        let _reading = self.reading.lock().await;
        let mut last_chance = false;
        loop {
            let generation = {
                let state = self.lock_state();
                if let Some(value) = state.value.clone() {
                    return Ok(value);
                }
                state.generation
            };

            let value = read().await?;
            let mut state = self.lock_state();
            if state.generation == generation {
                state.value = Some(value.clone());
                return Ok(value);
            }
            if last_chance {
                return Ok(value);
            }
            last_chance = true;
        }
    }

    pub(super) fn forget(&self) {
        let mut state = self.lock_state();
        state.generation = state.generation.wrapping_add(1);
        state.value = None;
    }

    /// Reads the answer as it stands, without asking git for one. `None`
    /// where nothing has been read yet or the last answer was
    /// dropped.
    pub(super) fn peek<R>(&self, read: impl FnOnce(&T) -> R) -> Option<R> {
        self.lock_state().value.as_ref().map(read)
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, DerivedState<T>> {
        super::relock(&self.state)
    }
}

/// What is known about a path's line endings before its patch is read.
pub(super) enum EndingContext {
    /// git calls the path something other than text, so nothing is said
    /// about it. Also where a failed reading lands: not knowing whether
    /// a path is binary is a reason to stay quiet.
    Excluded,
    /// Worth reading, with the neighbours' opinion if one was needed and
    /// could be had.
    Open(Option<crate::eol::Baseline>),
}
