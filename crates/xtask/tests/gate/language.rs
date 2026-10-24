//! What the user reads is Japanese: the hook's answers for an English line
//! between calls, an English closing message and an English wrap-up box,
//! through the binary the harness runs.

use std::path::Path;

use crate::support::Sandbox;

/// The session every payload here comes from.
const SESSION: &str = "gate-language";

const ENGLISH: &str = "The fix is committed in seat c, and the gate passed on both sides.";

const JAPANESE: &str = "席 c で直しを commit し、gate は両側で PASS した。";

fn forward(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn block_line(message: &str, block: &str) -> String {
    format!(
        "{{\"parentUuid\":\"p\",\"isSidechain\":false,\"message\":{{\"model\":\"m\",\
         \"id\":\"{message}\",\"type\":\"message\",\"role\":\"assistant\",\
         \"content\":[{block}]}},\"type\":\"assistant\",\"uuid\":\"u\"}}"
    )
}

/// A session record: an English line ahead of one call, a Japanese one
/// ahead of the next.
fn record() -> String {
    let line = |message: &str, words: &str, call: &str| {
        [
            block_line(
                message,
                &format!("{{\"type\":\"text\",\"text\":\"{words}\"}}"),
            ),
            block_line(
                message,
                &format!(
                    "{{\"type\":\"tool_use\",\"id\":\"{call}\",\"name\":\"Bash\",\"input\":{{}}}}"
                ),
            ),
        ]
        .join("\n")
    };
    [
        line("msg_1", ENGLISH, "toolu_en"),
        line("msg_2", JAPANESE, "toolu_ja"),
    ]
    .join("\n")
}

#[test]
fn an_english_line_between_calls_gets_a_note_for_its_event() {
    let sb = Sandbox::new("language-calls");
    let transcript = sb.root.join("session.jsonl");
    std::fs::write(&transcript, record()).expect("record");
    let after = |event: &str, call: &str| {
        sb.hook(
            "post-tool",
            &format!(
                "{{\"session_id\":\"{SESSION}\",\"transcript_path\":\"{}\",\"cwd\":\"{}\",\
                 \"hook_event_name\":\"{event}\",\"tool_name\":\"Bash\",\"tool_input\":{{}},\
                 \"tool_response\":{{}},\"tool_use_id\":\"{call}\"}}",
                forward(&transcript),
                forward(&sb.repo)
            ),
        )
    };
    let noted = after("PostToolUse", "toolu_en");
    assert!(
        noted.starts_with(
            "{\"hookSpecificOutput\":{\"hookEventName\":\"PostToolUse\",\"additionalContext\":\"直前に"
        ),
        "{noted}"
    );
    let failed = after("PostToolUseFailure", "toolu_en");
    assert!(
        failed.contains("\"hookEventName\":\"PostToolUseFailure\""),
        "{failed}"
    );
    assert_eq!(after("PostToolUse", "toolu_ja"), "");
}

#[test]
fn an_english_closing_is_sent_back_once() {
    let sb = Sandbox::new("language-stop");
    let stop = |active: &str, message: &str| {
        sb.hook(
            "stop",
            &format!(
                "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\"hook_event_name\":\"Stop\",\
                 \"stop_hook_active\":{active},\"last_assistant_message\":\"{message}\"}}",
                forward(&sb.repo)
            ),
        )
    };
    let asked = stop("false", ENGLISH);
    assert!(
        asked.starts_with("{\"decision\":\"block\",\"reason\":\"締めの文が英語だった"),
        "{asked}"
    );
    assert_eq!(stop("true", ENGLISH), "");
    assert_eq!(stop("false", JAPANESE), "");
}

#[test]
fn an_english_wrap_up_box_is_refused_before_it_is_drawn() {
    let sb = Sandbox::new("language-box");
    let wrap_up = |recap: &str| {
        sb.hook(
            "pre-tool",
            &format!(
                "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\"hook_event_name\":\"PreToolUse\",\
                 \"tool_name\":\"mcp__ccd_turn__wrap_up\",\"tool_input\":{{\"recap\":\"{recap}\"}}}}",
                forward(&sb.repo)
            ),
        )
    };
    let refused = wrap_up(ENGLISH);
    assert!(
        refused.contains("\"permissionDecision\":\"deny\""),
        "{refused}"
    );
    assert_eq!(wrap_up(JAPANESE), "");
}

/// A chip stacked right after an English line: the chips' note and the
/// language's ride in one JSON object — two would be read as plain text.
#[test]
fn the_chips_note_and_the_language_share_one_answer() {
    let sb = Sandbox::new("language-chip");
    let transcript = sb.root.join("session.jsonl");
    std::fs::write(&transcript, record()).expect("record");
    let answer = sb.hook(
        "post-tool",
        &format!(
            "{{\"session_id\":\"{SESSION}\",\"transcript_path\":\"{}\",\"cwd\":\"{}\",\
             \"hook_event_name\":\"PostToolUse\",\"tool_name\":\"mcp__ccd_session__start_session\",\
             \"tool_input\":{{\"initiation\":\"own_initiative\",\"title\":\"1. 直す\",\
             \"context\":\"fresh\",\"prompt\":\"直す\"}},\"tool_response\":[{{\"type\":\"text\",\
             \"text\":\"Proposed 1. 直す (task_id: task_a18c448e).\"}}],\"tool_use_id\":\"toolu_en\"}}",
            forward(&transcript),
            forward(&sb.repo)
        ),
    );
    assert_eq!(answer.lines().count(), 1, "{answer}");
    assert!(
        answer.starts_with("{\"hookSpecificOutput\":{\"hookEventName\":\"PostToolUse\""),
        "{answer}"
    );
    assert!(answer.contains("names no path"), "{answer}");
    assert!(answer.contains("直前に"), "{answer}");
}

/// A seat ahead of main has a notice for the turn's end. A turn sent back
/// for its language has not ended: the notice comes with the Stop that
/// ends it.
#[test]
fn a_turn_sent_back_shows_its_notice_once_at_its_end() {
    let sb = Sandbox::new("language-notice");
    let seat = sb.roster_tree("c", "worktree-c");
    sb.write(&seat, "internal-docs/notes.md", "# notes\n\nthe work\n");
    sb.commit_all(
        &seat,
        "docs: the work in this seat",
        &[("PGG_GATE_SKIP", "1")],
    );
    let stop = |active: &str, message: &str| {
        sb.hook(
            "stop",
            &format!(
                "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\"hook_event_name\":\"Stop\",\
                 \"stop_hook_active\":{active},\"last_assistant_message\":\"{message}\"}}",
                forward(&seat)
            ),
        )
    };
    let sent_back = stop("false", ENGLISH);
    assert!(
        sent_back.starts_with("{\"decision\":\"block\""),
        "{sent_back}"
    );
    assert!(!sent_back.contains("systemMessage"), "{sent_back}");
    let ended = stop("true", JAPANESE);
    assert!(
        ended.starts_with("{\"systemMessage\":\"gate: seat c is 1 commit(s) ahead"),
        "{ended}"
    );
    assert_eq!(stop("false", JAPANESE), ended);
}
