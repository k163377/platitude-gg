//! `GIT_SEQUENCE_EDITOR` helper for interactive rebase: stands in for the
//! human editing the todo list. A separate executable because git runs it
//! through a shell and reads its exit code, which a windowed application
//! cannot serve.
//!
//! Usage: `pgg-todo-editor --todo-editor <todo>` with the plan on stdin
//! (git appends `<todo>`, and hands its own stdin on).

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use platitude_core::sequencer::{TODO_EDITOR_FLAG, apply_plan, unframed};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [flag, todo] = args.as_slice() else {
        return fail(&format!(
            "usage: pgg-todo-editor {TODO_EDITOR_FLAG} <todo>, with the plan on stdin"
        ));
    };
    if flag != TODO_EDITOR_FLAG {
        return fail(&format!("unknown argument `{flag}`"));
    }
    // To its end, which can come early: only a plan whose whole length came
    // is installed (`sequencer::unframed`).
    let input = match std::io::read_to_string(std::io::stdin()) {
        Ok(input) => input,
        Err(error) => return fail(&format!("could not read the rebase plan: {error}")),
    };
    if input.is_empty() {
        return fail("no rebase plan came on stdin");
    }
    let Some(plan) = unframed(&input) else {
        return fail("the rebase plan came cut short");
    };
    match apply_plan(plan, &PathBuf::from(todo)) {
        Ok(orphaned) => {
            // A line the plan does not cover (the range moved in between).
            // Reported, not carried: placed anywhere it would move a ref
            // nobody asked to move.
            for line in orphaned {
                say(&format!("left out of the plan: {line}"));
            }
            ExitCode::SUCCESS
        }
        Err(error) => fail(&format!("could not install the rebase plan: {error}")),
    }
}

/// A word on stderr that does not stop the rebase.
fn say(message: &str) {
    let mut err = std::io::stderr();
    if let Err(e) = writeln!(err, "pgg-todo-editor: {message}") {
        drop(e);
    }
}

/// Reports on stderr, where git shows editor failures, and fails the
/// rebase: a zero exit would run git's own todo list.
fn fail(message: &str) -> ExitCode {
    say(message);
    ExitCode::FAILURE
}
