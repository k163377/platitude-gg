//! Compaction and the user's own words. A compaction leaves the
//! conversation with Claude Code's summary of it, and the summary retells
//! what the user said: premises, prohibitions and decisions given early
//! come back paraphrased, or not at all. Before the summary is written
//! (PreCompact) the summarizer is told what to keep; after it
//! (SessionStart `compact`) the user's messages and answers come back as
//! written, read off the session record.
//!
//! Claude Code gives each skill used before the compaction back up to
//! 5,000 tokens (25,000 in all) and names the ones it cut or left out; the
//! note adds where their files are, for a step past what came back.
//!
//! Claude Code cuts a hook's output at 10,000 characters, and the
//! SessionStart note rides with the seat greeting, so the note stays
//! within [`NOTE`]: the longest messages are shortened first, each keeping
//! its start and its end — where the user's own words follow a paste, and
//! where a question's answer stands.

use std::collections::HashSet;
use std::io::BufRead;
use std::path::PathBuf;

use super::payload::{array_items, object_entries, own_string_field, own_value_field};

/// PreCompact: plain stdout joins the instructions the summary is written
/// to.
const SUMMARY: &str = "Beside the summary's own sections, keep:
- In All user messages, each message the user typed, word for word in its own language. A long \
pasted block may be cut to its first lines; say where it was cut.
- A section Standing instructions: every prohibition, premise, decision and correction of the \
user's that still holds, in the user's words, each with the message it came from. One the user \
withdrew stays out, said withdrawn.
- The skill being followed (land, gate, verify-ui, film, review-points or another) and the step \
it stands at.
- Seat letters, branches, commit SHAs and paths exactly as written.
After the compaction the user's messages come back word for word from the session record, beside \
this summary.";

/// The characters the whole SessionStart note may take, leaving room under
/// the hook output's cut for the greeting it rides with.
const NOTE: usize = 7_000;

/// The longest a message may stay, tried in turn until the messages fit.
const CAPS: [usize; 8] = [usize::MAX, 2_000, 1_200, 800, 500, 300, 200, 120];

/// The skill files named at most, the latest loaded.
const SKILLS: usize = 8;

const HEADER: &str = "What the user said in this conversation, word for word from the session \
                      record (UTC). The summary above retells it; where the two differ, these \
                      words hold.";

/// Room kept for the line that says how many messages were left out.
const LEFT_OUT: usize = 120;

/// PreCompact, manual or automatic.
pub(super) fn pre_compact(_input: &str) -> Result<(), String> {
    println!("{SUMMARY}");
    Ok(())
}

/// SessionStart after a compaction: the user's words in this conversation,
/// as written, and the skill files it loaded; `None` when the record holds
/// neither. Every file of the session is read — a session that entered a
/// seat writes beside where it began — and a line copied into more than
/// one counts once.
pub(super) fn restored(input: &str) -> Option<String> {
    let session = own_string_field(input, "session_id").unwrap_or_default();
    let mut files = crate::seats::transcripts(&session);
    let spelled = |path: &PathBuf| path.to_string_lossy().replace('\\', "/").to_lowercase();
    if let Some(named) = own_string_field(input, "transcript_path").map(PathBuf::from)
        && !files.iter().any(|file| spelled(file) == spelled(&named))
    {
        files.push(named);
    }
    let (said, skills) = read(&files);
    told(&said, &skills)
}

#[derive(Debug, PartialEq)]
enum Kind {
    /// Typed at the prompt.
    Typed,
    /// Typed while the session was at work, and handed to it mid-turn.
    WhileWorking,
    /// A choice made in a question the session asked.
    Answer,
    /// The first message, when the app wrote it — a task another session
    /// handed over.
    Began,
}

#[derive(Debug)]
struct Said {
    at: String,
    kind: Kind,
    text: String,
}

/// What the user said, in time order, and the skill files the
/// conversation loaded, in the order it first did. Only lines with a
/// `uuid` are the conversation's; the request records and the queue's
/// bookkeeping carry none.
fn read(files: &[PathBuf]) -> (Vec<Said>, Vec<String>) {
    let mut seen = HashSet::new();
    let mut said = Vec::new();
    let mut first: Option<Said> = None;
    let mut skills = Vec::new();
    for file in files {
        let Ok(handle) = std::fs::File::open(file) else {
            continue;
        };
        for line in std::io::BufReader::new(handle)
            .lines()
            .map_while(Result::ok)
        {
            // A record runs to hundreds of megabytes, nearly all of it
            // tool results; only a result that carries answers is read.
            let wanted = line.contains("\"queued_command\"")
                || (line.contains("\"type\":\"user\"")
                    && (!line.contains("\"tool_result\"") || line.contains("\"answers\":")));
            if !wanted {
                continue;
            }
            let Some(uuid) = own_string_field(&line, "uuid") else {
                continue;
            };
            if !seen.insert(uuid) {
                continue;
            }
            let at = own_string_field(&line, "timestamp").unwrap_or_default();
            match own_string_field(&line, "type").as_deref() {
                Some("user") => {
                    user_line(&line, at, &mut said, &mut first, &mut skills);
                }
                Some("attachment") => said.extend(queued(&line, at)),
                _ => {}
            }
        }
    }
    if let Some(began) = first.filter(|began| began.kind == Kind::Began) {
        said.push(began);
    }
    said.sort_by(|a, b| a.at.cmp(&b.at));
    (said, skills)
}

/// A user line: a message the user typed, a question's answers, a skill
/// the conversation loaded, or the first message of all.
fn user_line(
    line: &str,
    at: String,
    said: &mut Vec<Said>,
    first: &mut Option<Said>,
    skills: &mut Vec<String>,
) {
    if own_value_field(line, "isSidechain") == Some("true")
        || own_value_field(line, "isCompactSummary") == Some("true")
    {
        return;
    }
    if let Some(answers) =
        own_value_field(line, "toolUseResult").and_then(|result| own_value_field(result, "answers"))
    {
        for (question, answer) in object_entries(answers) {
            if let (Some(question), Some(answer)) = (decoded(question), decoded(answer)) {
                said.push(Said {
                    at: at.clone(),
                    kind: Kind::Answer,
                    text: format!("{question} → {answer}"),
                });
            }
        }
        return;
    }
    let Some(text) = own_value_field(line, "message")
        .and_then(|message| own_value_field(message, "content"))
        .and_then(content_text)
    else {
        return;
    };
    if let Some(base) = text.strip_prefix("Base directory for this skill: ") {
        let dir = base.lines().next().unwrap_or_default().trim();
        let skill = format!("{}/SKILL.md", dir.replace('\\', "/"));
        if !skills.contains(&skill) {
            skills.push(skill);
        }
        return;
    }
    let human = own_string_field(line, "turnOrigin").as_deref() == Some("human");
    // The app's own lines open with a tag; what the user typed may too —
    // a slash command, a paste, or a reminder the app put ahead of it.
    if own_value_field(line, "isMeta") == Some("true")
        || (!human && text.trim_start().starts_with('<'))
    {
        return;
    }
    let text = if human { spoken(&text) } else { text };
    if text.is_empty() {
        return;
    }
    if first.as_ref().is_none_or(|earliest| at < earliest.at) {
        *first = Some(Said {
            at: at.clone(),
            kind: if human { Kind::Typed } else { Kind::Began },
            text: text.clone(),
        });
    }
    if human {
        said.push(Said {
            at,
            kind: Kind::Typed,
            text,
        });
    }
}

/// What the user wrote in a typed message: the reminders the app put into
/// it left out, and a slash command as it was typed.
fn spoken(text: &str) -> String {
    const OPEN: &str = "<system-reminder>";
    const CLOSE: &str = "</system-reminder>";
    let mut rest = text.to_string();
    while let Some(start) = rest.find(OPEN) {
        let Some(length) = rest[start..].find(CLOSE) else {
            break;
        };
        rest.replace_range(start..start + length + CLOSE.len(), "");
    }
    let rest = rest.trim();
    if rest.starts_with("<command-message>") || rest.starts_with("<command-name>") {
        let name = between(rest, "<command-name>", "</command-name>").unwrap_or_default();
        let args = between(rest, "<command-args>", "</command-args>").unwrap_or_default();
        return format!("{name} {args}").trim().to_string();
    }
    rest.to_string()
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let end = start + text[start..].find(close)?;
    Some(&text[start..end])
}

/// A message the user typed while the session worked; the app queues
/// notifications and other sessions' messages the same way, under another
/// origin.
fn queued(line: &str, at: String) -> Option<Said> {
    let attachment = own_value_field(line, "attachment")?;
    if own_string_field(attachment, "type").as_deref() != Some("queued_command") {
        return None;
    }
    let origin = own_value_field(attachment, "origin")?;
    if own_string_field(origin, "kind").as_deref() != Some("human") {
        return None;
    }
    let text = spoken(&content_text(own_value_field(attachment, "prompt")?)?);
    (!text.is_empty()).then_some(Said {
        at,
        kind: Kind::WhileWorking,
        text,
    })
}

/// A message's text, from content written as a string or as blocks; `None`
/// for a tool's result.
fn content_text(content: &str) -> Option<String> {
    if content.starts_with('"') {
        return decoded(content);
    }
    let mut text = Vec::new();
    for block in array_items(content) {
        match own_string_field(block, "type").as_deref() {
            Some("text") => text.extend(own_value_field(block, "text").and_then(decoded)),
            Some("tool_result") => return None,
            _ => {}
        }
    }
    (!text.is_empty()).then(|| text.join("\n"))
}

/// A JSON string, quotes and all, decoded whole: the user's words are
/// given back as written, which the payload reader's path-minded unescape
/// does not promise.
fn decoded(written: &str) -> Option<String> {
    let body = written.strip_prefix('"')?.strip_suffix('"')?;
    let mut text = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            text.push(c);
            continue;
        }
        match chars.next()? {
            'n' => text.push('\n'),
            't' => text.push('\t'),
            'r' => {}
            'b' => text.push('\u{8}'),
            'f' => text.push('\u{c}'),
            'u' => {
                let unit = hex4(&mut chars)?;
                let code = if (0xD800..0xDC00).contains(&unit) {
                    let (Some('\\'), Some('u')) = (chars.next(), chars.next()) else {
                        return None;
                    };
                    let low = hex4(&mut chars)?;
                    0x10000 + ((unit - 0xD800) << 10) + low.checked_sub(0xDC00)?
                } else {
                    unit
                };
                text.push(char::from_u32(code)?);
            }
            other => text.push(other),
        }
    }
    Some(text)
}

fn hex4(chars: &mut std::str::Chars<'_>) -> Option<u32> {
    let digits: String = chars.take(4).collect();
    if digits.len() != 4 {
        return None;
    }
    u32::from_str_radix(&digits, 16).ok()
}

/// The note given after a compaction, within [`NOTE`]; `None` when there
/// is nothing to give.
fn told(said: &[Said], skills: &[String]) -> Option<String> {
    if said.is_empty() && skills.is_empty() {
        return None;
    }
    let skills_line = (!skills.is_empty()).then(|| {
        let latest = &skills[skills.len().saturating_sub(SKILLS)..];
        format!(
            "Skill files this conversation loaded before the compaction: {}. When a step you are \
             on lies past what came back of its skill, read the file again.",
            latest.join(", ")
        )
    });
    let mut note = Vec::new();
    if !said.is_empty() {
        let fixed = HEADER.chars().count()
            + 1
            + LEFT_OUT
            + skills_line
                .as_ref()
                .map_or(0, |line| line.chars().count() + 1);
        note.push(HEADER.to_string());
        let (entries, left_out) = fitted(said, NOTE.saturating_sub(fixed));
        note.extend(entries);
        if left_out > 0 {
            note.push(format!(
                "({left_out} messages between the first and these are left out here for length; \
                 the session record has them.)"
            ));
        }
    }
    note.extend(skills_line);
    Some(note.join("\n"))
}

/// The entries at the longest cap under which all of them fit `room`
/// characters; past the shortest cap, the first message and as many of
/// the latest as fit, with how many were left out between.
fn fitted(said: &[Said], room: usize) -> (Vec<String>, usize) {
    let length = |entries: &[String]| entries.iter().map(|e| e.chars().count() + 1).sum::<usize>();
    let mut shortest = Vec::new();
    for cap in CAPS {
        let entries = said.iter().map(|s| entry(s, cap)).collect::<Vec<_>>();
        if length(&entries) <= room {
            return (entries, 0);
        }
        shortest = entries;
    }
    let Some((head, rest)) = shortest.split_first() else {
        return (Vec::new(), 0);
    };
    let mut room = room.saturating_sub(length(std::slice::from_ref(head)));
    let mut latest = Vec::new();
    for entry in rest.iter().rev() {
        let cost = entry.chars().count() + 1;
        if cost > room {
            break;
        }
        room -= cost;
        latest.push(entry.clone());
    }
    let left_out = rest.len() - latest.len();
    latest.reverse();
    let mut kept = vec![head.clone()];
    kept.extend(latest);
    (kept, left_out)
}

/// One message as a list item, cut to about `cap` characters: a quarter
/// from its start, the rest from its end.
fn entry(said: &Said, cap: usize) -> String {
    let when = said.at.get(5..16).unwrap_or(&said.at).replace('T', " ");
    let label = match said.kind {
        Kind::Typed => when,
        Kind::WhileWorking => format!("{when}, sent while you worked"),
        Kind::Answer => format!("{when}, answer to your question"),
        Kind::Began => format!("{when}, the message this conversation began with"),
    };
    let length = said.text.chars().count();
    let text = if length > cap {
        let head = cap / 4;
        let tail = cap - head;
        let start: String = said.text.chars().take(head).collect();
        let end: String = said.text.chars().skip(length - tail).collect();
        format!("{start} […{} characters cut…] {end}", length - cap)
    } else {
        said.text.clone()
    };
    format!("- [{label}] {}", text.trim().replace('\n', "\n  "))
}

#[cfg(test)]
mod tests {
    use super::{
        CAPS, Kind, NOTE, SUMMARY, Said, decoded, entry, fitted, read, restored, spoken, told,
    };

    fn line(fields: &str) -> String {
        format!("{{{fields}}}\n")
    }

    fn record() -> String {
        [
            line(
                r#""parentUuid":null,"isSidechain":false,"type":"user","message":{"role":"user","content":"Background (from the session that started this one): fix the menu"},"uuid":"u0","timestamp":"2026-11-22T05:00:00.000Z","turnOrigin":"sdk""#,
            ),
            line(
                r#""parentUuid":"u0","isSidechain":false,"promptId":"p1","type":"user","message":{"role":"user","content":"指示するまで修正は禁止\nまず rebase"},"uuid":"u1","timestamp":"2026-11-22T05:01:00.000Z","turnOrigin":"human""#,
            ),
            // A request record carries a message but no uuid.
            line(
                r#""type":"api-request-blob","sessionId":"s","hash":"h","message":{"type":"user","role":"user","content":[{"type":"text","text":"a copy in a request record"}]}"#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":[{"type":"text","text":"Base directory for this skill: C:\\w\\.claude\\skills\\land\n\n# Land"}]},"isMeta":true,"uuid":"u2","timestamp":"2026-11-22T05:02:00.000Z""#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"done"}]},"uuid":"u3","timestamp":"2026-11-22T05:03:00.000Z","toolUseResult":{"questions":[{"question":"どちらにしますか？"}],"answers":{"どちらにしますか？":"A で"}}"#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t2","content":"ok"}]},"uuid":"u4","timestamp":"2026-11-22T05:04:00.000Z""#,
            ),
            line(
                r#""isSidechain":false,"attachment":{"type":"queued_command","prompt":"<task-notification>done</task-notification>","origin":{"kind":"task-notification"}},"type":"attachment","uuid":"u5","timestamp":"2026-11-22T05:05:00.000Z""#,
            ),
            line(
                r#""isSidechain":false,"attachment":{"type":"queued_command","prompt":"（要するに 600k で）","commandMode":"prompt","origin":{"kind":"human"}},"type":"attachment","uuid":"u6","timestamp":"2026-11-22T05:06:00.000Z""#,
            ),
            // The summary is the earliest line of a file that starts at a
            // compaction, and is never the first message.
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":"This session is being continued from a previous conversation."},"isCompactSummary":true,"uuid":"u7","timestamp":"2026-11-22T04:59:00.000Z""#,
            ),
            line(
                r#""isSidechain":true,"type":"user","message":{"role":"user","content":"a subagent's prompt"},"uuid":"u8","timestamp":"2026-11-22T05:08:00.000Z","turnOrigin":"human""#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":[{"type":"text","text":"\u30c6\u30b9\u30c8 \"quoted\" \ud83d\ude00"}]},"uuid":"u9","timestamp":"2026-11-22T05:09:00.000Z","turnOrigin":"human""#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":"<side-sessions-event>a side session ended</side-sessions-event>"},"uuid":"u10","timestamp":"2026-11-22T05:10:00.000Z","turnOrigin":"sdk""#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":"<pasted_content id=\"1\">review</pasted_content>\nこれを直して"},"uuid":"u11","timestamp":"2026-11-22T05:11:00.000Z","turnOrigin":"human""#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":[{"type":"text","text":"<system-reminder>\nYou are a session started from another session\n</system-reminder>"},{"type":"text","text":"rebase して OK"}]},"uuid":"u12","timestamp":"2026-11-22T05:12:00.000Z","turnOrigin":"human""#,
            ),
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":"<command-message>land</command-message>\n<command-name>/land</command-name>\n<command-args>push</command-args>"},"uuid":"u13","timestamp":"2026-11-22T05:13:00.000Z","turnOrigin":"human""#,
            ),
        ]
        .concat()
    }

    #[test]
    fn the_user_s_words_come_back_from_every_file_of_the_session_once() {
        let yard = crate::yard::Yard::new("compact-words");
        let first = yard.join("first.jsonl");
        let second = yard.join("second.jsonl");
        std::fs::write(&first, record()).expect("write the record");
        // The seat's file repeats a line of the first under its own uuid.
        std::fs::write(
            &second,
            line(
                r#""isSidechain":false,"type":"user","message":{"role":"user","content":"指示するまで修正は禁止\nまず rebase"},"uuid":"u1","timestamp":"2026-11-22T05:01:00.000Z","turnOrigin":"human""#,
            ),
        )
        .expect("write the copy");
        let (said, skills) = read(&[first, second]);
        let got = said
            .iter()
            .map(|s| (&s.kind, s.text.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            got,
            vec![
                (
                    &Kind::Began,
                    "Background (from the session that started this one): fix the menu"
                ),
                (&Kind::Typed, "指示するまで修正は禁止\nまず rebase"),
                (&Kind::Answer, "どちらにしますか？ → A で"),
                (&Kind::WhileWorking, "（要するに 600k で）"),
                (&Kind::Typed, "テスト \"quoted\" 😀"),
                (
                    &Kind::Typed,
                    "<pasted_content id=\"1\">review</pasted_content>\nこれを直して"
                ),
                (&Kind::Typed, "rebase して OK"),
                (&Kind::Typed, "/land push"),
            ]
        );
        assert_eq!(
            skills,
            vec!["C:/w/.claude/skills/land/SKILL.md".to_string()]
        );
    }

    #[test]
    fn a_compaction_start_names_the_record_in_its_payload() {
        let yard = crate::yard::Yard::new("compact-restored");
        let path = yard.join("record.jsonl");
        std::fs::write(&path, record()).expect("write the record");
        let named = path.display().to_string().replace('\\', "\\\\");
        let payload = format!(
            r#"{{"session_id":"compact-test-no-such-session","transcript_path":"{named}","source":"compact"}}"#
        );
        let note = restored(&payload).expect("a note");
        assert!(
            note.contains("- [11-22 05:01] 指示するまで修正は禁止\n  まず rebase"),
            "{note}"
        );
        assert!(
            note.contains("sent while you worked] （要するに 600k で）"),
            "{note}"
        );
        assert!(note.contains("C:/w/.claude/skills/land/SKILL.md"), "{note}");
        assert!(!note.contains("a copy in a request record"), "{note}");
        assert!(!note.contains("task-notification"), "{note}");
        assert!(!note.contains("a subagent's prompt"), "{note}");
        assert!(!note.contains("being continued"), "{note}");
        assert!(!note.contains("system-reminder"), "{note}");
        let empty = format!(
            r#"{{"session_id":"compact-test-no-such-session","transcript_path":"{}","source":"compact"}}"#,
            yard.join("none.jsonl")
                .display()
                .to_string()
                .replace('\\', "\\\\")
        );
        assert_eq!(restored(&empty), None);
    }

    #[test]
    fn long_words_are_cut_longest_first_until_the_note_fits() {
        let said = |count: usize, each: usize| {
            (0..count)
                .map(|n| Said {
                    at: format!("2026-11-22T05:{:02}:00.000Z", n % 60),
                    kind: Kind::Typed,
                    text: format!("{n}:{}", "語".repeat(each)),
                })
                .collect::<Vec<_>>()
        };
        let skills = (0..12)
            .map(|n| format!("C:/w/.claude/skills/skill-{n}/SKILL.md"))
            .collect::<Vec<_>>();
        let short = said(3, 50);
        let (entries, left_out) = fitted(&short, NOTE);
        assert_eq!((entries.len(), left_out), (3, 0));
        assert!(entries.iter().all(|e| !e.contains("characters cut")));
        let long = said(10, 3_000);
        let note = told(&long, &skills).expect("a note");
        assert!(note.chars().count() <= NOTE, "{}", note.chars().count());
        assert_eq!(note.matches("\n- [").count(), 10);
        assert!(note.contains("characters cut…]"), "{note}");
        assert!(note.contains("skill-11/SKILL.md") && !note.contains("skill-3/"));
        // Past the shortest cap: the first and the latest that fit.
        let many = said(NOTE / CAPS[CAPS.len() - 1], 3_000);
        let note = told(&many, &skills).expect("a note");
        assert!(note.chars().count() <= NOTE, "{}", note.chars().count());
        assert!(note.contains("\n- [11-22 05:00] 0:"), "{note}");
        assert!(
            note.contains(" messages between the first and these"),
            "{note}"
        );
    }

    #[test]
    fn a_cut_message_keeps_its_start_and_its_end() {
        let pasted = Said {
            at: "2026-11-22T05:00:00.000Z".to_string(),
            kind: Kind::Typed,
            text: format!(
                "<pasted_content>{}</pasted_content>\nここを直して",
                "x".repeat(5_000)
            ),
        };
        let cut = entry(&pasted, 400);
        assert!(
            cut.contains("<pasted_content>") && cut.ends_with("\n  ここを直して"),
            "{cut}"
        );
        assert!(cut.chars().count() < 500, "{cut}");
        let answered = Said {
            at: "2026-11-22T05:00:00.000Z".to_string(),
            kind: Kind::Answer,
            text: format!("{}？ → A で", "長い問い".repeat(80)),
        };
        assert!(entry(&answered, 120).ends_with("→ A で"));
    }

    #[test]
    fn a_typed_message_loses_the_app_s_additions() {
        assert_eq!(
            spoken(
                "<system-reminder>\nnote\n</system-reminder>\n\n直して<system-reminder>x</system-reminder>"
            ),
            "直して"
        );
        assert_eq!(
            spoken("<command-message>land</command-message>\n<command-name>/land</command-name>"),
            "/land"
        );
        assert_eq!(spoken("<system-reminder>only</system-reminder>"), "");
    }

    #[test]
    fn strings_decode_whole() {
        assert_eq!(
            decoded(r#""a\r\nb\t\"c\"\\d\/e""#).as_deref(),
            Some("a\nb\t\"c\"\\d/e")
        );
        assert_eq!(decoded(r#""\u65e5\ud83d\ude00""#).as_deref(), Some("日😀"));
        assert_eq!(decoded(r#""\ud83d""#), None);
        assert_eq!(decoded("not quoted"), None);
    }

    #[test]
    fn the_summary_s_instructions_stay_under_the_hook_output_cut() {
        assert!(SUMMARY.chars().count() < 10_000 / 4);
    }
}
