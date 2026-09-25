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
    let body = after_colon.strip_prefix('"')?;
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
    use super::{bool_field, string_field};
    use crate::seats::worktree_root;

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
}
