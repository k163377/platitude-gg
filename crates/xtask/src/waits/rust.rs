//! The Rust rules: every statement of test code that reads the clock,
//! sleeps, throws a wait's answer away, or awaits a silent completion
//! with nothing under it — and, of a body read whole (this runner's own
//! and the app harness's Rust half, [`Scope::reads_the_body`]), every
//! statement that sleeps, reads the monotonic clock or receives under a
//! budget anywhere but `crate::wait` ([`tool_rule_of`]).

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

/// The monotonic clock: the start of a deadline of the seat's own, or a
/// measurement a verdict gets hung on. The name without its `()`, so
/// that the call and the function a `get_or_init` is handed are the one
/// clock read they both are — which is the spelling the harness starts
/// its clocks with ([`source::has_token`] guards the far end).
const MONOTONIC: &str = "Instant::now";

/// A clock read: [`MONOTONIC`], or the wall clock, which a test reads
/// for the same two reasons.
const CLOCKS: [&str; 2] = [MONOTONIC, "SystemTime::now"];

/// The waits that take a budget. The suite's budget is a named constant;
/// a budget spelled out in the seat ([`OWN_BUDGETS`]) is the test's own.
const BUDGETED: [&str; 4] = [
    "timeout(",
    "recv_timeout(",
    "wait_timeout(",
    "park_timeout(",
];
const OWN_BUDGETS: [&str; 2] = ["Duration::from_", "Duration::new("];

/// PowerShell's sleep, in the scripts this runner writes for its
/// samplers: the runner's own pace in another syntax, standing in a
/// string the code view has blanked — so it is looked for in the strings
/// view ([`source::strings_view`]), where a comment names nothing.
const SCRIPT_SLEEP: &str = "Start-Sleep";

/// How much of a Rust file is read, and by which rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scope {
    /// Test code from top to bottom: every statement, by the tests' rules
    /// ([`rule_of`]).
    Whole,
    /// A product crate's source: its `#[cfg(test)]` blocks, by the tests'
    /// rules. The product's own clocks are the product's business.
    Tests,
    /// This runner's own source: its test blocks by the tests' rules, and
    /// the rest — the tool's body — by the runner's ([`tool_rule_of`]).
    Tool,
    /// The app harness's Rust half, read the same way — the rules are
    /// the same shapes and the reason is the same one, so the only
    /// thing this arm changes is that its files are counted apart. The
    /// strings stay out: the scripts are this runner's habit, and a
    /// `Start-Sleep` in the harness's text would be a string about
    /// PowerShell.
    Harness,
}

impl Scope {
    /// Whether the statements outside the test blocks are read too.
    fn reads_the_body(self) -> bool {
        matches!(self, Scope::Tool | Scope::Harness)
    }
}

/// What reading one file found, and how much of it was read.
pub(super) struct Scanned {
    pub findings: Vec<Finding>,
    pub exceptions: Vec<Exception>,
    /// Statements read as test code.
    pub test_statements: usize,
    /// Statements read as a body ([`Scope::reads_the_body`]): none of a
    /// test file's, and none of a product crate's. Which body it was is
    /// the caller's to know, from the scope it asked for.
    pub body_statements: usize,
}

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
/// one. What stays out of sight is a wait threaded through a closure
/// or macro body (the `{` splits the statement) — this reads the shape
/// the suites write in.
///
/// `said` is the same text with its strings kept
/// ([`source::strings_view`]), or `code` itself where the strings are
/// nobody's business; it says where a statement begins, so that one that
/// is nothing but a string — a script handed back whole — is a statement
/// all the same. A boundary with nothing before it (a closing brace on a
/// line of its own) is none.
pub(super) fn statements(code: &str, said: &str) -> Vec<Statement> {
    let mut found = Vec::new();
    let mut text = String::new();
    let mut first = 1;
    let mut line = 1;
    let mut fresh = true;
    for (ch, spoken) in code.chars().zip(said.chars()) {
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
            if fresh && !spoken.is_whitespace() {
                first = line;
                fresh = false;
            }
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

/// The rule a statement of test code breaks, if any. The most specific
/// reading wins: a wait whose answer is thrown away is named for that
/// before it is named for anything it also does.
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

/// The rule a statement of this runner's body breaks, if any, and where
/// it is shown — which of the statement's lines, as an offset from its
/// first. A verb's waits come from `crate::wait` (.claude/rules-refs/core.md:
/// xtask の待ちは `crate::wait` 1 本から取る), so what is named is what that
/// module is the one place for: a sleep, in the code or in a script the
/// code writes ([`SCRIPT_SLEEP`], found in `said`, the statement's lines
/// with their strings kept and their comments blanked, and shown where it
/// stands); a read of the monotonic clock; a receive under a budget —
/// any budget, since a verb has no suite's to take. The wall clock is a
/// timestamp in a verb; a `Duration` is a ceiling declared where its
/// reason stands; and nothing here awaits, so the silent waits have no
/// shape to take.
pub(super) fn tool_rule_of(statement: &str, said: &[&str]) -> Option<(&'static str, usize)> {
    if SLEEPS.iter().any(|t| has_token(statement, t)) {
        return Some(("sleep", 0));
    }
    if let Some(at) = said.iter().position(|line| has_token(line, SCRIPT_SLEEP)) {
        return Some(("sleep", at));
    }
    if has_token(statement, MONOTONIC) || BUDGETED.iter().any(|t| has_token(statement, t)) {
        return Some(("deadline", 0));
    }
    None
}

/// Judges one Rust file, as much of it as `scope` says.
pub(super) fn scan(file: &str, text: &str, scope: Scope) -> Scanned {
    let code = source::code_view(text, Lang::Rust);
    let regions = if scope == Scope::Whole {
        Vec::new()
    } else {
        source::test_regions(&code)
    };
    // What the strings say is read of a tool body alone, for the scripts
    // this runner writes; a test's strings are nobody's business.
    let said = (scope == Scope::Tool).then(|| source::strings_view(text, Lang::Rust));
    let said_lines: Vec<&str> = said
        .as_deref()
        .map(|s| s.lines().collect())
        .unwrap_or_default();
    let (mut test_statements, mut body_statements) = (0, 0);
    let mut candidates = Vec::new();
    for s in statements(&code, said.as_deref().unwrap_or(&code)) {
        let of_tests = scope == Scope::Whole || regions.iter().any(|r| r.contains(&s.first));
        let named = if of_tests {
            test_statements += 1;
            rule_of(&s.text).map(|rule| (rule, s.first))
        } else if scope.reads_the_body() {
            body_statements += 1;
            let lines = said_lines
                .get(s.first.saturating_sub(1)..s.last.min(said_lines.len()))
                .unwrap_or_default();
            tool_rule_of(&s.text, lines).map(|(rule, at)| (rule, s.first + at))
        } else {
            None
        };
        if let Some((rule, shown)) = named {
            candidates.push(Candidate {
                first: s.first,
                last: s.last,
                shown,
                rule,
            });
        }
    }
    let (findings, exceptions) = source::judged(file, text, &code, candidates);
    Scanned {
        findings,
        exceptions,
        test_statements,
        body_statements,
    }
}

#[cfg(test)]
mod tests {
    use super::{Scanned, Scope, Statement, rule_of, scan, statements};

    fn named(text: &str, scope: Scope) -> Vec<(usize, &'static str)> {
        scan("t.rs", text, scope)
            .findings
            .iter()
            .map(|f| (f.line, f.rule))
            .collect()
    }

    fn naked(text: &str) -> Vec<(usize, &'static str)> {
        named(text, Scope::Whole)
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
        let Scanned {
            findings,
            exceptions,
            ..
        } = scan("t.rs", text, Scope::Whole);
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
        let Scanned {
            findings,
            exceptions,
            ..
        } = scan("t.rs", text, Scope::Whole);
        assert!(exceptions.is_empty());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "marker");
        assert_eq!(findings[0].line, 1);
    }

    #[test]
    fn a_product_crates_source_is_read_for_its_tests_and_this_runners_for_its_body_too() {
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
        let product = scan("src/x.rs", text, Scope::Tests);
        assert_eq!(
            product.findings.iter().map(|f| f.line).collect::<Vec<_>>(),
            vec![8]
        );
        assert_eq!(
            (product.test_statements, product.body_statements),
            (3, 0),
            "the test block: its head, `fn t()` and the sleep"
        );
        for scope in [Scope::Tool, Scope::Harness] {
            let body = scan("src/x.rs", text, scope);
            assert_eq!(
                body.findings.iter().map(|f| f.line).collect::<Vec<_>>(),
                vec![2, 8],
                "{scope:?}"
            );
            assert_eq!(
                (body.test_statements, body.body_statements),
                (3, 2),
                "the body: `fn production()` and its sleep ({scope:?})"
            );
        }
    }

    /// The scripts are this runner's habit: a `Start-Sleep` in the
    /// harness's text is a string about PowerShell.
    #[test]
    fn a_script_sleep_is_the_runners_alone_and_not_the_harnesss() {
        let text = "let wake = \"Start-Sleep -Seconds 20\";\n";
        assert_eq!(named(text, Scope::Tool), vec![(1, "sleep")]);
        assert_eq!(named(text, Scope::Harness), vec![]);
    }

    #[test]
    fn the_runners_body_takes_its_waits_from_the_wait_module_and_is_named_for_any_other() {
        let text = "\
let mut wait = Wait::new(\"the gate\", Budget::whole(CEILING), LOOK_AGAIN);
wait.look_again(\"a free lane\")?;
let stood_for = crate::wait::stood(Duration::from_millis(STAND_MS), TRY_AGAIN, look);
let answer = crate::wait::receive(\"its output\", \"a line\", &rx, Budget::whole(CEILING));
const CEILING: Duration = Duration::from_secs(120);
let stamp = SystemTime::now();
let at = Instant::now();
let got = rx.recv_timeout(CEILING);
std::thread::sleep(PACE);
";
        assert_eq!(
            named(text, Scope::Tool),
            vec![(7, "deadline"), (8, "deadline"), (9, "sleep")],
            "the wait module's own calls, a ceiling declared and a stamp read all pass"
        );
    }

    #[test]
    fn a_sleep_in_a_script_the_runner_writes_is_shown_where_it_stands_and_covered_at_its_statement()
    {
        let text = "\
// waits(paced): the sampler's tick — one reading a beat, the loop ending on the process's exit
let script = format!(
    \"while($true){{ Write-Output 1;\\
       Start-Sleep -Milliseconds {SAMPLE_MS};\\
     }}\"
);
let wake = format!(
    \"Start-Sleep -Seconds {WAKE_SECS};\"
);
";
        let scanned = scan("t.rs", text, Scope::Tool);
        assert_eq!(
            scanned
                .findings
                .iter()
                .map(|f| (f.line, f.rule))
                .collect::<Vec<_>>(),
            vec![(8, "sleep")],
            "shown at the script's own line"
        );
        assert_eq!(scanned.exceptions.len(), 1);
        assert_eq!(scanned.exceptions[0].line, 1);
        assert_eq!(
            named(text, Scope::Whole),
            vec![(1, "marker")],
            "a test's strings are not read for one, so its marker covers nothing"
        );
    }

    #[test]
    fn a_scripts_sleep_is_read_off_the_strings_and_never_off_a_comment() {
        let text = "\
fn wake() -> &'static str {
    \"Start-Sleep -Seconds 20\"
}
let pace = SAMPLE_MS; // the pace is the script's Start-Sleep, not this
let script = build(
    // a note beside the argument, naming Start-Sleep
    mode,
);
";
        assert_eq!(
            named(text, Scope::Tool),
            vec![(2, "sleep")],
            "a statement that is nothing but the script is one, and a comment is no sleep"
        );
    }

    #[test]
    fn a_closing_brace_on_its_own_line_is_no_statement() {
        let code = "fn f() {\n    g();\n}\n";
        let found = statements(code, code);
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].text.trim(), "g()");
    }

    #[test]
    fn a_statement_knows_the_lines_it_spans() {
        let code = "let a =\n  f();\n\nlet b = 1;";
        let found: Vec<Statement> = statements(code, code);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].first, found[0].last), (1, 2));
        assert_eq!((found[1].first, found[1].last), (4, 4));
    }
}
