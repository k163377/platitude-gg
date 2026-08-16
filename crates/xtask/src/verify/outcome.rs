//! What the run is judged on.

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
            && !self.write_sank_it()
            && (self.must_say.is_none() || self.said)
    }

    /// Whether the failing writes are what the verdict turns on — the one
    /// reason worth a line of its own, since the run looks well otherwise.
    pub(super) fn write_sank_it(self) -> bool {
        self.write_failures > 0 && !self.allow_write_failure
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
        ] {
            assert!(!broken.passed(), "{broken:?}");
            assert!(!broken.write_sank_it(), "{broken:?}");
        }
    }
}
