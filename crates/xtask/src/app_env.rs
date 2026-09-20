//! Isolation boundary for app automation launched by xtask.

use std::ffi::OsString;
use std::process::Command;

/// Prefix every app automation variable lives under (core's
/// `settings::AUTOMATION_PREFIX` — xtask links no crates, so the pair is
/// kept by hand: change one, change both).
const AUTOMATION_PREFIX: &str = "PGG_";

/// The pass-through names: `PGG_*` variables that say nothing about who
/// is driving (core's `settings::NOT_AUTOMATION`, same hand-kept pair).
/// Everything else under the prefix is cleared by predicate, so an
/// automation knob added to the app later cannot leak a parent shell's
/// value into a harness child by being missing from a list here.
///
/// `PGG_STEP` rides through because it is the mark every process of a
/// gate's verb carries inside the gate's container, the app and what
/// the app starts included: a stop is a walk for that mark, and a
/// child it was cleared from is a child the walk cannot end
/// (`linux::container`).
const NOT_AUTOMATION: &[&str] = &["PGG_CONFIG_DIR", "PGG_LOG", "PGG_ALLOW_GUI", "PGG_STEP"];

/// Whether one environment variable is an automation input to clear.
/// Compared case-folded: Windows resolves environment names without
/// case, so a parent's `pgg_auto_act` reaches the child's
/// `std::env::var("PGG_AUTO_ACT")` all the same.
fn is_automation(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.starts_with(AUTOMATION_PREFIX) && !NOT_AUTOMATION.contains(&upper.as_str())
}

/// The parent-environment names [`clear_automation`] removes, from an
/// iterator of raw names. A name that is not Unicode cannot be read by
/// the app's `std::env::var` and is left alone — the walk goes on past
/// it (`std::env::vars` panics on one, which is why this takes
/// `vars_os`-shaped input).
fn names_to_clear(names: impl Iterator<Item = OsString>) -> Vec<OsString> {
    names
        .filter(|name| name.to_str().is_some_and(is_automation))
        .collect()
}

/// Removes every automation variable the parent process carries, so a
/// parent shell, another harness, or a previous diagnostic command cannot
/// compose two automation protocols in one child. The harness sets its
/// own values after this — on a `Command`, a later set wins over the
/// remove.
pub(super) fn clear_automation(command: &mut Command) {
    for name in names_to_clear(std::env::vars_os().map(|(name, _)| name)) {
        command.env_remove(&name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_is_the_prefix_minus_the_pass_through_names() {
        // The protocol variables of today…
        assert!(is_automation("PGG_AUTO_ACT"));
        assert!(is_automation("PGG_AUTO_WATCHDOG_MS"));
        assert!(is_automation("PGG_SHOT_DIR"));
        assert!(is_automation("PGG_FAKE_PR"));
        // …the knob of tomorrow, which no list has heard of yet…
        assert!(is_automation("PGG_SOME_FUTURE_KNOB"));
        // …and the spelling Windows resolves without case.
        assert!(is_automation("pgg_auto_act"));
        // What rides through: who is driving is not in these.
        assert!(!is_automation("PGG_CONFIG_DIR"));
        assert!(!is_automation("pgg_config_dir"));
        assert!(!is_automation("PGG_LOG"));
        assert!(!is_automation("PGG_ALLOW_GUI"));
        // The step's mark rides through to the app and to what the app
        // starts: a stop addresses every process of a verb by it.
        assert!(!is_automation("PGG_STEP"));
        assert_eq!(
            NOT_AUTOMATION.len(),
            4,
            "core's list is the same four, kept by hand"
        );
        assert!(!is_automation("PATH"));
    }

    #[test]
    fn a_synthetic_environment_is_cleared_deterministically() {
        let names = [
            OsString::from("PGG_AUTO_ACT"),
            OsString::from("pgg_shot_dir"),
            OsString::from("PGG_CONFIG_DIR"),
            OsString::from("PATH"),
        ];
        let cleared = names_to_clear(names.into_iter());
        assert_eq!(
            cleared,
            vec![
                OsString::from("PGG_AUTO_ACT"),
                OsString::from("pgg_shot_dir")
            ],
            "automation knobs go, either case; pass-through and foreign names stay"
        );
    }
}
