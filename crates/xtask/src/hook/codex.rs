//! Codex lifecycle events update only sleep activity. Claude's permission
//! handlers and conversation-owned seat claims are not invoked here.

use super::payload::{own_string_field, own_value_field};
use crate::awake::{Mark, codex::record, digest};

pub(super) fn awake(input: &str) {
    let (Some(session), Some(cwd), Some(event)) = (
        own_string_field(input, "session_id"),
        own_string_field(input, "cwd"),
        own_string_field(input, "hook_event_name"),
    ) else {
        return;
    };
    let agent = own_string_field(input, "agent_id");
    if matches!(event.as_str(), "SubagentStart" | "SubagentStop") && agent.is_none() {
        return;
    }
    let id = own_string_field(input, "tool_use_id");
    let name = own_string_field(input, "tool_name").unwrap_or_default();
    let name = activity_name(&name);
    let digest = digest(own_value_field(input, "tool_input").unwrap_or(""));
    let mut marks = Vec::new();
    match event.as_str() {
        "SessionStart" | "Stop" | "Interrupt" => marks.push(Mark::Idle),
        "SessionEnd" => marks.push(Mark::Gone),
        "SubagentStart" => marks.push(Mark::Working),
        "SubagentStop" => {
            marks.push(Mark::CallsEnd);
            marks.push(Mark::Gone);
        }
        "UserPromptSubmit" => marks.push(Mark::Prompt),
        "PreToolUse" => {
            marks.push(Mark::Working);
            if let Some(id) = id.as_deref() {
                marks.push(Mark::ToolStart {
                    id,
                    name,
                    input: &digest,
                });
                if asks_person(name) {
                    marks.push(Mark::Asked {
                        id: Some(id),
                        name,
                        input: &digest,
                    });
                }
            }
        }
        "PostToolUse" => {
            marks.push(Mark::Working);
            if let Some(id) = id.as_deref() {
                marks.push(Mark::ToolEnd(id));
            }
        }
        // This hook precedes approval policy and can be allowed without
        // asking anyone. Only an actual Desktop server request proves a wait.
        "PermissionRequest" => return,
        _ => return,
    }
    // Activity is advisory; a failed power-control hook cannot veto work.
    if let Err(error) = record(&session, &cwd, agent.as_deref(), &marks) {
        eprintln!("Codex sleep activity: {error}");
    }
}

fn asks_person(name: &str) -> bool {
    matches!(
        name,
        "request_user_input" | "functions.request_user_input" | "AskUserQuestion" | "ExitPlanMode"
    )
}

/// Codex's process serves multiple chats. Its unrelated children cannot
/// be attributed by Claude's per-process shell time spans. Unified exec's
/// call remains open until its completion hook, including across polls.
fn activity_name(name: &str) -> &str {
    match name {
        "Bash" | "PowerShell" | "Monitor" => "CodexShell",
        "spawn_agent" | "collaboration.spawn_agent" | "wait_agent" | "collaboration.wait_agent" => {
            "Agent"
        }
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::{activity_name, asks_person};

    #[test]
    fn shell_children_are_not_attributed_across_codex_chats() {
        assert_eq!(activity_name("Bash"), "CodexShell");
        assert_eq!(activity_name("mcp__x__y"), "mcp__x__y");
        assert!(asks_person("request_user_input"));
        assert!(!asks_person("request_user_input_async"));
    }
}
