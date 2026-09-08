//! The QML rules, for the `tst_*.qml` files QtTest runs: a `wait(ms)`
//! spent in place of an answer, a `tryCompare` / `tryVerify` given a
//! deadline of its own in place of the runner's, and a rendering or
//! polish wait whose verdict nobody reads.

use super::source::{self, Lang, has_token, line_of};
use super::{Candidate, Exception, Finding};

/// The waits that answer with a bool, which a test has to read: a false
/// left unread is a paint that never happened, read as one that did.
const ANSWERED_WAITS: [&str; 2] = ["waitForRendering(", "waitForItemPolished("];

/// The QtTest calls that take a timeout, and which argument it is.
const OWN_DEADLINES: [(&str, usize); 2] = [("tryCompare(", 4), ("tryVerify(", 2)];

/// Judges one QML test file.
pub(super) fn scan(file: &str, text: &str) -> (Vec<Finding>, Vec<Exception>) {
    let code = source::code_view(text, Lang::Qml);
    let chars: Vec<char> = code.chars().collect();
    let mut candidates = Vec::new();
    for (at, line) in code.lines().enumerate() {
        let number = at + 1;
        if has_token(line, "wait(") || has_token(line, "sleep(") {
            candidates.push(Candidate {
                first: number,
                last: number,
                shown: number,
                rule: "sleep",
            });
        }
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
    use super::scan;

    fn found(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) = scan("tst_x.qml", text);
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
        let (findings, exceptions) = scan("tst_x.qml", text);
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
        let (findings, exceptions) = scan("tst_x.qml", text);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exceptions.len(), 1);
    }
}
