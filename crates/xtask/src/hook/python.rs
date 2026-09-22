//! The python guard: on Windows the `python` in PATH is an App
//! Execution Alias, a stub that prints an invitation to the Microsoft
//! Store and exits 49. It answers every spelling the same way, so a
//! session that types it spends the call, reads an advert, and guesses
//! again. `uv` carries the real interpreter and needs no project.

use super::payload::{deny, string_field};
use super::shell::{pipe_pieces, program_at, program_name};

/// How a throwaway script is run on this host.
const REAL: &str = "uv run --no-project python";

/// The names the Store stub answers to.
const STUBBED: [&str; 4] = ["python", "python3", "python.exe", "python3.exe"];

/// What carries a real interpreter of its own: the container image, and
/// uv.
const CARRIES_ONE: [&str; 3] = ["xtask linux", "docker", "uv run"];

/// PreToolUse(Bash|PowerShell). Answers whether it refused, like
/// `pre_git`.
pub(super) fn pre_shell(input: &str) -> Result<bool, String> {
    if !cfg!(windows) {
        return Ok(false);
    }
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    if !names_the_stub(&command) {
        return Ok(false);
    }
    deny(&format!(
        "`python` in PATH on this host is the Microsoft Store stub: it prints an invitation \
         and exits 49, whatever it was asked to run. Run it as `{REAL} <script>` \
         (`{REAL} -c '<source>'` for a line), which is the interpreter this machine has."
    ));
    Ok(true)
}

/// Whether the line runs the stub: python in command position of a
/// piece, in a line that reaches no interpreter of its own.
fn names_the_stub(command: &str) -> bool {
    if CARRIES_ONE.iter().any(|carrier| command.contains(carrier)) {
        return false;
    }
    pipe_pieces(command).into_iter().any(|piece| {
        program_at(piece)
            .is_some_and(|(tokens, at)| STUBBED.contains(&program_name(tokens[at]).as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::names_the_stub;

    #[test]
    fn the_stub_is_seen_wherever_the_line_runs_it() {
        for line in [
            "python script.py",
            "python3 -c 'print(1)'",
            "PYTHONIOENCODING=utf-8 python analyse.py",
            "cat data.json | python -m json.tool",
        ] {
            assert!(names_the_stub(line), "{line}");
        }
    }

    #[test]
    fn a_line_that_carries_its_own_interpreter_is_not_the_stub() {
        for line in [
            "uv run --no-project python analyse.py",
            "cargo xtask linux python3 -V",
            "docker exec pgg python3 -c 'print(1)'",
            // Named as data, not run.
            "grep -rn python crates/xtask/src",
            // A pattern is one argument, and the pipes inside it are
            // the expression's own.
            "cargo test -p xtask 2>&1 | grep -E \"result|python|shell\"",
        ] {
            assert!(!names_the_stub(line), "{line}");
        }
    }
}
