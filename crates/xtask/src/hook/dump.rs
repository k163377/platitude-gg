//! The dump guard: a read whose output is the whole file.
//!
//! What a shell line prints is read back by every call after it, so a
//! file printed whole is paid for again on each one — and a source file
//! here runs to thousands of tokens. The part that answers the question
//! costs a fraction of that, and the tools that take a part
//! (`grep -n`, `sed -n '<from>,<to>p'`, the Read tool's offset and
//! limit) are the same ones that say where in the file the answer was.

use super::payload::{deny, string_field};
use super::shell::{pipe_pieces, program_at, program_name, shell_segments};

/// Where printing a file whole starts costing more than it tells:
/// roughly two thousand tokens, and every call after the dump reads
/// them again.
const LIMIT: u64 = 8 * 1024;

/// The programs whose output is the file they are handed.
const DUMPS: [&str; 4] = ["cat", "type", "get-content", "gc"];

/// The parameters that bound `Get-Content` to a part of the file, and so
/// bound what reaches the conversation.
const BOUNDED: [&str; 5] = ["-totalcount", "-tail", "-head", "-first", "-last"];

/// PreToolUse(Bash|PowerShell). Answers whether it refused, like
/// `pre_git`.
pub(super) fn pre_shell(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let Some((file, size)) = dumped(&command, &cwd) else {
        return Ok(false);
    };
    deny(&format!(
        "`{file}` is {}KB, and a file printed whole is read again by every call after this \
         one. Take the part that answers the question: `grep -n <pattern> {file}` for where \
         something stands, `sed -n '<from>,<to>p' {file}` for a passage, or the Read tool, \
         which numbers the lines and takes offset and limit. Files under {}KB print freely.",
        size.div_ceil(1024),
        LIMIT / 1024
    ));
    Ok(true)
}

/// The first file a line would print whole that is too big to print
/// whole, with its size.
fn dumped(command: &str, cwd: &str) -> Option<(String, u64)> {
    for file in dump_targets(command) {
        let path = std::path::Path::new(cwd).join(file.trim_matches(['"', '\'']));
        let Ok(found) = std::fs::metadata(&path) else {
            // A path this process cannot resolve — a glob, a shell
            // variable, a path in the container — is nobody's dump.
            continue;
        };
        if found.is_file() && found.len() > LIMIT {
            return Some((file.to_string(), found.len()));
        }
    }
    None
}

/// The files a line hands to a dump whose output reaches the
/// conversation: the last program of each piece, since anything piped
/// onward is the next program's to bound, and nothing redirected to a
/// file is read by anyone here.
fn dump_targets(command: &str) -> Vec<&str> {
    shell_segments(command)
        .into_iter()
        .filter_map(|segment| pipe_pieces(segment).last().copied())
        .filter(|piece| !redirects(piece) && !piece.contains("<<"))
        .filter_map(files_dumped)
        .flatten()
        .collect()
}

fn files_dumped(piece: &str) -> Option<Vec<&str>> {
    let (tokens, at) = program_at(piece)?;
    if !DUMPS.contains(&program_name(tokens[at]).as_str()) {
        return None;
    }
    if tokens
        .iter()
        .any(|token| BOUNDED.contains(&token.to_ascii_lowercase().as_str()))
    {
        return None;
    }
    Some(
        tokens[at + 1..]
            .iter()
            .copied()
            .filter(|token| !token.starts_with('-'))
            .collect(),
    )
}

/// Whether the piece sends its output to a file. `2>&1` moves one stream
/// onto another and leaves the output where it was.
fn redirects(piece: &str) -> bool {
    piece.replace("2>&1", " ").contains('>')
}

#[cfg(test)]
mod tests {
    use super::dump_targets;

    #[test]
    fn a_file_printed_whole_is_the_dump() {
        assert_eq!(
            dump_targets("cat crates/pgg/src/wide.rs"),
            ["crates/pgg/src/wide.rs"]
        );
        assert_eq!(dump_targets("cat -n ui/Wide.qml"), ["ui/Wide.qml"]);
        assert_eq!(dump_targets("cd crates && cat a.qml"), ["a.qml"]);
        assert_eq!(dump_targets("cat a.qml && cat b.qml"), ["a.qml", "b.qml"]);
        assert_eq!(
            dump_targets("Get-Content target/run.log"),
            ["target/run.log"]
        );
    }

    #[test]
    fn a_read_that_is_already_bounded_or_goes_nowhere_is_not() {
        for line in [
            // Bounded by what follows it.
            "cat big.qml | head -50",
            "cat big.qml | grep -n Layout",
            "Get-Content run.log -Tail 20",
            // Written to a file, not read here.
            "cat a.qml > /tmp/copy.qml",
            "cat part1 part2 >> joined.txt",
            // Not a read at all.
            "cat > notes.md <<'EOF'",
            "grep -n Layout big.qml",
            "sed -n '1,40p' big.qml",
            // The dump is inside a pattern, where it is data.
            "grep -nE \"cat|Get-Content\" notes/day.md",
        ] {
            assert!(dump_targets(line).is_empty(), "{line}");
        }
    }

    #[test]
    fn a_dump_in_the_middle_of_a_pipe_is_the_next_programs_to_bound() {
        // The piece that reaches the conversation is `tail`, and the
        // whole file never arrives.
        assert!(dump_targets("cat a.log | sort | tail -3").is_empty());
        // Both halves of `&&` are pieces of their own.
        assert_eq!(dump_targets("cat a.log | tail -3 && cat b.log"), ["b.log"]);
    }
}
