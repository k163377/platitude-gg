//! Core domain logic for platitude-gg: git subprocess execution, output
//! parsing, commit-graph computation and the domain model.
//!
//! This crate must stay free of Qt (and any bridge) dependencies so that it
//! can be unit-tested on any machine and reused unchanged if the QML bridge
//! implementation is swapped (Qt Bridges -> CXX-Qt).

pub mod avatar;
pub mod branch;
pub mod commit;
pub mod conflict;
pub mod details;
pub mod error;
pub mod graph;
pub mod identity;
pub mod integrate;
pub mod model;
pub mod oid;
pub mod opstate;
pub mod parse;
pub mod patch;
pub mod preview;
pub mod process;
pub mod publish;
pub mod reachable;
pub mod refs;
pub mod remote;
pub mod repo;
pub mod scratch;
pub mod sequencer;
pub mod session;
pub mod settings;
pub mod stage;
pub mod stash;
pub mod status;
pub mod tag;
pub mod version;
pub mod worktrees;

pub use error::GitError;
pub use model::{CommitMeta, StrPool};
pub use oid::Oid;
pub use process::{
    CommandEnd, CommandObserver, DEFAULT_TIMEOUT, GitCommand, GitExecutor, GitOutput,
};
pub use repo::{ObjectFormat, RepoInfo};
pub use version::{GitVersion, MINIMUM_GIT};
