//! The execution slots: how many git processes this application has
//! running at once, and which of the ones waiting goes next.
//!
//! **One set per application, shared by handle** — the application hands
//! one [`Slots`] to the executor ([`super::GitExecutor::scheduled`]) and
//! every session, screen and dialog clones it, so the caps are the whole
//! process's. A bare executor is unbounded.
//!
//! **A slot is a child process and nothing else** — taken just before the
//! spawn and given back at the reap, by the executor alone. A compound
//! operation holds no slot between its commands, so the deadlock of a
//! nested acquire cannot be made. Write order (`session::write_order`),
//! the log (`Kept`), cancellation and the write lane (`operation::Lane`)
//! live elsewhere: this decides admission only.
//!
//! **One pool, with a reserve for the click** ([`Limits`]). Commands paced
//! elsewhere ([`Pace::Elsewhere`]) can sit for minutes at a remote or a
//! passphrase prompt, and background reads come in bursts; both share
//! what the reserve leaves, so however many are running or stalled, a
//! click finds a slot. A background command overtaken by
//! [`OVERTAKEN_LIMIT`] interactive ones is served next.
//!
//! **Waiting ends on the token and leaves nothing behind** —
//! [`super::GitExecutor::execute`] selects on the command's token, the
//! queue entry goes with the future ([`Unqueue`]) and a grant that crossed
//! the leaving is given straight back, so a closed screen spawns nothing.
//!
//! **Identical background reads share one process** ([`Group`]), but only
//! while the first is still queued: one that has started may have looked
//! before the second asked (`session::ReadFlight` draws the same line). A
//! leader that leaves without an outcome hands its followers back to the
//! queue.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::GitError;

/// Who is waiting on a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Priority {
    /// Somebody: a click, a write, a page that has to paint. Served first.
    Interactive,
    /// Nobody: the reads a session makes on its own timer. Kept out of
    /// the click's reserve.
    Background,
}

/// What sets a command's pace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pace {
    /// This machine: as slow as the repository is large.
    Here,
    /// Something outside: a remote, a credential prompt, a person in
    /// another program. Kept out of the click's reserve.
    Elsewhere,
}

/// Interactive admissions that may pass a queued background command
/// before it is served next. Four is a burst (a diff open is three
/// commands), so a single click goes straight through and a stream still
/// lets a background read through about once a burst.
pub const OVERTAKEN_LIMIT: u32 = 4;

/// The cap on git processes at once, whatever the settings say — past this
/// the machine is the cap.
pub const MAX_CONCURRENCY: u32 = 32;

/// Bounds of [`default_concurrency`].
pub const DEFAULT_CONCURRENCY_CEILING: u32 = 8;
pub const DEFAULT_CONCURRENCY_FLOOR: u32 = 4;

/// The concurrency a fresh settings file starts from: a third of the
/// machine's threads, held between the floor and the ceiling.
///
/// **A third, because a `status` is not one thread**: over a large tree git
/// spreads the stat across cores (preload-index), so a couple of them fill
/// the machine and more slots only queue for cores
/// (ci/baseline/git-slots-windows-x64.md).
///
/// **The floor is four**: the smallest pool whose reserve fits a diff
/// open's three commands ([`Limits::of`]; same record, §枠の床).
#[must_use]
pub fn default_concurrency() -> u32 {
    std::thread::available_parallelism().map_or(DEFAULT_CONCURRENCY_FLOOR, |threads| {
        (threads.get() as u32 / 3).clamp(DEFAULT_CONCURRENCY_FLOOR, DEFAULT_CONCURRENCY_CEILING)
    })
}

/// The concurrency that will actually apply, for a number a person
/// asked for: zero means one, past [`MAX_CONCURRENCY`] means that.
///
/// The one place the range is applied: the settings screen and a
/// hand-written `settings.toml` write the same field.
#[must_use]
pub fn concurrency(asked: u32) -> u32 {
    asked.clamp(1, MAX_CONCURRENCY)
}

/// How many commands may run at once, and how many of those slots are
/// the click's alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Children alive at once, all told. The number the settings name.
    pub total: usize,
    /// Of `total`, the slots only an interactive command paced
    /// [`Pace::Here`] may take: the click's reserve. Everything else
    /// shares the rest ([`Limits::shared`]).
    pub reserve: usize,
}

impl Limits {
    /// The limits one number from the settings stands for: that many
    /// processes all told, of which a quarter (at least one, or no
    /// background read would ever run) is shared by background reads and
    /// commands paced elsewhere — the rest is the click's.
    ///
    /// **A quarter, because opening a diff is three commands at once**
    /// (`diff-tree`, `check-attr`, `rev-parse` → `cat-file`): a reserve
    /// under three queues the head of that chain behind the session's own
    /// reads. At a half the reserve reaches three only at eight slots; at a
    /// quarter, from four up (ci/baseline/git-slots-windows-x64.md).
    #[must_use]
    pub fn of(asked: u32) -> Self {
        let total = concurrency(asked) as usize;
        Self {
            total,
            reserve: total - (total / 4).max(1),
        }
    }

    /// No cap: what a bare executor runs with.
    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            total: usize::MAX,
            reserve: 0,
        }
    }

    /// What background reads and commands paced elsewhere may hold
    /// between them.
    #[must_use]
    pub fn shared(self) -> usize {
        self.total.saturating_sub(self.reserve)
    }
}

/// What the slots are doing at this moment, and what they have done.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// Children alive now, all told; then those in the background and
    /// those paced elsewhere (a child can be both).
    pub running: usize,
    pub running_background: usize,
    pub running_elsewhere: usize,
    /// Commands waiting for a slot, by who is waiting on them.
    pub queued_interactive: usize,
    pub queued_background: usize,
    /// Commands given a slot so far.
    pub admitted: u64,
    /// Commands that left the queue on their token.
    pub left_waiting: u64,
    /// Commands answered by another's run.
    pub shared: u64,
    /// Total and longest wait of admitted commands, then the same for
    /// interactive ones alone — a click's wait is read off those, since
    /// background reads wait by design.
    pub waited: Duration,
    pub waited_most: Duration,
    pub waited_interactive: Duration,
    pub waited_most_interactive: Duration,
}

/// The application's slots; every clone of the executor shares one `Arc`.
pub struct Slots {
    state: Mutex<State>,
    groups: Mutex<HashMap<GroupKey, Group>>,
    /// Raised after every change to who is queued or running
    /// ([`Slots::settled`]). `notify_waiters`, not `notify_one`: several
    /// may watch, and a change concerns all of them.
    moved: tokio::sync::Notify,
}

impl std::fmt::Debug for Slots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let report = self.report();
        f.debug_struct("Slots")
            .field("limits", &lock(&self.state).limits)
            .field("report", &report)
            .finish()
    }
}

struct State {
    limits: Limits,
    running: Running,
    /// Oldest first; the front is not necessarily next ([`State::pick`]).
    queue: VecDeque<Waiting>,
    next_ticket: u64,
    admitted: u64,
    left_waiting: u64,
    shared: u64,
    waited: Duration,
    waited_most: Duration,
    waited_interactive: Duration,
    waited_most_interactive: Duration,
}

#[derive(Default, Clone, Copy)]
struct Running {
    all: usize,
    /// Those outside the click's reserve: background, paced elsewhere,
    /// or both — counted once.
    shared: usize,
    background: usize,
    elsewhere: usize,
}

fn reserved_for(pace: Pace, priority: Priority) -> bool {
    pace == Pace::Here && priority == Priority::Interactive
}

impl Running {
    fn add(&mut self, pace: Pace, priority: Priority) {
        self.all += 1;
        if !reserved_for(pace, priority) {
            self.shared += 1;
        }
        if priority == Priority::Background {
            self.background += 1;
        }
        if pace == Pace::Elsewhere {
            self.elsewhere += 1;
        }
    }

    fn sub(&mut self, pace: Pace, priority: Priority) {
        self.all = self.all.saturating_sub(1);
        if !reserved_for(pace, priority) {
            self.shared = self.shared.saturating_sub(1);
        }
        if priority == Priority::Background {
            self.background = self.background.saturating_sub(1);
        }
        if pace == Pace::Elsewhere {
            self.elsewhere = self.elsewhere.saturating_sub(1);
        }
    }
}

struct Waiting {
    ticket: u64,
    priority: Priority,
    pace: Pace,
    since: Instant,
    /// Interactive commands admitted from behind this one; counted for
    /// background commands only.
    passed_by: u32,
    grant: tokio::sync::oneshot::Sender<Slot>,
}

impl State {
    fn eligible(&self, waiting: &Waiting) -> bool {
        self.running.all < self.limits.total
            && (reserved_for(waiting.pace, waiting.priority)
                || self.running.shared < self.limits.shared())
    }

    /// Which queued command goes next, if any can: a background one
    /// overtaken often enough, else the first interactive one that fits,
    /// else the first background one that fits.
    fn pick(&self) -> Option<usize> {
        let mut first_interactive = None;
        let mut first_background = None;
        for (at, waiting) in self.queue.iter().enumerate() {
            if !self.eligible(waiting) {
                continue;
            }
            match waiting.priority {
                Priority::Background if waiting.passed_by >= OVERTAKEN_LIMIT => return Some(at),
                Priority::Background => first_background.get_or_insert(at),
                Priority::Interactive => first_interactive.get_or_insert(at),
            };
        }
        first_interactive.or(first_background)
    }

    fn note_admitted(&mut self, priority: Priority, waited: Duration) {
        self.admitted += 1;
        self.waited += waited;
        self.waited_most = self.waited_most.max(waited);
        if priority == Priority::Interactive {
            self.waited_interactive += waited;
            self.waited_most_interactive = self.waited_most_interactive.max(waited);
        }
    }
}

fn lock(state: &Mutex<State>) -> std::sync::MutexGuard<'_, State> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl Slots {
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self {
            state: Mutex::new(State {
                limits,
                running: Running::default(),
                queue: VecDeque::new(),
                next_ticket: 1,
                admitted: 0,
                left_waiting: 0,
                shared: 0,
                waited: Duration::ZERO,
                waited_most: Duration::ZERO,
                waited_interactive: Duration::ZERO,
                waited_most_interactive: Duration::ZERO,
            }),
            groups: Mutex::new(HashMap::new()),
            moved: tokio::sync::Notify::new(),
        }
    }

    /// Slots that cap nothing (see [`Limits::unbounded`]).
    #[must_use]
    pub fn unbounded() -> Self {
        Self::new(Limits::unbounded())
    }

    #[must_use]
    pub fn limits(&self) -> Limits {
        lock(&self.state).limits
    }

    /// Wider limits admit what is waiting at once; narrower ones never
    /// stop a child already running.
    pub fn set_limits(self: &Arc<Self>, limits: Limits) {
        {
            let mut state = lock(&self.state);
            state.limits = limits;
            self.drain(&mut state);
        }
        self.stirred();
    }

    #[must_use]
    pub fn report(&self) -> Report {
        let state = lock(&self.state);
        let (queued_interactive, queued_background) =
            state
                .queue
                .iter()
                .fold((0, 0), |(i, b), w| match w.priority {
                    Priority::Interactive => (i + 1, b),
                    Priority::Background => (i, b + 1),
                });
        Report {
            running: state.running.all,
            running_background: state.running.background,
            running_elsewhere: state.running.elsewhere,
            queued_interactive,
            queued_background,
            admitted: state.admitted,
            left_waiting: state.left_waiting,
            shared: state.shared,
            waited: state.waited,
            waited_most: state.waited_most,
            waited_interactive: state.waited_interactive,
            waited_most_interactive: state.waited_most_interactive,
        }
    }

    /// Waits for a slot. The future is the place in the queue: dropping
    /// it (a cancelled `select!`) leaves the queue as if it had never
    /// asked, and gives back a slot granted in the same instant. `None`
    /// only if the entry went without a grant, which nothing here does.
    pub async fn acquire(self: &Arc<Self>, priority: Priority, pace: Pace) -> Option<Slot> {
        let (grant, granted) = tokio::sync::oneshot::channel();
        let ticket = {
            let mut state = lock(&self.state);
            let ticket = state.next_ticket;
            state.next_ticket += 1;
            state.queue.push_back(Waiting {
                ticket,
                priority,
                pace,
                since: Instant::now(),
                passed_by: 0,
                grant,
            });
            self.drain(&mut state);
            ticket
        };
        // After the state is in place: a woken waiter reads the queue itself.
        self.stirred();
        let _leaving = Unqueue {
            slots: self,
            ticket,
        };
        granted.await.ok()
    }

    /// Admits everything that may run now, in [`State::pick`]'s order.
    /// Called with the lock held wherever the answer can have changed.
    fn drain(self: &Arc<Self>, state: &mut State) {
        while let Some(at) = state.pick() {
            let Some(waiting) = state.queue.remove(at) else {
                break;
            };
            if waiting.priority == Priority::Interactive {
                for overtaken in state.queue.iter_mut().take(at) {
                    if overtaken.priority == Priority::Background {
                        overtaken.passed_by += 1;
                    }
                }
            }
            state.running.add(waiting.pace, waiting.priority);
            state.note_admitted(waiting.priority, waiting.since.elapsed());
            let slot = Slot {
                slots: Some(Arc::clone(self)),
                pace: waiting.pace,
                priority: waiting.priority,
            };
            if let Err(mut slot) = waiting.grant.send(slot) {
                // The asker left at the grant. Given back by hand and
                // disarmed: `Slot::drop` would take the lock held here.
                drop(slot.slots.take());
                state.running.sub(slot.pace, slot.priority);
            }
        }
    }

    fn release(self: &Arc<Self>, pace: Pace, priority: Priority) {
        {
            let mut state = lock(&self.state);
            state.running.sub(pace, priority);
            self.drain(&mut state);
        }
        self.stirred();
    }

    fn unqueue(&self, ticket: u64) {
        {
            let mut state = lock(&self.state);
            let before = state.queue.len();
            state.queue.retain(|waiting| waiting.ticket != ticket);
            if state.queue.len() < before {
                state.left_waiting += 1;
            }
        }
        self.stirred();
    }

    /// Called with no lock held: waking a waiter runs its task, and the
    /// lock would be held across somebody else's work.
    fn stirred(&self) {
        self.moved.notify_waiters();
    }

    /// Waits until the slots answer `done`, and answers at once if they
    /// already do.
    ///
    /// **The waiting side is taken out before the look**: `notify_waiters`
    /// leaves no permit, so a stir between the look and a later
    /// `notified()` would be lost and the wait would hang (`enable` keeps
    /// this right for a `notify_one` too). Pinned by
    /// `a_stir_from_inside_the_look_still_ends_the_wait`. A stir is only a
    /// reason to look again; the state under the lock ends the wait.
    ///
    /// For what no event announces: the executor announces a command
    /// before it asks for a slot, so "announced" cannot be told from
    /// "queued" (`tests/it/slots.rs`).
    pub async fn settled(&self, mut done: impl FnMut(&Report) -> bool) {
        loop {
            let moved = self.moved.notified();
            tokio::pin!(moved);
            moved.as_mut().enable();
            if done(&self.report()) {
                return;
            }
            moved.await;
        }
    }

    // --- sharing one run between identical background asks ---------------

    /// Joins the group for `key`, or opens one and leads it: the leader
    /// runs the command and answers everyone who joins before it spawns.
    pub(super) fn lead_or_follow(self: &Arc<Self>, key: GroupKey) -> WayIn {
        let mut groups = match self.groups.lock() {
            Ok(groups) => groups,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(group) = groups.get_mut(&key) {
            let (tell, told) = tokio::sync::oneshot::channel();
            group.followers.push(tell);
            lock(&self.state).shared += 1;
            return WayIn::Follow(told);
        }
        groups.insert(
            key.clone(),
            Group {
                followers: Vec::new(),
            },
        );
        WayIn::Lead(Lead {
            slots: Arc::clone(self),
            key: Some(key),
            followers: Vec::new(),
        })
    }

    /// Takes the group off the board, with whoever joined it.
    fn close_group(&self, key: &GroupKey) -> Vec<tokio::sync::oneshot::Sender<Arc<Shared>>> {
        let mut groups = match self.groups.lock() {
            Ok(groups) => groups,
            Err(poisoned) => poisoned.into_inner(),
        };
        groups
            .remove(key)
            .map(|group| group.followers)
            .unwrap_or_default()
    }
}

/// One process's slot, held from the spawn to the reap and given back by
/// dropping it — whichever way the run ended.
#[must_use = "a slot is a running process until it is dropped"]
pub struct Slot {
    /// `None` once the slot has been given back by hand (a grant that
    /// crossed the asker leaving), so the drop gives nothing back twice.
    slots: Option<Arc<Slots>>,
    pace: Pace,
    priority: Priority,
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot")
            .field("pace", &self.pace)
            .field("priority", &self.priority)
            .finish()
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        if let Some(slots) = self.slots.take() {
            slots.release(self.pace, self.priority);
        }
    }
}

/// The place in the queue, given back with the wait: a future dropped
/// before its grant takes its entry out, and one dropped after it drops
/// the [`Slot`] in the channel, which gives the slot back.
struct Unqueue<'a> {
    slots: &'a Arc<Slots>,
    ticket: u64,
}

impl Drop for Unqueue<'_> {
    fn drop(&mut self) {
        self.slots.unqueue(self.ticket);
    }
}

/// What tells two background asks they are the same command over the
/// same tree — everything the process would be built from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct GroupKey {
    pub(super) program: OsString,
    pub(super) cwd: Option<PathBuf>,
    pub(super) args: Vec<OsString>,
    pub(super) env: Vec<(OsString, OsString)>,
}

struct Group {
    followers: Vec<tokio::sync::oneshot::Sender<Arc<Shared>>>,
}

/// One run's outcome for its followers. A cancelled leader sends nothing —
/// its followers were not cancelled, and ask again.
pub(super) struct Shared {
    outcome: Result<super::command::GitOutput, SharedFailure>,
}

impl Shared {
    /// `None` for a cancellation.
    pub(super) fn of(outcome: &Result<super::command::GitOutput, GitError>) -> Option<Self> {
        match outcome {
            Ok(output) => Some(Self {
                outcome: Ok(output.clone()),
            }),
            Err(error) => SharedFailure::of(error).map(|failure| Self {
                outcome: Err(failure),
            }),
        }
    }

    /// The follower's own copy of the answer.
    pub(super) fn outcome(&self) -> Result<super::command::GitOutput, GitError> {
        match &self.outcome {
            Ok(output) => Ok(output.clone()),
            Err(failure) => Err(failure.error()),
        }
    }
}

/// [`GitError`] carries an `io::Error` and is not `Clone`, so this keeps
/// enough to rebuild it for each follower.
enum SharedFailure {
    NotFound {
        kind: std::io::ErrorKind,
        message: String,
    },
    Spawn {
        command: String,
        kind: std::io::ErrorKind,
        message: String,
    },
    Io {
        command: String,
        kind: std::io::ErrorKind,
        message: String,
    },
    TimedOut {
        command: String,
        timeout: Duration,
    },
}

impl SharedFailure {
    fn of(error: &GitError) -> Option<Self> {
        Some(match error {
            GitError::GitNotFound { source } => Self::NotFound {
                kind: source.kind(),
                message: source.to_string(),
            },
            GitError::Spawn { command, source } => Self::Spawn {
                command: command.clone(),
                kind: source.kind(),
                message: source.to_string(),
            },
            GitError::Io { command, source } => Self::Io {
                command: command.clone(),
                kind: source.kind(),
                message: source.to_string(),
            },
            GitError::TimedOut { command, timeout } => Self::TimedOut {
                command: command.clone(),
                timeout: *timeout,
            },
            GitError::Cancelled { .. } => return None,
            // A raw run ends in none of these; said as I/O so every
            // follower gets an answer.
            other => Self::Io {
                command: String::new(),
                kind: std::io::ErrorKind::Other,
                message: other.to_string(),
            },
        })
    }

    fn error(&self) -> GitError {
        match self {
            Self::NotFound { kind, message } => GitError::GitNotFound {
                source: std::io::Error::new(*kind, message.clone()),
            },
            Self::Spawn {
                command,
                kind,
                message,
            } => GitError::Spawn {
                command: command.clone(),
                source: std::io::Error::new(*kind, message.clone()),
            },
            Self::Io {
                command,
                kind,
                message,
            } => GitError::Io {
                command: command.clone(),
                source: std::io::Error::new(*kind, message.clone()),
            },
            Self::TimedOut { command, timeout } => GitError::TimedOut {
                command: command.clone(),
                timeout: *timeout,
            },
        }
    }
}

pub(super) enum WayIn {
    Lead(Lead),
    Follow(tokio::sync::oneshot::Receiver<Arc<Shared>>),
}

/// The leader's side of a group, alive from the ask to the answer. Dropped
/// before [`Lead::spawning`], it closes the group so its followers ask
/// again.
pub(super) struct Lead {
    slots: Arc<Slots>,
    key: Option<GroupKey>,
    /// Who joined before the spawn; told nothing if the run ends with no
    /// outcome to share.
    followers: Vec<tokio::sync::oneshot::Sender<Arc<Shared>>>,
}

impl Lead {
    /// The command is about to spawn: from here on an identical ask is
    /// its own, and everyone who joined before is answered by this run.
    pub(super) fn spawning(&mut self) {
        if let Some(key) = self.key.take() {
            self.followers = self.slots.close_group(&key);
        }
    }

    #[cfg(test)]
    fn has_followers(&self) -> bool {
        !self.followers.is_empty()
    }

    /// Hands the run's outcome to everyone who joined it.
    pub(super) fn answer(mut self, shared: Shared) {
        let shared = Arc::new(shared);
        for follower in self.followers.drain(..) {
            if follower.send(Arc::clone(&shared)).is_err() {
                tracing::trace!("a shared run's follower had already left");
            }
        }
    }
}

impl Drop for Lead {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            drop(self.slots.close_group(&key));
        }
    }
}

#[cfg(test)]
mod tests;
