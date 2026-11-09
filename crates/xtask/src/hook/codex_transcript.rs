//! Desktop transcripts are a fallback, not a stable hook interface. Only
//! a root user's message with explicit provenance is accepted; unknown
//! or incomplete records stop landing rather than reuse older consent.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::payload::{own_string_field as string, own_value_field as value};
use crate::hook::payload::array_items;

pub(super) fn latest(sessions: &Path, session: &str) -> Result<(String, String, u64), String> {
    if session.is_empty()
        || !session
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("invalid Codex thread id".into());
    }
    let mut paths = Vec::new();
    find(sessions, &format!("-{session}.jsonl"), 4, &mut paths)?;
    let [path] = paths.as_slice() else {
        return Err("expected one local Codex transcript for this thread".into());
    };
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    read(BufReader::new(file), session)
}

fn find(dir: &Path, suffix: &str, depth: u8, found: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() && depth > 0 {
            find(&entry.path(), suffix, depth - 1, found)?;
        } else if kind.is_file() && entry.file_name().to_string_lossy().ends_with(suffix) {
            found.push(entry.path());
        }
    }
    Ok(())
}

fn read(mut source: impl BufRead, session: &str) -> Result<(String, String, u64), String> {
    let mut line = String::new();
    let mut identity = false;
    let mut latest = None;
    loop {
        line.clear();
        if source.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        if !line.ends_with('\n') {
            return Err(
                "Codex transcript has an incomplete record; retry after it is written".into(),
            );
        }
        let kind = string(&line, "type").ok_or("unknown Codex transcript record")?;
        if !matches!(kind.as_str(), "session_meta" | "response_item") {
            continue;
        }
        let payload = value(&line, "payload").ok_or("missing Codex transcript payload")?;
        if kind == "session_meta" {
            identity = string(payload, "id").as_deref() == Some(session)
                && string(payload, "thread_source").as_deref() == Some("user");
            if !identity {
                return Err("Codex transcript is not this root user's thread".into());
            }
        } else if kind == "response_item" && string(payload, "role").as_deref() == Some("user") {
            let meta = value(payload, "internal_chat_message_metadata_passthrough")
                .ok_or("unsupported Codex user record: missing provenance")?;
            let items =
                array_items(value(meta, "content_item_metadata").ok_or("missing user provenance")?);
            if items.is_empty()
                || items.iter().any(|item| {
                    !matches!(
                        value(item, "provenance")
                            .and_then(|p| string(p, "type"))
                            .as_deref(),
                        Some("user" | "harness" | "agents_md" | "skills")
                    )
                })
            {
                return Err("unsupported Codex message provenance".into());
            }
            let user = items.iter().any(|item| {
                value(item, "provenance")
                    .and_then(|p| string(p, "type"))
                    .as_deref()
                    == Some("user")
            });
            if !user {
                continue;
            }
            if !items.iter().all(|item| {
                value(item, "provenance")
                    .and_then(|p| string(p, "type"))
                    .as_deref()
                    == Some("user")
            }) {
                return Err("unsupported mixed Codex message provenance".into());
            }
            let content = array_items(value(payload, "content").ok_or("missing user content")?);
            let mut text = Vec::new();
            for part in content {
                if string(part, "type").as_deref() == Some("input_text") {
                    text.push(string(part, "text").ok_or("missing user text")?);
                }
            }
            latest = Some((
                string(meta, "turn_id").ok_or("missing user turn id")?,
                text.join("\n"),
                millis(value(meta, "create_time").ok_or("missing user message time")?)?,
            ));
        }
    }
    if !identity {
        return Err("missing Codex transcript identity".into());
    }
    latest.ok_or_else(|| "no supported user message in Codex transcript".into())
}

fn millis(time: &str) -> Result<u64, String> {
    let (secs, fraction) = time.split_once('.').unwrap_or((time, ""));
    let secs: u64 = secs.parse().map_err(|_| "invalid Codex message time")?;
    if !fraction.bytes().all(|c| c.is_ascii_digit()) {
        return Err("invalid Codex message fraction".into());
    }
    let fraction: String = fraction
        .chars()
        .chain(std::iter::repeat('0'))
        .take(3)
        .collect();
    secs.checked_mul(1000)
        .and_then(|s| s.checked_add(fraction.parse().ok()?))
        .ok_or_else(|| "Codex message time overflow".into())
}

#[cfg(test)]
mod tests {
    use super::read;

    const META: &str =
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"thread\",\"thread_source\":\"user\"}}\n";
    fn user(turn: &str, text: &str) -> String {
        format!(
            "{{\"type\":\"response_item\",\"payload\":{{\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{text}\"}}],\"internal_chat_message_metadata_passthrough\":{{\"turn_id\":\"{turn}\",\"create_time\":123.4567,\"content_item_metadata\":[{{\"provenance\":{{\"type\":\"user\"}}}}]}}}}}}\n"
        )
    }

    #[test]
    fn unknown_provenance_cannot_reuse_an_earlier_land() {
        for metadata in ["[]", r#"[{"provenance":{"type":"unknown"}}]"#] {
            let input = format!(
                "{META}{}{{\"type\":\"response_item\",\"payload\":{{\"role\":\"user\",\"internal_chat_message_metadata_passthrough\":{{\"content_item_metadata\":{metadata}}}}}}}\n",
                user("first", "/land")
            );
            assert!(read(input.as_bytes(), "thread").is_err(), "{metadata}");
        }
    }

    #[test]
    fn reads_latest_user_not_assistant_or_injected_skill() {
        let input = format!(
            "{META}{}{}{}{}",
            user("first", "/land"),
            r#"{"type":"response_item","payload":{"role":"assistant","content":[{"text":"/land"}]}}
"#,
            user("second", "continue"),
            r#"{"type":"response_item","payload":{"role":"user","content":[{"text":"/land"}],"internal_chat_message_metadata_passthrough":{"content_item_metadata":[{"provenance":{"type":"skills"}}]}}}
"#
        );
        assert_eq!(
            read(input.as_bytes(), "thread").unwrap(),
            ("second".into(), "continue".into(), 123_456)
        );
        assert!(read(input.as_bytes(), "other").is_err());
        assert!(read(format!("{input}{{").as_bytes(), "thread").is_err());
        assert!(
            read(
                format!(
                    "{input}{{\"type\":\"response_item\",\"payload\":{{\"role\":\"user\"}}}}\n"
                )
                .as_bytes(),
                "thread"
            )
            .is_err()
        );
    }
}
