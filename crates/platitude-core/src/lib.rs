//! Core domain logic for platitude-gg: git subprocess execution, output
//! parsing, commit-graph computation and the domain model.
//!
//! Free of Qt and any bridge, so it survives a bridge swap unchanged.

pub mod avatar;
pub mod branch;
pub mod commit;
pub(crate) mod config;
pub mod conflict;
#[cfg(test)]
mod conflict_tests;
pub mod details;
pub mod eol;
pub mod error;
pub mod find;
pub mod graph;
pub mod highlight;
pub mod identity;
pub mod integrate;
pub mod intraline;
pub mod mem;
pub mod model;
pub mod offers;
pub mod oid;
pub mod operation;
pub mod opstate;
pub mod parse;
pub mod patch;
#[cfg(test)]
mod patch_tests;
pub mod picture;
pub mod preview;
pub mod process;
pub mod publish;
pub mod reachable;
pub mod rebase_plan;
pub mod refs;
#[cfg(test)]
mod refusing;
pub mod remote;
pub mod repo;
pub mod report;
pub mod scratch;
pub mod sequencer;
#[cfg(test)]
mod sequencer_tests;
pub mod session;
pub mod settings;
pub mod stage;
pub mod stash;
#[cfg(test)]
mod stash_tests;
pub mod status;
#[cfg(test)]
mod status_tests;
pub mod tag;
pub mod trailers;
pub mod version;
#[cfg(test)]
mod wait;
pub mod worktrees;

// Declarations and re-exports only (.claude/rules/structure.md §分割「クレート root」).
pub use error::GitError;
pub use model::{CommitMeta, Name, StrPool};
pub use oid::Oid;
pub use operation::{Lane, OperationId, OperationKind};
pub use process::{
    CommandEnd, CommandObserver, DEFAULT_TIMEOUT, GitCommand, GitExecutor, GitOutput, Kept,
};
pub use repo::{ObjectFormat, RepoInfo};
pub use report::{ReportKind, WriteReport};
pub use version::{GitVersion, MINIMUM_GIT};
