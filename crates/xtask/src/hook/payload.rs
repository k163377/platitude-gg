//! The hook's JSON: values read out of the payload, and answers spliced
//! into hand-built JSON.

/// Returns the first JSON string value for `key` in `input`, unescaped just
/// enough for paths (\\ \" \/ \n \t). The payload is machine-produced JSON,
/// so the first occurrence of a key like "file_path" is the tool input's.
///
/// `\uXXXX` is not decoded (only its backslash is dropped): the harness
/// writes non-ASCII as raw UTF-8, and a wrong decode would corrupt a path.
pub(super) fn string_field(input: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let after_key = &input[input.find(&needle)? + needle.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    unquoted(after_colon.strip_prefix('"')?)
}

/// The payload's own string field `key` — the top-level object's —
/// unescaped as [`string_field`] does. A key of the same name nested in a
/// tool's input or response is never it: Claude Code writes `tool_use_id`
/// after both, and a session's own payload has no `agent_id` ahead of them.
pub(super) fn own_string_field(input: &str, key: &str) -> Option<String> {
    unquoted(own_value_field(input, key)?.strip_prefix('"')?)
}

/// The payload's own field `key` — the top-level object's — as written
/// ([`written_value`]); `None` when the field is absent or cut short.
pub(super) fn own_value_field<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let wanted = format!("\"{key}\"");
    let mut rest = input.trim_start().strip_prefix('{')?;
    loop {
        rest = rest.trim_start();
        let name = written_value(rest)?;
        rest = rest[name.len()..]
            .trim_start()
            .strip_prefix(':')?
            .trim_start();
        let value = written_value(rest)?;
        if name == wanted {
            return Some(value);
        }
        rest = rest[value.len()..].trim_start().strip_prefix(',')?;
    }
}

/// A JSON string's body, from past its opening quote to its closing one,
/// unescaped just enough for paths.
fn unquoted(body: &str) -> Option<String> {
    let mut value = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(value),
            '\\' => match chars.next()? {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                other => value.push(other),
            },
            other => value.push(other),
        }
    }
    None
}

/// Returns the first JSON boolean value for `key` in `input`; `None` when
/// the field is absent, which is not `false`.
pub(super) fn bool_field(input: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let after_key = &input[input.find(&needle)? + needle.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    if after_colon.starts_with("true") {
        return Some(true);
    }
    after_colon.starts_with("false").then_some(false)
}

/// Returns the first JSON whole number for `key` in `input`; `None` when
/// the field is absent or holds anything else.
pub(super) fn number_field(input: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let after_key = &input[input.find(&needle)? + needle.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let digits = after_colon
        .find(|c: char| !c.is_ascii_digit())
        .map_or(after_colon, |end| &after_colon[..end]);
    digits.parse().ok()
}

/// The JSON value `value` starts with, as written — an object or an array
/// with everything nested in it, a string with its quotes; `None` when cut
/// short.
fn written_value(value: &str) -> Option<&str> {
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for (at, c) in value.char_indices() {
        if quoted {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' if depth == 0 => return Some(&value[..=at]),
                '"' => quoted = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '{' | '[' => depth += 1,
            '}' | ']' if depth == 0 => return Some(value[..at].trim_end()),
            '}' | ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&value[..=at]);
                }
            }
            ',' if depth == 0 => return Some(value[..at].trim_end()),
            _ => {}
        }
    }
    None
}

/// Prints a PreToolUse deny, with `reason` made safe for the JSON.
pub(super) fn deny(reason: &str) {
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"{}\"}}}}",
        printable(reason)
    );
}

/// `text` made safe to splice into a hand-built JSON string.
pub(super) fn printable(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '"' | '\\' => '\'',
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{bool_field, number_field, own_string_field, own_value_field, string_field};
    use crate::seats::worktree_root;

    #[test]
    fn reads_the_payload_s_own_field_past_nested_ones_of_its_name() {
        let payload = r#"{"session_id":"s","tool_input":{"agent_id":"in","tool_use_id":"in"} , "tool_response":{"tool_use_id":"in2","t":"\"tool_use_id\":\"x\""},"tool_use_id":"toolu_1","n": 7 }"#;
        assert_eq!(
            own_string_field(payload, "tool_use_id").as_deref(),
            Some("toolu_1")
        );
        assert_eq!(own_string_field(payload, "agent_id"), None);
        assert_eq!(
            own_value_field(payload, "tool_input"),
            Some(r#"{"agent_id":"in","tool_use_id":"in"}"#)
        );
        assert_eq!(own_value_field(payload, "n"), Some("7"));
        assert_eq!(own_string_field(payload, "n"), None);
        assert_eq!(own_value_field(payload, "absent"), None);
        assert_eq!(own_value_field(r#"{"cut":{"a":1"#, "cut"), None);
        assert_eq!(own_value_field("{}", "a"), None);
    }

    #[test]
    fn reads_a_backslashed_cwd_out_of_a_payload_and_into_a_root() {
        let payload = r#"{"session_id":"x","cwd":"C:\\Users\\x\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\nice-satoshi-45da22"}"#;
        let cwd = string_field(payload, "cwd").expect("cwd");
        assert_eq!(
            cwd,
            "C:\\Users\\x\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\nice-satoshi-45da22"
        );
        assert_eq!(
            worktree_root(&cwd).as_deref(),
            Some("C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/nice-satoshi-45da22")
        );
    }

    #[test]
    fn tells_an_absent_flag_from_one_that_says_no() {
        let payload = r#"{"tool_input":{"command":"ls","run_in_background": true,"quiet":false}}"#;
        assert_eq!(bool_field(payload, "run_in_background"), Some(true));
        assert_eq!(bool_field(payload, "quiet"), Some(false));
        assert_eq!(bool_field(payload, "timeout"), None);
    }

    #[test]
    fn reads_a_whole_number_and_nothing_else() {
        let payload = r#"{"tool_input":{"delaySeconds": 1800,"reason":"wait","stop":true}}"#;
        assert_eq!(number_field(payload, "delaySeconds"), Some(1800));
        assert_eq!(number_field(payload, "reason"), None);
        assert_eq!(number_field(payload, "stop"), None);
        assert_eq!(number_field(payload, "prompt"), None);
    }

    #[test]
    fn reads_a_value_as_written_nested_and_quoted_braces_and_all() {
        let payload = r#"{"tool_name":"Bash","tool_input":{"command":"echo \"}\" [x]","o":{"a":[1,{"b":2}]}},"n": 7 ,"s":"a,b"}"#;
        assert_eq!(
            own_value_field(payload, "tool_input"),
            Some(r#"{"command":"echo \"}\" [x]","o":{"a":[1,{"b":2}]}}"#)
        );
        assert_eq!(own_value_field(payload, "n"), Some("7"));
        assert_eq!(own_value_field(payload, "s"), Some(r#""a,b""#));
        assert_eq!(own_value_field(payload, "tool_name"), Some(r#""Bash""#));
        assert_eq!(own_value_field(payload, "absent"), None);
        assert_eq!(own_value_field(r#"{"cut":{"a":1"#, "cut"), None);
    }
}
