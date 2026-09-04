//! The command log: git invocations turned into session events, so they
//! travel the same path as everything else the UI shows.

use super::*;

/// What a session's command log holds: the commands the user asked for,
/// or those and the reads the session makes on its own.
///
/// Where a session starts is settled when it is created
/// ([`RepoSession::open_recording`]) rather than switched on once it has
/// been handed back. Opening spawns its own reads — refs, status, the
/// walk, the author, an open fetch — as soon as the path is accepted, so
/// a caller flipping this afterwards catches whichever of them the
/// scheduler had not reached yet, and how much of an opening a command
/// log holds becomes a scheduling accident.
///
/// Moving it later ([`RepoSession::set_recording`]) is still how the log
/// panel's switch works, and is exact for everything the session is asked
/// for from then on: the ask spawns its work after the store, so the
/// spawn carries the store to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Recording {
    /// The commands the user asked for, and nothing else. What the
    /// application opens with: a poll tick runs five commands and would
    /// bury the operations somebody actually performed.
    #[default]
    UserOnly,
    /// Those and the session's own reads, from the opening's first
    /// command on.
    WithBackground,
}

impl Recording {
    fn background(self) -> bool {
        self == Self::WithBackground
    }
}

/// Turns invocations into session events, so the command log travels the
/// same path as everything else the UI shows.
pub(super) struct CommandFeed {
    sink: Arc<dyn SessionSink>,
    next_id: AtomicU64,
    /// Whether the reads the session makes on its own are kept too (see
    /// [`Recording`]).
    record_background: std::sync::atomic::AtomicBool,
    /// The invocations that are worth a row only if git says no
    /// ([`Kept::UnaskedUnlessItFails`]), held from the spawn until the
    /// end that decides it. Emptied by every end, so what it holds is
    /// what is running.
    held: Mutex<HashMap<u64, Held>>,
}

/// What a held invocation needs to become a row after the fact. The line
/// cannot be rebuilt at the end — the command it describes is gone by
/// then — and the clock is the spawn's, not the failure's.
struct Held {
    display: String,
    full: String,
    at_ms: i64,
}

/// Whether git itself said no, which is the only end a command nobody
/// asked for leaves a row for. A cancelled read was stopped by this
/// window, and a code the command named as an answer is an answer.
fn said_no(end: CommandEnd) -> bool {
    match end {
        CommandEnd::Exited(code) => code != 0,
        CommandEnd::TimedOut | CommandEnd::Failed => true,
        CommandEnd::Answered(_) | CommandEnd::Cancelled => false,
    }
}

impl CommandFeed {
    pub(super) fn new(sink: Arc<dyn SessionSink>, recording: Recording) -> Self {
        Self {
            sink,
            // Ids start above zero so a log that has never spawned
            // anything cannot be answered by a default.
            next_id: AtomicU64::new(1),
            record_background: std::sync::atomic::AtomicBool::new(recording.background()),
            held: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn set_recording(&self, recording: Recording) {
        self.record_background
            .store(recording.background(), Ordering::Relaxed);
    }

    fn background(&self) -> bool {
        self.record_background.load(Ordering::Relaxed)
    }

    fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default()
    }
}

impl crate::process::CommandObserver for CommandFeed {
    fn records(&self, kept: Kept) -> bool {
        kept != Kept::Unasked || self.background()
    }

    fn started(&self, display: &str, full: &str, kept: Kept) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if kept == Kept::UnaskedUnlessItFails && !self.background() {
            relock(&self.held).insert(
                id,
                Held {
                    display: display.to_string(),
                    full: full.to_string(),
                    at_ms: Self::now_ms(),
                },
            );
            return id;
        }
        self.sink.event(SessionEvent::CommandStarted {
            id,
            display: display.to_string(),
            full: full.to_string(),
            at_ms: Self::now_ms(),
            asked: kept == Kept::Asked,
        });
        id
    }

    fn finished(&self, id: u64, end: CommandEnd, elapsed_ms: u64, message: &str) {
        let held = relock(&self.held).remove(&id);
        if let Some(held) = held {
            if !said_no(end) {
                return;
            }
            // The row lands whole: its start first, so whatever reads the
            // log builds the same row it would have built at the spawn.
            self.sink.event(SessionEvent::CommandStarted {
                id,
                display: held.display,
                full: held.full,
                at_ms: held.at_ms,
                asked: false,
            });
        }
        self.sink.event(SessionEvent::CommandFinished {
            id,
            end,
            elapsed_ms,
            message: message.to_string(),
        });
    }
}
