//! Interactive rebase driven from the GUI.
//!
//! The plan is written to a file and `GIT_SEQUENCE_EDITOR` points at the
//! `pgg-todo-editor` helper, which installs it as git's todo
//! (rules-refs/core.md の `pgg-todo-editor` の行).
//!
//! Rewording is `pick` plus `exec git commit --amend --file`, not a
//! `reword` line: `reword` opens `GIT_EDITOR`, pinned to `true`, so the
//! message would silently stay as it was.

mod plan;
mod run;
mod todo;

pub use plan::{Edit, EditPlan, plan_edit, plan_for};
pub use run::{
    HELPER_NAME, TODO_EDITOR_FLAG, apply_plan, helper_in, helper_path, rebase_interactive,
    sequence_editor_command,
};
pub use todo::{MergedTodo, RebaseStep, TodoAction, TodoLine, merge_todo, render_todo};

pub(crate) use plan::{Base, base_of, range_arg};

// Reached only from `crate::sequencer_tests`.
#[cfg(test)]
pub(crate) use plan::has_parent_header;
#[cfg(test)]
pub(crate) use run::sh_quote;
#[cfg(test)]
pub use todo::parse_todo;
