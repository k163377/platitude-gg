//! The Rust rules: every statement of test code that reads the clock,
//! sleeps, throws a wait's answer away, or awaits a silent completion
//! with nothing under it.

use super::source::{self, Lang, has_token};
use super::{Candidate, Exception, Finding};

/// The completions that resolve through a channel nothing else is
/// watching. A statement that holds one *and* an `.await` is a silent
/// wait; without the await it is just a handle changing hands.
const SILENT_WAITS: [&str; 4] = [
    ".outcome()",
    "wait_for_graph_passes()",
    "wait_for_snapshot_reads()",
    ".tick()",
];

/// What discharges one, looked for in the same statement — the unit that
/// survives however rustfmt wraps the call around its wrapper.
const BACKSTOPS: [&str; 2] = ["bounded(", "timeout("];

/// A stretch of clock, or a turn of the scheduler, spent in place of an
/// answer.
const SLEEPS: [&str; 3] = ["sleep(", "sleep_until(", "yield_now("];

/// A clock read — the start of a deadline of the test's own, or a
/// measurement a verdict gets hung on.
const CLOCKS: [&str; 2] = ["Instant::now()", "SystemTime::now()"];

/// The waits that take a budget. The suite's budget is a named constant;
/// a budget spelled out in the seat ([`OWN_BUDGETS`]) is the test's own.
const BUDGETED: [&str; 4] = [
    "timeout(",
    "recv_timeout(",
    "wait_timeout(",
    "park_timeout(",
];
const OWN_BUDGETS: [&str; 2] = ["Duration::from_", "Duration::new("];

/// One statement of the code view: what sits between `;`, `{` and `}`.
pub(super) struct Statement {
    pub text: String,
    /// The line its first character is on, and the line it ends on.
    pub first: usize,
    pub last: usize,
}

/// The statements of `code`, a code view ([`source::code_view`]) — so a
/// `;` inside a string is nobody's boundary. Coarse, but exact where it
/// matters: rustfmt may wrap a call across any number of lines and never
/// across a statement, so a wrapper and the wait it wraps always share
/// one. What stays out of sight is a wait threaded through a closure or
/// macro body (the `{` splits the statement) — this reads the shape the
/// suites write in, not the language.
pub(super) fn statements(code: &str) -> Vec<Statement> {
    let mut found = Vec::new();
    let mut text = String::new();
    let mut first = 1;
    let mut line = 1;
    let mut fresh = true;
    for ch in code.chars() {
        if fresh && !ch.is_whitespace() {
            first = line;
            fresh = false;
        }
        if matches!(ch, ';' | '{' | '}') {
            if !fresh {
                found.push(Statement {
                    text: std::mem::take(&mut text),
                    first,
                    last: line,
                });
            }
            text.clear();
            fresh = true;
        } else {
            text.push(ch);
        }
        if ch == '\n' {
            line += 1;
        }
    }
    if !fresh {
        found.push(Statement {
            text,
            first,
            last: line,
        });
    }
    found
}

/// The rule a statement breaks, if any. The most specific reading wins:
/// a wait whose answer is thrown away is named for that before it is
/// named for anything it also does.
pub(super) fn rule_of(statement: &str) -> Option<&'static str> {
    let awaits = statement.contains(".await");
    let silent = SILENT_WAITS.iter().any(|t| has_token(statement, t));
    let backstopped = BACKSTOPS.iter().any(|t| has_token(statement, t));
    if statement.trim_start().starts_with("let _ =") && awaits && (silent || backstopped) {
        return Some("ignored");
    }
    if SLEEPS.iter().any(|t| has_token(statement, t)) {
        return Some("sleep");
    }
    let own_budget = BUDGETED.iter().any(|t| has_token(statement, t))
        && OWN_BUDGETS.iter().any(|b| statement.contains(b));
    if CLOCKS.iter().any(|t| has_token(statement, t)) || own_budget {
        return Some("deadline");
    }
    if awaits && silent && !backstopped {
        return Some("naked");
    }
    None
}

/// Judges one Rust file. `whole` says the file is test code from top to
/// bottom; otherwise only its `#[cfg(test)] mod` blocks are read.
pub(super) fn scan(file: &str, text: &str, whole: bool) -> (Vec<Finding>, Vec<Exception>) {
    let code = source::code_view(text, Lang::Rust);
    let regions = if whole {
        vec![1..=usize::MAX]
    } else {
        source::test_regions(&code)
    };
    let candidates = statements(&code)
        .into_iter()
        .filter(|s| regions.iter().any(|r| r.contains(&s.first)))
        .filter_map(|s| {
            rule_of(&s.text).map(|rule| Candidate {
                first: s.first,
                last: s.last,
                rule,
            })
        })
        .collect();
    source::judged(file, text, &code, candidates)
}

#[cfg(test)]
mod tests {
    use super::{Statement, rule_of, scan, statements};

    fn naked(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) = scan("t.rs", text, true);
        findings.iter().map(|f| (f.line, f.rule)).collect()
    }

    #[test]
    fn a_wrapped_wait_passes_and_a_naked_one_is_named() {
        let text = "\
let a = bounded(\"x\", task.outcome()).await;
let b = task.outcome().await;
";
        assert_eq!(naked(text), vec![(2, "naked")]);
    }

    #[test]
    fn the_backstop_is_seen_across_rustfmts_wrapping() {
        let text = "\
let a = crate::support::wait::bounded(
    \"the tracked poll\",
    session.refresh_poll_tracked().outcome(),
)
.await;
let b = session
    .refresh_poll_tracked()
    .outcome()
    .await;
";
        assert_eq!(
            naked(text),
            vec![(6, "naked")],
            "the naked one is named at its own statement"
        );
    }

    #[test]
    fn boundary_waits_and_ticks_are_counted_too() {
        let text = "\
session.wait_for_graph_passes().await;
let took = ticker.tick().await;
tokio::time::timeout(BUDGET, session.wait_for_snapshot_reads()).await;
let handle = session.refresh_poll_tracked();
";
        assert_eq!(
            naked(text),
            vec![(1, "naked"), (2, "naked")],
            "an unawaited handle is not a wait"
        );
    }

    #[test]
    fn a_wait_described_in_a_comment_or_a_string_is_not_a_wait() {
        let text = "\
// waiting on task.outcome() here would .await forever
let a = \"task.outcome().await\";
";
        assert!(naked(text).is_empty());
    }

    #[test]
    fn a_url_in_a_string_does_not_weld_two_statements_together() {
        let text = "\
let a = bounded(\"x\", t.outcome()).await.unwrap_or(\"file://z\");
session.wait_for_snapshot_reads().await;
";
        assert_eq!(naked(text), vec![(2, "naked")]);
    }

    #[test]
    fn a_timeout_of_another_name_is_no_backstop() {
        let text = "\
let out = exec.no_timeout().run(cmd).outcome().await;
let b = unbounded(task.outcome()).await;
let ok = tokio::time::timeout(BUDGET, task.outcome()).await;
";
        assert_eq!(naked(text), vec![(1, "naked"), (2, "naked")]);
    }

    #[test]
    fn a_sleep_a_yield_and_a_clock_are_each_named() {
        let text = "\
tokio::time::sleep(Duration::from_millis(10)).await;
std::thread::sleep(PACE);
tokio::task::yield_now().await;
let deadline = Instant::now() + QUIET_BUDGET;
let took = tokio::time::timeout(std::time::Duration::from_secs(5), ticker.tick()).await;
let sleeper = spawn_sleeper();
";
        assert_eq!(
            naked(text),
            vec![
                (1, "sleep"),
                (2, "sleep"),
                (3, "sleep"),
                (4, "deadline"),
                (5, "deadline")
            ]
        );
    }

    #[test]
    fn a_channels_own_timeout_and_a_sleep_until_are_named_and_the_suites_budget_is_not() {
        let text = "\
let got = rx.recv_timeout(Duration::from_millis(100));
tokio::time::sleep_until(deadline).await;
let ok = rx.recv_timeout(OVERALL_BUDGET);
let held = pair.1.wait_timeout(guard, Duration::new(1, 0));
";
        assert_eq!(
            naked(text),
            vec![(1, "deadline"), (2, "sleep"), (4, "deadline")]
        );
    }

    #[test]
    fn a_wait_whose_answer_is_thrown_away_is_named_for_that() {
        assert_eq!(
            rule_of("let _ = bounded(\"x\", ticker.tick()).await"),
            Some("ignored")
        );
        assert_eq!(
            rule_of("let _ = reading.send(())"),
            None,
            "a send is not a wait"
        );
        assert_eq!(
            rule_of("let _ = session.wait_for_graph_passes().await"),
            Some("ignored")
        );
    }

    #[test]
    fn a_marker_lets_a_statement_stand_and_is_counted_by_its_purpose() {
        let text = "\
// The retry is a real run each time; only its pace is timed.
// waits(paced): the answer ends the loop, the sleep only spaces the asks
std::thread::sleep(PACE);
std::thread::sleep(PACE);
";
        let (findings, exceptions) = scan("t.rs", text, true);
        assert_eq!(
            findings.iter().map(|f| f.line).collect::<Vec<_>>(),
            vec![4],
            "the marker covers the statement under its comment block and no other"
        );
        assert_eq!(exceptions.len(), 1);
        assert_eq!(exceptions[0].line, 2);
    }

    #[test]
    fn a_marker_that_covers_no_wait_is_a_finding_of_its_own() {
        let text = "\
// waits(measured): nothing timed stands under this
let a = 1;
";
        let (findings, exceptions) = scan("t.rs", text, true);
        assert!(exceptions.is_empty());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "marker");
        assert_eq!(findings[0].line, 1);
    }

    #[test]
    fn only_the_test_module_of_a_source_file_is_read() {
        let text = "\
fn production() {
    std::thread::sleep(POLL);
}
#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        std::thread::sleep(POLL);
    }
}
";
        let (findings, _) = scan("src/x.rs", text, false);
        assert_eq!(findings.iter().map(|f| f.line).collect::<Vec<_>>(), vec![8]);
    }

    #[test]
    fn a_statement_knows_the_lines_it_spans() {
        let found: Vec<Statement> = statements("let a =\n  f();\n\nlet b = 1;");
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].first, found[0].last), (1, 2));
        assert_eq!((found[1].first, found[1].last), (4, 4));
    }
}
