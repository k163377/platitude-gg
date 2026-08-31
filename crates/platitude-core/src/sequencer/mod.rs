//! Interactive rebase driven from the GUI.
//!
//! git asks an editor to write the todo list; this module supplies that
//! editor. The plan the user assembled is written to a file, and
//! `GIT_SEQUENCE_EDITOR` is pointed at the `pg-todo-editor` helper that
//! ships beside the application, which copies the plan over git's todo
//! file and exits (実装計画 §6, P3-確認事項 §残っている実装).
//!
//! Rewording is expressed as `pick` plus an `exec git commit --amend
//! --file`, not as a `reword` line. A `reword` would open `GIT_EDITOR`,
//! which the process layer pins to `true` so nothing hangs — and the
//! message would silently stay as it was. The `exec` form states the new
//! message outright, so it either applies or fails loudly.

mod plan;
mod run;
mod todo;

pub use plan::{Edit, EditPlan, plan_edit, plan_for};
pub use run::{
    HELPER_NAME, TODO_EDITOR_FLAG, apply_plan, helper_in, helper_path, rebase_interactive,
    sequence_editor_command,
};
pub use todo::{RebaseStep, TodoAction, TodoLine, render_todo};

pub(crate) use plan::{range_arg, resolve};

// Reached only from `crate::sequencer_tests`, which pins the two pieces
// of string work the modules above keep to themselves.
#[cfg(test)]
pub(crate) use run::sh_quote;
#[cfg(test)]
pub use todo::parse_todo;
