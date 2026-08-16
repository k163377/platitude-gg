//! The one read every handler starts from: a value out of the hook's
//! JSON payload.

/// Returns the first JSON string value for `key` in `input`, unescaped just
/// enough for paths (\\ \" \/). The payload is machine-produced JSON, so the
/// first occurrence of a key like "file_path" is the tool input's.
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

#[cfg(test)]
mod tests {
    use super::string_field;
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
}
