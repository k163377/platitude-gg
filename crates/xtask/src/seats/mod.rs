//! The worktree seat roster: the survey (`cargo xtask seats`) the
//! session-start greeting shares, and the letters it hands out. Whose a
//! seat is, and the three ways a claim moves, is `claim`'s to say.
//!
//! This module is the one place a seat is measured, so the command and
//! the greeting cannot drift apart. The greeting's snapshot goes stale;
//! the command is the live answer.

pub(crate) mod age;
mod claim;
pub(crate) mod commands;
pub(crate) mod entry;
mod management;
mod offer;
mod pictures;
pub(crate) mod start;
mod survey;
mod transcripts;
pub(crate) mod worktrees;

pub(crate) use age::format_age;
pub(crate) use claim::{
    Held, Identity, Standing, is_a_seat_claim, lock_reason, standing, take_seat, unlock_seat, whose,
};
pub(crate) use commands::COMMANDS;
pub(crate) use survey::{Seat, run, survey};
pub(crate) use worktrees::{
    RIG, SEATS, SeatEntry, WorktreeBlock, in_rig, primary_checkout, rig_path, roster_dir,
    roster_letter, same_tree, seat_entries, seat_in_repository, slashed, worktree_blocks,
    worktree_root,
};
// Built only by the greeting's tests: everything else reads what the
// survey measured.
#[cfg(test)]
pub(crate) use survey::SeatState;

pub(crate) use management::{carrying, conversation_seat, how_claims_move, take, treeless};
pub(crate) use pictures::may_picture;
pub(crate) use worktrees::{commits_in, dirty_lines, has_a_directory};
