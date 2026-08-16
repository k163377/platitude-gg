//! The command log: git invocations turned into session events, so they
//! travel the same path as everything else the UI shows.

use super::*;

/// Turns invocations into session events, so the command log travels the
/// same path as everything else the UI shows.
pub(super) struct CommandFeed {
    sink: Arc<dyn SessionSink>,
    next_id: AtomicU64,
    /// Off by default: a poll tick runs five commands and would bury the
    /// operations the user actually performed.
    pub(super) record_background: std::sync::atomic::AtomicBool,
}

/// Id of a command that is not being recorded; its end is dropped too.
const UNRECORDED: u64 = 0;

impl CommandFeed {
    pub(super) fn new(sink: Arc<dyn SessionSink>) -> Self {
        Self {
            sink,
            next_id: AtomicU64::new(UNRECORDED + 1),
            record_background: std::sync::atomic::AtomicBool::new(false),
        }
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
