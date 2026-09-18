//! The execution slots: how many git processes this application has
//! running at once, and which of the ones waiting goes next.
//!
//! **One set of slots per application, shared by handle.** The
//! application makes one [`Slots`] and hands it to the executor it
//! spawns everything through ([`super::GitExecutor::scheduled`]); every
//! session, every settings screen and every dialog clones that handle,
//! so what they run is one population and the caps here are the whole
//! process's. A bare executor is unbounded, which is what a test wants
//! of one: the cap is the application's.
//!
//! **A slot is a child process and nothing else.** It is taken just
//! before the spawn and given back when the child has been reaped,
//! by the executor and by nothing above it. A compound operation — a
//! carry that stashes, moves and pops, a settling that reads three
//! listings — holds no slot of its own between its commands, so it
//! cannot sit on one while its next command waits for one: the
//! deadlock a nested acquire would make cannot be made here. The order
//! writes run in is a separate matter and stays where it is
//! (`session::write_order`); so are the log (`Kept`), the token that
//! stops a command (`CancellationToken`) and the lane that supervises a
//! write (`operation::Lane`). This decides admission and nothing else.
//!
//! **One pool, and a reserve in it for the click.** The pool is the
//! number the settings name (`Limits::total`): that many children at
//! once, whatever paces them and whoever is waiting. Most commands are
//! as slow as this machine makes them — a `status` over a hundred
//! thousand files, a walk of two hundred thousand commits — and the cap
//! is what keeps a burst of them from taking the machine away from a
//! click. A fetch, a push, an `ls-remote`, a merge tool and a signature
//! check are paced by something else: the far end of a connection, a
//! credential helper waiting for a passphrase, a person in another
//! program. Those can sit for minutes costing nothing, and a slot they
//! sat in would be a click's slot gone for the length of somebody else's
//! prompt. So part of the pool is a reserve (`Limits::reserve`) that
//! only a command somebody is waiting on and this machine paces may
//! take: what is paced elsewhere ([`Pace::Elsewhere`]) and what nobody
//! is waiting on ([`Priority::Background`]) share the rest between them,
//! and however many of either are running or stalled, a click finds a
//! slot.
//!
//! **Priority is who is waiting.** A command somebody is waiting on —
//! the details of the row they clicked, the diff they opened, the write
//! they pressed, the page they just opened — goes ahead of the reads the
//! session makes on its own ([`Priority`]). What keeps those reads
//! moving under a stream of clicks is aging: a background command
//! overtaken by [`OVERTAKEN_LIMIT`] interactive ones is served next.
//!
//! **Waiting costs nothing and ends on the token.** A command waits
//! here as a future; the token that would stop the command stops the
//! wait instead ([`super::GitExecutor::execute`] selects on it), and a
//! wait that ends that way leaves nothing behind — the queue entry goes
//! with the future ([`Unqueue`]), and a grant that crossed the leaving
//! is given straight back. So a selection that moved on, a screen that
//! closed and a session that shut down take their queued reads with
//! them, and nobody spawns a process for a question nobody is asking.
//!
//! **Identical background reads share one process** — the second one
//! waits for the first one's answer, and only while the first
//! is still queued: one that has started may have looked
//! before the second asked, and an answer older than its
//! question is not an answer
//! (`session::ReadFlight` draws the same line). What is shared is the
//! run's outcome ([`Group`]); a leader that leaves without one hands
//! its followers back to the queue to lead for themselves.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::GitError;

/// Who is waiting on a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Priority {
    /// Somebody: the row they clicked, the diff they opened, the write
    /// they pressed, the page that has to paint. Served before the
    /// reads below.
    Interactive,
    /// Nobody: the reads a session makes on a timer of its own — the
    /// other working copies' status, the walk behind a chip, the
    /// interval's fetch. Kept out of the click's reserve, so a click
    /// always finds a slot.
    Background,
}

/// What sets a command's pace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pace {
    /// This machine: the command is as slow as the repository is large,
    /// and runs against everything else this machine is doing.
    Here,
    /// Something outside it: the far end of a network connection, a
    /// credential helper's prompt, a person in another program. Kept out
    /// of the click's reserve, so a stalled one leaves the click's
    /// slots free.
    Elsewhere,
}

/// Interactive admissions a queued background command is passed by
/// before it is served next whatever is waiting beside it. Four is a
/// burst — a click opening a diff spends three commands and a poll
/// tick two — so a stream of them still lets a background read through
/// about once a burst, and a single click goes straight through.
pub const OVERTAKEN_LIMIT: u32 = 4;

/// Most git processes the application runs at once, whatever the
/// settings say: past this the machine is the cap, and a number typed
/// on purpose above it is this number.
pub const MAX_CONCURRENCY: u32 = 32;

/// Where the default concurrency stops growing with the machine, and
/// where it stops shrinking (see [`default_concurrency`]).
pub const DEFAULT_CONCURRENCY_CEILING: u32 = 8;
pub const DEFAULT_CONCURRENCY_FLOOR: u32 = 4;

/// The concurrency a fresh settings file starts from: a third of the
/// machine's threads, held between the floor and the ceiling.
///
/// **A third, because a `status` is not one thread.** Over a hundred
/// thousand tracked files git spreads the stat across the machine's
/// cores (preload-index), so one such process is eleven cores wide for
/// its two hundred milliseconds on the reference machine, and two of
/// them in parallel are the whole machine — what the click's `show`
/// then waits on is not a slot but a core. Measured over eight copies
/// of the corpus read in a loop (ci/baseline/git-slots-windows-x64.md):
/// on twenty-four threads sixteen slots are no better than eight, and
/// the pass over those copies is no faster wide than narrow — the
/// machine runs out before the slots do. Twenty-four over three is
/// eight.
///
/// **The floor is four, because that is where the click's three fit.**
/// Two slots is the one setting where the median suffers: opening
/// a diff spends three commands at once, so with a pool of two the
/// head of its serial chain queues for a slot on every open, on a
/// machine with nothing else running at all — a whole
/// process's worth of wait added to the one point a person waits at
/// (same record, §枠の床). A laptop's eight threads therefore come to
/// four, where the three fit and one background read runs beside them.
#[must_use]
pub fn default_concurrency() -> u32 {
    std::thread::available_parallelism().map_or(DEFAULT_CONCURRENCY_FLOOR, |threads| {
        (threads.get() as u32 / 3).clamp(DEFAULT_CONCURRENCY_FLOOR, DEFAULT_CONCURRENCY_CEILING)
    })
}

/// The concurrency that will actually apply, for a number a person
/// asked for. Zero and everything past the ceiling mean the nearest
/// number that runs anything: one process, and the ceiling.
///
/// The one place the range is applied, for the reason the auto-fetch
/// ceiling has one (`session::auto_fetch_minutes`): the settings screen
/// and a hand-written `settings.toml` write the same field.
#[must_use]
pub fn concurrency(asked: u32) -> u32 {
    asked.clamp(1, MAX_CONCURRENCY)
}

/// How many commands may run at once, and how many of those slots are
/// the click's alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Children alive at once, all told — whatever paces them and
    /// whoever is waiting on them. The number the settings name.
    pub total: usize,
    /// Of `total`, the slots only a command somebody is waiting on and
    /// this machine paces may take: the click's reserve. Everything
    /// else — the reads a session makes on its own, whatever their
    /// pace, and the commands paced elsewhere, whoever waits on them —
    /// shares what is left ([`Limits::shared`]), so neither a burst of
    /// background reads nor a fetch stalled at a passphrase prompt can
    /// hold a slot a click needs.
    pub reserve: usize,
}

impl Limits {
    /// The limits one number from the settings stands for: that many
    /// processes all told, a quarter of them at most (at least one,
    /// or no background read would ever run) for the reads nobody
    /// is waiting on and the commands paced elsewhere together — the
    /// rest is the click's.
    ///
    /// **A quarter, because opening a diff is three commands at once.**
    /// The click spends `diff-tree`, `check-attr` and the
    /// `rev-parse` → `cat-file` chain in parallel, so a reserve under
    /// three puts one of them behind whatever the session is reading for
    /// itself — and the one that queues is the head of the chain, so the
    /// wait lands on the critical path. At a half
    /// the reserve only reaches three at eight slots, which no machine
    /// under eighteen threads defaults to; at a quarter it is three from
    /// four slots up. What it costs is background width, which the pass
    /// over the other working copies barely spends: those reads saturate
    /// the machine long before they fill the slots, so the pass takes
    /// about as long however many of it run at once
    /// (ci/baseline/git-slots-windows-x64.md).
    #[must_use]
    pub fn of(asked: u32) -> Self {
        let total = concurrency(asked) as usize;
        Self {
            total,
            reserve: total - (total / 4).max(1),
        }
    }

    /// No cap at all: what a bare executor runs with, and what every
    /// test that is not about the slots gets.
    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            total: usize::MAX,
            reserve: 0,
        }
    }

    /// What the reads nobody is waiting on and the commands paced
    /// elsewhere may hold between them: the pool less the reserve.
    #[must_use]
    pub fn shared(self) -> usize {
        self.total.saturating_sub(self.reserve)
    }
}

/// What the slots are doing at this moment, and what they have done.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// Children alive now, all told — and of those, the ones nobody is
    /// waiting on and the ones paced elsewhere (a child can be both).
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
    /// What every admitted command spent waiting, summed, and the most
    /// any one of them waited — and the same for the interactive ones
    /// alone, which is the number a click's wait is read off: a
    /// background read queued behind its own kind waits by design.
    pub waited: Duration,
    pub waited_most: Duration,
    pub waited_interactive: Duration,
    pub waited_most_interactive: Duration,
}

/// The application's slots. Cheap to share: the executor holds an
/// `Arc` of it and every clone of the executor points at the same one.
pub struct Slots {
    state: Mutex<State>,
    groups: Mutex<HashMap<GroupKey, Group>>,
    /// Raised after every change to who is queued or running, so that
    /// somebody outside can wait for one ([`Slots::settled`]).
    ///
    /// **`notify_waiters`, not `notify_one`**: several may be watching,
    /// and a change concerns all of them. It wakes what is registered at
    /// that instant and leaves no permit behind, which is what makes the
    /// order in [`Slots::settled`] load-bearing — the registration is
    /// made before the look, or a change landing between the two would
    /// be lost and the wait would never end.
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
    /// Commands waiting for a slot, oldest first. The front is not
    /// necessarily next: what goes next is decided by [`State::pick`].
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
    /// Children alive, all told.
    all: usize,
    /// Of those, the ones outside the click's reserve: for nobody, or
    /// paced elsewhere, or both — counted once.
    shared: usize,
    background: usize,
    elsewhere: usize,
}

/// Whether a command may take a slot from the click's reserve: somebody
/// is waiting on it, and this machine paces it.
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
    /// Interactive commands admitted from behind this one since it
    /// queued — only counted for a background command, which is the
    /// only kind anything overtakes.
    passed_by: u32,
    grant: tokio::sync::oneshot::Sender<Slot>,
}

impl State {
    /// Whether a queued command fits now: under the pool's cap, and —
    /// unless it is one the reserve is kept for — under what the rest
    /// share.
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

    /// Puts new limits in force. Wider ones admit what is waiting at
    /// once; narrower ones admit nothing more until the running fall
    /// below them — a child already running runs on.
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
    /// it — which is what a cancelled `select!` does — leaves the queue
    /// as if it had never asked, and gives back a slot granted in the
    /// same instant. `None` only where the queue let go of the entry
    /// without a grant, which nothing here does; it is typed so the
    /// wait comes back as a value.
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
        // Said after the lock is gone, and after the state it is about
        // already holds: a waiter woken here looks at the queue itself,
        // and has to find what it was told about.
        self.stirred();
        let _leaving = Unqueue {
            slots: self,
            ticket,
        };
        granted.await.ok()
    }

    /// Admits everything that may run now, in the order [`State::pick`]
    /// decides. Called with the lock held wherever the answer can have
    /// changed: a command queued, a slot given back, the limits moved.
    fn drain(self: &Arc<Self>, state: &mut State) {
        while let Some(at) = state.pick() {
            let Some(waiting) = state.queue.remove(at) else {
                break;
            };
            // An interactive command admitted from behind a background
            // one has overtaken it; what it overtook is what ages.
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
                // The asker left in the instant of the grant. The slot
                // goes straight back — by hand, since `Slot::drop` would
                // take the lock this holds; disarmed, its drop does
                // nothing.
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

    /// Says that the queue or the running have moved. **Called with no
    /// lock held** — waking a waiter runs its task, and a lock held
    /// across that is a lock held across somebody else's work.
    fn stirred(&self) {
        self.moved.notify_waiters();
    }

    /// Waits until the slots answer `done`, and answers at once if they
    /// already do.
    ///
    /// **The waiting side is taken out before the look.**
    /// `notify_waiters` leaves no permit, so a stir that lands between a
    /// look and a `notified()` taken out after it would be nobody's and
    /// the wait would hang. Taken out first, the same stir is held for
    /// this one however long the look takes — that is the line, and
    /// `enable` registers it eagerly on top, so the order still holds if
    /// the stir ever becomes a `notify_one`. The order is a test, not
    /// only a claim: `done` is called inside that very window, so a
    /// predicate that joins the queue from within it stirs exactly there
    /// (`a_stir_from_inside_the_look_still_ends_the_wait`).
    ///
    /// **And the notification is not the answer** — it is only a reason
    /// to look again. What ends the wait is the state, read under the
    /// lock, which is why a stir that does not reach `done` leaves this
    /// waiting.
    ///
    /// This exists for what cannot be watched any other way: a command
    /// reaching the queue is not an event anybody sends — the executor
    /// announces the command before it asks for a slot, so nothing
    /// downstream can tell "announced" from "queued" (`tests/it/slots.rs`).
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

    /// Joins the group for `key`, or opens one. Answers the way in: lead
    /// (run the command, and answer everyone who joins before it spawns)
    /// or follow (wait for the leader's answer).
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

    /// Takes the group off the board, with whoever joined it. Called by
    /// the leader as it spawns — from here on an identical ask leads for
    /// itself — and by a leader leaving without an answer, whose
    /// followers are then told nothing and ask again.
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

/// One run's outcome as its followers receive it: the output, or the
/// reason there is none. A cancelled leader sends nothing at all — its
/// followers were not cancelled, and ask again.
pub(super) struct Shared {
    outcome: Result<super::command::GitOutput, SharedFailure>,
}

impl Shared {
    /// What a run ended in, for the followers. `None` for an ending
    /// that stays the leader's: its token fired, and theirs did not.
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

/// A failure as it can be handed to more than one caller: [`GitError`]
/// carries the OS's own error and is not `Clone`, so what is kept is
/// enough to say the same thing again to each follower.
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
    /// `None` for a cancellation, which is the leader's alone.
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
            // A raw run ends in none of these; said as an I/O
            // failure, so every follower gets an
            // answer.
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

/// The leader's side of a group: alive from the ask to the answer. It
/// takes the followers when the command spawns ([`Lead::spawning`]) and,
/// dropped before that, closes the group so its followers ask again.
pub(super) struct Lead {
    slots: Arc<Slots>,
    key: Option<GroupKey>,
    /// Who joined before the spawn — taken at the spawn, answered at
    /// the end, and told nothing by a leader that ends without an
    /// answer of its own to share.
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
mod tests {
    use super::*;
    use crate::wait::poll_once;

    fn slots(total: usize, reserve: usize) -> Arc<Slots> {
        Arc::new(Slots::new(Limits { total, reserve }))
    }

    /// Asks for a slot and drives the ask to its first wait: `Ready` is a
    /// grant, `Pending` a place in the queue. The future is kept, since
    /// dropping it is leaving the queue.
    type Ask = std::pin::Pin<Box<dyn std::future::Future<Output = Option<Slot>> + Send>>;

    fn ask(slots: &Arc<Slots>, priority: Priority, pace: Pace) -> Ask {
        let slots = Arc::clone(slots);
        Box::pin(async move { slots.acquire(priority, pace).await })
    }

    fn granted(ask: &mut Ask) -> Option<Slot> {
        match poll_once(ask) {
            std::task::Poll::Ready(slot) => slot,
            std::task::Poll::Pending => None,
        }
    }

    #[tokio::test]
    async fn the_cap_holds_and_a_slot_given_back_admits_the_next() {
        let slots = slots(2, 1);
        let mut first = ask(&slots, Priority::Interactive, Pace::Here);
        let mut second = ask(&slots, Priority::Interactive, Pace::Here);
        let mut third = ask(&slots, Priority::Interactive, Pace::Here);
        let held_first = granted(&mut first).expect("the first fits");
        let _held_second = granted(&mut second).expect("the second fits");
        assert!(granted(&mut third).is_none(), "the third waits for a slot");
        assert_eq!(slots.report().running, 2);
        assert_eq!(slots.report().queued_interactive, 1);

        drop(held_first);
        let _held_third = granted(&mut third).expect("the slot given back admits the third");
        assert_eq!(slots.report().running, 2);
        assert_eq!(slots.report().queued_interactive, 0);
        assert_eq!(slots.report().admitted, 3);
    }

    #[tokio::test]
    async fn background_reads_are_kept_out_of_the_reserve_and_a_click_still_fits() {
        let slots = slots(4, 3);
        let mut first = ask(&slots, Priority::Background, Pace::Here);
        let mut second = ask(&slots, Priority::Background, Pace::Here);
        let _held = granted(&mut first).expect("one background read runs");
        assert!(
            granted(&mut second).is_none(),
            "the second background read waits under the background cap"
        );
        let mut click = ask(&slots, Priority::Interactive, Pace::Here);
        let _clicked = granted(&mut click).expect("the reserve is the click's");
        let report = slots.report();
        assert_eq!(report.running, 2);
        assert_eq!(report.running_background, 1);
        assert_eq!(report.queued_background, 1);
    }

    #[tokio::test]
    async fn an_interactive_ask_goes_ahead_of_a_background_one_queued_before_it() {
        let slots = slots(1, 0);
        let mut held = ask(&slots, Priority::Interactive, Pace::Here);
        let holding = granted(&mut held).expect("the pool is one slot");
        let mut background = ask(&slots, Priority::Background, Pace::Here);
        let mut click = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut background).is_none());
        assert!(granted(&mut click).is_none());

        drop(holding);
        let _clicked = granted(&mut click).expect("the click was served first");
        assert!(
            granted(&mut background).is_none(),
            "the background read is still waiting behind it"
        );
    }

    /// The fairness the interactive priority is paid for with: a
    /// background read passed by `OVERTAKEN_LIMIT` clicks is served next
    /// even with a click waiting beside it.
    #[tokio::test]
    async fn a_background_read_overtaken_often_enough_is_served_next() {
        let slots = slots(1, 0);
        let mut held = ask(&slots, Priority::Interactive, Pace::Here);
        let mut holding = granted(&mut held).expect("the pool is one slot");
        let mut background = ask(&slots, Priority::Background, Pace::Here);
        assert!(granted(&mut background).is_none());

        for nth in 0..OVERTAKEN_LIMIT {
            let mut click = ask(&slots, Priority::Interactive, Pace::Here);
            assert!(
                granted(&mut click).is_none(),
                "click {nth} waits for the slot"
            );
            drop(holding);
            holding = granted(&mut click).expect("the click overtakes the background read");
            assert!(
                granted(&mut background).is_none(),
                "overtaken {} times, the background read still waits",
                nth + 1
            );
        }
        let mut one_more = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut one_more).is_none());
        drop(holding);
        let _read = granted(&mut background).expect("overtaken enough, it goes next");
        assert!(
            granted(&mut one_more).is_none(),
            "and the click behind it waits its turn"
        );
    }

    #[tokio::test]
    async fn a_wait_dropped_leaves_the_queue_and_holds_nothing() {
        let slots = slots(1, 0);
        let mut held = ask(&slots, Priority::Interactive, Pace::Here);
        let holding = granted(&mut held).expect("the pool is one slot");
        let mut left = ask(&slots, Priority::Interactive, Pace::Here);
        let mut stays = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut left).is_none());
        assert!(granted(&mut stays).is_none());
        assert_eq!(slots.report().queued_interactive, 2);

        // The token fired: the executor's `select!` drops the wait.
        drop(left);
        assert_eq!(slots.report().queued_interactive, 1);
        assert_eq!(slots.report().left_waiting, 1);

        drop(holding);
        let _next = granted(&mut stays).expect("the one that stayed is served, not the ghost");
        assert_eq!(slots.report().running, 1);
    }

    /// A grant and a leaving in the same instant: the slot given to a
    /// wait nobody is holding any more comes straight back.
    #[tokio::test]
    async fn a_grant_that_crosses_the_leaving_is_given_straight_back() {
        let slots = slots(1, 0);
        let mut held = ask(&slots, Priority::Interactive, Pace::Here);
        let holding = granted(&mut held).expect("the pool is one slot");
        let mut waiting = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut waiting).is_none());

        // The slot is granted into the channel while the wait has not
        // been polled again; then the wait is dropped with it inside.
        drop(holding);
        assert_eq!(slots.report().running, 1, "granted, unpolled");
        drop(waiting);
        assert_eq!(
            slots.report().running,
            0,
            "the slot in the dropped channel came back"
        );
        let mut next = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut next).is_some());
    }

    /// A fetch stalled at a prompt and a background read together fill
    /// what they share, and a click still finds its reserve — while a
    /// second round trip and a second read wait for one of theirs.
    #[tokio::test]
    async fn what_is_paced_elsewhere_and_what_nobody_waits_on_share_the_slots_outside_the_reserve()
    {
        let slots = slots(4, 2);
        let mut fetch = ask(&slots, Priority::Interactive, Pace::Elsewhere);
        let mut read = ask(&slots, Priority::Background, Pace::Here);
        let _fetching = granted(&mut fetch).expect("a fetch runs");
        let _reading = granted(&mut read).expect("beside a background read");
        let mut push = ask(&slots, Priority::Interactive, Pace::Elsewhere);
        let mut another = ask(&slots, Priority::Background, Pace::Here);
        assert!(
            granted(&mut push).is_none(),
            "a second round trip waits: what is shared outside the reserve is full"
        );
        assert!(
            granted(&mut another).is_none(),
            "and so does a second background read"
        );
        let mut click = ask(&slots, Priority::Interactive, Pace::Here);
        let mut second_click = ask(&slots, Priority::Interactive, Pace::Here);
        let mut third_click = ask(&slots, Priority::Interactive, Pace::Here);
        let _clicked = granted(&mut click).expect("the reserve is the click's");
        let _clicked_again = granted(&mut second_click).expect("all of it");
        assert!(
            granted(&mut third_click).is_none(),
            "and the pool as a whole is four"
        );
        let report = slots.report();
        assert_eq!(report.running, 4);
        assert_eq!(report.running_elsewhere, 1);
        assert_eq!(report.running_background, 1);
        assert_eq!(report.queued_interactive, 2, "the push and the third click");
        assert_eq!(report.queued_background, 1);
    }

    #[tokio::test]
    async fn wider_limits_admit_what_is_waiting_and_narrower_ones_admit_nothing_more() {
        let slots = slots(1, 0);
        let mut first = ask(&slots, Priority::Interactive, Pace::Here);
        let mut second = ask(&slots, Priority::Interactive, Pace::Here);
        let _held = granted(&mut first).expect("one runs");
        assert!(granted(&mut second).is_none());

        slots.set_limits(Limits {
            total: 2,
            reserve: 1,
        });
        let held_second = granted(&mut second).expect("the wider pool admits the waiter");

        slots.set_limits(Limits {
            total: 1,
            reserve: 0,
        });
        let mut third = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(
            granted(&mut third).is_none(),
            "two are running over a pool of one"
        );
        drop(held_second);
        assert!(
            granted(&mut third).is_none(),
            "one given back still leaves the pool full"
        );
        assert_eq!(slots.report().running, 1);
    }

    #[tokio::test]
    async fn unbounded_slots_admit_everything_and_still_count() {
        let slots = Arc::new(Slots::unbounded());
        let mut asks: Vec<Ask> = (0..16)
            .map(|_| ask(&slots, Priority::Background, Pace::Here))
            .collect();
        let held: Vec<Slot> = asks
            .iter_mut()
            .map(|a| granted(a).expect("nothing waits"))
            .collect();
        assert_eq!(slots.report().running_background, 16);
        drop(held);
        assert_eq!(slots.report().running, 0);
    }

    #[test]
    fn the_limits_one_number_stands_for() {
        assert_eq!(
            Limits::of(1),
            Limits {
                total: 1,
                reserve: 0
            },
            "one process: the background read has to be able to run at all"
        );
        assert_eq!(
            Limits::of(2),
            Limits {
                total: 2,
                reserve: 1
            }
        );
        assert_eq!(
            Limits::of(3),
            Limits {
                total: 3,
                reserve: 2
            },
            "a quarter rounds down for what is shared"
        );
        assert_eq!(
            Limits::of(4),
            Limits {
                total: 4,
                reserve: 3
            },
            "four slots is where the click's three commands all fit"
        );
        assert_eq!(
            Limits::of(8),
            Limits {
                total: 8,
                reserve: 6
            }
        );
        assert_eq!(Limits::of(8).shared(), 2);
        assert_eq!(
            Limits::of(0).total,
            1,
            "zero is the nearest number that runs anything"
        );
        assert_eq!(
            Limits::of(MAX_CONCURRENCY + 1).total,
            MAX_CONCURRENCY as usize
        );
    }

    /// The second identical background ask follows the first while the
    /// first is still queued, and leads for itself once the first has
    /// spawned — the answer of a run that may have looked before the ask
    /// is not that ask's answer.
    #[tokio::test]
    async fn an_identical_ask_follows_a_queued_leader_and_leads_after_a_spawned_one() {
        let slots = Arc::new(Slots::unbounded());
        let key = GroupKey {
            program: "git".into(),
            cwd: None,
            args: vec!["status".into()],
            env: Vec::new(),
        };
        let WayIn::Lead(mut lead) = slots.lead_or_follow(key.clone()) else {
            panic!("the first ask leads");
        };
        let WayIn::Follow(mut follows) = slots.lead_or_follow(key.clone()) else {
            panic!("the second ask follows the queued leader");
        };
        lead.spawning();
        assert!(lead.has_followers(), "the follower joined before the spawn");
        let WayIn::Lead(_late) = slots.lead_or_follow(key) else {
            panic!("an ask after the spawn leads for itself");
        };
        drop(lead);
        assert!(
            poll_once(&mut follows).is_ready(),
            "followers dropped unanswered are told so, and ask again"
        );
        assert_eq!(slots.report().shared, 1);
    }

    #[tokio::test]
    async fn a_leader_that_leaves_before_spawning_closes_its_group() {
        let slots = Arc::new(Slots::unbounded());
        let key = GroupKey {
            program: "git".into(),
            cwd: None,
            args: vec!["status".into()],
            env: Vec::new(),
        };
        let WayIn::Lead(lead) = slots.lead_or_follow(key.clone()) else {
            panic!("the first ask leads");
        };
        let WayIn::Follow(mut follows) = slots.lead_or_follow(key.clone()) else {
            panic!("the second follows");
        };
        drop(lead);
        assert!(
            poll_once(&mut follows).is_ready(),
            "the follower is told the leader left"
        );
        let WayIn::Lead(_) = slots.lead_or_follow(key) else {
            panic!("with the group closed, the next ask leads");
        };
    }

    // --- waiting for the queue to move (`settled`) ------------------------
    //
    // Driven by hand throughout: every one of these is about the order two
    // things happen in, and a wait that needed time to pass would be
    // answering about the machine instead.

    /// Waiting for what is already true answers without waiting at all.
    #[tokio::test]
    async fn a_wait_for_what_already_stands_answers_at_once() {
        let slots = slots(1, 0);
        let mut first = ask(&slots, Priority::Interactive, Pace::Here);
        let _held = granted(&mut first).expect("the first fits");
        let mut second = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut second).is_none(), "the second queues");

        let mut waiting = Box::pin(slots.settled(|report| report.queued_interactive == 1));
        assert!(
            poll_once(&mut waiting).is_ready(),
            "the queue already answers, so nothing is waited for"
        );
    }

    /// And a wait begun first is woken by the ask that arrives after it.
    #[tokio::test]
    async fn a_wait_begun_first_is_woken_by_the_ask_that_follows() {
        let slots = slots(1, 0);
        let mut first = ask(&slots, Priority::Interactive, Pace::Here);
        let _held = granted(&mut first).expect("the first fits");

        let mut waiting = Box::pin(slots.settled(|report| report.queued_interactive == 1));
        assert!(
            poll_once(&mut waiting).is_pending(),
            "nothing is queued yet, so it registers and waits"
        );

        let mut second = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut second).is_none(), "the second queues");
        assert!(
            poll_once(&mut waiting).is_ready(),
            "and the wait it was registered for woke it"
        );
    }

    /// **A stir landing inside the look is not lost** — the window
    /// [`Slots::settled`] takes its `Notified` out ahead of, driven
    /// through `settled` itself.
    ///
    /// The predicate is the way in: it is called between the two, on a
    /// `Report` already read, so an ask joining the queue from inside it
    /// lands in exactly the window — no second thread, and nothing in the
    /// slots that exists for a test. The predicate then answers `false`,
    /// because the report it was handed was taken before the join.
    ///
    /// So a `settled` that takes its `Notified` out *after* the look
    /// misses this stir — `notify_waiters` leaves no permit — and waits
    /// for a queue that has already stopped moving.
    #[tokio::test]
    async fn a_stir_from_inside_the_look_still_ends_the_wait() {
        let slots = slots(1, 0);
        let mut first = ask(&slots, Priority::Interactive, Pace::Here);
        let _held = granted(&mut first).expect("the first fits");
        let mut second = ask(&slots, Priority::Interactive, Pace::Here);

        let looks = std::cell::Cell::new(0);
        let mut waiting = Box::pin(slots.settled(|report| {
            looks.set(looks.get() + 1);
            if looks.get() == 1 {
                assert_eq!(report.queued_interactive, 0, "read before the join");
                assert!(
                    granted(&mut second).is_none(),
                    "the second joins the queue from inside the look"
                );
            }
            report.queued_interactive == 1
        }));

        assert!(
            poll_once(&mut waiting).is_ready(),
            "the stir made inside the look was held, and the look taken again answered"
        );
        drop(waiting);
        assert_eq!(
            looks.get(),
            2,
            "it looked again rather than answering on the stir"
        );
    }

    /// **The state is the answer, and the stir is only a reason to look.**
    /// A queue that moved before anybody was waiting leaves nothing behind
    /// to satisfy a later wait (`notify_waiters` stores no permit), and a
    /// stir that does not reach the answer leaves the wait where it was.
    #[tokio::test]
    async fn a_stir_is_not_the_answer_and_leaves_none_behind() {
        let slots = slots(1, 0);
        let mut first = ask(&slots, Priority::Interactive, Pace::Here);
        let _held = granted(&mut first).expect("the first fits");
        // Two stirs with nobody registered: one queues, one leaves.
        let mut early = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut early).is_none(), "it queues");
        drop(early);
        assert_eq!(slots.report().queued_interactive, 0, "and it leaves");

        let mut waiting = Box::pin(slots.settled(|report| report.queued_interactive == 1));
        assert!(
            poll_once(&mut waiting).is_pending(),
            "the stirs before it began left nothing it could take for an answer"
        );

        // A stir that moves the running rather than the queue: told, looked
        // at, and still not the answer.
        let mut background = ask(&slots, Priority::Background, Pace::Elsewhere);
        assert!(granted(&mut background).is_none(), "the pool is full");
        assert!(
            poll_once(&mut waiting).is_pending(),
            "it was woken and looked, and the interactive queue is still empty"
        );

        let mut second = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(granted(&mut second).is_none(), "the second queues");
        assert!(poll_once(&mut waiting).is_ready());
    }
}
