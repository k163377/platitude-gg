use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use platitude_core::session::Recording;

use crate::encode::{plain_byte, plain_ranges};
use crate::hub::{CommandMsg, Feed};

use super::{impl_notify_runs, push_run, qml_register};

mod selection;
#[cfg(test)]
mod selection_tests;

use selection::clock_of;

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
    /// The time of day this went out, `HH:mm:ss`, already in the reader's
    /// own zone (`selection::clock_of`). Made here rather than at the row
    /// so that the clock the reader copies and the clock they are looking
    /// at cannot be two different times.
    clock: String,
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
    /// Where the reader's own selection falls on this row, for the wash
    /// the delegate lays down (`selection::spell`). Empty on a row it
    /// does not reach.
    sel: String,
}

/// A row's invocation: the id its end arrives under, and whether the
/// reader is the one who asked for it. A fetch nobody asked for reaches
/// the log only when git said no, and its row says so without raising
/// anything (デザイン規約 §git が言ったことを読む場所).
struct Invocation {
    id: u64,
    asked: bool,
}

#[derive(Default)]
pub struct CommandsModel {
    rows: Vec<CommandItem>,
    /// One per row: what a `Finished` message finds its row by, and
    /// whose doing the row is. Kept beside the rows rather than in them —
    /// QML has no use for either.
    ids: Vec<Invocation>,
    /// Whether a command is in flight right now.
    running: bool,
    /// Whether the last command that ended failed. Cleared by the next
    /// one that does not.
    failed: bool,
    background_reads: bool,
    /// Minutes to add to local time to reach UTC, as the display side
    /// reads it off the machine (`Date.getTimezoneOffset()`). What stamps
    /// the rows that arrive from here on.
    zone_minutes: i32,
    /// Whether the reader is holding a selection at all. The four numbers
    /// below are the two ends, in the order the hand made them.
    sel_active: bool,
    sel_from_row: i32,
    sel_from_at: i32,
    sel_to_row: i32,
    sel_to_at: i32,
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

impl_notify_runs!(CommandsModel);

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
    /// this; a background read failing stays quiet
    /// (デザイン規約 §git が言ったことを読む場所).
    #[qsignal]
    fn failure(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.commands, invoker);
    }

    /// Empties the log. Only the record goes; nothing about the
    /// repository changes.
    #[qslot]
    fn clear(&mut self) {
        self.reset();
        self.running = false;
        self.failed = false;
        // The rows the selection named are gone, so the selection is too:
        // left standing it would name rows that arrive later.
        self.forget_selection();
        self.changed();
    }

    /// The last failure has been answered somewhere else on screen: the
    /// far side turned a write down under a rule of its own, and the page
    /// brought its words down in a report (デザイン規約 §可否・警告の出し場所).
    ///
    /// **Only the mark goes quiet.** It is the one thing here that
    /// fetches somebody — a failure nothing else has said — and there is
    /// nothing left for it to fetch them to. The row keeps git's words
    /// and its red edge: that is the record, and the record is what the
    /// panel is for.
    #[qslot]
    fn note_answered(&mut self) {
        if !self.failed {
            return;
        }
        self.failed = false;
        self.changed();
    }

    /// How many rows the log is holding, for the automation that has to
    /// weigh what a copy handed out against what it was made from
    /// (`PGG_AUTO_ACT=commands-copy`). The panel's own count is the view's
    /// (`ListView.count`), which is not the same number in the frame a
    /// press lands in — measured: 1 there against 4 commands on
    /// the clipboard, read microseconds apart in one tick.
    #[qslot]
    fn rows_held(&self) -> i32 {
        i32::try_from(self.rows.len()).unwrap_or(i32::MAX)
    }

    /// The machine's offset from UTC, which is the one thing about the
    /// clock this side cannot work out for itself (`selection::clock_of`).
    /// Asked for when the tab attaches and again whenever the panel comes
    /// up, so a session carried across a change of offset stamps what
    /// arrives afterwards with the new one.
    #[qslot]
    fn set_zone_minutes(&mut self, minutes: i32) {
        self.zone_minutes = minutes;
    }

    /// The text of one of a row's three columns, for the pane to lay out
    /// and read a place off (`LineRuler`). The line those columns are cut
    /// from is here, and where their characters are drawn is there
    /// (デザイン規約 §git が言ったことを読む場所).
    #[qslot]
    fn column_text(&self, row: i32, at: i32) -> String {
        self.column(row, at)
    }

    /// Which byte of a row's line a press landed on: the pane says which
    /// column it was in and which place of that column the row laid the
    /// pointer over. Pixels stop at the pane — only the column that was
    /// laid out knows where its characters are drawn, and only the line is
    /// here (デザイン規約 §git が言ったことを読む場所).
    #[qslot]
    fn hit_at(&self, row: i32, at: i32, place: i32) -> i32 {
        self.hit(row, at, place)
    }

    /// A press landed: the selection starts here and holds nothing yet.
    #[qslot]
    fn begin_select(&mut self, row: i32, at: i32) {
        if self.start_select(row, at) {
            self.changed();
        }
    }

    /// The hand has moved to here.
    #[qslot]
    fn extend_select(&mut self, row: i32, at: i32) {
        if self.drag_select(row, at) {
            self.changed();
        }
    }

    #[qslot]
    fn clear_select(&mut self) {
        if self.drop_selection() {
            self.changed();
        }
    }

    /// What Ctrl+C puts on the clipboard: the lines the selection covers,
    /// cut at its two ends, with git's own words under the ones taken
    /// whole.
    #[qslot]
    fn selection_text(&self) -> String {
        self.copied()
    }

    /// Also record the reads the session makes on its own. Applies to
    /// commands spawned from here on — what has already run is gone.
    #[qslot]
    fn set_background_reads(&mut self, on: bool) {
        self.background_reads = on;
        let recording = if on {
            Recording::WithBackground
        } else {
            Recording::UserOnly
        };
        crate::hub::with_session(self.tab_id, |s| s.set_recording(recording));
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
                    asked,
                } => {
                    let mut gone = 0;
                    while self.rows.len() >= KEEP {
                        self.remove(0);
                        gone += 1;
                    }
                    // The rows under the selection's two ends have moved
                    // (`shift_selection`); a wash left on its old numbers
                    // would name commands nobody picked.
                    if gone > 0 {
                        self.shift_selection(gone);
                    }
                    self.ids.push(Invocation { id, asked });
                    let args = display
                        .split_once(' ')
                        .map(|(_, rest)| rest.to_string())
                        .unwrap_or(display);
                    self.push(CommandItem {
                        clock: clock_of(at_ms, self.zone_minutes),
                        args,
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
                    let Some(index) = self.ids.iter().rposition(|got| got.id == id) else {
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
                    // The line just grew the two words about how it went,
                    // so a selection standing on this row reaches further
                    // than the wash it was last spelled with. Told again
                    // afterwards: the `set` above carried the old one.
                    if self.respell_row(index) {
                        self.notify_runs([(index, index)]);
                    }
                    // "Still running" is about the whole list, not this
                    // row: a fetch can outlive the write that started it.
                    self.running = self.rows.iter().any(|r| r.state == "running");
                    touched = true;
                    // A command nobody asked for leaves its row and
                    // nothing else. The mark and the panel answer for
                    // what the reader did, and the one thing an unasked
                    // fetch is entitled to — the first failure of a run —
                    // is the tab's to raise, once (`fetch_settled`).
                    if !self.ids[index].asked {
                        continue;
                    }
                    self.failed = !ok;
                    if !ok {
                        self.failure();
                    }
                }
            }
        }
        if touched {
            self.changed();
        }
        if crate::harness::memprobe::enabled() {
            crate::harness::memprobe::note("command-rows", self.tab_id, &self.rows);
        }
    }
}
qml_register!(CommandsModel, "CommandsModel", singleton = false);

impl platitude_core::mem::Footprint for CommandItem {
    fn heap_bytes(&self) -> usize {
        self.clock.heap_bytes()
            + self.args.heap_bytes()
            + self.full.heap_bytes()
            + self.state.heap_bytes()
            + self.result.heap_bytes()
            + self.duration.heap_bytes()
            + self.output.heap_bytes()
            + self.sel.heap_bytes()
    }
}

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
