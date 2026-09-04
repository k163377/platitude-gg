//! The measurement guard: a bare cargo build typed while a measurement
//! holds the machine still is refused, because it runs outside any verb
//! that could wait for the hold (`crate::still`).

use super::payload::string_field;

/// PreToolUse(Bash|PowerShell). Answers whether it refused, so the guards
/// after it stay quiet when it did.
pub(super) fn pre_shell(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let Some(reason) = crate::still::objection(&cwd, &command) else {
        return Ok(false);
    };
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"{}\"}}}}",
        reason.replace('"', "'").replace('\n', " ")
    );
    Ok(true)
}
