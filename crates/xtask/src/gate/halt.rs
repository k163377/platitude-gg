//! A gate that stops at its first red.
//!
//! **A red decides the run**, and what the rest of it could still earn is
//! stamps for the steps that pass. A stamp is keyed by what its step reads
//! ([`super::stamp`]), and every verb reads the app and core trees whole —
//! so a fix to the app, the core or the verbs' harness, which is what a
//! red asks for nearly every time, takes every one of those stamps away
//! again, and the run that earns them is minutes of the machine spent for
//! nothing while the seat waits for a verdict it already has. So the
//! first red stops the run: nothing more is started on either side, a step
//! still waiting for room gives its place up, and a step already running is
//! ended where what it started ends with it ([`crate::check::run_step`]) —
//! a `linux` step that brought up a container of its own is left to finish
//! ([`super::runner`]). Every step it kept from running or ended is filed
//! as `halted`.
//!
//! **`--keep-going` runs the rest anyway** — the whole picture, and the
//! stamps a red outside what the others read leaves standing. Stage 3
//! (`--all`) always runs this way. **A red among the always-steps stops
//! the run whichever way it was asked**: they are seconds and go first
//! ([`super::run_sides`]), and what they catch is fixed before anything
//! else is worth running.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct Halt {
    raised: AtomicBool,
    keep_going: bool,
    /// The step whose red raised it, for the rows it leaves behind, and
    /// whether it was an always-step.
    by: Mutex<Option<(String, bool)>>,
}

impl Halt {
    pub(super) fn new(keep_going: bool) -> Self {
        Self {
            raised: AtomicBool::new(false),
            keep_going,
            by: Mutex::new(None),
        }
    }

    /// Hears a red: `always` is whether the step was one of the
    /// always-steps, which stop the run under `--keep-going` too.
    pub(super) fn red(&self, id: &str, always: bool) {
        if self.keep_going && !always {
            return;
        }
        let mut by = self
            .by
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if by.is_none() {
            *by = Some((id.to_string(), always));
        }
        self.raised.store(true, Ordering::SeqCst);
    }

    pub(super) fn raised(&self) -> bool {
        self.raised.load(Ordering::SeqCst)
    }

    /// The run's closing word on its halt, once one stopped it: the step
    /// that did, and the way on where there is one — `--keep-going` stops
    /// for an always-step as well, and a landing's gate is `land`'s, which
    /// takes no such word.
    pub(super) fn said(&self, landing: bool) -> Option<String> {
        let (by, always) = self
            .by
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()?;
        let rest = if always || landing {
            ""
        } else {
            "; `--keep-going` runs the rest"
        };
        Some(format!(
            "gate: stopped at the first red ({by}) — what it kept from running is filed as halted{rest}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::Halt;

    #[test]
    fn the_first_red_stops_the_run_and_is_the_one_it_names() {
        let halt = Halt::new(false);
        assert!(!halt.raised());
        halt.red("test it (all)", false);
        halt.red("verify amend", false);
        assert!(halt.raised());
        let said = halt.said(false).expect("a halt says so");
        assert!(said.contains("(test it (all))"), "{said}");
        assert!(said.ends_with("`--keep-going` runs the rest"), "{said}");
        let landing = halt.said(true).expect("a halt says so");
        assert!(!landing.contains("--keep-going"), "{landing}");
    }

    #[test]
    fn keep_going_stops_for_an_always_step_alone() {
        let halt = Halt::new(true);
        halt.red("verify amend", false);
        assert!(!halt.raised(), "a red verb leaves the rest running");
        assert_eq!(halt.said(false), None);
        halt.red("fmt", true);
        assert!(halt.raised(), "a red always-step stops it all the same");
        let said = halt.said(false).expect("a halt says so");
        assert!(said.contains("(fmt)"), "{said}");
        assert!(!said.contains("--keep-going"), "{said}");
    }
}
