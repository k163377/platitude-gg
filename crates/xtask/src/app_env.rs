//! Isolation boundary for app automation launched by xtask.

use std::process::Command;

/// Every variable that can change app automation independently of the
/// harness which owns the child.  Clearing these first prevents a parent
/// shell, another harness, or a previous diagnostic command from composing
/// two automation protocols in one process.
const AUTOMATION_ENV: &[&str] = &[
    "PG_AUTO_ACT",
    "PG_AUTO_ACT_ARG",
    "PG_AUTO_IDENTITY",
    "PG_AUTO_IDENTITY_SAVE",
    "PG_AUTO_OPEN",
    "PG_AUTO_PERF",
    "PG_AUTO_QUIT_MS",
    "PG_AUTO_SCROLL",
    "PG_AUTO_SELECT",
    "PG_AUTO_WATCHDOG_MS",
    "PG_AUTO_WIP",
    "PG_FAKE_PR",
    "PG_MEM_REPORT",
    "PG_PLAIN_CHROME",
    "PG_SCROLL_TO",
    "PG_SHOT_DIR",
];

pub(super) fn clear_automation(command: &mut Command) {
    for name in AUTOMATION_ENV {
        command.env_remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_automation_variable_is_explicitly_removed() {
        let mut command = Command::new("app");
        clear_automation(&mut command);
        let removed: Vec<_> = command
            .get_envs()
            .filter_map(|(name, value)| value.is_none().then_some(name.to_string_lossy()))
            .collect();
        for name in AUTOMATION_ENV {
            assert!(
                removed.iter().any(|removed| removed.as_ref() == *name),
                "{name}"
            );
        }
    }
}
