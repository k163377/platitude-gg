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
mod tests;
