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
    /// What a verb whose failure the camera cannot see has to be caught
    /// saying. `solo` photographs a perfectly good ordinary window if the
    /// lock was never held, `details-fit` frames a pane whose content ran
    /// off the right of the window the same as one that fits, and
    /// `window-fill` is about a maximised window, which leaves no desktop
    /// beside itself for a short edge to show against — nor any way to
    /// photograph the frame it paints out past the screen.
    pub(super) must_say: Option<&'static str>,
    pub(super) said: bool,
}

impl Outcome {
    pub(super) fn passed(self) -> bool {
        self.exit_ok
            && self.saved
            && !self.timed_out
            && !self.watchdog_expired
            && !self.write_sank_it()
            && (self.must_say.is_none() || self.said)
    }

    /// Whether the failing writes are what the verdict turns on — the one
    /// reason worth a line of its own, since the run looks well otherwise.
    pub(super) fn write_sank_it(self) -> bool {
        self.write_failures > 0 && !self.allow_write_failure
    }
}

/// What the run's own lines make of it.
pub(super) fn judge(opts: &super::options::Options, ran: &super::child::Ran) -> Outcome {
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
        must_say,
        said: must_say.is_none_or(|wanted| {
            err_lines
                .iter()
                .chain(out_lines.iter())
                .any(|l| l.contains(wanted))
        }),
    }
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
    // Onto the board as the run goes, not when somebody remembers: the
    // seat is read from the working directory there, so a picture that
    // is registered is a picture that says which tree took it. A pass is
    // not the condition — a failing run's picture is the one most worth
    // looking at.
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
        if timed_out { ", TIMED OUT" } else { "" },
    );
    if let Some(wanted) = outcome.must_say
        && !outcome.said
    {
        println!(
            "  the run never said `{wanted}` — and this verb's picture reads the same \
             whether it should have or not."
        );
    }
    if outcome.watchdog_expired {
        println!(
            "  the app's own watchdog ended this run — the verb never reached its \
             completion, and a run that wedged between the two grabs still leaves app.png \
             behind to pass on."
        );
    }
    if outcome.write_sank_it() {
        println!(
            "  git refused the write this verb asked for — the shot is of the state it \
             never reached. If the refusal is what the verb shows, say so with \
             --allow-write-failure."
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
    use super::Outcome;

    /// A run that reached the end and took its picture.
    const WELL: Outcome = Outcome {
        exit_ok: true,
        saved: true,
        timed_out: false,
        watchdog_expired: false,
        write_failures: 0,
        allow_write_failure: false,
        must_say: None,
        said: true,
    };

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
