//! The command log: git invocations turned into session events, so they
//! travel the same path as everything else the UI shows.

use super::*;

/// What a session's command log holds: the commands the user asked for,
/// or those and the reads the session makes on its own.
///
/// Set at creation ([`RepoSession::open_recording`]): opening spawns its
/// reads as soon as the path is accepted, so flipping this afterwards
/// catches a scheduling-dependent share of them.
/// [`RepoSession::set_recording`] (the log panel's switch) is exact for
/// everything asked from then on, whose work is spawned after the store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Recording {
    /// The commands the user asked for, and nothing else. The default:
    /// poll ticks would bury the operations somebody actually performed.
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

pub(super) struct CommandFeed {
    sink: Arc<dyn SessionSink>,
    next_id: AtomicU64,
    record_background: std::sync::atomic::AtomicBool,
    /// Invocations worth a row only if git says no
    /// ([`Kept::UnaskedUnlessItFails`]), held from the spawn until the end
    /// decides; every end removes its entry.
    held: Mutex<HashMap<u64, Held>>,
}

/// What a held invocation needs to become a row after the fact: the line
/// cannot be rebuilt at the end, and the clock is the spawn's.
struct Held {
    display: String,
    full: String,
    at_ms: i64,
    operation: Option<OperationId>,
}

/// Whether git itself said no — the only end that gives an unasked
/// command a row. A cancel was this window's doing, and a code the
/// command named as an answer is an answer.
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

    fn started(
        &self,
        display: &str,
        full: &str,
        kept: Kept,
        operation: Option<OperationId>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if kept == Kept::UnaskedUnlessItFails && !self.background() {
            relock(&self.held).insert(
                id,
                Held {
                    display: display.to_string(),
                    full: full.to_string(),
                    at_ms: Self::now_ms(),
                    operation,
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
            operation,
        });
        id
    }

    fn finished(&self, id: u64, end: CommandEnd, waited_ms: u64, elapsed_ms: u64, message: &str) {
        let held = relock(&self.held).remove(&id);
        if let Some(held) = held {
            if !said_no(end) {
                return;
            }
            // Start first, so the log builds the row it would have built
            // at the spawn.
            self.sink.event(SessionEvent::CommandStarted {
                id,
                display: held.display,
                full: held.full,
                at_ms: held.at_ms,
                asked: false,
                operation: held.operation,
            });
        }
        self.sink.event(SessionEvent::CommandFinished {
            id,
            end,
            waited_ms,
            elapsed_ms,
            message: message.to_string(),
        });
    }
}
