//! What the run is judged on, how the judgement is made off its own
//! lines, and how it is said.

use std::path::PathBuf;

/// What the run is judged on.
///
/// A refused write counts against it: the verb asked for one, and a
/// picture of the state it never reached proves nothing. Verbs that
/// exist to walk a refusal (`fetch-fail`, `delete-branch-refused`) say
/// so with `--allow-write-failure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Outcome {
    pub(super) exit_ok: bool,
    pub(super) saved: bool,
    pub(super) timed_out: bool,
    /// Whether the app's own watchdog ended the run. It quits cleanly when
    /// it fires, so the parent sees an ordinary exit — and a run that
    /// wedged after its first `grabToImage` came back has a screenshot to
    /// show for itself as well (observed: a half-finished shot pair
    /// passed on the strength of `app.png` alone). Reaching the ceiling is
    /// never a pass: it is the harness saying it stopped waiting.
    pub(super) watchdog_expired: bool,
    pub(super) write_failures: usize,
    pub(super) allow_write_failure: bool,
    /// Whether the app was refused the settings it was handed *and
    /// nothing staged that* — the two verbs whose subject is a held
    /// store are not counted here ([`super::child`]).
    ///
    /// A run's config directory is made for it alone, so the only
    /// reading left is that something else got hold of it, and the run
    /// goes on with an empty store and a window about *that*, which the
    /// verb's own waiting never comes back from. Said in the second it
    /// happens rather than read off the top of the log once the watchdog
    /// has spent the whole of its ceiling (`super::options`).
    pub(super) store_refused: bool,
    /// What a verb whose failure the camera cannot see has to be caught
    /// saying. `solo` photographs a perfectly good ordinary window if the
    /// lock was never held, `details-fit` frames a pane whose content ran
    /// off the right of the window the same as one that fits, and
    /// `window-fill` is about a maximised window, which leaves no desktop
    /// beside itself for a short edge to show against — nor any way to
    /// photograph the frame it paints out past the screen.
    pub(super) must_say: Option<&'static str>,
    pub(super) said: bool,
    /// Whether the identity a held save wrote is in the run's own
    /// gitconfig, read once the app has ended — the witness outside the
    /// app that the exit waited for the save (`quit-save-held`,
    /// `super::shim::held_save_landed`). `None` for every verb with no
    /// such witness.
    pub(super) held_save_landed: Option<bool>,
}

impl Outcome {
    pub(super) fn passed(self) -> bool {
        self.exit_ok
            && self.saved
            && !self.timed_out
            && !self.watchdog_expired
            && !self.write_sank_it()
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

    let must_say = super::verbs::must_say(&opts.verb, &opts.arg);
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

/// Which of the two reds this is: the one no ceiling ended.
///
/// The app's own watchdog is a QML `Timer` (`auto/AutoShotDriver.qml`),
/// so a run it ended answered its event loop when the timer fired.
/// This says nothing about earlier stalls or which completion was missing.
/// The other red is a process that stopped answering, and
/// it is read off the ceiling instead (`super::wedge`), whose account
/// already carries the machine's load. **Exclusive, so the load is said
/// once**: a run whose watchdog fired and whose teardown then wedged is
/// the ceiling's, and the account under it is the fuller reading.
pub(super) fn loop_was_turning(outcome: &Outcome, ran: &super::child::Ran) -> bool {
    outcome.watchdog_expired && !super::wedge::at_a_ceiling(ran)
}

/// The pictures the run left, named, and filed on the board as the run
/// goes rather than when somebody remembers: the seat is read from the
/// working directory there, so a picture that is registered is a picture
/// that says which tree took it. A pass is not the condition — a failing
/// run's picture is the one most worth looking at.
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
            // The board is not what this run is judging. Say the reason
            // and let the verdict stand on the pictures themselves.
            Err(message) => println!("board: not updated ({message})"),
        }
    }
    shots
}

/// Prints what the run said, files its pictures on the board, and gives
/// the verdict — the whole of what a person reads off one run.
pub(super) fn announce(
    opts: &super::options::Options,
    shot_dir: &std::path::Path,
    ran: &super::child::Ran,
    outcome: &Outcome,
) -> Result<(), String> {
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
        if outcome.passed() { "PASS" } else { "FAIL" },
        opts.verb,
        elapsed.as_secs_f32(),
        status.map_or_else(
            || "?".into(),
            |s| s.code().map_or("signal".into(), |c| c.to_string())
        ),
        outcome.saved,
        outcome.write_failures,
        // Reaped where it was ordered to hold, or at the ceiling: two
        // different ends, and the words `wedge-check` reads them by
        // (`super::faults`).
        match (&ran.held_at, timed_out) {
            (Some(station), _) => format!(", HELD AT {station}"),
            (None, true) => ", TIMED OUT".to_string(),
            (None, false) => String::new(),
        },
    );
    // A run ended by a ceiling is the one that says nothing for itself:
    // the app's own account of where it stood, and what only the parent
    // can see about it, are the whole of what the next occurrence is read
    // from ([`super::wedge`]).
    if super::wedge::at_a_ceiling(ran) {
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
    // Said either way: the pass is the witness having been read, and a
    // reader who sees the line knows what was read.
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
        println!(
            "  the app's own watchdog ended this run — the act, screenshot, and census did \
             not all finish; a run stalled between grabs can still leave app.png behind."
        );
        // The other half of the reading, and the half a red under load is
        // told from a red that is wrong by. This watchdog is a QML
        // `Timer`, so its firing proves the loop answered at that point.
        // Earlier stalls and lost completion edges are still possible.
        if loop_was_turning(outcome, ran) {
            println!(
                "  the loop answered its watchdog — this does not rule out earlier stalls. {}",
                super::wedge::lanes_line()
            );
            // The trail also contains the teardown after the watchdog
            // asked to quit. Reaching `exiting` does not explain which
            // completion was missing while the loop was up.
            for line in super::wedge::trail(shot_dir) {
                println!("{line}");
            }
        }
    }
    if outcome.write_sank_it() {
        println!(
            "  git refused the write this verb asked for — the shot is of the state it \
             never reached. If the refusal is what the verb shows, say so with \
             --allow-write-failure."
        );
    }
    if outcome.store_refused {
        println!(
            "  the settings this run was handed were already held, so it opened the window \
             that says so rather than the one the verb is about. Nothing else knows that \
             directory, so two runs were given one: on the container side, the host \
             directory mounted at /out."
        );
    }
    if outcome.passed() {
        Ok(())
    } else {
        Err(format!("verify-ui {} failed", opts.verb))
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
        store_refused: false,
        must_say: None,
        said: true,
        held_save_landed: None,
    };

    /// The witness on disk is the verdict for the one verb that has it:
    /// a run that printed every line it should have and left the run's
    /// gitconfig without the identity is a process that ended before its
    /// save wrote, and that is the failure the verb is for.
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

    /// A run's settings are its own — nothing else knows the directory —
    /// so a store that came back held means two runs were handed one, and
    /// the picture is of a window about that. Waiting out the watchdog to
    /// discover it costs the whole ceiling — the backstop's height, not a
    /// verb's — and says only that the verb never finished.
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
        // `solo` photographs an ordinary window if the lock was never
        // held, and an ordinary window takes a perfectly good picture.
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
        // The app quits itself when the watchdog fires, and one half of a
        // shot pair can already be on disk by then: exit 0, screenshot
        // saved, nothing refused.
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

    /// The two reds are read apart, and the load is said under one of
    /// them: a QML timer can only fire from a loop that is turning, so a
    /// run it ended is not a process that stopped answering — and how
    /// full the machine was is what tells a verb that is wrong from a
    /// verb that was starved.
    #[test]
    fn a_watchdog_red_is_told_from_a_process_that_stopped_answering() {
        let wedged = Outcome {
            watchdog_expired: true,
            ..WELL
        };
        assert!(loop_was_turning(&wedged, &ran(false, Some(0))));
        // Both ceilings are the other reading, whose account carries the
        // load already: said twice, the two would disagree about which
        // moment they were probed at.
        assert!(!loop_was_turning(&wedged, &ran(true, None)));
        assert!(!loop_was_turning(&wedged, &ran(false, Some(97))));
        // Nothing to say about the load of a run that answered for
        // itself in words anybody can read.
        assert!(!loop_was_turning(&WELL, &ran(false, Some(0))));
        assert!(!loop_was_turning(
            &Outcome {
                write_failures: 1,
                ..WELL
            },
            &ran(false, Some(0))
        ));
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
