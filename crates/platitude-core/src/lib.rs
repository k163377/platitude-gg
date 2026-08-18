//! Core domain logic for platitude-gg: git subprocess execution, output
//! parsing, commit-graph computation and the domain model.
//!
//! This crate must stay free of Qt (and any bridge) dependencies so that it
//! can be unit-tested on any machine and reused unchanged if the QML bridge
//! implementation is swapped (Qt Bridges -> CXX-Qt).

pub mod avatar;
pub mod branch;
pub mod commit;
pub(crate) mod config;
pub mod conflict;
pub mod details;
pub mod eol;
pub mod error;
pub mod find;
pub mod graph;
pub mod highlight;
pub mod identity;
pub mod integrate;
pub mod mem;
pub mod model;
pub mod oid;
pub mod opstate;
pub mod parse;
pub mod patch;
pub mod picture;
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
pub mod trailers;
pub mod version;
pub mod worktrees;

/// A short string that lives inline when it fits, and on the heap when it
/// does not.
///
/// **What it is for.** The resident data of a large repository is mostly
/// names — one per ref, one per chip — and a `String` puts every one of
/// them in its own allocation, twenty-odd bytes of payload behind an
/// allocator header of comparable size. Measured on `JetBrains/kotlin`,
/// the live heap held 394,355 allocations of 16 to 32 bytes at once, and
/// a ref name averages 20.2 characters — inside the 24 this type keeps
/// inline.
///
/// **It stays out of the consumer's way.** Reading one needs no mention of
/// the type (`as_str`, `==` against `&str`, `Display`), so the bridge side
/// never names it and core's API is still swappable — which is what the
/// pure-Rust-types rule is for (.claude/rules/core.md).
pub type Name = compact_str::CompactString;

pub use error::GitError;
pub use model::{CommitMeta, StrPool};
pub use oid::Oid;
pub use process::{
    CommandEnd, CommandObserver, DEFAULT_TIMEOUT, GitCommand, GitExecutor, GitOutput,
};
pub use repo::{ObjectFormat, RepoInfo};
pub use version::{GitVersion, MINIMUM_GIT};
