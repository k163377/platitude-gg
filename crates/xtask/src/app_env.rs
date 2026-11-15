//! Isolation boundary for app automation launched by xtask.

use std::ffi::OsString;
use std::process::Command;

/// Core's `settings::AUTOMATION_PREFIX`, kept by hand (xtask links no
/// crates): change one, change both. Only harness knobs start with it, so
/// the product's `PGG_*` and xtask's `PGGDEV_*` ride through — `PGGDEV_STEP`
/// among them, which a stop in the gate's container walks for
/// (`linux::container`).
const AUTOMATION_PREFIX: &str = "_PGG_";

/// Case-folded: Windows resolves environment names without case, so a
/// parent's `_pgg_auto_act` still reaches the app's `_PGG_AUTO_ACT`.
fn is_automation(name: &str) -> bool {
    name.to_ascii_uppercase().starts_with(AUTOMATION_PREFIX)
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
    fn automation_is_the_prefix_alone() {
        assert!(is_automation("_PGG_AUTO_ACT"));
        assert!(is_automation("_PGG_AUTO_WATCHDOG_MS"));
        assert!(is_automation("_PGG_SHOT_DIR"));
        assert!(is_automation("_PGG_FAKE_PR"));
        // A knob nothing in the tree names yet.
        assert!(is_automation("_PGG_SOME_FUTURE_KNOB"));
        assert!(is_automation("_pgg_auto_act"));
        assert!(!is_automation("PGG_CONFIG_DIR"));
        assert!(!is_automation("pgg_config_dir"));
        assert!(!is_automation("PGG_LOG"));
        assert!(!is_automation("PGGDEV_ALLOW_GUI"));
        assert!(!is_automation("PGGDEV_STEP"));
        assert!(!is_automation("PATH"));
    }

    #[test]
    fn a_synthetic_environment_is_cleared_deterministically() {
        let names = [
            OsString::from("_PGG_AUTO_ACT"),
            OsString::from("_pgg_shot_dir"),
            OsString::from("PGG_CONFIG_DIR"),
            OsString::from("PGGDEV_STEP"),
            OsString::from("PATH"),
        ];
        let cleared = names_to_clear(names.into_iter());
        assert_eq!(
            cleared,
            vec![
                OsString::from("_PGG_AUTO_ACT"),
                OsString::from("_pgg_shot_dir")
            ],
            "harness knobs go, either case; the product's, xtask's and foreign names stay"
        );
    }
}
