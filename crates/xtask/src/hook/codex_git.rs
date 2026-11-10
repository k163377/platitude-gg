//! Codex's PreToolUse(Bash): the git a Codex shell line may not run — a
//! hand-written main, and a line that takes the chat's git out from under
//! the gate. The git hook already holds a Codex chat's git to the gate on
//! main by its thread id (`gate::hooks::SESSION_MARKS`), but it never sees
//! a refspec writing a remote's main, and it lets a git through that the
//! line has taken the mark off. The rest of Claude's guards are not
//! Codex's (AGENTS.md: it keeps those rules itself).
//!
//! Codex names every shell tool `Bash` here and hands the line over as
//! `tool_input.command`, a string, on Windows too; the payload's `cwd` is
//! the call's own workdir when it names one. A refused call never runs,
//! and no PostToolUse follows it.

use super::payload::{own_string_field, own_value_field, string_field};
use crate::gate::hooks::{CODEX_SESSION, SKIP};

/// What a Codex PreToolUse payload is refused for, if anything.
pub(super) fn refusal(input: &str) -> Option<String> {
    if own_string_field(input, "hook_event_name").as_deref() != Some("PreToolUse")
        || own_string_field(input, "tool_name").as_deref() != Some("Bash")
    {
        return None;
    }
    let command = string_field(own_value_field(input, "tool_input")?, "command")?;
    let cwd = own_string_field(input, "cwd").unwrap_or_default();
    super::hooks_path::refusal(&command)
        .or_else(|| gate_control_refusal(&command))
        .or_else(|| super::git::main_write_refusal(&command, &cwd))
}

/// The gate's skip flag, spelled at all, and a line that sets or removes
/// the thread id for what it runs: either lets the chat's git past the
/// gate as if it were the user's. Windows reads environment names without
/// case, so neither does this.
fn gate_control_refusal(command: &str) -> Option<String> {
    let line = command.to_ascii_lowercase();
    let name = if line.contains(&SKIP.to_ascii_lowercase()) {
        SKIP
    } else if changes_the_mark(&line) {
        CODEX_SESSION
    } else {
        return None;
    };
    Some(format!(
        "{name} decides whether refs/heads/main answers to the gate: {SKIP} is the user's own \
         manual test-skip control, and {CODEX_SESSION} is how the gate knows this chat's git \
         from the user's — reading it is fine, but a line that sets or removes it, or empties \
         the environment it hands on, takes this chat's git out from under the gate. Run `cargo \
         xtask gate` (or `land`, which gates on the way) and require a passing stamp for main. \
         If validation appears incorrect, stop and report the validation error to the user."
    ))
}

/// PowerShell's verbs (and their aliases) that change an `Env:` item.
const ITEM_EDITS: &[&str] = &[
    "remove-item",
    "ri",
    "rm",
    "del",
    "erase",
    "rd",
    "rmdir",
    "clear-item",
    "cli",
    "set-item",
    "si",
    "new-item",
    "ni",
    "rename-item",
    "rni",
    "ren",
    "move-item",
    "mi",
    "mv",
];

/// Commands that search for or print what they are given, and set nothing
/// for what runs after them: `NAME=` in their arguments is text.
const READERS: &[&str] = &[
    "rg",
    "grep",
    "egrep",
    "fgrep",
    "git",
    "findstr",
    "select-string",
    "sls",
    "echo",
    "printf",
    "write-output",
    "write-host",
    "printenv",
    "cat",
    "type",
    "get-content",
    "gc",
    "head",
    "tail",
    "less",
    "more",
    "wc",
    "sort",
];

/// Whether the lowercased `line` sets or removes the thread id for what it
/// runs, in any of the shells Codex runs — sh, PowerShell, cmd — or hands
/// on an emptied environment. A read goes through: `$NAME`, `${NAME}`,
/// `%NAME%`, `$env:NAME`, a comparison, a command that only searches for or
/// prints the name, and an `Env:` item named for something else.
fn changes_the_mark(line: &str) -> bool {
    let name = CODEX_SESSION.to_ascii_lowercase();
    let pipelines: Vec<Vec<Vec<&str>>> = line
        .split([';', '&', '\n'])
        .map(|pipeline| pipeline.split('|').map(words).collect())
        .collect();
    if pipelines.iter().flatten().any(|words| empties(words)) {
        return true;
    }
    // `Remove-Item Env:NAME`, `Remove-Item Env:*`, `gci env: | ri`.
    let names_the_item = |word: &&str| {
        word.strip_prefix("env:").is_some_and(|item| {
            let item = item.trim_start_matches(['\\', '/']);
            item.is_empty() || item.contains(['*', '?', '[']) || item.contains(&name)
        })
    };
    let edits_the_item = pipelines.iter().any(|pipeline| {
        pipeline.iter().flatten().any(names_the_item)
            && pipeline
                .iter()
                .any(|words| words.first().is_some_and(|verb| ITEM_EDITS.contains(verb)))
    });
    if edits_the_item {
        return true;
    }
    if !line.contains(&name) {
        return false;
    }
    // `[Environment]::SetEnvironmentVariable('NAME', …)`.
    let sets_it_by_call = line
        .split([';', '&', '\n', '|'])
        .any(|command| command.contains("setenvironmentvariable") && !is_a_reader(&words(command)));
    sets_it_by_call
        || line
            .match_indices(&name)
            .any(|(at, _)| sets_or_removes(&line[..at], &line[at + name.len()..]))
}

/// A command's words, its quotes and brackets trimmed off.
fn words(command: &str) -> Vec<&str> {
    command
        .split_whitespace()
        .map(|word| word.trim_matches(['"', '\'', '(', ')', '{', '}', '[', ']']))
        .filter(|word| !word.is_empty())
        .collect()
}

fn is_a_reader(words: &[&str]) -> bool {
    words.first().is_some_and(|first| READERS.contains(first))
}

fn is_env(word: &str) -> bool {
    word == "env" || word.ends_with("/env")
}

/// Whether the command runs what follows it in an emptied environment:
/// `env -i` (with its options in any order), `env -`, `exec -c`,
/// `Start-Process -UseNewEnvironment` (or any prefix PowerShell takes for it).
fn empties(words: &[&str]) -> bool {
    let Some((first, rest)) = words.split_first() else {
        return false;
    };
    let mut rest = rest.iter().copied();
    if is_env(first) {
        while let Some(word) = rest.next() {
            match word {
                "-" | "--ignore-environment" => return true,
                "-u" | "--unset" | "-c" | "--chdir" => {
                    rest.next();
                }
                _ if word.starts_with("--") || word.contains('=') => {}
                _ if word.starts_with('-') => {
                    if word.contains('i') {
                        return true;
                    }
                }
                _ => return false,
            }
        }
        return false;
    }
    if *first == "exec" {
        while let Some(word) = rest.next() {
            match word {
                "-a" => {
                    rest.next();
                }
                _ if word.starts_with('-') => {
                    if word.contains('c') {
                        return true;
                    }
                }
                _ => return false,
            }
        }
        return false;
    }
    matches!(*first, "start-process" | "saps" | "start")
        && rest.any(|word| word.starts_with("-u") && "-usenewenvironment".starts_with(word))
}

/// Whether the name, with `before` and `after` it on the line, is set or
/// removed: `NAME=`, `export NAME=`, `set NAME=`, `$env:NAME =`,
/// `${env:NAME} =`, `@{NAME=…}`, `unset [-v] NAME`, `export -n NAME`,
/// `env -u NAME`, `env -uNAME`, `env --unset[=]NAME`.
fn sets_or_removes(before: &str, after: &str) -> bool {
    let command = before
        .rsplit([';', '|', '&', '\n', '(', '{'])
        .next()
        .unwrap_or(before);
    let words = words(command);
    let first = words.first().copied().unwrap_or_default();
    let last = words.last().copied().unwrap_or_default();
    let after = after.trim_start();
    let after = after.strip_prefix('}').unwrap_or(after).trim_start();
    let read = before.ends_with('$') || before.ends_with("${") || before.ends_with('%');
    let assigns =
        after.starts_with('=') && !after.starts_with("==") && !read && !is_a_reader(&words);
    assigns
        || first == "unset"
        || (first == "export" && words.contains(&"-n"))
        || (is_env(first) && (matches!(last, "-u" | "--unset") || last.ends_with("--unset=")))
}

#[cfg(test)]
mod tests {
    use super::gate_control_refusal;

    #[test]
    fn refuses_a_line_that_takes_the_mark_off_or_turns_the_gate_off() {
        for command in [
            "CODEX_THREAD_ID= git commit -m x",
            "env -u CODEX_THREAD_ID git merge --ff-only worktree-b",
            "env -uCODEX_THREAD_ID git commit",
            "env --unset=CODEX_THREAD_ID git commit",
            "env --unset CODEX_THREAD_ID git commit",
            "env -i PATH=/usr/bin git commit -m x",
            "/usr/bin/env -i git commit",
            "env -v -i git commit",
            "env -u FOO -i git commit",
            "env - git commit",
            "exec -c git commit -m x",
            "unset CODEX_THREAD_ID && git commit -m x",
            "cd .. ; unset -v CODEX_THREAD_ID; git commit",
            "export CODEX_THREAD_ID=''",
            "export -n CODEX_THREAD_ID",
            "$env:CODEX_THREAD_ID = ''; git commit -m x",
            "$env:codex_thread_id=$null",
            "${env:CODEX_THREAD_ID} = $null",
            "Remove-Item Env:CODEX_THREAD_ID; git commit -m x",
            "Remove-Item -Path Env:\\CODEX_THREAD_ID",
            "Remove-Item Env:*",
            "Remove-Item Env:\\CODEX*",
            "gci env: | ri",
            "[Environment]::SetEnvironmentVariable('CODEX_THREAD_ID', $null)",
            "cmd /c \"set CODEX_THREAD_ID= && git commit\"",
            "set \"CODEX_THREAD_ID=\"",
            "Start-Process git -ArgumentList commit -UseNewEnvironment",
            "Start-Process git -UseNew",
            "Start-Process git -Environment @{CODEX_THREAD_ID=$null}",
            "PGG_GATE_SKIP=1 git commit -m x",
            "$env:pgg_gate_skip = '1'",
        ] {
            assert!(gate_control_refusal(command).is_some(), "{command}");
        }
    }

    /// The chat reads its own id to find its seat; a search for the name
    /// in the source, and a line that prints it, are reads too.
    #[test]
    fn lets_a_line_that_only_reads_the_mark_through() {
        for command in [
            "echo $CODEX_THREAD_ID",
            "echo ${CODEX_THREAD_ID}",
            "echo CODEX_THREAD_ID=$CODEX_THREAD_ID",
            "$env:CODEX_THREAD_ID",
            "$id = $env:CODEX_THREAD_ID; rm .\\scratch.txt",
            "Get-Item Env:CODEX_THREAD_ID",
            "Get-Item Env:CODEX_THREAD_ID; rm foo.txt",
            "Remove-Item Env:RUST_LOG",
            "printenv CODEX_THREAD_ID",
            "rg -n CODEX_THREAD_ID crates/xtask/src",
            "rg -u CODEX_THREAD_ID crates",
            "rg -n \"CODEX_THREAD_ID=\" .",
            "rg UseNewEnvironment crates",
            "rg SetEnvironmentVariable crates",
            "[ \"$CODEX_THREAD_ID\" = codex-test ] && echo same",
            "[ $CODEX_THREAD_ID = codex-test ]",
            "if ($env:CODEX_THREAD_ID -eq 'x') { 'same' }",
            "env | sort",
            "git commit -m \"env -i\"",
            "cargo xtask seat",
        ] {
            assert!(gate_control_refusal(command).is_none(), "{command}");
        }
    }
}
