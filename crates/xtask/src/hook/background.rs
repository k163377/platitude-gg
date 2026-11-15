//! Work a session leaves running in the background and is woken by when it
//! ends: the wake has to arrive.

use super::git::xtask_verb;
use super::payload::{bool_field, deny, own_string_field, string_field};
use super::shell::shell_segments;

/// The xtask runs the land skill starts in the background and waits on —
/// its waits, gate and landing — and verify-ui, which sessions start the
/// same way.
const WAITED_ON: &[&str] = &["gate", "land", "verify-ui"];

/// PreToolUse(Bash|PowerShell): an xtask run waited on in the background is
/// started from the Bash tool. A PowerShell wrapper around a background run
/// can stay running after the run ends; then no notification comes and the
/// session sleeps until the user asks after it. A Bash wrapper ends with the
/// run. Why the PowerShell one stays is not known, so the guard names the
/// tool, not a form of the command. The deny pays by the calls it removes: a
/// run that never reports costs the user's message, and under a landing the
/// permit that message closes. Answers whether it refused.
pub(super) fn pre_shell(input: &str) -> bool {
    if own_string_field(input, "tool_name").as_deref() != Some("PowerShell")
        || bool_field(input, "run_in_background") != Some(true)
    {
        return false;
    }
    let Some(verb) = string_field(input, "command").and_then(|command| waited_on(&command)) else {
        return false;
    };
    deny(&format!(
        "Start this background `cargo xtask {verb}` run from the Bash tool instead: the same \
         command with every argument it has, its output sent the bash way (`<the command> > \
         <log> 2>&1; echo exit=$?`). A PowerShell wrapper around a background xtask run can \
         stay running after the run ends; then no notification comes and the session sleeps \
         until the user asks after it. A Bash wrapper ends with the run, and its notification \
         wakes the session."
    ));
    true
}

/// The first verb of [`WAITED_ON`] the line runs through `cargo xtask`.
fn waited_on(command: &str) -> Option<&'static str> {
    shell_segments(command).into_iter().find_map(|segment| {
        let tokens: Vec<&str> = segment.split_whitespace().collect();
        WAITED_ON
            .iter()
            .copied()
            .find(|verb| xtask_verb(&tokens, verb))
    })
}

#[cfg(test)]
mod tests {
    use super::{pre_shell, waited_on};

    #[test]
    fn refuses_only_a_powershell_run_left_in_the_background() {
        let shell = |tool: &str, command: &str, background: bool| {
            format!(
                r#"{{"session_id":"s","cwd":"C:/w","tool_name":"{tool}","tool_input":{{"command":"{command}","run_in_background":{background}}}}}"#
            )
        };
        let gate = "cargo xtask gate *> C:/t/gate.log";
        assert!(pre_shell(&shell("PowerShell", gate, true)));
        assert!(!pre_shell(&shell("PowerShell", gate, false)));
        assert!(!pre_shell(&shell("Bash", gate, true)));
        assert!(!pre_shell(&shell(
            "PowerShell",
            "Get-Content C:/t/gate.log -Wait",
            true
        )));
    }

    #[test]
    fn names_the_xtask_runs_the_skills_wait_on() {
        for (command, verb) in [
            (
                "cargo xtask gate *> C:\\t\\gate.log; \"gate exit=$LASTEXITCODE\"",
                "gate",
            ),
            ("cargo xtask land wait head *> C:\\t\\wait.log", "land"),
            (
                "git rev-parse main; cargo xtask land worktree-f *> C:\\t\\land.log",
                "land",
            ),
            ("cargo xtask verify-ui commit --preset basic", "verify-ui"),
            ("cargo run -p xtask -- gate", "gate"),
        ] {
            assert_eq!(waited_on(command), Some(verb), "{command}");
        }
    }

    #[test]
    fn leaves_every_other_line_alone() {
        for command in [
            "cargo xtask docs",
            "cargo xtask land-helper",
            "cargo test -p xtask land",
            "git log --oneline -1",
            "Get-Content C:\\t\\gate.log -Tail 5",
            "echo 'cargo xtask gate'",
        ] {
            assert_eq!(waited_on(command), None, "{command}");
        }
    }
}
