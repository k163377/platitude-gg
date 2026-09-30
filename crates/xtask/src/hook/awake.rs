//! The hooks' half of `crate::awake`: which event says a session — or one
//! of its subagents, when the payload names one — is in a turn, has
//! stopped, or starts, ends, or asks a person about a tool call.
//!
//! A tool's hooks inside a subagent carry the session's `session_id` and
//! the subagent's `agent_id`, and its session's `transcript_path`
//! (measured); Stop does not run for a subagent, SubagentStop does. No
//! hook runs when a person lets a call through or turns it down (the
//! hooks reference). PermissionRequest runs when a call needs a person's
//! decision, after the call's PreToolUse — beside the prompt sent to the
//! desktop app, when that app asks — and names the tool and its input but
//! not the call; a prompt Claude Code keeps to the person alone
//! (`personOnly`) runs none (the 2.1.281 and 2.1.284 binaries).
//! PermissionDenied is auto mode's.

use super::payload::{bool_field, number_field, own_string_field, own_value_field};
use crate::awake::{Mark, Whose, digest};

/// The tools whose call is itself a question to a person.
const ASKS_A_PERSON: [&str; 2] = ["AskUserQuestion", "ExitPlanMode"];

/// UserPromptSubmit: a turn starts.
pub(super) fn prompt(input: &str) {
    mark(input, Mark::Prompt);
}

/// PreToolUse, of any tool: working, and the call opens — a shell's
/// processes count until they end, however long it runs, in the foreground
/// or not. A question to a person is asked as it opens; a wake-up is
/// scheduled or called off.
pub(super) fn tool_start(input: &str) {
    mark(input, Mark::Working);
    let Some(call) = Call::of(input) else {
        return;
    };
    mark(
        input,
        Mark::ToolStart {
            id: &call.id,
            name: &call.name,
            input: &call.input,
        },
    );
    if ASKS_A_PERSON.contains(&call.name.as_str()) {
        mark(
            input,
            Mark::Asked {
                id: Some(&call.id),
                name: &call.name,
                input: &call.input,
            },
        );
    }
    if call.name == "ScheduleWakeup" {
        let asked = own_value_field(input, "tool_input").unwrap_or("");
        let scheduled = if bool_field(asked, "stop") == Some(true) {
            Mark::NoWake
        } else {
            number_field(asked, "delaySeconds").map_or(Mark::Working, Mark::WakeIn)
        };
        mark(input, scheduled);
    }
}

/// PostToolUse or PostToolUseFailure, of any tool, and an auto-mode
/// denial: working, and the call ends — the processes it started may run
/// on.
pub(super) fn after_tool(input: &str) {
    mark(input, Mark::Working);
    if let Some(id) = own_string_field(input, "tool_use_id") {
        mark(input, Mark::ToolEnd(&id));
    }
}

/// PermissionRequest: a person is asked about the call — by its id when
/// the payload has one, else by its tool and input.
pub(super) fn permission_request(input: &str) {
    let Some(name) = own_string_field(input, "tool_name") else {
        return;
    };
    let id = own_string_field(input, "tool_use_id");
    let digest = digest(own_value_field(input, "tool_input").unwrap_or(""));
    mark(
        input,
        Mark::Asked {
            id: id.as_deref(),
            name: &name,
            input: &digest,
        },
    );
}

/// Elicitation (`pending`) or ElicitationResult: an MCP server asks a
/// person for input in the middle of one of its calls, or has the answer.
/// Neither names the call, nor a subagent — the 2.1.281 and 2.1.284
/// binaries build them with no call's context — so the request's own id is
/// all that tells one from another.
pub(super) fn elicitation(input: &str, pending: bool) {
    if let Some(server) = own_string_field(input, "mcp_server_name") {
        let server = server_in_tool_names(&server);
        let id = own_string_field(input, "elicitation_id");
        mark(
            input,
            Mark::Elicited {
                server: &server,
                id: id.as_deref(),
                pending,
            },
        );
    }
}

/// Stop or StopFailure: the turn is over. A subagent's end is its own
/// event (`subagent_stop`).
pub(super) fn stopped(input: &str) {
    mark(input, Mark::Idle);
}

/// SubagentStart: the subagent works, under a claim of its own.
pub(super) fn subagent_start(input: &str) {
    mark(input, Mark::Working);
}

/// SubagentStop: the subagent's calls that never ended end, and its claim
/// goes; what its calls started runs on under its session's claim.
pub(super) fn subagent_stop(input: &str) {
    mark(input, Mark::CallsEnd);
    mark(input, Mark::Gone);
}

/// SessionEnd: the session's claims go. A conversation the machine's
/// sleep ended claims again with its next hook after the wake.
pub(super) fn session_end(input: &str) {
    mark(input, Mark::Gone);
}

/// A tool call as its PreToolUse names it.
struct Call {
    id: String,
    name: String,
    input: String,
}

impl Call {
    fn of(input: &str) -> Option<Self> {
        Some(Self {
            id: own_string_field(input, "tool_use_id")?,
            name: own_string_field(input, "tool_name")?,
            input: digest(own_value_field(input, "tool_input").unwrap_or("")),
        })
    }
}

/// An MCP server's name as its tools' names spell it (`mcp__<server>__…`):
/// what is not a letter, a digit, `_` or `-` becomes `_`, and a claude.ai
/// connector's runs of `_` fold into one, none at either end (as the
/// 2.1.281 binary spells them).
fn server_in_tool_names(server: &str) -> String {
    let spelled: String = server
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !server.starts_with("claude.ai ") {
        return spelled;
    }
    let mut folded = String::new();
    for c in spelled.chars() {
        if !(c == '_' && folded.ends_with('_')) {
            folded.push(c);
        }
    }
    folded.trim_matches('_').to_string()
}

/// Whose event a payload reports, and from where.
#[derive(Debug, PartialEq, Eq)]
struct Source {
    session: String,
    cwd: String,
    agent: Option<String>,
    transcript: Option<String>,
}

impl Source {
    fn of(input: &str) -> Option<Self> {
        Some(Self {
            session: own_string_field(input, "session_id")?,
            cwd: own_string_field(input, "cwd")?,
            agent: own_string_field(input, "agent_id"),
            transcript: own_string_field(input, "transcript_path"),
        })
    }
}

fn mark(input: &str, mark: Mark<'_>) {
    let Some(source) = Source::of(input) else {
        return;
    };
    let whose = source.agent.as_deref().map_or(Whose::Session, Whose::Agent);
    crate::awake::mark(
        &source.session,
        whose,
        &source.cwd,
        source.transcript.as_deref(),
        mark,
    );
}

#[cfg(test)]
mod tests {
    use super::{Call, Source, server_in_tool_names};

    /// A session's own PostToolUse, whose tool's input and response nest
    /// keys of the payload's own names — the call's id after them, where
    /// Claude Code puts it.
    const NESTED: &str = r#"{"session_id":"s1","transcript_path":"C:\\t\\s1.jsonl","cwd":"C:\\w","hook_event_name":"PostToolUse","tool_name":"mcp__x__y","tool_input":{"agent_id":"nested","tool_use_id":"inner","cwd":"elsewhere"},"tool_response":{"tool_use_id":"inner2"},"tool_use_id":"toolu_1"}"#;

    /// The payload's own fields, never a tool's input's or response's.
    #[test]
    fn reads_the_payload_s_own_fields_alone() {
        assert_eq!(
            Source::of(NESTED),
            Some(Source {
                session: "s1".into(),
                cwd: r"C:\w".into(),
                agent: None,
                transcript: Some(r"C:\t\s1.jsonl".into()),
            })
        );
        let call = Call::of(NESTED).expect("a call");
        assert_eq!(
            (call.id.as_str(), call.name.as_str()),
            ("toolu_1", "mcp__x__y")
        );
    }

    #[test]
    fn spells_a_server_name_as_its_tools_do() {
        assert_eq!(server_in_tool_names("ccd_session_mgmt"), "ccd_session_mgmt");
        assert_eq!(server_in_tool_names("my server.v2"), "my_server_v2");
        assert_eq!(
            server_in_tool_names("claude.ai Google  Drive"),
            "claude_ai_Google_Drive"
        );
        assert_eq!(server_in_tool_names("claude.ai .x."), "claude_ai_x");
    }
}
