//! What the user reads is Japanese: the line a session writes between
//! tool calls, its closing message and the wrap-up box. A long turn drifts
//! into the English of the tool results around it, so the drift is said
//! where it shows — after the call an English line went out with, at the
//! Stop that would end on an English message, and before a wrap-up box is
//! drawn in English. Identifiers, code, commit messages and logs stay
//! English, and quoted code and paths are not counted.

use std::io::{Read, Seek, SeekFrom};

use super::payload::{array_items, deny, own_string_field, own_value_field, string_field};

/// The wrap-up box the app draws under the closing message.
const WRAP_UP: &str = "mcp__ccd_turn__wrap_up";

/// How much of the session record's end is read for a call's own message.
/// Its lines sit ahead of the results of the calls made beside it, which a
/// wide read can push past this; a message not found is not judged.
const TAIL: u64 = 1 << 20;

/// Fewer Latin letters than this is too short to tell.
const LETTERS: usize = 25;

/// Kana and kanji under this share (percent) of the letters is English.
const JAPANESE_SHARE: usize = 8;

const BETWEEN_CALLS: &str = "直前にユーザーへ書いた一言が英語だった。ユーザーが読む文は、ツールの合間の一言も報告も \
     wrap_up も日本語で書く — ツール結果や通知が英語でも、書き出しは日本語。識別子・コード・コミット\
     メッセージ・ログは英語のまま。";

const CLOSING: &str = "締めの文が英語だった。同じ内容を日本語で書き直し、それだけ書いてターンを終える(ツールは\
     呼ばない。識別子・コード・コミットメッセージは英語のまま)。英語で出すべき物 — コミット\
     メッセージの案など — だったなら、その旨を日本語で 1 行書いて終える。";

const BOX: &str = "wrap_up の recap / ask が英語だった。ユーザーが読む欄なので、日本語で書いて呼び直す\
     (識別子・コマンド・SHA はそのまま。英語の件名などを引く時は、日本語の説明で包む)。";

/// PostToolUse: the note for an English line written just ahead of this
/// call. A subagent's lines reach nobody but the session.
pub(super) fn post_tool(input: &str) -> Option<&'static str> {
    if own_value_field(input, "agent_id").is_some() {
        return None;
    }
    let call = own_string_field(input, "tool_use_id")?;
    let record = tail(&own_string_field(input, "transcript_path")?, TAIL)?;
    english(&ahead_of(&record, &call)?).then_some(BETWEEN_CALLS)
}

/// Stop: the reason to go on when the closing message is English. Asked
/// once — a turn going on because a Stop hook said so is not asked again.
pub(super) fn stop(input: &str) -> Option<&'static str> {
    if own_value_field(input, "stop_hook_active") == Some("true") {
        return None;
    }
    english(&own_string_field(input, "last_assistant_message")?).then_some(CLOSING)
}

/// PreToolUse: a wrap-up box asked for in English is refused before it is
/// drawn. Answers whether it refused.
pub(super) fn pre_tool(input: &str) -> bool {
    if own_string_field(input, "tool_name").as_deref() != Some(WRAP_UP) {
        return false;
    }
    let asked = own_value_field(input, "tool_input").unwrap_or_default();
    let words = ["recap", "ask"]
        .iter()
        .filter_map(|field| string_field(asked, field))
        .collect::<Vec<_>>()
        .join("\n");
    let refused = english(&words);
    if refused {
        deny(BOX);
    }
    refused
}

/// The text the session wrote ahead of `call` in the message that made it,
/// when `call` is that message's first — calls made side by side share the
/// line, and the first says it for all. `None` when the message is not in
/// `record`.
fn ahead_of(record: &str, call: &str) -> Option<String> {
    let messages = record.lines().filter_map(assistant).collect::<Vec<_>>();
    let id = messages.iter().find_map(|(id, blocks)| {
        blocks
            .iter()
            .any(|block| own_string_field(block, "id").as_deref() == Some(call))
            .then_some(id)
    })?;
    let mut text = Vec::new();
    for block in messages
        .iter()
        .filter(|(message, _)| message == id)
        .flat_map(|(_, blocks)| blocks)
    {
        match own_string_field(block, "type").as_deref() {
            Some("text") => text.extend(own_string_field(block, "text")),
            Some("tool_use") => {
                return (own_string_field(block, "id").as_deref() == Some(call))
                    .then(|| text.join("\n"));
            }
            _ => {}
        }
    }
    None
}

/// A line of the session record that the model wrote: its message id and
/// content blocks. Claude Code writes one line per block, each naming the
/// message.
fn assistant(line: &str) -> Option<(String, Vec<&str>)> {
    if !line.contains("\"assistant\"")
        || own_string_field(line, "type").as_deref() != Some("assistant")
    {
        return None;
    }
    let message = own_value_field(line, "message")?;
    let id = own_string_field(message, "id")?;
    Some((id, array_items(own_value_field(message, "content")?)))
}

/// The whole lines among the last `bytes` of the file at `path`. One byte
/// more is read, so a window that starts on a line's first byte sees the
/// newline ahead of it and keeps that line.
fn tail(path: &str, bytes: u64) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let start = file.metadata().ok()?.len().saturating_sub(bytes + 1);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut read = Vec::new();
    file.read_to_end(&mut read).ok()?;
    let whole = if start == 0 {
        &read[..]
    } else {
        read.iter()
            .position(|&byte| byte == b'\n')
            .map_or(&[][..], |at| &read[at + 1..])
    };
    Some(String::from_utf8_lossy(whole).into_owned())
}

/// Whether `text` is English prose: enough Latin letters, and almost no
/// kana or kanji among them.
fn english(text: &str) -> bool {
    let prose = prose(text);
    let latin = prose.chars().filter(char::is_ascii_alphabetic).count();
    let japanese = prose.chars().filter(|&c| kana_or_kanji(c)).count();
    latin >= LETTERS && japanese * 100 < (japanese + latin) * JAPANESE_SHARE
}

fn kana_or_kanji(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}')
}

/// `text` without what is code or a place: fenced blocks, inline code,
/// link targets, and ASCII words that hold a slash (paths, URLs).
fn prose(text: &str) -> String {
    let mut kept = String::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let unquoted = line.split('`').step_by(2).collect::<Vec<_>>().join(" ");
        for word in without_link_targets(&unquoted).split_whitespace() {
            if !(word.is_ascii() && word.contains(['/', '\\'])) {
                kept.push_str(word);
                kept.push(' ');
            }
        }
    }
    kept
}

/// `line` with each markdown link's `(target)` dropped and its text kept.
fn without_link_targets(line: &str) -> String {
    let mut kept = String::new();
    let mut rest = line;
    while let Some(at) = rest.find("](") {
        kept.push_str(&rest[..=at]);
        rest = rest[at..].find(')').map_or("", |end| &rest[at + end + 1..]);
    }
    kept.push_str(rest);
    kept
}

#[cfg(test)]
mod tests {
    use super::{BETWEEN_CALLS, CLOSING, ahead_of, english, post_tool, pre_tool, stop, tail};

    fn block_line(message: &str, block: &str) -> String {
        format!(
            r#"{{"parentUuid":"p","isSidechain":false,"message":{{"model":"m","id":"{message}","type":"message","role":"assistant","content":[{block}]}},"type":"assistant","uuid":"u"}}"#
        )
    }

    fn text(words: &str) -> String {
        format!(r#"{{"type":"text","text":"{words}"}}"#)
    }

    fn call(id: &str) -> String {
        format!(r#"{{"type":"tool_use","id":"{id}","name":"Bash","input":{{"command":"ls"}}}}"#)
    }

    /// Two messages: an English line ahead of two calls made side by side,
    /// then a Japanese one ahead of a third, with the results between.
    fn record() -> String {
        [
            block_line("msg_1", r#"{"type":"thinking","thinking":"","signature":"s"}"#),
            block_line("msg_1", &text("Both pictures are in. Restoring the committed wording now.")),
            block_line("msg_1", &call("toolu_a")),
            block_line("msg_1", &call("toolu_b")),
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_a","content":"ok"}]}}"#.to_string(),
            block_line("msg_2", &text("`gate` を回して、通ったら commit する。")),
            block_line("msg_2", &call("toolu_c")),
        ]
        .join("\n")
    }

    #[test]
    fn tells_english_from_japanese_that_names_english_things() {
        assert!(english(
            "The gate is running (I'm not touching the tree while it does)."
        ));
        assert!(english(
            "Next OpExitRow: putting the `//` comment back above `wordColor`."
        ));
        assert!(!english("gate を背景で回している間は木に触らない。"));
        assert!(!english(
            "Windows と Linux の両方で PASS、`cargo xtask gate` も PASS。[mod.rs](crates/xtask/src/hook/mod.rs:12) を直した。"
        ));
        assert!(!english("rebase → gate → fast-forward の順で land する。"));
    }

    /// Each case is English once the part it drops is counted.
    #[test]
    fn code_paths_and_short_lines_are_not_prose() {
        assert!(!english(
            "```\nerror: the gate failed on worktree-c because the clone was cut short\n```\n直す。"
        ));
        assert!(!english(
            "[一覧](#the-list-of-what-is-left-to-decide-before-the-next-landing-runs-on-main-again-after-the-review-and-the-gate) を直した。"
        ));
        assert!(!english(
            "`cargo xtask land worktree-c --verbose --no-wait`"
        ));
        assert!(!english("All green."));
        assert!(!english(
            "C:/Users/x/IdeaProjects/platitude-gg/crates/xtask/src/hook/language.rs"
        ));
    }

    #[test]
    fn a_line_is_judged_at_the_first_call_of_its_message() {
        let record = record();
        assert_eq!(
            ahead_of(&record, "toolu_a").as_deref(),
            Some("Both pictures are in. Restoring the committed wording now.")
        );
        assert_eq!(ahead_of(&record, "toolu_b"), None);
        assert_eq!(
            ahead_of(&record, "toolu_c").as_deref(),
            Some("`gate` を回して、通ったら commit する。")
        );
        assert_eq!(ahead_of(&record, "toolu_gone"), None);
    }

    #[test]
    fn the_note_comes_from_the_record_the_payload_names() {
        let path = crate::temp::root().join(format!("language-note-{}.jsonl", std::process::id()));
        std::fs::write(&path, record()).expect("write the record");
        let named = path.display().to_string().replace('\\', "\\\\");
        let payload = |call: &str, extra: &str| {
            format!(
                r#"{{"session_id":"s","transcript_path":"{named}","hook_event_name":"PostToolUse","tool_name":"Bash","tool_input":{{"command":"ls"}},"tool_response":{{}},"tool_use_id":"{call}"{extra}}}"#
            )
        };
        let english_line = post_tool(&payload("toolu_a", ""));
        let japanese_line = post_tool(&payload("toolu_c", ""));
        let subagent = post_tool(&payload("toolu_a", r#","agent_id":"a1""#));
        std::fs::remove_file(&path).expect("remove the record");
        assert_eq!(english_line, Some(BETWEEN_CALLS));
        assert_eq!(japanese_line, None);
        assert_eq!(subagent, None);
    }

    #[test]
    fn a_tail_starts_at_a_whole_line() {
        let path = crate::temp::root().join(format!("language-tail-{}.jsonl", std::process::id()));
        std::fs::write(&path, "first line\nsecond\nthird\n").expect("write");
        let read = |bytes| tail(&path.display().to_string(), bytes);
        let (cut, on_a_line, whole) = (read(10), read(6), read(1000));
        std::fs::remove_file(&path).expect("remove");
        assert_eq!(cut.as_deref(), Some("third\n"));
        assert_eq!(on_a_line.as_deref(), Some("third\n"));
        assert_eq!(whole.as_deref(), Some("first line\nsecond\nthird\n"));
    }

    #[test]
    fn an_english_closing_is_asked_again_once() {
        let closing = |active: &str, message: &str| {
            format!(
                r#"{{"session_id":"s","hook_event_name":"Stop","stop_hook_active":{active},"last_assistant_message":"{message}"}}"#
            )
        };
        let english_line =
            "The clone is still running; I'll start the measurement when it finishes.";
        assert_eq!(stop(&closing("false", english_line)), Some(CLOSING));
        assert_eq!(stop(&closing("true", english_line)), None);
        assert_eq!(
            stop(&closing(
                "false",
                "clone がまだ終わらないので、終わったら計測を始める。"
            )),
            None
        );
        assert_eq!(stop(r#"{"session_id":"s","stop_hook_active":false}"#), None);
    }

    #[test]
    fn only_an_english_wrap_up_box_is_refused() {
        let wrap_up = |recap: &str| {
            format!(
                r#"{{"session_id":"s","tool_name":"mcp__ccd_turn__wrap_up","tool_input":{{"recap":"{recap}"}}}}"#
            )
        };
        assert!(pre_tool(&wrap_up(
            "The fix is committed in seat c and the gate passed on it."
        )));
        assert!(!pre_tool(&wrap_up(
            "席 c で直しを commit し、gate が PASS した。"
        )));
        assert!(!pre_tool(
            r#"{"session_id":"s","tool_name":"Bash","tool_input":{"recap":"The fix is committed in seat c and the gate passed."}}"#
        ));
    }
}
