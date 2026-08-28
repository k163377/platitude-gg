use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use platitude_core::session::Recording;

use crate::hub::{CommandMsg, Feed};

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

/// The gap between a command and the two words about how it went. Two
/// spaces, because the row draws them as a column of their own and one
/// space would read as another argument.
const GAP: &str = "  ";
/// git's own words, set in from the command they belong to the way the
/// row indents them.
const UNDER: &str = "    ";

/// The whole log as one block of text: one line per command in the order
/// they ran, with git's parting words under the ones that failed
/// (デザイン規約 §git が言ったことを読む場所).
///
/// What is on screen and nothing else. Exit 0 is not written, a
/// successful command's stderr is not shown and is not taken either, and
/// a command still running has neither word yet — its line stops at what
/// was asked of git, which is all that is known about it.
///
/// The clock is the one column left behind: the time of day is made on
/// the display side out of `at_ms`, by the only part of this that knows
/// the reader's zone (`Qt.formatDateTime`), and what a log is pasted
/// somewhere else for is the sequence, the answer and how long it took.
fn transcript(rows: &[CommandItem]) -> String {
    let mut out = String::new();
    for row in rows {
        if !out.is_empty() {
            out.push('\n');
        }
        // The program the rows draw rather than read, written out here: a
        // transcript is read away from the panel that supplied it.
        out.push_str("git ");
        out.push_str(&row.args);
        if !row.result.is_empty() {
            out.push_str(GAP);
            out.push_str(&row.result);
        }
        if !row.duration.is_empty() {
            out.push_str(GAP);
            out.push_str(&row.duration);
        }
        if row.state != "failed" {
            continue;
        }
        for line in row.output.lines() {
            out.push('\n');
            out.push_str(UNDER);
            out.push_str(line);
        }
    }
    out
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
        self.changed();
    }

    /// Everything the panel is showing, for the clipboard: the band's
    /// `Copy` takes the whole log the way its `Clear` empties the whole
    /// of it. Empty while there are no rows, which is why the button is
    /// off over an empty panel — an empty clipboard is not an answer
    /// anyone meant (デザイン規約 §diff の中身をコピーする).
    #[qslot]
    fn copy_text(&self) -> String {
        transcript(&self.rows)
    }

    /// How many rows the log is holding, for the automation that has to
    /// weigh what `copyText` handed out against what it was made from
    /// (`PG_AUTO_ACT=commands-copy`). The panel's own count is the view's
    /// (`ListView.count`), which is not the same number in the frame a
    /// press lands in — 2026-08-28 実測: 1 there against 4 commands on
    /// the clipboard, read microseconds apart in one tick.
    #[qslot]
    fn rows_held(&self) -> i32 {
        i32::try_from(self.rows.len()).unwrap_or(i32::MAX)
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
        if crate::memprobe::enabled() {
            crate::memprobe::note("command-rows", self.tab_id, &self.rows);
        }
    }
}
qml_register!(CommandsModel, "CommandsModel", singleton = false);

impl platitude_core::mem::Footprint for CommandItem {
    fn heap_bytes(&self) -> usize {
        self.args.heap_bytes()
            + self.full.heap_bytes()
            + self.state.heap_bytes()
            + self.result.heap_bytes()
            + self.duration.heap_bytes()
            + self.output.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(args: &str, state: &str, result: &str, duration: &str, output: &str) -> CommandItem {
        CommandItem {
            args: args.to_string(),
            full: format!("git {args}"),
            state: state.to_string(),
            result: result.to_string(),
            duration: duration.to_string(),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn an_empty_log_has_nothing_to_put_on_the_clipboard() {
        assert_eq!(transcript(&[]), "");
    }

    #[test]
    fn a_command_that_went_through_keeps_only_how_long_it_took() {
        assert_eq!(
            transcript(&[row("status --porcelain=v2", "ok", "", "84 ms", "")]),
            "git status --porcelain=v2  84 ms"
        );
    }

    #[test]
    fn a_command_still_running_stops_at_what_was_asked_of_git() {
        assert_eq!(
            transcript(&[row("fetch origin", "running", "", "", "")]),
            "git fetch origin"
        );
    }

    #[test]
    fn a_failure_carries_gits_own_words_under_it() {
        assert_eq!(
            transcript(&[row(
                "switch nope",
                "failed",
                "exit 128",
                "12 ms",
                "fatal: invalid reference: nope\nhint: try again",
            )]),
            "git switch nope  exit 128  12 ms\n    fatal: invalid reference: nope\n    hint: try again"
        );
    }

    #[test]
    fn what_the_rows_do_not_show_is_not_taken_either() {
        // stderr is reported for every command that ends, and the panel
        // draws it under the failures alone (P3-確認事項「成功した git の
        // stderr をコマンドログへ出すことを、EOL を理由には決めない」).
        assert_eq!(
            transcript(&[row(
                "fetch origin",
                "ok",
                "",
                "1.42 s",
                "From github.com:o/r"
            )]),
            "git fetch origin  1.42 s"
        );
    }

    #[test]
    fn the_rows_come_out_in_the_order_they_ran_one_line_each() {
        let text = transcript(&[
            row("add -- a.txt", "ok", "", "9 ms", ""),
            row("reset -- a.txt", "ok", "", "11 ms", ""),
        ]);
        assert_eq!(text, "git add -- a.txt  9 ms\ngit reset -- a.txt  11 ms");
        assert_eq!(text.lines().count(), 2);
    }

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
