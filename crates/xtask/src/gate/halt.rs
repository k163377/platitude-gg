//! A gate that stops at its first red (反映前テストの機械化.md §最初の赤で止まる).
//!
//! A red decides the run, and the fix it asks for nearly always voids the
//! stamps the rest could earn. So on either side nothing more starts, a step
//! waiting for room gives its place up, and a running step is ended with what
//! it started ([`crate::check::run_step`]) — except a `linux` step with a
//! container of its own, which is left to finish ([`super::runner`]). Every
//! step it kept from running or ended is filed as `halted`.
//!
//! `--keep-going` (implied by `--all`) runs the rest, but a red always-step
//! stops the run either way ([`super::sides::run_sides`]).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct Halt {
    raised: AtomicBool,
    keep_going: bool,
    /// The step whose red raised it, and whether it was an always-step.
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

    /// `always`: the red step is an always-step, which stops the run under
    /// `--keep-going` too.
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

    /// The run's closing line once a halt stopped it. It offers
    /// `--keep-going` only where that would run the rest: not after an
    /// always-step, and not in a landing's gate (`land` takes no such flag).
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
