//! `GIT_SEQUENCE_EDITOR` helper for interactive rebase: stands in for the
//! human editing the todo list. A separate executable because git runs it
//! through a shell and reads its exit code, which a windowed application
//! cannot serve.
//!
//! Usage: `pgg-todo-editor --todo-editor <plan> <todo>` (git appends `<todo>`).

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use platitude_core::sequencer::{TODO_EDITOR_FLAG, apply_plan};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [flag, plan, todo] = args.as_slice() else {
        return fail(&format!(
            "usage: pgg-todo-editor {TODO_EDITOR_FLAG} <plan> <todo>"
        ));
    };
    if flag != TODO_EDITOR_FLAG {
        return fail(&format!("unknown argument `{flag}`"));
    }
    match apply_plan(&PathBuf::from(plan), &PathBuf::from(todo)) {
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
