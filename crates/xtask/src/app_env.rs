//! Isolation boundary for app automation launched by xtask.

use std::ffi::OsString;
use std::process::Command;

/// Core's `settings::AUTOMATION_PREFIX`, kept by hand (xtask links no
/// crates): change one, change both.
const AUTOMATION_PREFIX: &str = "PGG_";

/// The `PGG_*` names that say nothing about who is driving (core's
/// `settings::NOT_AUTOMATION`, same hand-kept pair). The rest of the
/// prefix is cleared by predicate, so a knob added later cannot leak by
/// being missing from a list here.
///
/// `PGG_STEP` must ride through: a stop in the gate's container walks
/// for that mark, and cannot end a child it was cleared from
/// (`linux::container`).
const NOT_AUTOMATION: &[&str] = &["PGG_CONFIG_DIR", "PGG_LOG", "PGG_ALLOW_GUI", "PGG_STEP"];

/// Case-folded: Windows resolves environment names without case, so a
/// parent's `pgg_auto_act` still reaches the app's `PGG_AUTO_ACT`.
fn is_automation(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.starts_with(AUTOMATION_PREFIX) && !NOT_AUTOMATION.contains(&upper.as_str())
}

/// A name that is not Unicode is left alone (the app's `std::env::var`
/// cannot read it); the input is `vars_os`-shaped because
/// `std::env::vars` panics on one.
fn names_to_clear(names: impl Iterator<Item = OsString>) -> Vec<OsString> {
    names
        .filter(|name| name.to_str().is_some_and(is_automation))
        .collect()
}

/// Removes every automation variable the parent process carries, so two
/// automation protocols never compose in one child. The harness sets its
/// own values after this: on a `Command`, a later set wins over the
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
        assert!(is_automation("PGG_AUTO_ACT"));
        assert!(is_automation("PGG_AUTO_WATCHDOG_MS"));
        assert!(is_automation("PGG_SHOT_DIR"));
        assert!(is_automation("PGG_FAKE_PR"));
        // A knob no list has heard of yet.
        assert!(is_automation("PGG_SOME_FUTURE_KNOB"));
        assert!(is_automation("pgg_auto_act"));
        assert!(!is_automation("PGG_CONFIG_DIR"));
        assert!(!is_automation("pgg_config_dir"));
        assert!(!is_automation("PGG_LOG"));
        assert!(!is_automation("PGG_ALLOW_GUI"));
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
