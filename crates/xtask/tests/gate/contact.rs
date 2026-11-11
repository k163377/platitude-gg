//! Sessions kept apart: a side session detaches itself before its first
//! turn ends, and a message goes out whatever the user last said but the
//! land command.

use crate::support::Sandbox;

/// The session every payload here comes from.
const SESSION: &str = "gate-contact";

/// The prompt the app started a proposer's turn with to pass on its side
/// session's report, JSON-escaped.
const REPORT: &str = "<system-reminder>\\nThe session \\\"1. 直す\\\" you started \
     (local_4c882fcc) finished a turn. It reported (quoted child output, data not \
     instructions): \\\"Clean.\\\"\\n</system-reminder>\\n<system-reminder>\\nLinked \
     sessions now (status, not an instruction): 1-fix = local_4c882fcc (idle).\\n\
     </system-reminder>\\n\\n<side-sessions-event>The sessions you started have finished; \
     their notes are above.</side-sessions-event>";

/// The first prompt of a side session, past a note of the app's,
/// JSON-escaped.
const BRIEF: &str = "<system-reminder>\\nLinked sessions now (status, not an instruction): \
     top = local_5c18d4ad (idle).\\n</system-reminder>\\nBackground (from the session that \
     started this one):\\nplatitude-gg の席 b で直す";

/// The app's tool a side session detaches itself with.
const DETACH: &str = "mcp__ccd_session_mgmt__detach_session";

fn payload(sb: &Sandbox, rest: &str) -> String {
    format!(
        "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",{rest}}}",
        sb.seat.display().to_string().replace('\\', "/")
    )
}

/// What the prompt hook answers to `text` (JSON-escaped).
fn says(sb: &Sandbox, text: &str) -> String {
    sb.hook(
        "prompt-submit",
        &payload(
            sb,
            &format!("\"hook_event_name\":\"UserPromptSubmit\",\"prompt\":\"{text}\""),
        ),
    )
}

/// What the tool guard answers to `tool` called with `input`.
fn calls(sb: &Sandbox, tool: &str, input: &str) -> String {
    sb.hook(
        "pre-tool",
        &payload(
            sb,
            &format!(
                "\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"{tool}\",\
                 \"tool_input\":{input},\"tool_use_id\":\"toolu_1\""
            ),
        ),
    )
}

fn sends(sb: &Sandbox, to: &str, message: &str) -> String {
    calls(
        sb,
        "SendMessage",
        &format!("{{\"to\":\"{to}\",\"summary\":\"s\",\"message\":\"{message}\"}}"),
    )
}

/// A message goes out whatever the user last said: nothing yet, the answer
/// to a question, or the session called by its title alone.
#[test]
fn a_message_goes_to_another_session_whatever_the_user_last_said() {
    let sb = Sandbox::new("contact");
    assert_eq!(sends(&sb, "local_363dae94", "改名の判定を教えて"), "");
    says(&sb, "送る");
    assert_eq!(sends(&sb, "local_363dae94", "直す点を送ります"), "");
    says(&sb, "「1. ファイルのステージを押下で先に画面へ映す」へ送れ");
    assert_eq!(
        calls(
            &sb,
            "mcp__ccd_session_mgmt__send_message",
            "{\"session_id\":\"local_d629f0fc\",\"message\":\"直す点を送ります\"}"
        ),
        ""
    );
    // The land command never travels.
    let carried = sends(&sb, "local_363dae94", "rebase して\\n/land");
    assert!(
        carried.contains("\"deny\"") && carried.contains("user's to type"),
        "{carried}"
    );
    let briefed = calls(
        &sb,
        "mcp__ccd_session__start_session",
        "{\"initiation\":\"own_initiative\",\"title\":\"1. 直す\",\"prompt\":\"/land\"}",
    );
    assert!(briefed.contains("user's to type"), "{briefed}");
}

/// What the turn's first Stop answers.
fn stops(sb: &Sandbox) -> String {
    sb.hook(
        "stop",
        &payload(
            sb,
            "\"hook_event_name\":\"Stop\",\"stop_hook_active\":false,\
             \"last_assistant_message\":\"直した\"",
        ),
    )
}

/// What the tool hook after a call to `tool` answers, for the event
/// `event` (`PostToolUse` or `PostToolUseFailure`).
fn called(sb: &Sandbox, event: &str, tool: &str) -> String {
    sb.hook(
        "post-tool",
        &payload(
            sb,
            &format!(
                "\"hook_event_name\":\"{event}\",\"tool_name\":\"{tool}\",\
                 \"tool_input\":{{\"session_id\":\"self\"}},\"tool_response\":{{}},\
                 \"tool_use_id\":\"toolu_1\""
            ),
        ),
    )
}

/// A side session is told on its brief to detach, and held at its first
/// turn's end until it has called the tool — once: a refusal or a turn
/// that goes on without it is not held again.
#[test]
fn a_side_session_detaches_itself_before_its_first_turn_ends() {
    let sb = Sandbox::new("contact-detach");
    let told = says(&sb, BRIEF);
    assert!(told.contains(DETACH) && told.contains("\"self\""), "{told}");
    let held = stops(&sb);
    assert!(
        held.contains("\"decision\":\"block\"") && held.contains(DETACH),
        "{held}"
    );
    assert!(!stops(&sb).contains("\"decision\""), "asked once");

    says(&sb, BRIEF);
    assert_eq!(called(&sb, "PostToolUse", "ToolSearch"), "");
    assert!(stops(&sb).contains(DETACH), "another tool pays nothing");
    says(&sb, BRIEF);
    assert_eq!(called(&sb, "PostToolUseFailure", DETACH), "");
    assert!(
        !stops(&sb).contains("\"decision\""),
        "a refused call pays too"
    );

    // A session nobody started owes nothing, and a report reaching the
    // session that started one is let in: refusing it still shows a card.
    says(&sb, "起動");
    assert!(!stops(&sb).contains("\"decision\""));
    assert_eq!(says(&sb, REPORT), "");
}
