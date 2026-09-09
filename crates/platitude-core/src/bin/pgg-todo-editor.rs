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
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&format!("could not install the rebase plan: {error}")),
    }
}

/// Reports on stderr, where git shows editor failures, and fails the
/// rebase rather than letting it run git's own todo list.
fn fail(message: &str) -> ExitCode {
    let mut err = std::io::stderr();
    if let Err(e) = writeln!(err, "pgg-todo-editor: {message}") {
        // Nothing left to report with; the exit code still stops the rebase.
        drop(e);
    }
    ExitCode::FAILURE
}
