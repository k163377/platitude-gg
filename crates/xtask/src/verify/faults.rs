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
//!
//! **And the parent's own look at a stopped run** (`super::wedge::look_at`),
//! which is a process too and can stall like any other: the fourth case
//! orders that stall (`--fault-stall-look`) and reads back that the look
//! was ended at its ceiling, said so, and that the app was still reaped
//! and the run still reported after it.

use std::time::Instant;

/// A verb cheap enough to be run three times for something that is not
/// about the verb. What it shows does not matter here — every case below
/// is about where the process stopped, and one that never completed its
/// act reaches the same stations by its own watchdog.
const VERB: &str = "band";

/// The ceiling the held run with its deadline thread up is given. Above
/// what the verb costs on a loaded machine, so the act completes and the
/// hold is past it; and small, because that case is paid in wall clock —
/// this plus the ten seconds the thread waits past it before it looks
/// (`harness::deadline`). The one race left here: a verb that outruns
/// this on a busier machine reads as an aborted act.
const HELD_MS: &str = "4000";

/// The ceiling the two held runs with no deadline thread are given. **A
/// backstop, not what they cost**: the parent ends those on the trail's
/// word, the moment the station they were ordered to hold at is on the
/// disk (`super::child::ordered_hold`), so this is only what a run that
/// never gets there pays — and wide enough that no loaded machine reaches
/// it with the act still under way.
const BACKSTOP_MS: &str = "60000";

/// The ceiling the turning run is given. Long enough that the verb runs
/// and the loop goes on turning after it — **the ceiling is not what
/// withholds the completion here**, the fault is (`--fault-hold-act`).
///
/// It used to be one millisecond, on the reading that no verb could
/// complete inside a QML timer's own floor. A verb can: the ceiling is
/// armed when the run begins and the act completed in the loop turn
/// before that timer's first tick, so the case that forbids `complete=`
/// went red on a product that was working (2026-09-12, one gate running
/// eight jobs; the same tree answered in a second on its own). A
/// judgement that is a race between a ceiling and a loop is one the
/// machine decides.
const TURNING_MS: &str = "2000";

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
    /// A watchdog abort also reaches `exiting`, but is not an act that
    /// completed before stopping. Do not accept it as the held case.
    forbids: &'static [&'static str],
}

impl Case {
    fn missing(&self, said: &str) -> Vec<String> {
        self.wants
            .iter()
            .filter(|want| !said.contains(**want))
            .map(|want| format!("missing `{want}`"))
            .chain(
                self.forbids
                    .iter()
                    .filter(|word| said.contains(**word))
                    .map(|word| format!("unexpected `{word}`")),
            )
            .collect()
    }
}

/// The two mouths, the one that is only half of a mouth — a hold with
/// the deadline thread still up, which is what says the two records agree
/// when both can be written — and the parent's own look, stalled.
const CASES: &[Case] = &[
    Case {
        shape: "finished its act, then held past the exit with no deadline thread — \
                the observed shape, where nothing inside the process can report",
        args: &[
            "--watchdog-ms",
            BACKSTOP_MS,
            "--fault-hang",
            "exiting",
            "--fault-no-deadline",
        ],
        wants: &[
            "auto_act complete=band",
            "screenshot saved=true",
            "HELD AT exiting",
            "the station it was ordered to hold at",
            "the stations it reached:",
            "> exiting ",
            "it got as far as `exiting`",
            "left no wedge.txt",
            // The listing answered, not only the heading it is under:
            // this is the shape the next occurrence is read from.
            "threads at the ceiling:",
            " alive — ",
        ],
        forbids: &["auto-act watchdog expired"],
    },
    Case {
        shape: "held past the exit with its deadline thread up — both records, agreeing",
        args: &["--watchdog-ms", HELD_MS, "--fault-hang", "exiting"],
        wants: &[
            "auto_act complete=band",
            "screenshot saved=true",
            "the stations it reached:",
            "> exiting ",
            "it got as far as `exiting`",
            "the app's own account: wedged in `exiting`",
        ],
        forbids: &["auto-act watchdog expired"],
    },
    Case {
        shape: "the loop turning and the verb's completion never arriving",
        args: &["--watchdog-ms", TURNING_MS, "--fault-hold-act"],
        wants: &[
            "auto-act watchdog expired",
            "the loop answered its watchdog",
            "the stations it reached:",
            "event-loop ",
        ],
        forbids: &["auto_act complete=band"],
    },
    Case {
        shape: "held past the exit with the look at it stalled — the listing is ended at its own \
                ceiling, and the app is still reaped and the run reported",
        args: &[
            "--watchdog-ms",
            BACKSTOP_MS,
            "--fault-hang",
            "exiting",
            "--fault-no-deadline",
            "--fault-stall-look",
        ],
        wants: &[
            "auto_act complete=band",
            "screenshot saved=true",
            "HELD AT exiting",
            "the station it was ordered to hold at",
            "the stations it reached:",
            "> exiting ",
            "threads at the ceiling: could not be listed",
            "the listing ran out of time after",
            "and was ended",
            "under the app:",
            "pictures on disk:",
        ],
        // A listing that came back is a stall that did not happen.
        forbids: &["auto-act watchdog expired", " alive — "],
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
        let absent = case.missing(&said);
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
/// Every case fails on purpose. Both the failure exit and the record
/// must come back, so a runner that swallows the failure cannot pass.
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
    if out.status.success() {
        return Err(format!(
            "the injected fault passed verify-ui: {}",
            case.shape
        ));
    }
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

#[cfg(test)]
mod tests {
    use super::{BACKSTOP_MS, CASES, HELD_MS, TURNING_MS, VERB};

    /// The two runs with no deadline thread are ended on the trail's word
    /// and never at their ceiling, so the ceiling they carry is the
    /// backstop and the words they want are a hold's, not a timeout's.
    /// The one whose thread is up keeps the ceiling it is paid in.
    #[test]
    fn the_runs_with_no_deadline_are_read_as_held_and_carry_only_a_backstop() {
        let mut seen = 0;
        for case in CASES
            .iter()
            .filter(|case| case.args.contains(&"--fault-no-deadline"))
        {
            seen += 1;
            assert!(case.args.contains(&BACKSTOP_MS), "{}", case.shape);
            assert!(!case.args.contains(&HELD_MS), "{}", case.shape);
            assert!(case.wants.contains(&"HELD AT exiting"), "{}", case.shape);
            assert!(!case.wants.contains(&"TIMED OUT"), "{}", case.shape);
        }
        assert_eq!(seen, 2);
        let own = CASES
            .iter()
            .find(|case| {
                case.wants
                    .contains(&"the app's own account: wedged in `exiting`")
            })
            .expect("the case with the thread up");
        assert!(own.args.contains(&HELD_MS));
        assert!(!own.args.contains(&"--fault-no-deadline"));
    }

    /// The listing that answers is asserted to have answered: the observed
    /// shape is read from it, and a case that wanted only the heading
    /// would pass on a listing that never came back.
    #[test]
    fn the_observed_shape_reads_a_listing_that_answered() {
        let observed = CASES
            .iter()
            .find(|case| case.wants.contains(&"left no wedge.txt"))
            .expect("the observed shape");
        assert!(observed.wants.contains(&" alive — "));
    }

    #[test]
    fn an_aborted_act_is_not_a_completed_act_held_at_exit() {
        for case in &CASES[..2] {
            let said = case.wants.join("\n");
            assert!(case.missing(&said).is_empty());
            assert!(
                !case
                    .missing(&format!("{said}\nauto-act watchdog expired"))
                    .is_empty()
            );
            let unfinished = said.replace("auto_act complete=band", "");
            assert!(!case.missing(&unfinished).is_empty());
        }
    }

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
    /// time. **What withholds the completion is the fault that swallows
    /// it**, never a ceiling short enough to outrun the act — that is a
    /// race, and the machine decides it (`TURNING_MS`). A hold is no way
    /// to make this shape either: it stops the very loop the case is
    /// about, and a run with no deadline leaves nothing to read.
    #[test]
    fn one_case_is_the_loop_that_kept_turning() {
        let turning = CASES
            .iter()
            .find(|case| case.wants.contains(&"the loop answered its watchdog"))
            .expect("the other mouth");
        assert!(turning.args.contains(&TURNING_MS));
        assert!(
            turning.args.contains(&"--fault-hold-act"),
            "the completion is withheld on purpose rather than outrun"
        );
        assert!(
            !turning.args.contains(&"--fault-hang")
                && !turning.args.contains(&"--fault-no-deadline"),
            "a held run cannot turn its loop, and one with no deadline leaves nothing to read"
        );
    }

    /// The parent's own diagnostics are bounded, and the bound has to be
    /// seen working from outside: a look that stalls is ended at its
    /// ceiling and said to have been, and the run still reaches the
    /// reaping and the verdict. Ordered on the observed shape itself, so
    /// that the look is the only thing that differs from it.
    #[test]
    fn one_case_stalls_the_look_and_still_reaps_the_app() {
        let stalled = CASES
            .iter()
            .find(|case| case.args.contains(&"--fault-stall-look"))
            .expect("the stalled look");
        assert!(stalled.args.contains(&"--fault-hang"));
        assert!(stalled.args.contains(&"--fault-no-deadline"));
        for want in [
            "could not be listed",
            "ran out of time",
            "was ended",
            "under the app:",
            "HELD AT",
        ] {
            assert!(
                stalled.wants.iter().any(|said| said.contains(want)),
                "{want}"
            );
        }
        assert!(stalled.forbids.contains(&" alive — "));
    }

    /// The verb is beside the point and has to stay cheap: a case is
    /// about where the process stopped.
    #[test]
    fn the_verb_carries_no_argument() {
        assert!(!VERB.contains(' '), "{VERB}");
    }
}
