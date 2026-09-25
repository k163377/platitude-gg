use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use platitude_core::session::Recording;

use qtbridge::qtbridge_type_lib::QVariantMap;

use crate::encode::{Fields, Optional, Record, Runs, field, plain_byte, plain_ranges};
use crate::hub::{CommandMsg, Feed};

use super::{impl_notify_runs, push_run, qml_register};

mod selection;
#[cfg(test)]
mod selection_tests;

use selection::clock_of;

// ---------------------------------------------------------------------------
// CommandsModel: the git invocations this tab made, newest last. In memory
// only (P3-確認事項「コマンドログに保存が無い」).
// ---------------------------------------------------------------------------

/// Rows kept per tab. The oldest fall off the end.
const KEEP: usize = 500;

#[derive(QModelItem, Default, Clone)]
pub struct CommandItem {
    /// When this went out, `HH:mm:ss` in the reader's zone
    /// (`selection::clock_of`) — made here so the copied clock is the one
    /// on screen.
    clock: String,
    /// Everything after the program name (`push origin main`); the row
    /// draws `git` itself.
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
    /// This row's share of the reader's selection (`selection::spell`);
    /// none on a row it does not reach.
    sel: Optional<CommandWash>,
}

/// One row's share of the reader's selection, as the delegate reads it:
/// each column's runs (`encode::plain_ranges`, empty where the column holds
/// none of it), and whether the line is taken end to end — which brings
/// git's own words with it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandWash {
    pub clock: Runs,
    pub command: Runs,
    pub outcome: Runs,
    pub whole: bool,
}

impl Record for CommandWash {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("clock", &self.clock)
            .put("command", &self.command)
            .put("outcome", &self.outcome)
            .put("whole", &self.whole)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            clock: field(map, "clock")?,
            command: field(map, "command")?,
            outcome: field(map, "outcome")?,
            whole: field(map, "whole")?,
        })
    }
}

impl platitude_core::mem::Footprint for CommandWash {
    fn heap_bytes(&self) -> usize {
        self.clock.heap_bytes() + self.command.heap_bytes() + self.outcome.heap_bytes()
    }
}

/// A row's invocation: the id its end arrives under, and whether the
/// reader asked for it (an unasked fetch's row raises nothing —
/// デザイン規約 §git が言ったことを読む場所). `run` is part of the key
/// because the log outlives a session (`CommandMsg`).
struct Invocation {
    run: u64,
    id: u64,
    asked: bool,
}

#[derive(Default)]
pub struct CommandsModel {
    rows: Vec<CommandItem>,
    /// One per row, beside the rows because QML has no use for them.
    ids: Vec<Invocation>,
    running: bool,
    /// Whether the last command the reader asked for failed.
    failed: bool,
    background_reads: bool,
    /// Minutes to add to local time to reach UTC
    /// (`Date.getTimezoneOffset()`); stamps the rows that arrive from here on.
    zone_minutes: i32,
    /// Whether a selection stands; the four below are its two ends, press
    /// first.
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

/// Milliseconds in the unit they read best in.
fn humanize(ms: i64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{}.{:02} s", ms / 1000, (ms % 1000) / 10)
    }
}

/// What a row shows for time: the run, and any wait for a slot beside it —
/// kept apart because they blame different things (a slow git, a busy
/// application).
fn spent(elapsed_ms: i64, waited_ms: i64) -> String {
    if waited_ms > 0 {
        format!("{} (queued {})", humanize(elapsed_ms), humanize(waited_ms))
    } else {
        humanize(elapsed_ms)
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

    /// A command the user asked for failed; a background read failing
    /// stays quiet (デザイン規約 §git が言ったことを読む場所).
    ///
    /// Do not raise the panel on this: whether the operation failed is its
    /// own answer's to say, on the tab's feed, which drains in no fixed
    /// order with this one (`RepoPage`). Read for bringing the newest row
    /// into view (`CommandsPane`) and re-quieting an answered mark
    /// (`note_answered`).
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
        // Left standing, the selection would name rows that arrive later.
        self.forget_selection();
        self.changed();
    }

    /// The last failure was answered elsewhere on screen, by a report of a
    /// refused write (デザイン規約 §可否・警告の出し場所). Only the mark goes
    /// quiet; the row keeps git's words and its red edge as the record.
    #[qslot]
    fn note_answered(&mut self) {
        if !self.failed {
            return;
        }
        self.failed = false;
        self.changed();
    }

    /// How many rows the log holds, for the automation
    /// (`PGG_AUTO_ACT=commands-copy`): `ListView.count` lags the model in
    /// the frame a press lands in.
    #[qslot]
    fn rows_held(&self) -> i32 {
        i32::try_from(self.rows.len()).unwrap_or(i32::MAX)
    }

    /// The machine's UTC offset (`selection::clock_of`), sent when the tab
    /// attaches and whenever the panel comes up, so a change of offset
    /// reaches the rows that arrive afterwards.
    #[qslot]
    fn set_zone_minutes(&mut self, minutes: i32) {
        self.zone_minutes = minutes;
    }

    /// The text of one of a row's three columns, for the pane to lay out
    /// (`LineRuler`).
    #[qslot]
    fn column_text(&self, row: i32, at: i32) -> String {
        self.column(row, at)
    }

    /// Which byte of a row's line a press landed on, given the column and
    /// the place in it the pane measured; pixels stop at the pane.
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
                    run,
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
                    if gone > 0 {
                        self.shift_selection(gone);
                    }
                    self.ids.push(Invocation { run, id, asked });
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
                    run,
                    id,
                    code,
                    note,
                    answered,
                    waited_ms,
                    elapsed_ms,
                    message,
                } => {
                    // From the end: it is nearly always the last row.
                    let Some(index) = self
                        .ids
                        .iter()
                        .rposition(|got| got.id == id && got.run == run)
                    else {
                        continue;
                    };
                    let Some(row) = self.rows.get(index).cloned() else {
                        continue;
                    };
                    // A command asked to answer by its exit code has not
                    // failed by answering.
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
                            duration: spent(elapsed_ms, waited_ms),
                            output: message,
                            ..row
                        },
                    );
                    // The line grew its outcome, so re-spell the wash; the
                    // `set` above carried the old one.
                    if self.respell_row(index) {
                        self.notify_runs([(index, index)]);
                    }
                    // "Still running" is about the whole list: a fetch
                    // can outlive the write that started it.
                    self.running = self.rows.iter().any(|r| r.state == "running");
                    touched = true;
                    // An unasked command leaves its row only; its run's
                    // first failure is the tab's to raise (`fetch_settled`).
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

    #[test]
    fn a_wait_for_a_slot_is_shown_beside_the_run_and_only_when_there_was_one() {
        assert_eq!(spent(84, 0), "84 ms");
        assert_eq!(spent(84, 120), "84 ms (queued 120 ms)");
        assert_eq!(spent(1425, 2000), "1.42 s (queued 2.00 s)");
    }
}
