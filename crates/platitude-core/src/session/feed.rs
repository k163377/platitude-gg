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
}

/// Id of a command that is not being recorded; its end is dropped too.
const UNRECORDED: u64 = 0;

impl CommandFeed {
    pub(super) fn new(sink: Arc<dyn SessionSink>, recording: Recording) -> Self {
        Self {
            sink,
            next_id: AtomicU64::new(UNRECORDED + 1),
            record_background: std::sync::atomic::AtomicBool::new(recording.background()),
        }
    }

    pub(super) fn set_recording(&self, recording: Recording) {
        self.record_background
            .store(recording.background(), Ordering::Relaxed);
    }
}

impl crate::process::CommandObserver for CommandFeed {
    fn records(&self, user: bool) -> bool {
        user || self.record_background.load(Ordering::Relaxed)
    }

    fn started(&self, display: &str, full: &str, user: bool) -> u64 {
        if !self.records(user) {
            return UNRECORDED;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default();
        self.sink.event(SessionEvent::CommandStarted {
            id,
            display: display.to_string(),
            full: full.to_string(),
            at_ms,
        });
        id
    }

    fn finished(&self, id: u64, end: CommandEnd, elapsed_ms: u64, message: &str) {
        if id == UNRECORDED {
            return;
        }
        self.sink.event(SessionEvent::CommandFinished {
            id,
            end,
            elapsed_ms,
            message: message.to_string(),
        });
    }
}
