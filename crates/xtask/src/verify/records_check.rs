//! A write's records held back from the command log, and what its
//! completion waits for (`cargo xtask records-check`).
//!
//! A write's answer reaches the tab and its commands reach the command
//! log on two feeds, each drained by its own consumer (`hub::Feeds`).
//! Which one drained first cannot be told from a run that passed: the
//! order is the event loop's. So the order the completion has to survive
//! is made the run's own. The log's records are held back from the
//! write's first asked command on (`PGG_FAULT_HOLD_RECORDS`,
//! `harness::held_records`), and let through only once a wait stands on
//! them alone: a sampler held by the page, and the page owing nothing but
//! the records (`WindowHarness`). A completion that never waited for them
//! lets them in at its walk, and its picture gains what they build.
//!
//! Each case must pass, say the release came while held and before the
//! completion, complete on the write's records in the log, and — where
//! the write raises the log — on its rows built.

use std::path::Path;
use std::time::Instant;

/// One write, and what it leaves in the log.
struct Case {
    /// The registered line, run with the hold.
    line: &'static str,
    /// Whether the write's answer raises the log, so its rows stand in
    /// the completion's walk.
    raises_log: bool,
    /// Whether the write runs more than one command.
    several: bool,
}

/// A write that lands, one refused into a report, and one refused in
/// git's own words after several commands: the success, refusal and
/// multi-command paths under the one completion.
const CASES: &[Case] = &[
    Case {
        line: "push",
        raises_log: false,
        several: false,
    },
    Case {
        line: "push --preset remote-lock --allow-write-failure",
        raises_log: false,
        several: false,
    },
    Case {
        line: "push-tag v1 --preset tag-hidden --allow-write-failure",
        raises_log: true,
        several: true,
    },
];

/// The registered lines the check runs, each held to its row's promise and
/// its judgment as a verb's run is (`gate::plan::steps::records_steps`).
pub(crate) fn lines() -> impl Iterator<Item = &'static str> {
    CASES.iter().map(|case| case.line)
}

/// The release's line (`WindowHarness.letRecordsThrough`).
const RELEASED: &str = "fault_records released=";
/// The completion's line (`Awaited.completed`).
const COMPLETE: &str = " complete run=";
/// The log's account of the write at the completion (`WindowHarness.finish`).
const RECORDS: &str = "fault_records op=";
/// The row a raised log builds (`CommandsPane`).
const ROW: &str = "CommandRowDelegate";

impl Case {
    /// What the run should have said and did not, or said wrong.
    fn wrong(&self, passed: bool, said: &str) -> Vec<String> {
        let mut out = Vec::new();
        if !passed {
            out.push("verify-ui failed".to_string());
        }
        let lines: Vec<&str> = said.lines().collect();
        let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
        match at(RELEASED) {
            None => out.push(format!("no `{RELEASED}` line: the hold never let go")),
            Some(release) => {
                let line = lines[release];
                if !line.contains("released=held ") {
                    out.push(format!(
                        "released other than while held — nothing waited on the records: {}",
                        line.trim()
                    ));
                }
                if number_after(line, " held=").is_none_or(|held| held < 2) {
                    out.push(format!(
                        "the hold took no command's start and end: {}",
                        line.trim()
                    ));
                }
                if at(COMPLETE).is_some_and(|complete| complete < release) {
                    out.push("the completion came before the release".to_string());
                }
            }
        }
        match lines.iter().find(|line| line.contains(RECORDS)) {
            None => out.push(format!("no `{RECORDS}` line at the completion")),
            Some(line) => {
                let rows = number_after(line, " rows=").unwrap_or(0);
                if !line.contains(" recorded=true") || rows < if self.several { 2 } else { 1 } {
                    out.push(format!(
                        "the completion stood on the write's records short: {}",
                        line.trim()
                    ));
                }
            }
        }
        let built = lines
            .iter()
            .any(|line| line.contains("census=") && line.contains(ROW));
        if built != self.raises_log {
            out.push(if self.raises_log {
                format!("the log came up without its rows: no {ROW} in the census")
            } else {
                format!("{ROW} in the census of a write that raises no log")
            });
        }
        out
    }
}

/// The number written after `key` on `line`.
fn number_after(line: &str, key: &str) -> Option<i64> {
    line.split(key)
        .nth(1)?
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .next()?
        .parse()
        .ok()
}

/// Every case must pass verify-ui with the records held and read back as
/// the hold's lines and the census say.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let reuse = super::shot_check::reuse_the_build(args, "records-check")?;
    let me = std::env::current_exe().map_err(|e| format!("could not find this binary: {e}"))?;
    let mut failed = Vec::new();
    for (nth, case) in CASES.iter().enumerate() {
        println!("records-check: {}", case.line);
        // waits(measured): what the case cost, for the line under it — the verdict is the run's lines
        let began = Instant::now();
        // Only the first case builds the app, and none when the release stands.
        let (passed, said) = drive(&me, case, nth == 0 && !reuse)?;
        let wrong = case.wrong(passed, &said);
        println!(
            "  {} in {:.1}s",
            if wrong.is_empty() {
                "waited for its records"
            } else {
                "did NOT wait for its records"
            },
            began.elapsed().as_secs_f32()
        );
        if !wrong.is_empty() {
            for line in said.lines() {
                println!("  > {line}");
            }
            failed.push(format!("{}: {wrong:?}", case.line));
        }
    }
    if failed.is_empty() {
        println!("records-check: every completion waited for its write's records in the log");
        return Ok(());
    }
    Err(format!(
        "a completion did not wait for its write's records:\n  {}",
        failed.join("\n  ")
    ))
}

/// Runs one case and answers whether it passed, with both streams
/// together.
fn drive(me: &Path, case: &Case, build: bool) -> Result<(bool, String), String> {
    let mut command = std::process::Command::new(me);
    command
        .arg("verify-ui")
        .args(super::suite_words(case.line))
        .arg("--fault-hold-records");
    // Nobody asked to look at a run made to hold its records.
    command.arg("--no-board");
    if !build {
        command.arg("--no-build");
    }
    let out = crate::subprocess::run_captured(&mut command)?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok((out.status.success(), said))
}

#[cfg(test)]
mod tests {
    use super::{CASES, Case, number_after};

    fn several() -> &'static Case {
        CASES
            .iter()
            .find(|case| case.several)
            .expect("a case of several commands")
    }

    /// The three paths are each a case: a write that lands, one refused,
    /// and one of several commands that raises the log.
    #[test]
    fn the_cases_cover_success_refusal_and_several_commands() {
        assert!(
            CASES
                .iter()
                .any(|case| !case.line.contains("--allow-write-failure"))
        );
        assert!(
            CASES
                .iter()
                .any(|case| case.line.contains("--allow-write-failure"))
        );
        assert!(several().raises_log);
        for case in CASES {
            let registered =
                std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/verb-lines.txt"))
                    .expect("the registry");
            assert!(
                registered
                    .lines()
                    .any(|row| row.split('\t').next() == Some(case.line)),
                "{} is no registered line",
                case.line
            );
        }
    }

    /// A run that waited for its records says the release while held,
    /// ahead of its completion, and completes on them; one that completed
    /// first is wrong however it ended.
    #[test]
    fn a_completion_that_waited_reads_back_and_one_that_did_not_is_named() {
        let case = several();
        let waited = "  | INFO bench: fault_records released=held name=write_settled held=12\n\
                      \x20 | INFO bench: complete run=r verb=push-tag owner=page op=write_barrier gen=1\n\
                      \x20 | INFO bench: fault_records op=2 recorded=true rows=3\n\
                      \x20 | INFO bench: census=CommandRowDelegate,CommandsPane\n";
        assert!(
            case.wrong(true, waited).is_empty(),
            "{:?}",
            case.wrong(true, waited)
        );
        let first = "  | INFO bench: complete run=r verb=push-tag owner=page op=write_barrier gen=1\n\
                     \x20 | INFO bench: fault_records op=2 recorded=false rows=0\n\
                     \x20 | INFO bench: fault_records released=after-completion held=12\n\
                     \x20 | INFO bench: census=CommandRowDelegate,CommandsPane\n";
        let wrong = case.wrong(false, first);
        assert_eq!(wrong.len(), 4, "{wrong:?}");
        assert_eq!(wrong[0], "verify-ui failed");
        assert!(
            wrong[1].starts_with("released other than while held"),
            "{wrong:?}"
        );
        assert_eq!(wrong[2], "the completion came before the release");
        assert!(wrong[3].starts_with("the completion stood on the write's records short"));
        let unraised = "  | INFO bench: fault_records released=held name=completion held=4\n\
                        \x20 | INFO bench: complete run=r\n\
                        \x20 | INFO bench: fault_records op=2 recorded=true rows=3\n\
                        \x20 | INFO bench: census=CommandsPane\n";
        assert_eq!(
            case.wrong(true, unraised),
            vec!["the log came up without its rows: no CommandRowDelegate in the census"]
        );
        assert_eq!(
            case.wrong(true, "nothing"),
            vec![
                "no `fault_records released=` line: the hold never let go",
                "no `fault_records op=` line at the completion",
                "the log came up without its rows: no CommandRowDelegate in the census",
            ]
        );
    }

    #[test]
    fn a_number_is_read_off_its_key() {
        assert_eq!(number_after("a held=12 b", " held="), Some(12));
        assert_eq!(number_after("a held=-1", " held="), Some(-1));
        assert_eq!(number_after("a rows=3\r", " rows="), Some(3));
        assert_eq!(number_after("a", " held="), None);
    }
}
