//! Sessions kept apart: a session another one started detaches itself, so
//! its turns wake nobody. Whether a message goes to another session is the
//! session's to read off the user's words; the one refusal here is the land
//! command carried in a message or a brief. The rules this module holds
//! sessions to are in internal-docs/反映前テストの機械化.md §セッション間の連絡.

use super::payload::{deny, own_value_field, string_field};
use super::permit;

/// The tools that put a message in front of another session, or hand it a
/// brief of this one's writing.
const CARRIERS: [&str; 4] = [
    "SendMessage",
    "mcp__ccd_session_mgmt__send_message",
    "mcp__ccd_session__start_session",
    "mcp__ccd_session__hand_off_to_session",
];

/// The app's tool that takes a side session out from under the session
/// that started it: that session is no longer told when its turns end.
/// The tool refuses a side session sharing another session's worktree and
/// one "the user placed (a fork, a task chip)"; such a session goes on
/// reporting. A session started from a proposal card the user clicked is
/// not one of them: it detaches itself with `"self"` (measured 2026-11-04).
const DETACH: &str = "mcp__ccd_session_mgmt__detach_session";

/// What a side session is told, `when` it is to call [`DETACH`].
fn detach_note(when: &str) -> String {
    format!(
        "Another session started this one, and the app tells that session each time a turn \
         here ends. platitude-gg keeps sessions apart (hook::contact): {when}, call {DETACH} \
         with session_id \"self\" (load it with ToolSearch)."
    )
}

/// UserPromptSubmit: the brief another session wrote opens a side session,
/// which then owes a detach (`stop`, `post_tool`).
///
/// A side session's report is never refused here: a refused prompt still
/// puts a card in front of the user, so the report is stopped where it
/// starts — the side session detaches before its first turn ends.
pub(super) fn prompt_submit(input: &str, prompt: &str) {
    if !permit::is_a_brief(prompt) {
        return;
    }
    // A debt that cannot be written reads as none owed: the note below
    // still asks, and only the Stop hold is lost.
    if let Some(path) = permit::session_file(input, "detach")
        && let Err(_unheard) = permit::write_whole(&path, "owed\n")
    {
        // Nobody to tell from inside a hook.
    }
    println!("{}", detach_note("as the first thing in this turn"));
}

/// PostToolUse and PostToolUseFailure: a call to [`DETACH`] pays the debt
/// whatever it answered — a refusal is not asked for again.
pub(super) fn post_tool(input: &str) {
    if string_field(input, "tool_name").as_deref() != Some(DETACH) {
        return;
    }
    if let Some(path) = permit::session_file(input, "detach")
        && let Err(_absent) = std::fs::remove_file(&path)
    {
        // Owed nothing.
    }
}

/// Stop, on the turn's first: a side session that has not called
/// [`DETACH`] goes on to call it. Asked once — the debt is taken here, so
/// a session that cannot detach is not held at every turn's end.
pub(super) fn stop(input: &str) -> Option<String> {
    let path = permit::session_file(input, "detach")?;
    std::fs::remove_file(&path).ok()?;
    Some(detach_note("before this turn ends"))
}

/// PreToolUse of any tool: the land command never travels in a message or
/// a brief. Answers whether it refused.
pub(super) fn pre_tool(input: &str) -> bool {
    let tool = string_field(input, "tool_name").unwrap_or_default();
    if !CARRIERS.contains(&tool.as_str()) {
        return false;
    }
    let asked = own_value_field(input, "tool_input").unwrap_or_default();
    let words = ["message", "prompt", "background", "remaining_plan", "title"]
        .iter()
        .filter_map(|field| string_field(asked, field))
        .collect::<Vec<String>>()
        .join("\n");
    if !permit::carries_land(&words) {
        return false;
    }
    deny(&format!(
        "{} is the user's to type in the session that lands (CLAUDE.md §Git 運用): carried in \
         a message or a brief it opens nothing there, and the session reading it would take \
         another Claude's words for the user's. Leave it out, and tell the user to type it in \
         that session.",
        permit::COMMAND
    ));
    true
}

#[cfg(test)]
mod tests {
    use super::pre_tool;

    /// What is answered without a repository: the detach a brief owes is
    /// `tests/gate/contact.rs`'s — it keeps its standing in one.
    #[test]
    fn everything_travels_but_the_land_command() {
        for passes in [
            r#"{"session_id":"x","cwd":"","tool_name":"SendMessage","tool_input":{"to":"local_363dae94","message":"改名の判定を教えて"}}"#,
            r#"{"session_id":"x","cwd":"","tool_name":"mcp__ccd_session_mgmt__send_message","tool_input":{"session_id":"local_1","message":"直す点を送ります"}}"#,
            r#"{"session_id":"x","cwd":"","tool_name":"SendMessage","tool_input":{"to":"a262effc79f37fa1e","message":"go on"}}"#,
            r#"{"session_id":"x","cwd":"","tool_name":"mcp__ccd_session__start_session","tool_input":{"title":"1. 直す","prompt":"直したら `/land` を打ってもらう"}}"#,
            r#"{"session_id":"x","cwd":"","tool_name":"Bash","tool_input":{"command":"/land"}}"#,
        ] {
            assert!(!pre_tool(passes), "{passes}");
        }
        for refused in [
            r#"{"session_id":"x","cwd":"","tool_name":"mcp__ccd_session__start_session","tool_input":{"title":"1. 直す","prompt":"直したら\n/land"}}"#,
            r#"{"session_id":"x","cwd":"","tool_name":"SendMessage","tool_input":{"to":"a262effc79f37fa1e","message":"/land"}}"#,
            r#"{"session_id":"x","cwd":"","tool_name":"mcp__ccd_session_mgmt__send_message","tool_input":{"session_id":"local_1","message":"直したら\n/land"}}"#,
            r#"{"session_id":"x","cwd":"","tool_name":"mcp__ccd_session__hand_off_to_session","tool_input":{"title":"t","remaining_plan":"rebase\n  /land push"}}"#,
        ] {
            assert!(pre_tool(refused), "{refused}");
        }
    }
}
