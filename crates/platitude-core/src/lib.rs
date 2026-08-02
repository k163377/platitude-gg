//! Core domain logic for platitude-gg: git subprocess execution, output
//! parsing, commit-graph computation and the domain model.
//!
//! This crate must stay free of Qt (and any bridge) dependencies so that it
//! can be unit-tested on any machine and reused unchanged if the QML bridge
//! implementation is swapped (Qt Bridges -> CXX-Qt).

pub mod branch;
pub mod commit;
pub mod details;
pub mod error;
pub mod graph;
pub mod model;
pub mod oid;
pub mod opstate;
pub mod parse;
pub mod patch;
pub mod process;
pub mod refs;
pub mod repo;
pub mod scratch;
pub mod session;
pub mod stage;
pub mod stash;
pub mod status;
pub mod version;
pub mod worktrees;

pub use error::GitError;
pub use model::{CommitMeta, StrPool};
pub use oid::Oid;
pub use process::{DEFAULT_TIMEOUT, GitCommand, GitExecutor, GitOutput};
pub use repo::{ObjectFormat, RepoInfo};
pub use version::{GitVersion, MINIMUM_GIT};
