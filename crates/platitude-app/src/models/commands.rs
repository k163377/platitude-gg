use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{CommandMsg, Feed, Hub};

use super::qml_register;

// ---------------------------------------------------------------------------
// CommandsModel: the git invocations this tab made, newest last.
//
// Nothing is written to disk and nothing survives the tab: this is the
// record of what the app just did, kept only while there is someone to
// read it (P3-確認事項「コマンドログに保存が無い」).
// ---------------------------------------------------------------------------

/// Rows kept per tab. The oldest fall off the end.
const KEEP: usize = 500;

#[derive(QModelItem, Default, Clone)]
pub struct CommandItem {
    /// Spawn time as epoch milliseconds; the delegate renders the clock.
    at_ms: i64,
    /// Everything after the program name (`push origin main`). The row
    /// draws `git` itself: it never varies, and what does is what the
    /// eye should land on.
    args: String,
    /// The same command with everything that is always applied spelled
    /// out, so copying it gives back what actually ran.
    full: String,
    /// "running" | "ok" | "failed"
    state: String,
    /// The right-hand word for a command that did not go through
    /// ("exit 1", "timed out"); empty when it did.
    result: String,
    /// How long it took, in the unit it reads best in.
    duration: String,
    /// git's own parting words. Only failures show it.
    output: String,
}

#[derive(Default)]
pub struct CommandsModel {
    rows: Vec<CommandItem>,
    /// Invocation ids, one per row: what a `Finished` message finds its
    /// row by. Kept beside the rows rather than in them — QML has no use
    /// for it.
    ids: Vec<u64>,
    /// Whether a command is in flight right now.
    running: bool,
    /// Whether the last command that ended failed. Cleared by the next
    /// one that does not.
    failed: bool,
    background_reads: bool,
    feed: Option<Arc<Feed<CommandMsg>>>,
    tab_id: i32,
}

impl QListModel for CommandsModel {
    type Item = CommandItem;

    fn len(&self) -> usize {
        self.rows.len()
    }
    fn get(&self, index: usize) -> Option<&CommandItem> {
        self.rows.get(index)
    }
    fn set_unnotified(&mut self, index: usize, value: CommandItem) -> bool {
        match self.rows.get_mut(index) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }
    fn remove_unnotified(&mut self, index: usize) -> CommandItem {
        self.ids.remove(index);
        self.rows.remove(index)
    }
    fn push_unnotified(&mut self, value: CommandItem) {
        self.rows.push(value);
    }
    fn reset_unnotified(&mut self) {
        self.rows.clear();
        self.ids.clear();
    }
}

/// Milliseconds in the unit they read best in. Sub-second work is what
/// most of this log is, and "1420 ms" hides how long 1.42 s felt.
fn humanize(ms: i64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{}.{:02} s", ms / 1000, (ms % 1000) / 10)
    }
}

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl CommandsModel {
    qproperty!("running", Member = running, Notify = changed);
    qproperty!("failed", Member = failed, Notify = changed);
    qproperty!(
        "backgroundReads",
        Member = background_reads,
        Notify = changed
    );

    #[qsignal]
    fn changed(&mut self);

    /// A command the user asked for failed. The page opens the panel on
    /// this; a background read failing stays quiet (境界の決定 1).
    #[qsignal]
    fn failure(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.commands);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
    }

    /// Empties the log. Only the record goes; nothing about the
    /// repository changes.
    #[qslot]
    fn clear(&mut self) {
        self.reset();
        self.running = false;
        self.failed = false;
        self.changed();
    }

    /// Also record the reads the session makes on its own. Applies to
    /// commands spawned from here on — what has already run is gone.
    #[qslot]
    fn set_background_reads(&mut self, on: bool) {
        self.background_reads = on;
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.set_record_background(on);
        }
        self.changed();
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let mut touched = false;
        for msg in feed.drain() {
            match msg {
                CommandMsg::Started {
                    id,
                    display,
                    full,
                    at_ms,
                } => {
                    while self.rows.len() >= KEEP {
                        self.remove(0);
                    }
                    self.ids.push(id);
                    self.push(CommandItem {
                        at_ms,
                        args: display
                            .split_once(' ')
                            .map(|(_, rest)| rest.to_string())
                            .unwrap_or(display),
                        full,
                        state: "running".to_string(),
                        ..Default::default()
                    });
                    self.running = true;
                    touched = true;
                }
                CommandMsg::Finished {
                    id,
                    code,
                    note,
                    answered,
                    elapsed_ms,
                    message,
                } => {
                    // Newest first: the command that just ended is nearly
                    // always the last row.
                    let Some(index) = self.ids.iter().rposition(|got| *got == id) else {
                        continue;
                    };
                    let Some(row) = self.rows.get(index).cloned() else {
                        continue;
                    };
                    // A command asked to answer by its exit code has not
                    // failed by answering: the row keeps the code it
                    // returned, and the log stays where the reader put it.
                    let ok = answered || code == Some(0);
                    self.set(
                        index,
                        CommandItem {
                            state: if ok { "ok" } else { "failed" }.to_string(),
                            result: match (ok, code) {
                                (true, _) => String::new(),
                                (false, Some(code)) => format!("exit {code}"),
                                (false, None) => note,
                            },
                            duration: humanize(elapsed_ms),
                            output: message,
                            ..row
                        },
                    );
                    // "Still running" is about the whole list, not this
                    // row: a fetch can outlive the write that started it.
                    self.running = self.rows.iter().any(|r| r.state == "running");
                    self.failed = !ok;
                    touched = true;
                    if !ok {
                        self.failure();
                    }
                }
            }
        }
        if touched {
            self.changed();
        }
    }
}
qml_register!(CommandsModel, "CommandsModel", singleton = false);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_in_the_unit_they_belong_to() {
        assert_eq!(humanize(0), "0 ms");
        assert_eq!(humanize(84), "84 ms");
        assert_eq!(humanize(999), "999 ms");
        assert_eq!(humanize(1000), "1.00 s");
        assert_eq!(humanize(1425), "1.42 s");
        assert_eq!(humanize(63_500), "63.50 s");
    }
}
