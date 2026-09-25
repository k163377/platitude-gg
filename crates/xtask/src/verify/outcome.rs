//! What the run is judged on, how the judgement is made off its own
//! lines, and how it is said.

use std::path::PathBuf;

/// What the run is judged on. A refused write counts against it — a
/// picture of the state it never reached proves nothing — unless the verb
/// walks a refusal (`--allow-write-failure`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Outcome {
    pub(super) exit_ok: bool,
    pub(super) saved: bool,
    pub(super) timed_out: bool,
    /// Whether the app's own watchdog ended the run: a red, though the app
    /// quits cleanly and may already have saved one shot of a pair.
    pub(super) watchdog_expired: bool,
    pub(super) write_failures: usize,
    pub(super) allow_write_failure: bool,
    /// Whether the run said its write barrier's contract broke
    /// (`AutoActDriver.sayBroken`): a red `--allow-write-failure` does not
    /// reach (rules-refs/app-ui.md「違反は取り消せない」).
    pub(super) contract_broken: bool,
    /// Whether the app was refused the settings it was handed, for a verb
    /// that did not stage that itself ([`super::child`]). A run's config
    /// directory is its own, so this is two runs given one — judged the
    /// moment it is said, not at the ceiling.
    pub(super) store_refused: bool,
    /// What a verb whose failure the camera cannot see has to be caught
    /// saying (`super::verbs::must_say`).
    pub(super) must_say: Option<&'static str>,
    pub(super) said: bool,
    /// Whether the identity a held save wrote is in the run's gitconfig
    /// once the app has ended — the witness outside the app that the exit
    /// waited for the save (`quit-save-held`,
    /// `super::shim::held_save_landed`). `None` for every other verb.
    pub(super) held_save_landed: Option<bool>,
}

impl Outcome {
    pub(super) fn passed(self) -> bool {
        self.exit_ok
            && self.saved
            && !self.timed_out
            && !self.watchdog_expired
            && !self.write_sank_it()
            && !self.contract_broken
            && !self.store_refused
            && (self.must_say.is_none() || self.said)
            && self.held_save_landed.is_none_or(|landed| landed)
    }

    /// Whether the failing writes are what the verdict turns on — the one
    /// reason worth a line of its own, since the run looks well otherwise.
    pub(super) fn write_sank_it(self) -> bool {
        self.write_failures > 0 && !self.allow_write_failure
    }
}

/// What the run's own lines make of it — and, for the one verb that has
/// one, what the witness on disk says (`config` is the run's gitconfig).
pub(super) fn judge(
    opts: &super::options::Options,
    ran: &super::child::Ran,
    config: &std::path::Path,
) -> Outcome {
    let (status, err_lines, out_lines, timed_out) =
        (&ran.status, &ran.err_lines, &ran.out_lines, ran.timed_out);

    let must_say = super::verbs::must_say(&opts.verb, &opts.arg, &opts.preset);
    Outcome {
        exit_ok: status.as_ref().is_some_and(|s| s.success()),
        saved: err_lines
            .iter()
            .chain(out_lines.iter())
            .any(|l| l.contains("screenshot saved=true")),
        timed_out,
        watchdog_expired: err_lines
            .iter()
            .chain(out_lines.iter())
            .any(|l| l.contains("auto-act watchdog expired")),
        write_failures: err_lines
            .iter()
            .filter(|l| l.contains("write failed"))
            .count(),
        allow_write_failure: opts.allow_write_failure,
        contract_broken: err_lines
            .iter()
            .chain(out_lines.iter())
            .any(|l| l.contains("write_contract ")),
        // ASCII only, and so readable on both sides: a Windows Qt writes
        // its log lines in the local code page (verify-ui skill).
        store_refused: !super::child::stages_a_held_store(&opts.verb)
            && err_lines
                .iter()
                .chain(out_lines.iter())
                .any(|l| l.contains("another platitude-gg is using these settings")),
        must_say,
        said: must_say.is_none_or(|wanted| {
            err_lines
                .iter()
                .chain(out_lines.iter())
                .any(|l| l.contains(wanted))
        }),
        held_save_landed: super::shim::held_save_landed(&opts.verb, config),
    }
}

/// Which of the two reds this is: the one no ceiling ended. The app's
/// watchdog is a QML `Timer` (`auto/AutoShotDriver.qml`), so a run it
/// ended had its event loop turning when it fired — which says nothing
/// about earlier stalls. Exclusive with the ceiling's reading
/// (`super::wedge`), whose account is the fuller one and already carries
/// the machine's load.
pub(super) fn loop_was_turning(outcome: &Outcome, ran: &super::child::Ran) -> bool {
    outcome.watchdog_expired && !super::wedge::at_a_ceiling(ran)
}

/// The pictures the run left, named, and filed on the board whatever the
/// verdict — a failing run's picture is the one most worth looking at.
fn filed_shots(opts: &super::options::Options, shot_dir: &std::path::Path) -> Vec<PathBuf> {
    let mut shots: Vec<PathBuf> = std::fs::read_dir(shot_dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "png"))
                .collect()
        })
        .unwrap_or_default();
    shots.sort();
    for shot in &shots {
        println!("shot: {}", shot.display());
    }
    if !shots.is_empty() && !opts.no_board {
        let label = if opts.label.is_empty() {
            format!("{} {}", opts.verb, opts.arg).trim().to_string()
        } else {
            opts.label.clone()
        };
        match crate::shots::record(&label, &opts.verb, &shots) {
            Ok(page) => println!("board: {}", crate::shots::shown(&page)),
            // A board that could not be updated leaves the verdict alone.
            Err(message) => println!("board: not updated ({message})"),
        }
    }
    shots
}

/// Prints what the run said, files its pictures, and gives the verdict.
/// `census_unwritten` is why a run that passed did not write the census
/// line it owed (`run::tell_the_census`), which fails it.
pub(super) fn announce(
    opts: &super::options::Options,
    shot_dir: &std::path::Path,
    ran: &super::child::Ran,
    outcome: &Outcome,
    census_unwritten: Option<&str>,
) -> Result<(), String> {
    let passed = outcome.passed() && census_unwritten.is_none();
    let (status, err_lines, out_lines, timed_out, elapsed) = (
        &ran.status,
        &ran.err_lines,
        &ran.out_lines,
        ran.timed_out,
        ran.elapsed,
    );

    for line in err_lines.iter().chain(out_lines.iter()) {
        println!("  | {line}");
    }
    let shots = filed_shots(opts, shot_dir);

    println!(
        "{}: {} in {:.1}s (exit {}, screenshot saved={}, write-failures {}{})",
        if passed { "PASS" } else { "FAIL" },
        opts.verb,
        elapsed.as_secs_f32(),
        status.map_or_else(
            || "?".into(),
            |s| s.code().map_or("signal".into(), |c| c.to_string())
        ),
        outcome.saved,
        outcome.write_failures,
        // Two different ends, and the words `wedge-check` reads them by
        // (`super::faults`).
        match (&ran.held_at, timed_out) {
            (Some(station), _) => format!(", HELD AT {station}"),
            (None, true) => ", TIMED OUT".to_string(),
            (None, false) => String::new(),
        },
    );
    // A run the parent stopped waiting on says why itself, and the
    // ceiling's diagnostics are about a run whose reason is unknown.
    if let Some(why) = super::wedge::gave_up_early(ran) {
        println!("  {why}");
    }
    // A run ended by a ceiling says nothing for itself: the account is
    // the whole of what the next occurrence is read from (`super::wedge`).
    else if super::wedge::at_a_ceiling(ran) {
        for line in super::wedge::account(shot_dir, ran, &shots) {
            println!("{line}");
        }
    }
    if let Some(wanted) = outcome.must_say
        && !outcome.said
    {
        println!(
            "  the run never said `{wanted}` — and this verb's picture reads the same \
             whether it should have or not."
        );
    }
    // Said either way, so a reader knows what the pass read.
    match outcome.held_save_landed {
        Some(true) => println!(
            "  witness on disk: the identity the held save wrote is in the run's gitconfig \
             — the exit waited for the save"
        ),
        Some(false) => println!(
            "  witness on disk: the identity the held save wrote is NOT in the run's gitconfig \
             — the process ended without waiting for the save, or the save did not land"
        ),
        None => {}
    }
    if outcome.watchdog_expired {
        say_the_watchdog(shot_dir, ran, outcome);
    }
    if outcome.write_sank_it() {
        println!(
            "  git refused the write this verb asked for — the shot is of the state it \
             never reached. If the refusal is what the verb shows, say so with \
             --allow-write-failure."
        );
    }
    if outcome.contract_broken {
        println!(
            "  the harness broke its own write barrier's contract — the `write_contract` line \
             above says which press and what it was armed over. The run ended there, and its \
             picture is of whatever page it had got to; a red whatever the flags say, because \
             the run stopped being about the write it was pressed for."
        );
    }
    if outcome.store_refused {
        println!(
            "  the settings this run was handed were already held, so the window it \
             opened is the one that says so. Nothing else knows that directory, so \
             two runs were given one: on the container side, the host directory \
             mounted at /out."
        );
    }
    if let Some(why) = census_unwritten {
        println!(
            "  the run was judged well and the census line it owes was not written: {why}. \
             Its repositories and pictures are kept, and {} is as it was before the run.",
            crate::gate::CENSUS_FILE
        );
    }
    if passed {
        Ok(())
    } else {
        Err(format!("verify-ui {} failed", opts.verb))
    }
}

/// What a run the app's own watchdog ended is read by.
fn say_the_watchdog(shot_dir: &std::path::Path, ran: &super::child::Ran, outcome: &Outcome) {
    println!(
        "  the app's own watchdog ended this run — the act, screenshot, and census did \
         not all finish; a run stalled between grabs can still leave app.png behind."
    );
    if loop_was_turning(outcome, ran) {
        println!(
            "  the loop answered its watchdog — this does not rule out earlier stalls. {}",
            super::wedge::lanes_line()
        );
        // The trail includes the teardown after the watchdog asked to
        // quit: reaching `exiting` does not say which completion was missing.
        for line in super::wedge::trail(shot_dir) {
            println!("{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Outcome, loop_was_turning};

    /// A run that reached the end and took its picture.
    const WELL: Outcome = Outcome {
        exit_ok: true,
        saved: true,
        timed_out: false,
        watchdog_expired: false,
        write_failures: 0,
        allow_write_failure: false,
        contract_broken: false,
        store_refused: false,
        must_say: None,
        said: true,
        held_save_landed: None,
    };

    #[test]
    fn a_broken_contract_fails_the_run_whatever_else_it_did() {
        let broken = Outcome {
            contract_broken: true,
            ..WELL
        };
        assert!(WELL.passed(), "the same run otherwise");
        assert!(!broken.passed());
        assert!(
            !Outcome {
                allow_write_failure: true,
                ..broken
            }
            .passed(),
            "and the flag for a refused write does not reach it"
        );
    }

    /// A run that printed every line and left the gitconfig without the
    /// identity ended before its save wrote — the failure the verb is for.
    #[test]
    fn the_held_saves_witness_on_disk_decides_where_it_exists() {
        let waited = Outcome {
            held_save_landed: Some(true),
            ..WELL
        };
        assert!(waited.passed());
        let left = Outcome {
            held_save_landed: Some(false),
            ..WELL
        };
        assert!(!left.passed());
        assert!(WELL.passed(), "and every other verb has no such witness");
    }

    /// Waiting out the watchdog to discover it would cost the whole
    /// ceiling and say only that the verb never finished.
    #[test]
    fn a_run_refused_its_own_settings_fails_where_it_stands() {
        let refused = Outcome {
            store_refused: true,
            ..WELL
        };
        assert!(!refused.passed());
        assert!(
            !refused.watchdog_expired,
            "the point is that this is said before the watchdog gets there"
        );
    }

    #[test]
    fn a_verb_the_harness_stages_has_to_be_caught_saying_so() {
        let quiet = Outcome {
            must_say: Some("solo blocked=true"),
            said: false,
            ..WELL
        };
        assert!(!quiet.passed());
        assert!(
            !quiet.write_sank_it(),
            "nothing was refused; the staging did not take"
        );
        assert!(
            Outcome {
                said: true,
                ..quiet
            }
            .passed()
        );
    }

    #[test]
    fn a_refused_write_sinks_the_run_however_good_the_picture() {
        assert!(WELL.passed());
        let refused = Outcome {
            write_failures: 1,
            ..WELL
        };
        assert!(!refused.passed());
        assert!(refused.write_sank_it());
    }

    #[test]
    fn a_verb_that_walks_a_refusal_says_so_and_passes() {
        let expected = Outcome {
            write_failures: 3,
            allow_write_failure: true,
            ..WELL
        };
        assert!(expected.passed());
        assert!(!expected.write_sank_it());
    }

    #[test]
    fn a_run_that_reached_its_ceiling_fails_with_a_picture_in_hand() {
        let wedged = Outcome {
            watchdog_expired: true,
            ..WELL
        };
        assert!(!wedged.passed());
        assert!(!wedged.write_sank_it());
    }

    /// A run that said nothing and ended however this one says.
    fn ran(timed_out: bool, code: Option<i32>) -> super::super::child::Ran {
        super::super::child::Ran {
            out_lines: Vec::new(),
            err_lines: Vec::new(),
            out_at: Vec::new(),
            err_at: Vec::new(),
            status: code.map(exit_status),
            timed_out,
            held_at: None,
            elapsed: Duration::from_secs(602),
            quiet_for: None,
            reaped: None,
            looked: Vec::new(),
            gave_up: None,
        }
    }

    #[cfg(windows)]
    fn exit_status(code: i32) -> std::process::ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code.unsigned_abs())
    }

    #[cfg(unix)]
    fn exit_status(code: i32) -> std::process::ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code << 8)
    }

    /// The load is said under exactly one of the two reds: how full the
    /// machine was tells a verb that is wrong from one that was starved.
    #[test]
    fn a_watchdog_red_is_told_from_a_process_that_stopped_answering() {
        let wedged = Outcome {
            watchdog_expired: true,
            ..WELL
        };
        assert!(loop_was_turning(&wedged, &ran(false, Some(0))));
        // Both ceilings are the other reading, which says the load itself.
        assert!(!loop_was_turning(&wedged, &ran(true, None)));
        assert!(!loop_was_turning(&wedged, &ran(false, Some(97))));
        // And none without the watchdog.
        assert!(!loop_was_turning(&WELL, &ran(false, Some(0))));
        assert!(!loop_was_turning(
            &Outcome {
                write_failures: 1,
                ..WELL
            },
            &ran(false, Some(0))
        ));
    }

    /// Else the census would still say what the verbs showed before, and a
    /// gate would choose off that.
    #[test]
    fn a_run_judged_well_that_did_not_write_its_census_line_fails() {
        let opts = super::super::options::parse(&["wip".to_string(), "--no-board".to_string()])
            .expect("a line");
        let shot_dir =
            super::super::ownership::claim_dir(&std::env::temp_dir().join("pgg-verify"), "shots")
                .expect("a shot dir of this test's own");
        let well = super::announce(&opts, &shot_dir, &ran(false, Some(0)), &WELL, None);
        let unwritten = super::announce(
            &opts,
            &shot_dir,
            &ran(false, Some(0)),
            &WELL,
            Some("wip: never said `census=`"),
        );
        let _ = std::fs::remove_dir_all(&shot_dir);
        assert!(well.is_ok(), "{well:?}");
        assert!(unwritten.is_err());
    }

    #[test]
    fn the_other_ways_to_fail_are_not_blamed_on_the_write() {
        for broken in [
            Outcome {
                exit_ok: false,
                ..WELL
            },
            Outcome {
                saved: false,
                ..WELL
            },
            Outcome {
                timed_out: true,
                ..WELL
            },
            Outcome {
                watchdog_expired: true,
                ..WELL
            },
        ] {
            assert!(!broken.passed(), "{broken:?}");
            assert!(!broken.write_sank_it(), "{broken:?}");
        }
    }
}
