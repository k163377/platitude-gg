//! The kill guard: a reap aimed by image name takes every seat's runs and
//! the user's own window with it.

use super::PROCESS_STOP_APPROVAL_FLAG;
use super::payload::string_field;

/// PreToolUse(Bash|PowerShell): a kill aimed at the app by image name
/// reaps every seat's runs and the user's own window in one line.
/// Answers whether it refused, like `pre_git`.
pub(super) fn pre_kill(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    if command.contains(PROCESS_STOP_APPROVAL_FLAG) || !broad_kill(&command) {
        return Ok(false);
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"This kill reaches by image name, so it takes every seat's runs \
         and the user's own window with it. The exe lock and the \
         second-instance gate both come from THIS tree's own stale run: \
         `{}` reaps exactly those (processes whose exe lives under this \
         tree) and nothing else. If the user asked to kill the others in so \
         many words, run the same command again with \
         {PROCESS_STOP_APPROVAL_FLAG}=1 in front of it.\"}}}}",
        crate::gui::KILL.line()
    );
    Ok(true)
}

/// Whether a shell line kills the app without pinning the kill to one
/// tree's processes. A verb counts only as a bare token — quoted, it is
/// somebody's search pattern, not an invocation (a kill smuggled whole
/// into `powershell -Command "..."` slips this net; the guard teaches,
/// it does not contain adversaries). `$_.Path`-filtered pipelines are
/// the one hand-written shape that is scoped; taskkill cannot filter by
/// path at all, so it is held regardless.
fn broad_kill(command: &str) -> bool {
    let line = command.to_lowercase();
    if !line.contains("platitude") {
        return false;
    }
    let invoked = |verb: &str| {
        line.split_whitespace()
            .any(|token| !token.starts_with(['"', '\'']) && token.trim_matches(['"', '\'']) == verb)
    };
    if invoked("taskkill") {
        return true;
    }
    let path_scoped = line.contains("$_.path") || line.contains("path -like");
    !path_scoped
        && (invoked("stop-process")
            || invoked("pkill")
            || invoked("killall")
            || (invoked("wmic") && line.contains("delete"))
            || line.contains("$_.kill("))
}

#[cfg(test)]
mod tests {
    use super::broad_kill;

    #[test]
    fn holds_kills_that_reach_past_this_tree() {
        for command in [
            "taskkill /F /IM platitude-gg.exe",
            "Stop-Process -Name platitude-gg -Force",
            "Get-Process platitude-gg | ForEach-Object { $_.Kill() }",
            "pkill -9 platitude-gg",
            "killall platitude-gg",
            "wmic process where \"name='platitude-gg.exe'\" delete",
        ] {
            assert!(broad_kill(command), "{command}");
        }
    }

    #[test]
    fn lets_scoped_and_unrelated_kills_through() {
        for command in [
            // The one hand-written scoped shape: pinned to a tree's path.
            "Get-Process platitude-gg -ErrorAction SilentlyContinue | \
             Where-Object { $_.Path -like \"*worktrees\\a*\" } | \
             ForEach-Object { $_.Kill() }",
            // Precise by pid, and the sanctioned verb.
            "taskkill /F /PID 1234",
            "cargo xtask kill",
            "kill -9 4321",
            "pkill -f some-other-tool",
            // A search that names the verbs as quoted data is not a kill
            // (a live false positive: this guard once held this grep).
            "grep -n \"Kill()\\|Stop-Process\\|taskkill\" .claude/skills/verify-ui/SKILL.md",
            "rg 'taskkill' C:/x/platitude-gg",
        ] {
            assert!(!broad_kill(command), "{command}");
        }
    }
}
