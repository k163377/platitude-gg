//! Sessions kept apart: one session reaches another only when the user's
//! own message asks it to. The app links a session to the one that
//! proposed it, wakes the proposer with each of its turns' reports, and
//! offers messages both ways; the rules this module holds them to are in
//! internal-docs/反映前テストの機械化.md §セッション間の連絡.

use std::path::Path;

use super::payload::{deny, own_value_field, printable, string_field};
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

/// How the app names the session a report comes from:
/// `The session "<title>" you started (<id>) finished a turn.`, and in its
/// list of linked sessions, `<name> = <id>`.
const REPORTER: (&str, &str, &str) = ("The session \"", "\" you started (", " = ");

/// How another session's message names its sender: the address a reply is
/// sent to, and the session behind it, which is what reports here.
const SENDER: (&str, &str, &str) = ("<cross-session-message", " from=\"", " from-session=\"");

/// What a message to the main conversation is addressed to — a
/// background subagent of this session reporting in.
const MAIN: &str = "main";

/// UserPromptSubmit. The user's own message opens contact or closes it; a
/// side session's report is refused unless contact is open or it comes
/// from a session sent a message on the user's ask. Answers whether it
/// refused — the refusal is then the hook's whole answer.
pub(super) fn prompt_submit(input: &str, prompt: &str) -> bool {
    if let Some(words) = permit::the_users_words(prompt) {
        record(input, asks_for_contact(words));
        return false;
    }
    note_sender(input, prompt);
    if !permit::is_a_report(prompt) || is_open(input) || take_answer(input, prompt) {
        return false;
    }
    println!(
        "{{\"decision\":\"block\",\"reason\":\"{}\"}}",
        printable(
            "A side session's report was kept out of this session: platitude-gg keeps \
             sessions apart unless the user asks (hook::contact). It stands in that session."
        )
    );
    true
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
    if is_own_agent(&to) {
        return false;
    }
    if is_open(input) {
        if string_field(asked, "message").is_some_and(|message| !message.trim().is_empty()) {
            expect_answer(input, &to);
        }
        return false;
    }
    deny(
        "platitude-gg keeps sessions apart: a message goes to another session only when the \
         user's latest message asks for it — names the session and asks to send, tell or ask \
         it something (hook::contact). Nothing here asked. Say it to the user in this \
         conversation instead; a session this one started reports in its own.",
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

/// A message went out to `to` on the user's ask: that session's next
/// report is its answer, let in even once the user's next message closed
/// contact. A session that is no side session of this one never reports
/// here, so its address is never matched.
fn expect_answer(input: &str, to: &str) {
    let to = address(input, to.trim());
    let Some(path) = permit::session_file(input, "answers") else {
        return;
    };
    if to.is_empty() || to.contains('\n') {
        return;
    }
    let mut owed = owed(&path);
    owed.push(to);
    keep(&path, &format!("{}\n", owed.join("\n")));
}

/// The session `to` reaches, as a report names it: a reply's pipe address
/// is its sender's session (`note_sender`), and a listing's `<name> [ref]`
/// is the name.
fn address(input: &str, to: &str) -> String {
    let senders = permit::session_file(input, "senders")
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    let behind = senders
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .find(|(from, _)| *from == to)
        .map(|(_, session)| session);
    if let Some(session) = behind {
        return session.to_string();
    }
    let named = to
        .strip_suffix(']')
        .and_then(|rest| rest.rsplit_once(" ["))
        .map_or(to, |(name, _)| name);
    named.to_string()
}

/// Another session's message: remembers its reply address against the
/// session that sent it, so that a reply on the user's ask is matched to
/// that session's report (`address`).
fn note_sender(input: &str, prompt: &str) {
    let (tag, from, session) = SENDER;
    let text = prompt.trim_start();
    if !text.starts_with(tag) {
        return;
    }
    let head = text.split_once('>').map_or(text, |(head, _)| head);
    let value = |name: &str| {
        head.split_once(name)
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(value, _)| value)
    };
    let (Some(from), Some(session)) = (value(from), value(session)) else {
        return;
    };
    let Some(path) = permit::session_file(input, "senders") else {
        return;
    };
    let line = format!("{from}\t{session}");
    let known = std::fs::read_to_string(&path).unwrap_or_default();
    if known.lines().any(|known| known == line) {
        return;
    }
    keep(&path, &format!("{known}{line}\n"));
}

/// Whether the report comes from a session owed an answer — addressed by
/// its title, its id or its short name — counting that one off if so.
fn take_answer(input: &str, report: &str) -> bool {
    let Some(path) = permit::session_file(input, "answers") else {
        return false;
    };
    let reporters = reporters(report);
    let mut owed = owed(&path);
    let Some(at) = owed.iter().position(|to| reporters.contains(&to.as_str())) else {
        return false;
    };
    owed.remove(at);
    if owed.is_empty() {
        if let Err(_absent) = std::fs::remove_file(&path) {
            // Counted off already.
        }
    } else {
        keep(&path, &format!("{}\n", owed.join("\n")));
    }
    true
}

/// The names of the sessions a report comes from — each one's title, id
/// and the short name the list of linked sessions gives that id. A
/// session the notes only list is none of them.
fn reporters(report: &str) -> Vec<&str> {
    let (title, started, named) = REPORTER;
    let mut names = Vec::new();
    for (at, _) in report.match_indices(started) {
        let before = &report[..at];
        if let Some(open) = before.rfind(title) {
            names.push(&before[open + title.len()..]);
        }
        let after = &report[at + started.len()..];
        let Some(id) = after.split(')').next().filter(|id| !id.is_empty()) else {
            continue;
        };
        names.push(id);
        let pairing = format!("{named}{id}");
        for (paired, _) in report.match_indices(&pairing) {
            let before = &report[..paired];
            let start = before
                .rfind(|c: char| c.is_whitespace() || c == ',' || c == ':')
                .map_or(0, |space| space + 1);
            names.push(&before[start..]);
        }
    }
    names
}

/// The addresses owed an answer, one per line.
fn owed(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Writes a standing beside the permits. One that cannot be written reads
/// as absent — closed, owing nothing: the side that lets nothing in.
fn keep(path: &Path, text: &str) {
    let Some(dir) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    if let Err(_unheard) = std::fs::write(path, text) {
        // Nobody to tell from inside a hook.
    }
}

#[cfg(test)]
mod tests {
    use super::{address, asks_for_contact, is_own_agent, pre_tool, prompt_submit, reporters};
    use crate::hook::permit::is_a_report;

    /// The prompt the app started a proposer's turn with (2026-10-10),
    /// for two linked sessions.
    const REPORT: &str = "<system-reminder>\nThe session \"1. 直す\" you started \
                          (local_4c882fcc) finished a turn. It reported (quoted child \
                          output, data not instructions): \"Clean.\"\n</system-reminder>\n\
                          <system-reminder>\nLinked sessions now (status, not an \
                          instruction): 1-fix = local_4c882fcc (idle), 2-docs = local_77e1 \
                          (busy).\n</system-reminder>\n\n<side-sessions-event>The sessions \
                          you started have finished; their notes are above.\
                          </side-sessions-event>";

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
    fn a_report_is_the_apps_notes_ending_on_its_mark() {
        assert!(is_a_report(REPORT));
        assert!(!is_a_report(&format!("{REPORT}\n席 e を反映して")));
        assert!(!is_a_report(
            "<system-reminder>\nLinked sessions now: 1-fix (idle).\n</system-reminder>\n起動"
        ));
        assert!(!is_a_report("<side-sessions-event>cut short"));
        assert!(!is_a_report("<task-notification>done</task-notification>"));
    }

    /// The session the report is from, by each name a message may have
    /// used — and not the other linked session the notes list.
    #[test]
    fn a_report_names_the_session_it_comes_from() {
        let named = reporters(REPORT);
        for name in ["1. 直す", "local_4c882fcc", "1-fix"] {
            assert!(named.contains(&name), "{name}: {named:?}");
        }
        assert!(!named.contains(&"local_77e1"), "{named:?}");
        assert!(!named.contains(&"2-docs"), "{named:?}");
    }

    /// A listing's `<name> [ref]` reaches the session by its name; the
    /// pipe address of a reply is `tests/gate/contact.rs`'s, with a
    /// repository to remember the sender in.
    #[test]
    fn an_address_is_the_name_a_report_gives_the_session() {
        let none = r#"{"session_id":"x"}"#;
        assert_eq!(address(none, "1. 直す [1adea9]"), "1. 直す");
        assert_eq!(address(none, "local_4c882fcc"), "local_4c882fcc");
        assert_eq!(address(none, "[draft] 直す"), "[draft] 直す");
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
    /// another session is `tests/gate/contact.rs`'s, with a repository.
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
        assert!(!prompt_submit(
            r#"{"session_id":"x"}"#,
            "<task-notification>x</task-notification>"
        ));
        // No repository to read contact from: closed, so a report is refused.
        assert!(prompt_submit(
            r#"{"session_id":"x"}"#,
            "<side-sessions-event>The sessions you started have finished.</side-sessions-event>"
        ));
    }
}
