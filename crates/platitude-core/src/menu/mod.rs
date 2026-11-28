//! The right-click menus built from element types: a subject is made of
//! parts, every part carries its rows, and a menu is the rows of the parts
//! its subject has.
//!
//! - **What is there decides the rows; its state decides only whether each
//!   can be pressed.** [`Class::on`] reads what a menu is opened on ([`On`])
//!   — for a ref or a graph row its [`Shape`]: the row, where it stands
//!   against the tab, and what of it exists, here or over there as last
//!   read — and [`lines`] reads the rest. A greyed row says why: one
//!   [`Why`], the first that holds. A fact moves from greying a row to
//!   deciding one only by moving into what the class reads.
//! - **A row belongs to the part it acts on** ([`Row::part`]): the part is
//!   there when that thing is, and whatever else the row needs is state.
//! - **Where a row stands is its kind's** ([`Row::place`]), the same in
//!   every menu that has it.
//!
//! Every menu row that acts on the repository is here, one entrance for
//! every subject ([`lines`] over [`On`]). Rows that only read what is on
//! screen (copying a path or a line, the GITHUB card) and the tables of a
//! text box and of the window's ☰ stay with the parts that hold their
//! facts, under the same rule. The table of every class's rows is a
//! snapshot of its tests.

mod class;
mod line;
mod row;
mod subject;
#[cfg(test)]
mod tests;

pub use class::{
    Below, BranchClass, Chosen, Class, RemoteClass, RemoteStand, Seen, Shape, Stand, StepClass,
    TagClass, Tie, UpstreamShape, WorktreeClass, WorktreeKind, WorktreeShape,
};
pub use line::{
    Can, Facts, History, Line, Nowhere, On, Placing, State, Why, lines, pull_runs, stash_why,
};
pub use row::{Card, CardGroup, Group, Part, Place, Row};
pub use subject::{RowWorktree, Subject, WorktreeOut, worktree_left};
