//! Shared startup before command dispatch. Command-specific dependencies belong to the command's module.

use std::process::ExitCode;

pub(crate) fn run(dispatch: impl FnOnce() -> ExitCode) -> ExitCode {
    crate::verify::git_shim::git_shim().unwrap_or_else(dispatch)
}
