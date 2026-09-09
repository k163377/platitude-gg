//! Runs made to stop, and what the parent could say about them
//! afterwards.
//!
//! The record a wedged run leaves ([`super::wedge`]) is only worth what
//! it says when a run actually stops, and a run that actually stops is
//! the one case nobody can arrange by waiting for it: the three reds it
//! was built for happened once each, under a gate, and every one of them
//! passed on the spot when it was run again by hand
//! (internal-docs/P3-確認事項.md §check ハング調査で残った観察). So the
//! stopping is asked for here instead — the app is held at a station it
//! names (`PGG_FAULT_HANG`), or started with no deadline thread at all
//! (`PGG_FAULT_NO_DEADLINE`) — and the check is that the words the next
//! occurrence will be read from come back.
//!
//! **Both mouths, because they are two different failures.** A run that
//! finished its act and would not end is stopped past the event loop and
//! answers with a station; a run that never reached its verb's completion
//! turned its loop the whole time and answers with the walk that proves
//! it. The observed pair had no way to be told apart, which is what this
//! is for.

use std::time::Instant;

/// A verb cheap enough to be run three times for something that is not
/// about the verb. What it shows does not matter here — every case below
/// is about where the process stopped, and one that never completed its
/// act reaches the same stations by its own watchdog.
const VERB: &str = "band";

/// The ceiling the two held runs are given. Above what the verb costs on
/// a loaded machine, so the act completes and the hold is past it; and
/// small, because a held run is paid in wall clock — this one plus the
/// parent's grace for the reaped case, plus ten seconds for the case that
/// ends itself (`super::child`, `harness::deadline`).
const HELD_MS: &str = "4000";

/// The ceiling the turning run is given. One millisecond, which is the
/// QML timer's own floor (`AutoShotDriver`): no verb completes inside it,
/// so the watchdog fires from a loop that is certainly turning — the red
/// this reproduces, arranged rather than waited for.
const TURNING_MS: &str = "1";

/// One run made to stop, and the words it has to come back with.
struct Case {
    /// What shape of stop this is, for the line the check prints.
    shape: &'static str,
    /// The words after `verify-ui`, less the ones every case carries.
    args: &'static [&'static str],
    /// Every one of these must appear in what the run printed. Quoted
    /// from [`super::wedge`] and [`super::outcome`] rather than shared
    /// with them: what is being checked is the sentence a person reads,
    /// so a rewording that leaves the reading behind is a red here.
    wants: &'static [&'static str],
}

/// The two mouths, and the one that is only half of a mouth: a hold with
/// the deadline thread still up, which is what says the two records agree
/// when both can be written.
const CASES: &[Case] = &[
    Case {
        shape: "finished its act, then held past the exit with no deadline thread — \
                the observed shape, where nothing inside the process can report",
        args: &[
            "--watchdog-ms",
            HELD_MS,
            "--fault-hang",
            "exiting",
            "--fault-no-deadline",
        ],
        wants: &[
            "TIMED OUT",
            "the stations it reached:",
            "> exiting ",
            "it got as far as `exiting`",
            "left no wedge.txt",
        ],
    },
    Case {
        shape: "held past the exit with its deadline thread up — both records, agreeing",
        args: &["--watchdog-ms", HELD_MS, "--fault-hang", "exiting"],
        wants: &[
            "the stations it reached:",
            "> exiting ",
            "it got as far as `exiting`",
            "the app's own account: wedged in `exiting`",
        ],
    },
    Case {
        shape: "the loop turning and the verb's completion never arriving",
        args: &["--watchdog-ms", TURNING_MS],
        wants: &[
            "auto-act watchdog expired",
            "the loop was turning the whole time",
            "the stations it reached:",
            "event-loop ",
        ],
    },
];

/// Runs every case and answers for the record, not for the runs: each one
/// is *meant* to fail, and what is being judged is whether the lines that
/// say why came back.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("wedge-check takes no arguments, and got {unknown}"));
    }
    let me = std::env::current_exe().map_err(|e| format!("could not find this binary: {e}"))?;
    let mut missed = Vec::new();
    for (nth, case) in CASES.iter().enumerate() {
        println!("wedge-check: {}", case.shape);
        // waits(measured): what the case cost, for the line under it — the verdict is the words
        let began = Instant::now();
        // The first case builds the app; the rest are handed what it
        // built, the way a suite's verbs are (`gate::plan`).
        let said = drive(&me, case, nth == 0)?;
        let absent: Vec<&str> = case
            .wants
            .iter()
            .copied()
            .filter(|want| !said.contains(want))
            .collect();
        println!(
            "  {} in {:.1}s",
            if absent.is_empty() {
                "read back"
            } else {
                "NOT read back"
            },
            began.elapsed().as_secs_f32()
        );
        if !absent.is_empty() {
            // The whole run, once, under the case that failed: what the
            // parent did say is the only way to see what it said instead.
            for line in said.lines() {
                println!("  > {line}");
            }
            missed.push(format!("{}: never said {absent:?}", case.shape));
        }
    }
    if missed.is_empty() {
        println!("wedge-check: a run that stops is readable from out here");
        return Ok(());
    }
    Err(format!(
        "the record a wedged run leaves did not come back:\n  {}",
        missed.join("\n  ")
    ))
}

/// Runs one case and answers with everything it printed, both streams
/// together.
///
/// **The exit code is not read.** Every case here is a run that fails on
/// purpose, and a case that came back green would mean the fault never
/// took — which the missing words say better than a number.
fn drive(me: &std::path::Path, case: &Case, build: bool) -> Result<String, String> {
    let mut command = std::process::Command::new(me);
    command.arg("verify-ui").arg(VERB).args(case.args);
    // Off the board and out of the census: nobody asked to look at a run
    // that was made to stop, and it shows nothing to record.
    command.arg("--no-board").arg("--no-census");
    if !build {
        command.arg("--no-build");
    }
    let out = crate::subprocess::run_captured(&mut command)?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

#[cfg(test)]
mod tests {
    use super::{CASES, TURNING_MS, VERB};

    /// Every case is a run that stops in a way the parent has to be able
    /// to read, so each has to ask for something. A case with nothing to
    /// want passes without checking anything.
    #[test]
    fn every_case_asks_for_words_back() {
        assert!(!CASES.is_empty());
        for case in CASES {
            assert!(!case.wants.is_empty(), "{}", case.shape);
            assert!(!case.shape.is_empty());
        }
    }

    /// The trail is what every case is really about: the record that does
    /// not need the process to still be answering. A case that only read
    /// the app's own report would pass on the half that was already there.
    #[test]
    fn every_case_reads_the_trail() {
        for case in CASES {
            assert!(
                case.wants.contains(&"the stations it reached:"),
                "{}",
                case.shape
            );
        }
    }

    /// One case has to be the observed shape itself: held with no report
    /// of its own, which is the run the parent used to have nothing to
    /// say about.
    #[test]
    fn one_case_leaves_no_report_of_its_own() {
        assert!(
            CASES.iter().any(|case| {
                case.args.contains(&"--fault-no-deadline")
                    && case.wants.contains(&"left no wedge.txt")
            }),
            "the shape the record was built for is the one that must be driven"
        );
    }

    /// And one has to be the other mouth: a loop that turned the whole
    /// time, which no hold can produce — the ceiling is what makes it.
    #[test]
    fn one_case_is_the_loop_that_kept_turning() {
        let turning = CASES
            .iter()
            .find(|case| case.wants.contains(&"the loop was turning the whole time"))
            .expect("the other mouth");
        assert!(turning.args.contains(&TURNING_MS));
        assert!(
            !turning.args.iter().any(|arg| arg.starts_with("--fault")),
            "a held run cannot turn its loop"
        );
    }

    /// The verb is beside the point and has to stay cheap: a case is
    /// about where the process stopped.
    #[test]
    fn the_verb_carries_no_argument() {
        assert!(!VERB.contains(' '), "{VERB}");
    }
}
