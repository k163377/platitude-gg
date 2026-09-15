//! `GIT_SEQUENCE_EDITOR` helper for interactive rebase.
//!
//! git invokes an editor to let a human write the todo list; there is no
//! human here, so this stands in for one. It is a separate executable
//! rather than a mode of the GUI binary because git runs it through a
//! shell, expects it to exit promptly, and reads its exit code — none of
//! which sits well with a windowed application.
//!
//! Usage: `pgg-todo-editor --todo-editor <plan> <todo>`
//! (the application supplies everything up to `<plan>`; git appends
//! `<todo>`.)

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
            // git wrote a line for a commit the plan has nothing to say
            // about — a range that moved between the two. Said rather
            // than carried: put anywhere else it would move a ref to a
            // place nobody asked for, and git shows what its editor said.
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
/// rebase rather than letting it run git's own todo list.
fn fail(message: &str) -> ExitCode {
    // Nothing left to report with if the write fails; the exit code still
    // stops the rebase.
    say(message);
    ExitCode::FAILURE
}
