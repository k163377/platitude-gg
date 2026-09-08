//! The QML rules. Two kinds of file are read, and a beat means a
//! different thing in each.
//!
//! For the `tst_*.qml` files QtTest runs, where one runner holds one
//! budget: a `wait(ms)` spent in place of an answer, a `tryCompare` /
//! `tryVerify` given a deadline of its own in place of the runner's, and
//! a rendering or polish wait whose verdict nobody reads.
//!
//! For the app's own harness — the driver `PG_AUTO_ACT` runs the product
//! through, which is test code that ships inside the window — there is
//! no runner and no QtTest call at all. What it has instead is beats,
//! and every one of them comes from the single type that names the
//! cadence, so what is named here is a span of time a file spelled for
//! itself ([`SPANS`]) and a count of milliseconds it keeps
//! ([`keeps_a_count`]). A beat taken from the named cadence and a read
//! of the run's own clock name nothing: those two are the harness's
//! budgets, the way `crate::wait` is this runner's.

use super::source::{self, Lang, has_token, line_of};
use super::{Candidate, Exception, Finding};

/// The waits that answer with a bool, which a test has to read: a false
/// left unread is a paint that never happened, read as one that did.
const ANSWERED_WAITS: [&str; 2] = ["waitForRendering(", "waitForItemPolished("];

/// The QtTest calls that take a timeout, and which argument it is.
const OWN_DEADLINES: [(&str, usize); 2] = [("tryCompare(", 4), ("tryVerify(", 2)];

/// A span of time spelled in the file itself. A `Timer` is one whatever
/// its `interval` says — the default it never spells included, which is
/// a whole second of Qt's choosing — and so is the `duration` an
/// animation runs for. `Date` is the clock that is not the run's own
/// (`PerfProbe.clockMs`): a second origin, with nothing started at the
/// beginning of the run behind it. The one type held out of all this is
/// `SampleTimer`, which exists to carry the harness's single cadence;
/// it ends in `Timer`, and the left word boundary is what tells an
/// object of it from one of `Timer` ([`spelled`]).
const SPANS: [&str; 4] = ["Timer", "duration:", "Date.now(", "new Date("];

/// Which rules a QML file is read by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// A QtTest file: the runner stands behind every wait in it.
    Test,
    /// The app's automation harness (`src/auto`): no runner, and the
    /// beat is the product's own event loop.
    Harness,
}

/// Judges one QML file by the rules of its [`Kind`].
pub(super) fn scan(file: &str, text: &str, kind: Kind) -> (Vec<Finding>, Vec<Exception>) {
    let code = source::code_view(text, Lang::Qml);
    let chars: Vec<char> = code.chars().collect();
    let mut candidates = Vec::new();
    for (at, line) in code.lines().enumerate() {
        let number = at + 1;
        let mut named = |rule| {
            candidates.push(Candidate {
                first: number,
                last: number,
                shown: number,
                rule,
            });
        };
        if has_token(line, "wait(") || has_token(line, "sleep(") {
            named("sleep");
        }
        if kind == Kind::Harness {
            if SPANS.iter().any(|span| spelled(line, span)) {
                named("clock");
            }
            if keeps_a_count(line) {
                named("elapsed");
            }
        }
    }
    if kind == Kind::Harness {
        candidates.sort_by_key(|c| c.first);
        return source::judged(file, text, &code, candidates);
    }
    for name in ANSWERED_WAITS {
        for call in calls(&code, name) {
            if unread_call(&chars, &call) {
                candidates.push(Candidate {
                    first: call.first,
                    last: call.last,
                    shown: call.first,
                    rule: "ignored",
                });
            }
        }
    }
    for (name, position) in OWN_DEADLINES {
        for call in calls(&code, name) {
            // `undefined` in the timeout's seat is how a message is passed
            // without one: the runner's own budget stands.
            let own = call
                .args
                .get(position - 1)
                .map(|arg| arg.trim())
                .is_some_and(|arg| !arg.is_empty() && arg != "undefined");
            if own {
                candidates.push(Candidate {
                    first: call.first,
                    last: call.last,
                    shown: call.first,
                    rule: "deadline",
                });
            }
        }
    }
    candidates.sort_by_key(|c| c.first);
    source::judged(file, text, &code, candidates)
}

/// Whether `line` spells `span` as a span of its own. A bare name is a
/// type, and counts only where an object of it is being built (`Timer {`)
/// — the cadence's own type ends in that name, and the word boundary is
/// the whole of what tells `SampleTimer {` from `Timer {`. A name that
/// carries its own punctuation (`duration:`, `Date.now(`) is spelled
/// wherever it stands.
///
/// A type name the line then ends on counts as well: a brace put on the
/// next line is the same object built, and nothing else in QML leaves a
/// type standing alone at the end of a line.
fn spelled(line: &str, span: &str) -> bool {
    if !span.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return has_token(line, span);
    }
    let mut from = 0;
    while let Some(found) = line[from..].find(span) {
        let at = from + found;
        from = at + span.len();
        let after_a_word = at > 0 && is_word(line[..at].chars().last());
        let rest = line[from..].trim_start();
        if !after_a_word && (rest.starts_with('{') || rest.is_empty()) {
            return true;
        }
    }
    false
}

/// Whether `line` keeps a count of milliseconds by adding a beat onto
/// it. What the count is then used for is the marker's business — the
/// two the harness has are a number printed in a report and a cut-off
/// that turns a silent watchdog into a line — but keeping it at all is
/// reading the clock, so both are named.
fn keeps_a_count(line: &str) -> bool {
    line.split_once("+=")
        .is_some_and(|(_, added)| has_token(added, "interval"))
}

/// One call of a name looked for, read off the whole code view rather
/// than off one line: rustfmt and a hand alike wrap a call across as
/// many lines as they please, and what reads its answer stands wherever
/// the expression around it stands.
struct Call {
    /// The index of the name's first character, and of its `(`.
    start: usize,
    /// The index of the `)`, absent for a call never closed.
    close: Option<usize>,
    /// The lines it opens and closes on.
    first: usize,
    last: usize,
    /// Its arguments, split at the commas of its own level.
    args: Vec<String>,
}

/// Whether `call`'s answer reaches nobody: it stands as a statement of
/// its own, with nothing but a receiver (`case.`) before it and nothing
/// but a `;` or a closing brace after it. Under a `verify(`, an `=` or
/// an `if (` — on its own line or the one it wrapped from — the answer
/// is read.
fn unread_call(chars: &[char], call: &Call) -> bool {
    let mut at = call.start;
    while at > 0 && (is_word(Some(chars[at - 1])) || chars[at - 1] == '.') {
        at -= 1;
    }
    if !opens_a_statement(chars, at) {
        return false;
    }
    // A call left open reads as one nobody closed around, either.
    let Some(close) = call.close else {
        return true;
    };
    let ends_the_line = chars[close + 1..]
        .iter()
        .take_while(|c| **c != '\n')
        .all(|c| c.is_whitespace() || matches!(c, ';' | '}'));
    // What continues an expression cannot begin a statement, so a line
    // the call ends may still be a line the next one reads.
    let continued = chars[close + 1..]
        .iter()
        .find(|c| !c.is_whitespace())
        .is_some_and(|c| matches!(c, '.' | '?' | ':' | '&' | '|' | '='));
    ends_the_line && !continued
}

/// Whether a statement begins at `at`: what stands before it ends one —
/// a `;`, a brace, the start of the file, or a line that ended on a
/// value, which QML closes for the author who left the `;` off.
fn opens_a_statement(chars: &[char], at: usize) -> bool {
    let Some(before) = chars[..at].iter().rposition(|c| !c.is_whitespace()) else {
        return true;
    };
    matches!(chars[before], ';' | '{' | '}')
        || (chars[before + 1..at].contains(&'\n') && !source::opens_a_value(chars, at))
}

/// Every call of `name` in the code view. A call that is never closed is
/// the last one read: past an unclosed `(` nothing parses.
fn calls(code: &str, name: &str) -> Vec<Call> {
    let chars: Vec<char> = code.chars().collect();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(hit) = code[from..].find(name) {
        let at = from + hit;
        from = at + name.len();
        if at > 0 && is_word(code[..at].chars().last()) {
            continue;
        }
        let start = code[..at].chars().count();
        let open = code[..from].chars().count() - 1;
        let closed = arguments(&chars, open);
        let ends_at = closed
            .as_ref()
            .map_or(chars.len().saturating_sub(1), |(close, _)| *close);
        let unclosed = closed.is_none();
        found.push(Call {
            start,
            close: closed.as_ref().map(|(close, _)| *close),
            first: line_of(&chars, open),
            last: line_of(&chars, ends_at),
            args: closed.map_or_else(Vec::new, |(_, args)| args),
        });
        if unclosed {
            break;
        }
    }
    found
}

fn is_word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// The arguments between the `(` at `open` and its `)`, split at the
/// commas that are not inside a nested `()` / `[]` / `{}`; `None` when
/// the call is never closed.
fn arguments(chars: &[char], open: usize) -> Option<(usize, Vec<String>)> {
    let mut depth = 0usize;
    let mut args = vec![String::new()];
    for (i, c) in chars.iter().enumerate().skip(open) {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, args));
                }
            }
            ',' if depth == 1 => {
                args.push(String::new());
                continue;
            }
            _ => {}
        }
        if i > open {
            args.last_mut()?.push(*c);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Kind, scan};

    fn found(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) = scan("tst_x.qml", text, Kind::Test);
        findings.iter().map(|f| (f.line, f.rule)).collect()
    }

    /// The same, by the harness's rules.
    fn harness(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) = scan("WindowActs.qml", text, Kind::Harness);
        findings.iter().map(|f| (f.line, f.rule)).collect()
    }

    #[test]
    fn a_wait_in_milliseconds_is_a_sleep_and_the_answered_waits_are_not() {
        let text = "\
wait(50)
waitForRendering(item)
verify(waitForRendering(item), \"painted\")
tryVerify(() => item.done)
";
        assert_eq!(found(text), vec![(1, "sleep"), (2, "ignored")]);
    }

    #[test]
    fn an_answered_wait_called_on_the_case_is_still_unread() {
        let text = "\
testCase.waitForRendering(item)
const ok = waitForRendering(item)
if (!waitForRendering(item)) fail(\"no paint\")
";
        assert_eq!(found(text), vec![(1, "ignored")]);
    }

    #[test]
    fn a_timeout_of_the_tests_own_is_a_deadline_and_the_runners_is_not() {
        let text = "\
tryCompare(card, \"opened\", true)
tryCompare(card, \"opened\", true, 1000)
tryVerify(() => root.n !== -1, 1000, \"reported, once the (double) window closes\")
tryVerify(() => root.n !== -1,
          \"a message with, a comma\")
tryVerify(() => root.n !== -1, undefined, \"the runner's own budget\")
";
        assert_eq!(found(text), vec![(2, "deadline"), (3, "deadline")]);
    }

    #[test]
    fn a_wrapped_answered_wait_is_read_across_the_lines_it_spans() {
        let text = "\
waitForRendering(
    item,
    500)
const painted =
    waitForRendering(item)
verify(waitForRendering(
    item), \"painted\")
if (!waitForItemPolished(
        item)) fail(\"no polish\")
";
        assert_eq!(
            found(text),
            vec![(1, "ignored")],
            "the answer is read where the expression around it stands, however it wrapped"
        );
    }

    #[test]
    fn a_marker_above_a_wrapped_wait_covers_the_whole_call() {
        let text = "\
// waits(timed): the paint is the product's own, and the bound is a floor
waitForRendering(
    item,
    500)
";
        let (findings, exceptions) = scan("tst_x.qml", text, Kind::Test);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exceptions.len(), 1);
    }

    #[test]
    fn a_regex_literal_hides_nothing_behind_it() {
        let text = "\
const bare = /'/
wait(50)
";
        assert_eq!(
            found(text),
            vec![(2, "sleep")],
            "the quote in the pattern opens no string"
        );
    }

    #[test]
    fn a_marker_above_the_line_lets_it_stand() {
        let text = "\
// The double-click window is the product's own, read off the style hints.
// waits(timed): the window is Qt's, and the bound is a floor no load can break
tryVerify(() => root.n !== -1, Qt.styleHints.mouseDoubleClickInterval * 2)
";
        let (findings, exceptions) = scan("tst_x.qml", text, Kind::Test);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exceptions.len(), 1);
    }

    #[test]
    fn the_harness_cadence_spells_no_span_and_a_timer_of_its_own_does() {
        let text = "\
SampleTimer {
    running: Harness.autoAct === \"tip\"
}
Timer {
    id: watchdog
    interval: Math.max(Harness.autoWatchdogMs, 1)
}
Timer {
}
NumberAnimation {
    duration: 12000
}
Timer
{
}
";
        assert_eq!(
            harness(text),
            vec![(4, "clock"), (8, "clock"), (11, "clock"), (13, "clock")],
            "the cadence has a name; a timer that spells none of its own still holds Qt's second, \
             and a brace on the next line builds the same object"
        );
    }

    #[test]
    fn the_harness_names_the_clock_it_keeps_and_not_the_runs_own() {
        let text = "\
Harness.report(\"perf_begin clock_ms=\" + PerfProbe.clockMs())
acts.at = Date.now()
Harness.report(\"wait=\" + (Date.now() - acts.at))
timer.waited += timer.interval
timer.done++
";
        assert_eq!(
            harness(text),
            vec![(2, "clock"), (3, "clock"), (4, "elapsed")],
            "the run's own clock is the harness's budget; a second origin and a kept count are not"
        );
    }

    #[test]
    fn the_qtest_rules_and_the_harness_rules_do_not_reach_into_each_other() {
        let beats = "Timer {\n    interval: 50\n}\n";
        assert_eq!(found(beats), vec![], "a QtTest file is judged on its calls");
        let calls = "tryCompare(card, \"opened\", true, 1000)\nwaitForRendering(item)\n";
        assert_eq!(
            harness(calls),
            vec![],
            "the harness has no runner to hold a budget for it"
        );
        assert_eq!(
            harness("wait(50)\n"),
            vec![(1, "sleep")],
            "a stretch of clock spent in place of an answer is neither one's to keep"
        );
    }

    #[test]
    fn a_marker_over_a_harness_beat_names_what_it_is_for() {
        let text = "\
// waits(ceiling): the run that stops answering is ended here and nowhere else
Timer {
    id: watchdog
    interval: Math.max(Harness.autoWatchdogMs, 1)
}
";
        let (findings, exceptions) = scan("WindowActs.qml", text, Kind::Harness);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exceptions.len(), 1);
    }
}
