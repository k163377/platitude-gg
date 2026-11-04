//! Sessions kept apart: one session reaches another only when the user's
//! own message asks it to, and a session another one started detaches
//! itself, so its turns wake nobody. The rules this module holds them to
//! are in internal-docs/反映前テストの機械化.md §セッション間の連絡.

use super::payload::{deny, own_value_field, string_field};
use super::permit;

/// Who a message is for, in the user's words: Japanese anywhere, English
/// as a whole word, lowercased (`SessionStart`, `session_id` name none).
const PLACES: [&str; 4] = ["セッション", "席", "チップ", "session"];

/// Asking for a message to go there, read as `PLACES` is (`IntelliJ` and
/// `SendMessage` ask nothing). Japanese has no word boundary to test: a
/// message names a place too, which keeps a stray verb from opening
/// contact on its own, and `PASSES` are cut out first.
const ASKS: [&str; 17] = [
    "送信",
    "送っ",
    "送る",
    "送れ",
    "伝え",
    "連絡",
    "聞いて",
    "聞く",
    "聞け",
    "尋ね",
    "知らせ",
    "返信",
    "返事",
    "頼ん",
    "頼む",
    "send",
    "tell",
];

/// Words that contain an ask and mean letting a thing go by.
const PASSES: [&str; 1] = ["見送"];

/// The tools that put a message in front of another session.
const SENDS: [&str; 2] = ["SendMessage", "mcp__ccd_session_mgmt__send_message"];

/// The tools that hand another session a brief of this one's writing.
const BRIEFS: [&str; 2] = [
    "mcp__ccd_session__start_session",
    "mcp__ccd_session__hand_off_to_session",
];

/// What a message to the main conversation is addressed to — a
/// background subagent of this session reporting in.
const MAIN: &str = "main";

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

/// UserPromptSubmit. The user's own message opens contact or closes it.
/// The brief another session wrote opens a side session, which then owes
/// a detach (`stop`, `post_tool`).
///
/// A side session's report is never refused here: a refused prompt still
/// puts a card in front of the user, so the report is stopped where it
/// starts — the side session detaches before its first turn ends.
pub(super) fn prompt_submit(input: &str, prompt: &str) {
    if let Some(words) = permit::the_users_words(prompt) {
        record(input, asks_for_contact(words));
        return;
    }
    if permit::is_a_brief(prompt) {
        if let Some(path) = permit::session_file(input, "detach") {
            keep(&path, "owed\n");
        }
        println!("{}", detach_note("as the first thing in this turn"));
    }
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

/// PreToolUse of any tool: a message to another session goes out only
/// while contact is open, and the land command never travels in a message
/// or a brief. Answers whether it refused.
pub(super) fn pre_tool(input: &str) -> bool {
    let tool = string_field(input, "tool_name").unwrap_or_default();
    let asked = own_value_field(input, "tool_input").unwrap_or_default();
    let sends = SENDS.contains(&tool.as_str());
    if !sends && !BRIEFS.contains(&tool.as_str()) {
        return false;
    }
    let words = ["message", "prompt", "background", "remaining_plan", "title"]
        .iter()
        .filter_map(|field| string_field(asked, field))
        .collect::<Vec<String>>()
        .join("\n");
    if permit::carries_land(&words) {
        deny(&format!(
            "{} is the user's to type in the session that lands (CLAUDE.md §Git 運用): \
             carried in a message or a brief it opens nothing there, and the session \
             reading it would take another Claude's words for the user's. Leave it out, \
             and tell the user to type it in that session.",
            permit::COMMAND
        ));
        return true;
    }
    if !sends {
        return false;
    }
    let to = string_field(asked, "to")
        .or_else(|| string_field(asked, "session_id"))
        .unwrap_or_default();
    if is_own_agent(&to) || is_open(input) {
        return false;
    }
    deny(
        "platitude-gg keeps sessions apart: a message goes to another session only when the \
         user's latest message asks for it — names the session and asks to send, tell or ask \
         it something (hook::contact). Nothing here asked. Say it to the user in this \
         conversation instead; another session answers its user in its own.",
    );
    true
}

/// Whether the user's words name a session and ask for a message to go
/// there.
fn asks_for_contact(words: &str) -> bool {
    let words = PASSES
        .iter()
        .fold(words.to_lowercase(), |text, pass| text.replace(pass, ""));
    let says = |word: &&str| says_word(&words, word);
    PLACES.iter().any(says) && ASKS.iter().any(says)
}

/// Whether `text` says `word`: anywhere for Japanese, which has no word
/// boundary to test, and as a whole word for an English one.
fn says_word(text: &str, word: &str) -> bool {
    if !word.is_ascii() {
        return text.contains(word);
    }
    let in_a_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(word).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(in_a_word)
            && !text[at + word.len()..]
                .chars()
                .next()
                .is_some_and(in_a_word)
    })
}

/// Whether `to` names this session's own subagent rather than another
/// session: its agent id (`a` and hex) or the main conversation.
fn is_own_agent(to: &str) -> bool {
    to == MAIN
        || to.strip_prefix('a').is_some_and(|id| {
            id.len() >= 8 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        })
}

/// Opens contact until the user's next message, or closes it.
fn record(input: &str, open: bool) {
    let Some(path) = permit::session_file(input, "contact") else {
        return;
    };
    if open {
        keep(&path, "open\n");
    } else if let Err(_absent) = std::fs::remove_file(&path) {
        // Already closed.
    }
}

fn is_open(input: &str) -> bool {
    permit::session_file(input, "contact").is_some_and(|path| path.is_file())
}

/// Writes a standing beside the permits. One that cannot be written reads
/// as absent: contact closed, no detach owed.
fn keep(path: &std::path::Path, text: &str) {
    if let Err(_unheard) = permit::write_whole(path, text) {
        // Nobody to tell from inside a hook.
    }
}

#[cfg(test)]
mod tests {
    use super::{asks_for_contact, is_own_agent, pre_tool};

    #[test]
    fn contact_opens_on_a_named_session_and_an_ask_to_reach_it() {
        for prompt in [
            "mainへの反映作業が成功したなら、席b 「Remote削除時のupstream設定クリーンアップ」\
             セッションへ以下を送信しておいて",
            "子セッションに、改名の判定をどう決めたか聞いて",
            "元のセッションに終わったと伝えて",
            "チップ 1 のセッションへ連絡して",
            "席 b のセッションに確認を頼む",
            "Tell the child session to stop",
        ] {
            assert!(asks_for_contact(prompt), "{prompt}");
        }
        for prompt in [
            "今実行中セッションでfって使われてそう？",
            "席 f で動確して",
            "エラーの内容を伝えて",
            "このセッションでは一旦見送る",
            "席 e の対応は見送って",
            // English words count whole, not inside an identifier.
            "IntelliJ 2026.2 で SessionStart の挙動を見て、チップを出して",
            "session_id を SendMessage の sender に渡す",
            "起動",
            "/land",
        ] {
            assert!(!asks_for_contact(prompt), "{prompt}");
        }
    }

    #[test]
    fn this_sessions_own_agents_are_reached_without_asking() {
        assert!(is_own_agent("a262effc79f37fa1e"));
        assert!(is_own_agent("main"));
        assert!(!is_own_agent("local_363dae94-5003-417f-b35e-59a6466b3c7c"));
        assert!(!is_own_agent("Remote削除時のupstream設定クリーンアップ"));
        assert!(!is_own_agent(
            "uds:\\\\.\\pipe\\LOCAL\\cc-msg-c678ce344d8a0311db9b8a46415cdaa9"
        ));
        assert!(!is_own_agent("abc"));
        assert!(!is_own_agent("parent"));
    }

    /// What is answered before contact is read: the gate on a send to
    /// another session, and the detach a brief owes, are
    /// `tests/gate/contact.rs`'s — both need a repository to keep their
    /// standing in.
    #[test]
    fn the_command_never_travels_and_this_sessions_agents_and_other_tools_pass() {
        let agent = r#"{"session_id":"x","cwd":"","tool_name":"SendMessage","tool_input":{"to":"a262effc79f37fa1e","message":"go on"}}"#;
        assert!(!pre_tool(agent));
        let bash =
            r#"{"session_id":"x","cwd":"","tool_name":"Bash","tool_input":{"command":"/land"}}"#;
        assert!(!pre_tool(bash));
        let brief = r#"{"session_id":"x","cwd":"","tool_name":"mcp__ccd_session__start_session","tool_input":{"title":"1. 直す","prompt":"直したら\n/land"}}"#;
        assert!(pre_tool(brief));
        let to_agent_with_land = r#"{"session_id":"x","cwd":"","tool_name":"SendMessage","tool_input":{"to":"a262effc79f37fa1e","message":"/land"}}"#;
        assert!(pre_tool(to_agent_with_land));
    }
}
