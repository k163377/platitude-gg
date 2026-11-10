//! Shell-level protection for the setting that selects Git's hooks.
//! Reads remain available for diagnosis; installing belongs to gate::hooks.

use super::shell::{pipe_pieces, program_at, program_name};

pub(super) fn refusal(command: &str) -> Option<String> {
    pipe_pieces(command).into_iter().any(changes_path).then(|| {
        format!(
            "Changing core.hooksPath takes this session's git out from under the gate. \
         Use `{}` to install the repository hook; report installation \
         or validation errors instead of disabling it.",
            crate::gate::INSTALL.line()
        )
    })
}

fn changes_path(piece: &str) -> bool {
    let Some((tokens, at)) = program_at(piece) else {
        return false;
    };
    if !matches!(program_name(tokens[at]).as_str(), "git" | "git.exe") {
        return false;
    }
    let raw = arguments(piece);
    let Some(start) = raw.iter().position(|s| s == tokens[at]) else {
        return false;
    };
    let original = &raw[start + 1..];
    let args: Vec<String> = original.iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        if original[at] == "-C" {
            at += 2;
        } else if arg == "-c" || arg == "--config-env" {
            if args
                .get(at + 1)
                .is_some_and(|v| v.starts_with("core.hookspath="))
            {
                return true;
            }
            at += 2;
        } else if arg.starts_with("-ccore.hookspath=")
            || arg.starts_with("--config-env=core.hookspath=")
        {
            return true;
        } else if arg == "--git-dir" || arg == "--work-tree" {
            at += 2;
        } else if arg.starts_with('-') {
            at += 1;
        } else {
            break;
        }
    }
    if args.get(at).map(String::as_str) != Some("config") {
        return false;
    }
    let config = &args[at + 1..];
    let reads = [
        "--get",
        "--get-all",
        "--get-regexp",
        "--get-urlmatch",
        "--list",
        "-l",
        "get",
        "list",
    ];
    let writes = [
        "--unset",
        "--unset-all",
        "--add",
        "--replace-all",
        "--rename-section",
        "--remove-section",
        "set",
        "unset",
        "rename-section",
        "remove-section",
    ];
    if config.iter().any(|s| writes.contains(&s.as_str())) {
        return config.iter().any(|s| s == "core.hookspath" || s == "core");
    }
    let key = config.iter().position(|s| s == "core.hookspath");
    if config[..key.unwrap_or(config.len())]
        .iter()
        .any(|s| reads.contains(&s.as_str()))
    {
        return false;
    }
    key.is_some_and(|i| i + 1 < config.len())
}

/// Keep quoted paths and empty values as single arguments. This is a
/// literal shell guard, not an evaluator of substitutions or script bodies.
fn arguments(piece: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for ch in piece.chars() {
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            } else {
                word.push(ch);
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            started = true;
        } else if ch.is_whitespace() {
            if started {
                words.push(std::mem::take(&mut word));
                started = false;
            }
        } else {
            word.push(ch);
            started = true;
        }
    }
    if started {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::refusal;

    #[test]
    fn refuses_hook_overrides_and_config_writes_but_allows_diagnosis() {
        for command in [
            "git -c core.hooksPath=/tmp commit",
            "git -ccore.hooksPath= commit",
            "git --config-env=core.hooksPath=HOOKS commit",
            "git config --unset core.hooksPath",
            "git config core.hooksPath ''",
            "git config core.hooksPath get",
            "git config set core.hooksPath /tmp",
            "git config --remove-section core",
            "echo ok; git config --local --unset-all CORE.HOOKSPATH",
            "git.exe -C repo -c core.hooksPath=/tmp status",
            "git -C 'a path with spaces' -c core.hooksPath=/tmp status",
        ] {
            assert!(refusal(command).is_some(), "{command}");
        }
        for command in [
            "git config --get core.hooksPath",
            "git config core.hooksPath",
            "git config get core.hooksPath",
            "git config --list",
            "rg core.hooksPath crates",
            "echo 'git -c core.hooksPath=x commit'",
            "git commit -m 'core.hooksPath'",
            "git -C repo status",
            "cargo xtask gate install",
        ] {
            assert!(refusal(command).is_none(), "{command}");
        }
    }

    #[test]
    fn both_agents_refuse_the_override_at_their_tool_entry() {
        let input = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","cwd":".","tool_input":{"command":"git -c core.hooksPath=/tmp status"}}"#;
        assert!(super::super::codex_git::refusal(input).is_some());
        assert!(super::super::git::pre_git(input).unwrap());
    }
}
