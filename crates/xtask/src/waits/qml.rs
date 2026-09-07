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
    let mut candidates = Vec::new();
    for (at, line) in code.lines().enumerate() {
        let number = at + 1;
        let trimmed = line.trim_start();
        if has_token(line, "wait(") || has_token(line, "sleep(") {
            candidates.push(Candidate {
                first: number,
                last: number,
                shown: number,
                rule: "sleep",
            });
        } else if ANSWERED_WAITS.iter().any(|w| unread_call(trimmed, w)) {
            candidates.push(Candidate {
                first: number,
                last: number,
                shown: number,
                rule: "ignored",
            });
        }
    }
    for (name, position) in OWN_DEADLINES {
        for (first, last, args) in calls(&code, name) {
            // `undefined` in the timeout's seat is how a message is passed
            // without one: the runner's own budget stands.
            let own = args
                .get(position - 1)
                .map(|arg| arg.trim())
                .is_some_and(|arg| !arg.is_empty() && arg != "undefined");
            if own {
                candidates.push(Candidate {
                    first,
                    last,
                    shown: first,
                    rule: "deadline",
                });
            }
        }
    }
    candidates.sort_by_key(|c| c.first);
    source::judged(file, text, &code, candidates)
}

/// Whether `line` is a bare call of `name`, its answer reaching nobody:
/// nothing but a receiver (`case.`) before it, and nothing but a `;`
/// after it. Under a `verify(`, an `=` or an `if (` the answer is read.
fn unread_call(line: &str, name: &str) -> bool {
    let Some(at) = line.find(name) else {
        return false;
    };
    let receiver = &line[..at];
    if !receiver.is_empty() && !receiver.ends_with('.') {
        return false;
    }
    if !receiver
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return false;
    }
    let chars: Vec<char> = line.chars().collect();
    let open = line[..at + name.len()].chars().count() - 1;
    match arguments(&chars, open) {
        Some((close, _)) => chars[close + 1..]
            .iter()
            .all(|c| c.is_whitespace() || *c == ';'),
        // A call left open on this line: nothing on the line reads it.
        None => true,
    }
}

/// Every call of `name` in the code view: the lines it opens and closes
/// on, and its arguments split at the commas of its own level.
fn calls(code: &str, name: &str) -> Vec<(usize, usize, Vec<String>)> {
    let chars: Vec<char> = code.chars().collect();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(hit) = code[from..].find(name) {
        let at = from + hit;
        from = at + name.len();
        if !has_token(&code[at..from], name) || at > 0 && is_word(code[..at].chars().last()) {
            continue;
        }
        let open = code[..from].chars().count() - 1;
        let Some((close, args)) = arguments(&chars, open) else {
            break;
        };
        found.push((line_of(&chars, open), line_of(&chars, close), args));
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
