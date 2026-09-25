//! All real-git integration tests, in one binary: `cargo test` runs test
//! binaries one after another, each with its own link. The rule and the
//! subset command are in `.claude/rules/core.md`.

mod support;

mod answer_reads;
mod branch_integration;
mod clone_integration;
mod commit_integration;
mod config_reads;
mod details_diff;
mod edge_strings;
mod eol_integration;
mod eol_setting_integration;
mod exec;
mod identity_integration;
mod integrate_integration;
mod logparse;
mod preview_integration;
mod push_default_integration;
mod reachable_integration;
mod refs_integration;
mod remote_integration;
mod remote_tags_integration;
mod rename_integration;
mod session_integration;
mod slots;
mod stage_integration;
mod stash_integration;
mod status_integration;
mod tag_integration;
mod upstream_integration;
mod worktree_state;
