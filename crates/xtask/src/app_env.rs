//! Isolation boundary for app automation launched by xtask.

use std::process::Command;

/// Prefix every app automation variable lives under (core's
/// `settings::AUTOMATION_PREFIX` — xtask links no crates, so the pair is
/// kept by hand: change one, change both).
const AUTOMATION_PREFIX: &str = "PG_";

/// The pass-through names: `PG_*` variables that say nothing about who
/// is driving (core's `settings::NOT_AUTOMATION`, same hand-kept pair).
/// Everything else under the prefix is cleared by predicate, so an
/// automation knob added to the app later cannot leak a parent shell's
/// value into a harness child by being missing from a list here.
const NOT_AUTOMATION: &[&str] = &["PG_CONFIG_DIR", "PG_LOG", "PG_ALLOW_GUI"];

/// Whether one environment variable is an automation input to clear.
fn is_automation(name: &str) -> bool {
    name.starts_with(AUTOMATION_PREFIX) && !NOT_AUTOMATION.contains(&name)
}

/// Removes every automation variable the parent process carries, so a
/// parent shell, another harness, or a previous diagnostic command cannot
/// compose two automation protocols in one child. The harness sets its
/// own values after this — on a `Command`, a later set wins over the
/// remove.
pub(super) fn clear_automation(command: &mut Command) {
    for (name, _) in std::env::vars() {
        if is_automation(&name) {
            command.env_remove(&name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_is_the_prefix_minus_the_pass_through_names() {
        // The protocol variables of today…
        assert!(is_automation("PG_AUTO_ACT"));
        assert!(is_automation("PG_AUTO_WATCHDOG_MS"));
        assert!(is_automation("PG_SHOT_DIR"));
        assert!(is_automation("PG_FAKE_PR"));
        // …and the knob of tomorrow, which no list has heard of yet.
        assert!(is_automation("PG_SOME_FUTURE_KNOB"));
        // What rides through: who is driving is not in these.
        assert!(!is_automation("PG_CONFIG_DIR"));
        assert!(!is_automation("PG_LOG"));
        assert!(!is_automation("PG_ALLOW_GUI"));
        assert!(!is_automation("PATH"));
    }

    #[test]
    fn only_prefixed_variables_are_ever_removed() {
        let mut command = Command::new("app");
        clear_automation(&mut command);
        for (name, value) in command.get_envs() {
            if value.is_none() {
                let name = name.to_string_lossy();
                assert!(is_automation(&name), "{name}");
            }
        }
    }
}
