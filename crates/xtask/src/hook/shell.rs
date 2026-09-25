//! How a shell line reads, for the guards that judge one: its pieces in
//! run order, and the program at the head of each. Text only — where a
//! reader cannot tell what a piece invokes it says so, and each guard's
//! own rule decides what that means.

/// The programs a command is run through, stepped over with their flags so
/// `env X=1 cargo build` and `cmd /c cargo build` read as cargo.
const WRAPPERS: [&str; 12] = [
    "env",
    "time",
    "nice",
    "nohup",
    "exec",
    "bash",
    "sh",
    "zsh",
    "pwsh",
    "powershell",
    "powershell.exe",
    "cmd",
];

/// The pieces a line runs one after another: cut at `;`, a line break,
/// `&&` and `||`, with pipes left inside (see [`pipe_pieces`]).
pub(super) fn shell_segments(command: &str) -> Vec<&str> {
    cut(command, Pipes::Inside)
}

/// The same line cut at its pipes as well, so that each piece is one
/// program and its arguments.
pub(super) fn pipe_pieces(command: &str) -> Vec<&str> {
    cut(command, Pipes::Cut)
}

/// Whether a pipe ends a piece.
#[derive(PartialEq)]
enum Pipes {
    Cut,
    Inside,
}

/// The line cut where the shell would cut it, never inside quotes: a
/// separator there is data (a pattern, a commit message), and cutting it
/// hands a guard a program nobody typed. An unclosed quote covers the rest
/// of the line, so no guard refuses a line this cannot parse.
fn cut(command: &str, pipes: Pipes) -> Vec<&str> {
    let bytes = command.as_bytes();
    let mut pieces = Vec::new();
    let (mut start, mut at) = (0, 0);
    let mut quote: Option<u8> = None;
    while at < bytes.len() {
        let byte = bytes[at];
        if let Some(open) = quote {
            quote = (byte != open).then_some(open);
            at += 1;
            continue;
        }
        let next = bytes.get(at + 1).copied();
        if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
            at += 1;
        } else if (byte == b'&' || byte == b'|') && next == Some(byte) {
            pieces.push(&command[start..at]);
            at += 2;
            start = at;
        } else if byte == b';' || byte == b'\n' || (byte == b'|' && pipes == Pipes::Cut) {
            pieces.push(&command[start..at]);
            at += 1;
            start = at;
        } else {
            at += 1;
        }
    }
    pieces.push(&command[start..]);
    pieces
}

/// A segment's tokens and where its program stands among them, past any
/// `NAME=value` and `WRAPPERS`. None where the segment invokes nothing.
pub(super) fn program_at(segment: &str) -> Option<(Vec<&str>, usize)> {
    let tokens: Vec<&str> = segment
        .split_whitespace()
        .map(|token| token.trim_matches(['"', '\'', '(', ')', '&']))
        .filter(|token| !token.is_empty())
        .collect();
    let mut at = 0;
    while at < tokens.len() {
        let token = tokens[at];
        let stepped_over = is_assignment(token)
            || WRAPPERS.contains(&program_name(token).as_str())
            || (at > 0 && (token.starts_with('-') || token.starts_with('/')));
        if !stepped_over {
            break;
        }
        at += 1;
    }
    (at < tokens.len()).then_some((tokens, at))
}

fn is_assignment(token: &str) -> bool {
    token.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty() && name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric())
    })
}

/// The program a token names, however it is spelled: the last path
/// segment, lowercase.
pub(super) fn program_name(token: &str) -> String {
    let path = token.replace('\\', "/");
    path.rsplit('/')
        .next()
        .unwrap_or(&path)
        .to_ascii_lowercase()
}

/// `cargo`, however it is spelled: a path to it, `cargo.exe`, any case.
pub(super) fn is_cargo(token: &str) -> bool {
    matches!(program_name(token).as_str(), "cargo" | "cargo.exe")
}

/// Whether the line runs cargo in command position anywhere in it, a
/// pipe's halves included.
pub(super) fn runs_cargo(command: &str) -> bool {
    pipe_pieces(command)
        .into_iter()
        .any(|piece| program_at(piece).is_some_and(|(tokens, at)| is_cargo(tokens[at])))
}

#[cfg(test)]
mod tests {
    use super::{pipe_pieces, program_at, program_name, runs_cargo, shell_segments};

    #[test]
    fn a_separator_inside_quotes_cuts_nothing() {
        // The pattern is one argument, not three programs.
        assert_eq!(
            pipe_pieces("grep -E \"ok|FAIL|python\" run.log"),
            ["grep -E \"ok|FAIL|python\" run.log"]
        );
        assert_eq!(
            shell_segments("git commit -m 'fix a && b' && git push"),
            ["git commit -m 'fix a && b' ", " git push"]
        );
        assert_eq!(
            pipe_pieces("cat a.log | tail -3"),
            ["cat a.log ", " tail -3"]
        );
        // A quote left open covers what follows, and nothing is cut.
        assert_eq!(
            pipe_pieces("echo \"unclosed | still one"),
            ["echo \"unclosed | still one"]
        );
    }

    #[test]
    fn a_segments_program_is_found_past_what_it_runs_through() {
        let (tokens, at) = program_at("PGG_X=1 env time cargo -q test").expect("a program");
        assert_eq!(tokens[at], "cargo");
        let (tokens, at) = program_at("  cat -n notes/day.md").expect("a program");
        assert_eq!(tokens[at], "cat");
        assert_eq!(program_at("   "), None);
        assert_eq!(program_name("C:/Users/x/.cargo/bin/CARGO.EXE"), "cargo.exe");
    }

    #[test]
    fn cargo_is_seen_wherever_in_the_line_it_stands() {
        for line in [
            "cargo xtask gate",
            "cd crates && cargo test -p xtask",
            "cargo build 2>&1 | tail -5",
            "tail -5 log.txt | cargo run -q -p xtask -- hook stop",
        ] {
            assert!(runs_cargo(line), "{line}");
        }
        for line in [
            "tail -2 target/gate.log",
            "git status --short",
            // Named as data, not run.
            "grep -n 'cargo build' notes/day.md",
        ] {
            assert!(!runs_cargo(line), "{line}");
        }
    }
}
