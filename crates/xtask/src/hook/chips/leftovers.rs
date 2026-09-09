//! What a turn says, in its own words, that it is leaving behind.
//!
//! Undone work has three places to go and they do not cost the same.
//! Finishing it in the turn that found it costs a turn — the seat is
//! entered and the files are open. Writing it into the 確認事項 doc
//! costs a line, in the list the user reads back. Stacking it as a chip
//! costs a whole session on a seat of its own, rebasing onto a main that
//! moved. The last kept being the first answer rather than the last,
//! until a session's own verification and cleanup were going out as
//! chips for other sessions to redo; and a sentence at the end of a
//! reply, which is none of the three, was missed altogether.
//!
//! So the reply is read back before the turn ends. When it says work is
//! undone — 未対応, 残件, 見送り, TODO — and the turn neither finished it,
//! wrote the doc nor stacked a chip, the turn does not end. One line
//! saying why this leftover needs none of them ends it too, because a
//! Stop block is not asked twice, so a sentence read wrong here costs a
//! line rather than a session.

use std::io::{Read, Seek, SeekFrom};

use crate::hook::payload::{printable, string_field};

/// The words a reply leaves work behind in.
const LEFTOVERS: [&str; 22] = [
    "未対応",
    "未着手",
    "未実装",
    "未配線",
    "未検証",
    "未確認",
    "未解決",
    "残作業",
    "残件",
    "積み残し",
    "手つかず",
    "やり残",
    "後回し",
    "保留",
    "見送",
    "別途",
    "要検討",
    "要判断",
    "TODO",
    "別セッション",
    "次のセッション",
    "後続セッション",
];

/// The words that turn one of those into work that is done: 未配線の操作
/// なし and 残件は無い carry a marker and say its opposite. Bare ない is
/// not among them on purpose — 直していない ends in it and means the
/// marker.
const SETTLED: [&str; 12] = [
    "なし",
    "無し",
    "はない",
    "は無い",
    "もない",
    "ありません",
    "ゼロ",
    "解消",
    "済み",
    "済んだ",
    "0 件",
    "0件",
];

/// How far past a marker one of those still answers it. A sentence
/// cancels its marker where it stands, not from the end of a paragraph.
const REACH: usize = 16;

/// How much of the record is read back. The turn is at the end of it and
/// a session's transcript runs to megabytes.
const WINDOW: u64 = 1 << 20;

/// The record of the user's own words: what the turn is measured back
/// to. A reply that spells this out is not mistaken for the thing
/// itself — the harness escapes the quotes of anything it quotes.
const HUMAN: &str = "\"origin\":{\"kind\":\"human\"}";

/// The same, for the call that stacks a chip.
const SPAWN: &str = "\"name\":\"mcp__ccd_session__spawn_task\"";

/// The tools that write a file, by the name a record calls them.
const WRITERS: [&str; 2] = ["\"name\":\"Write\"", "\"name\":\"Edit\""];

/// The doc a leftover goes into when it is written down rather than
/// spawned: `internal-docs/P<n>-確認事項.md`.
const DOC: &str = "確認事項.md";

/// Where one call's own half of a record ends: the next call in it. A
/// turn that read the doc and wrote something else carries both words in
/// one record otherwise.
const NEXT_CALL: &str = "\"type\":\"tool_use\"";

/// Stop: a turn does not end leaving work behind in prose alone.
pub(crate) fn stop(input: &str) -> Result<bool, String> {
    let Some(path) = string_field(input, "transcript_path") else {
        return Ok(false);
    };
    let Some(record) = tail(&path) else {
        return Ok(false);
    };
    let lines: Vec<&str> = record.lines().collect();
    // One chip or one write to the doc anywhere in the turn is enough:
    // the turn was already putting its leftovers somewhere, and which
    // one belongs where is a judgement the guard has no way to make.
    if answered(&lines) {
        return Ok(false);
    }
    let found = leftovers(&reply(&lines));
    if found.is_empty() {
        return Ok(false);
    }
    println!(
        "{{\"decision\":\"block\",\"reason\":\"{}\"}}",
        printable(&reason(&found))
    );
    Ok(true)
}

/// What the block says.
fn reason(found: &[String]) -> String {
    format!(
        "This reply leaves work behind in prose and did nothing else with it: {}. A \
         sentence at the end of a turn is read as it scrolls past and then gone. \
         There are three places it can go, in this order. Finish it here: the seat \
         is entered and the files are open, and the verification of a change, the \
         cleanup beside it and the fix it asks for next are this session's own \
         work, not a session's worth of somebody else's. Write it down: \
         internal-docs/P3-確認事項.md (P5 for what waits on distribution) is the \
         list the user reads back, and it costs no seat. Stack a chip only for \
         what cannot be done here at all — another machine, a real window, a \
         decision this session cannot get — because a chip is a whole session on a \
         seat of its own, rebasing onto a main that moved, and the user has asked \
         for fewer of them (mcp__ccd_session__spawn_task, title '<n>. …' with the \
         live set renumbered around where it belongs — or '<n>. [任意] …' when it \
         asks the user to decide rather than recommending work — written in \
         Japanese, its prompt naming the paths it will touch). Say in the reply \
         which of the three each leftover got. If one needs none of them — a live \
         chip already holds it, it is a question this conversation is waiting on \
         the user to answer, or it is not undone work at all — say which in one \
         line instead: this is asked once, so that answer ends the turn.",
        found
            .iter()
            .map(|sentence| format!("'{sentence}'"))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

/// The end of the record, cut to whole lines. A window rather than the
/// file because only this turn is being read, and the file holds the
/// session.
fn tail(path: &str) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let mut text = String::new();
    if len <= WINDOW {
        file.read_to_string(&mut text).ok()?;
        return Some(text);
    }
    file.seek(SeekFrom::Start(len - WINDOW)).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    // The cut lands mid-record as readily as mid-character, and both are
    // dropped by keeping only what follows the first newline.
    let from = bytes.iter().position(|byte| *byte == b'\n')? + 1;
    Some(String::from_utf8_lossy(&bytes[from..]).into_owned())
}

/// Whether this turn already put its leftovers somewhere: a chip
/// stacked, or the 確認事項 doc written. The turn reaches back to the
/// last thing the user typed; everything between is the harness and the
/// assistant.
fn answered(lines: &[&str]) -> bool {
    for line in lines.iter().rev() {
        if line.contains(HUMAN) {
            return false;
        }
        if line.contains(SPAWN) || wrote_the_doc(line) {
            return true;
        }
    }
    false
}

/// Whether this record wrote the doc. A tool's name comes right before
/// the input it belongs to, so the doc is looked for between a writer's
/// name and the next call rather than anywhere in the record — reading
/// the doc is not writing it, and the two calls sit in one record
/// readily enough.
fn wrote_the_doc(line: &str) -> bool {
    WRITERS
        .iter()
        .flat_map(|writer| line.match_indices(writer))
        .any(|(at, writer)| {
            let after = &line[at + writer.len()..];
            after.split(NEXT_CALL).next().unwrap_or(after).contains(DOC)
        })
}

/// The text the user is left looking at: every block the assistant spoke
/// after the last record that was not its own. Records of other kinds —
/// a hook's summary, an attachment, the session's bookkeeping — sit
/// between them and say nothing about the reply.
fn reply(lines: &[&str]) -> String {
    let mut spoken: Vec<String> = Vec::new();
    for line in lines.iter().rev() {
        if line.contains("\"type\":\"user\"") {
            break;
        }
        if line.contains("\"type\":\"assistant\"") {
            spoken.push(texts(line).join("\n"));
        }
    }
    spoken.reverse();
    spoken.join("\n")
}

/// Every text block in one record, decoded. Thinking carries its own key
/// and a tool call's arguments are on the far side of a tool result, so
/// the only `"text":` in a trailing assistant record is the reply's own.
fn texts(line: &str) -> Vec<String> {
    const KEY: &str = "\"text\":\"";
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find(KEY) {
        let body = &rest[at + KEY.len()..];
        let (text, len) = unescape(body);
        found.push(text);
        rest = &body[len..];
    }
    found
}

/// One JSON string body up to its closing quote: the text, and the bytes
/// it ran for. `\uXXXX` becomes a space — the harness writes its text as
/// raw UTF-8, so what arrives escaped is a control character, and no word
/// is made or unmade by one.
fn unescape(body: &str) -> (String, usize) {
    let mut text = String::new();
    let mut chars = body.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => return (text, at + 1),
            '\\' => match chars.next() {
                Some((_, 'n' | 'r')) => text.push('\n'),
                Some((_, 't')) => text.push('\t'),
                Some((_, 'u')) => {
                    for _ in 0..4 {
                        chars.next();
                    }
                    text.push(' ');
                }
                Some((_, other)) => text.push(other),
                None => break,
            },
            other => text.push(other),
        }
    }
    (text, body.len())
}

/// The sentences of a reply that leave work behind, as the user reads
/// them. A few is enough to name what was caught.
fn leftovers(reply: &str) -> Vec<String> {
    prose(reply)
        .flat_map(|part| part.split(['\n', '。']))
        .filter(|sentence| unfinished(sentence))
        .map(quote)
        .take(3)
        .collect()
}

/// The reply with everything it quotes dropped — a fenced block, and a
/// span between backticks. What is inside either is shown, not said: a
/// command, a file, a diff, or the very word this guard reads for, in a
/// reply explaining the guard.
fn prose(reply: &str) -> impl Iterator<Item = &str> {
    reply
        .split("```")
        .step_by(2)
        .flat_map(|part| part.split('`').step_by(2))
}

/// Whether one sentence says work is undone.
fn unfinished(sentence: &str) -> bool {
    LEFTOVERS.iter().any(|marker| {
        sentence
            .match_indices(marker)
            .any(|(at, _)| !settled(&sentence[at + marker.len()..]))
    })
}

fn settled(after: &str) -> bool {
    let reach: String = after.chars().take(REACH).collect();
    SETTLED.iter().any(|word| reach.contains(word))
}

/// One sentence, trimmed to what fits in a refusal.
fn quote(sentence: &str) -> String {
    let sentence = sentence.trim();
    let short: String = sentence.chars().take(60).collect();
    if short.chars().count() < sentence.chars().count() {
        format!("{short}…")
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::{answered, leftovers, quote, reply, unfinished};

    /// One record of each kind, in the shape the harness writes them.
    fn human(text: &str) -> String {
        format!(
            "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"{text}\"}},\
             \"origin\":{{\"kind\":\"human\"}}}}"
        )
    }

    fn said(text: &str) -> String {
        format!(
            "{{\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\
             \"text\":\"{text}\"}}]}},\"type\":\"assistant\"}}"
        )
    }

    fn called(name: &str) -> String {
        format!(
            "{{\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"tool_use\",\
             \"name\":\"{name}\",\"input\":{{}}}}]}},\"type\":\"assistant\"}}"
        )
    }

    /// One call over a file, as a record of its own.
    fn on_file(name: &str, path: &str) -> String {
        format!(
            "{{\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"tool_use\",\
             \"name\":\"{name}\",\"input\":{{\"file_path\":\"{path}\"}}}}]}},\
             \"type\":\"assistant\"}}"
        )
    }

    /// One record holding two calls: the doc read, another file written.
    fn read_the_doc_and_wrote_elsewhere() -> String {
        "{\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"tool_use\",\
         \"name\":\"Edit\",\"input\":{\"file_path\":\"crates/x/src/refs.rs\"}},\
         {\"type\":\"tool_use\",\"name\":\"Read\",\"input\":\
         {\"file_path\":\"internal-docs/P3-確認事項.md\"}}]},\"type\":\"assistant\"}"
            .to_string()
    }

    fn result() -> String {
        "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\
         [{\"type\":\"tool_result\",\"content\":\"ok\"}]}}"
            .to_string()
    }

    fn borrow(lines: &[String]) -> Vec<&str> {
        lines.iter().map(String::as_str).collect()
    }

    #[test]
    fn reads_back_what_was_spoken_after_the_last_tool_result() {
        let lines = [
            human("直して"),
            said("直す前に読む"),
            called("Bash"),
            result(),
            "{\"type\":\"system\",\"subtype\":\"stop_hook_summary\"}".to_string(),
            said("直した"),
            said("残りは 未対応 のまま"),
            "{\"type\":\"last-prompt\",\"lastPrompt\":\"直して\"}".to_string(),
        ];
        assert_eq!(reply(&borrow(&lines)), "直した\n残りは 未対応 のまま");
    }

    #[test]
    fn a_turn_that_put_its_leftovers_somewhere_has_answered_already() {
        let stacked = [
            human("直して"),
            called("mcp__ccd_session__spawn_task"),
            result(),
            said("1 件はチップにした"),
        ];
        let written = [
            human("直して"),
            on_file("Edit", "internal-docs/P3-確認事項.md"),
            result(),
            said("1 件は確認事項へ書いた"),
        ];
        let read_it = [
            human("直して"),
            on_file("Read", "internal-docs/P3-確認事項.md"),
            result(),
            said("未対応が 1 件"),
        ];
        let bare = [
            human("直して"),
            called("Bash"),
            result(),
            said("未対応が 1 件"),
        ];
        let quoted = [
            human("mcp__ccd_session__spawn_task の話"),
            said("未対応が 1 件"),
        ];
        assert!(answered(&borrow(&stacked)));
        assert!(answered(&borrow(&written)));
        // Reading the doc is not writing it — and neither is writing
        // something else in the record that read it.
        assert!(!answered(&borrow(&read_it)));
        assert!(!answered(&borrow(&[
            human("直して"),
            read_the_doc_and_wrote_elsewhere(),
            said("未対応が 1 件"),
        ])));
        assert!(!answered(&borrow(&bare)));
        // The user's own words are where the turn stops, whatever they
        // spell — a prompt about the tool is not a call to it.
        assert!(!answered(&borrow(&quoted)));
    }

    #[test]
    fn takes_a_leftover_and_leaves_the_sentence_that_settles_it() {
        assert!(unfinished("diff の横スクロールは未対応"));
        assert!(unfinished("PageUp/Down は別セッションで直す"));
        assert!(unfinished("TODO: 索引を張り直す"));
        // The marker is answered by what follows it, and only nearby.
        assert!(!unfinished("未配線の操作なし"));
        assert!(!unfinished("残件は無い"));
        assert!(unfinished("残件は無いと書いたが、まだ 1 件が要判断"));
        // ない at the end of a verb is not an answer to anything.
        assert!(unfinished("未対応: ハイライトを直していない"));
    }

    #[test]
    fn a_fence_is_quoted_rather_than_said() {
        let spoken = "直した\n```\n// TODO: 索引\n```\nテストは緑";
        assert!(leftovers(spoken).is_empty());
        assert_eq!(
            leftovers("直した。要判断が 1 件ある。緑"),
            vec!["要判断が 1 件ある".to_string()]
        );
    }

    #[test]
    fn a_backticked_word_is_shown_rather_than_said() {
        // The reply that explains the guard names the words it reads
        // for, and naming one is not leaving work behind.
        assert!(leftovers("`[要判断]` のような自作タグは deny する").is_empty());
        assert!(leftovers("拾う語は `未対応` から `TODO` まで").is_empty());
        // The sentence around the quote is still read.
        assert_eq!(
            leftovers("`未対応` を拾う。アバターの縮小は要判断"),
            vec!["アバターの縮小は要判断".to_string()]
        );
    }

    #[test]
    fn quotes_a_sentence_short_enough_to_read_in_a_refusal() {
        assert_eq!(quote("  未対応が 1 件  "), "未対応が 1 件");
        let long = "未対応".to_string() + &"あ".repeat(80);
        assert_eq!(quote(&long).chars().count(), 61);
        assert!(quote(&long).ends_with('…'));
    }
}
