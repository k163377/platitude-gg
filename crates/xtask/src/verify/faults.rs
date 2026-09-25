//! Runs made to stop, and what the parent could say about them
//! afterwards.
//!
//! The record a wedged run leaves ([`super::wedge`]) is only worth what
//! it says when a run actually stops, and a real stop cannot be arranged
//! by waiting for one (internal-docs/ハング調査.md). So the stopping is
//! asked for — the app is held at a station it names (`PGG_FAULT_HANG`),
//! or started with no deadline thread (`PGG_FAULT_NO_DEADLINE`) — and the
//! check is that the words the next occurrence will be read from come
//! back.
//!
//! Both mouths, because they are two different failures: a run that
//! finished its act and would not end answers with a station; a run that
//! never reached its verb's completion turned its loop the whole time and
//! answers with the walk that proves it. The fourth case stalls the
//! parent's own look (`super::look::look_at`), which is a process too, and
//! reads back that it was ended at its ceiling and the app still reaped.

use std::time::Instant;

/// A cheap verb: every case is about where the process stopped, not what
/// the verb shows.
const VERB: &str = "band";

/// The ceiling the three held runs are given, and only a backstop: the
/// parent ends the two with no deadline thread the moment their station
/// is on the disk (`super::child::ordered_hold`), and the third's thread
/// looks the grace past that moment (`harness::deadline::first_look`).
/// Wide enough that no loaded machine reaches it with the act under way.
const BACKSTOP_MS: &str = "60000";

/// The ceiling the turning run is given: long enough that the verb runs
/// and the loop goes on turning after it. The fault withholds the
/// completion (`--fault-hold-act`); a ceiling short enough to beat the
/// act instead is a race the machine decides — the act can complete
/// before a QML timer's first tick.
const TURNING_MS: &str = "2000";

/// One run made to stop, and the words it has to come back with.
struct Case {
    /// What shape of stop this is, for the line the check prints.
    shape: &'static str,
    /// The words after `verify-ui`, less the ones every case carries.
    args: &'static [&'static str],
    /// Must all appear in what the run printed: the sentences a person
    /// reads, quoted from [`super::wedge`] and [`super::outcome`], so
    /// rewording them there is a red here.
    wants: &'static [&'static str],
    /// The same, held to on Windows alone: only the listing there walks
    /// stacks (`super::look::threads_of`), and the gate's host side runs
    /// wherever the tree is checked out.
    wants_on_windows: &'static [&'static str],
    /// A watchdog abort also reaches `exiting`, so these words tell it
    /// from an act that completed before stopping.
    forbids: &'static [&'static str],
}

impl Case {
    fn missing(&self, said: &str) -> Vec<String> {
        let here: &[&str] = if cfg!(windows) {
            self.wants_on_windows
        } else {
            &[]
        };
        self.wants
            .iter()
            .chain(here.iter())
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

/// The two mouths, a hold with the deadline thread up (the two records
/// agree when both can be written), and the parent's own look, stalled.
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
            // Past the heading: the listing answered.
            "threads at the ceiling:",
            " alive — ",
        ],
        // The one line a stop inside the exit is read off.
        wants_on_windows: &["the main thread stands in:"],
        forbids: &["auto-act watchdog expired"],
    },
    Case {
        shape: "held past the exit with its deadline thread up — both records, agreeing",
        args: &["--watchdog-ms", BACKSTOP_MS, "--fault-hang", "exiting"],
        wants: &[
            "auto_act complete=band",
            "screenshot saved=true",
            "the stations it reached:",
            "> exiting ",
            "it got as far as `exiting`",
            "the app's own account: wedged in `exiting`",
        ],
        wants_on_windows: &[],
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
        wants_on_windows: &[],
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
        wants_on_windows: &[],
        // A listing that came back is a stall that did not happen.
        forbids: &["auto-act watchdog expired", " alive — "],
    },
];

/// Each case is meant to fail; what is judged is whether the lines that
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
        // Only the first case builds the app.
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
            // The whole run under the case that failed: what it said instead.
            for line in said.lines() {
                println!("  > {line}");
            }
            missed.push(format!("{}: missing {absent:?}", case.shape));
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

/// Runs one case and answers with both streams, together. Every case
/// fails on purpose, so a runner that swallows the failure exit is a red.
fn drive(me: &std::path::Path, case: &Case, build: bool) -> Result<String, String> {
    let mut command = std::process::Command::new(me);
    command.arg("verify-ui").arg(VERB).args(case.args);
    // Nobody asked to look at a run made to stop, and it shows nothing to
    // record.
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
    use super::{BACKSTOP_MS, CASES, TURNING_MS, VERB};

    /// Every held run carries only the backstop, and the two with no
    /// deadline thread want a hold's words: a ceiling counted from the
    /// start of the run is a race a loaded machine loses
    /// (rules-refs/core.md「天井の起点を因果の駅に置く」).
    #[test]
    fn every_held_run_carries_only_a_backstop() {
        let mut seen = 0;
        for case in CASES
            .iter()
            .filter(|case| case.args.contains(&"--fault-no-deadline"))
        {
            seen += 1;
            assert!(case.args.contains(&BACKSTOP_MS), "{}", case.shape);
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
        assert!(own.args.contains(&BACKSTOP_MS));
        assert!(!own.args.contains(&"--fault-no-deadline"));
    }

    /// A listing that never came back prints the heading too, so the
    /// observed shape wants the listing's own lines.
    #[test]
    fn the_observed_shape_reads_a_listing_that_answered() {
        let observed = CASES
            .iter()
            .find(|case| case.wants.contains(&"left no wedge.txt"))
            .expect("the observed shape");
        assert!(observed.wants.contains(&" alive — "));
        assert!(
            observed
                .wants_on_windows
                .contains(&"the main thread stands in:")
        );
        assert!(
            !CASES
                .iter()
                .any(|case| case.wants.contains(&"the main thread stands in:")),
            "the stack line is a Windows want alone"
        );
        let said = observed.wants.join("\n");
        let missing = observed.missing(&said);
        assert_eq!(
            missing.is_empty(),
            !cfg!(windows),
            "off Windows the wants alone satisfy the case; on it the stack is owed too: {missing:?}"
        );
    }

    #[test]
    fn an_aborted_act_is_not_a_completed_act_held_at_exit() {
        for case in &CASES[..2] {
            // What a run that read back whole would have said.
            let said = case
                .wants
                .iter()
                .chain(case.wants_on_windows.iter())
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
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

    /// A case with nothing to want passes without checking anything.
    #[test]
    fn every_case_asks_for_words_back() {
        assert!(!CASES.is_empty());
        for case in CASES {
            assert!(!case.wants.is_empty(), "{}", case.shape);
            assert!(!case.shape.is_empty());
        }
    }

    /// The trail is the record that does not need the process to still be
    /// answering; a case that read only the app's own report would pass on
    /// the half that was already there.
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

    /// The observed shape itself: the run the parent has to speak for alone.
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

    /// The other mouth: the fault withholds the completion, not a short
    /// ceiling (`TURNING_MS`).
    #[test]
    fn one_case_is_the_loop_that_kept_turning() {
        let turning = CASES
            .iter()
            .find(|case| case.wants.contains(&"the loop answered its watchdog"))
            .expect("the other mouth");
        assert!(turning.args.contains(&TURNING_MS));
        assert!(
            turning.args.contains(&"--fault-hold-act"),
            "the completion is withheld on purpose"
        );
        assert!(
            !turning.args.contains(&"--fault-hang")
                && !turning.args.contains(&"--fault-no-deadline"),
            "a held run cannot turn its loop, and one with no deadline leaves nothing to read"
        );
    }

    /// The bound on the parent's own look, seen working from outside.
    /// Ordered on the observed shape itself, so that the look is the only
    /// thing that differs from it.
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

    #[test]
    fn the_verb_carries_no_argument() {
        assert!(!VERB.contains(' '), "{VERB}");
    }
}
