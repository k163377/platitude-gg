//! RepoSession: one open repository = one session (実装計画 §2).
//!
//! Owns all git activity for a repository: the streaming log → graph
//! pipeline, parallel snapshot refreshes (refs / status / stash) and
//! on-demand queries (details, diffs). Everything runs on a tokio runtime;
//! results are pushed to the UI through a [`SessionSink`], which must be
//! cheap and non-blocking (the app bridge posts queued invocations to the
//! Qt main thread).
//!
//! Reads run concurrently; writes go through a single queue so two commands
//! can never touch one repository's index or refs at the same time
//! (実装計画 §2). A queue rather than a lock, because order is part of
//! the contract: "stage this, now commit" must not run the other way round,
//! and independently spawned tasks racing for a mutex give no such
//! guarantee. Every write refreshes afterwards — including a failed one,
//! because a command that stops halfway (a conflicted merge, an interrupted
//! rebase) has still changed the repository.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio_util::sync::CancellationToken;

use crate::branch::{self, CheckoutTarget};
use crate::commit::{self, CommitOptions};
use crate::conflict;
use crate::details::{self, CommitDetails, DiffTarget};
use crate::error::GitError;
use crate::graph::{GraphBuilder, Segment};
use crate::identity;
use crate::integrate;
use crate::model::CommitMeta;
use crate::oid::Oid;
use crate::opstate::{self, OpState};
use crate::parse::diff::FilePatch;
use crate::parse::log::{LOG_FORMAT_ARG, LogParser};
use crate::patch::HunkSelect;
use crate::preview::{self, FilePreview};
use crate::process::{CommandEnd, GitCommand, GitExecutor};
use crate::publish;
use crate::reachable;
use crate::refs::{self, HeadState, RefEntry, RefKind};
use crate::remote;
use crate::repo::RepoInfo;
use crate::sequencer;
use crate::stage;
use crate::stash::{self, StashEntry};
use crate::status::{self, WorkTreeStatus};
use crate::tag;

mod auto_fetch;
mod build;
mod chips;
mod eol;
mod event;
mod feed;
mod graph_refresh;
#[cfg(test)]
mod join_tests;
mod joins;
mod log;
mod model;
mod ops_integrate;
mod ops_remote;
mod ops_tree;
mod print;
mod query;
mod read_slot;
mod refresh;
mod remote_tags;
mod repo_session;
mod rows;
mod snapshot;
mod state;
mod write;

pub use auto_fetch::OpenFetch;
pub use event::SessionEvent;
use feed::CommandFeed;
pub use graph_refresh::{
    RefreshOutcome, RefreshTask, RemoteTagRefreshOutcome, RemoteTagRefreshTask,
};
pub(crate) use model::LabelIndex;
pub use model::{LabelKind, LogOptions, LogRow, RefLabel};
use print::RowPrint;
use read_slot::{ReadSlot, SlotHeld};
pub(crate) use remote_tags::RemoteTagIndex;
pub use repo_session::RepoSession;
pub use snapshot::{BranchItem, RefsSnapshot, TagItem};
pub use state::AutoFetchTicker;
use state::{
    AutoFetch, ConfigStamp, Derived, EndingContext, Footer, HeadHold, OpGate, OpenFetchState,
    Shared, WriteRequest,
};

/// First chunk is small so the first paint happens as early as possible.
const FIRST_CHUNK_ROWS: usize = 512;
const CHUNK_ROWS: usize = 4096;

/// Op name of the interval-driven fetch. The UI keeps this one off the
/// shared error surface, so both sides have to agree on the spelling.
pub const AUTO_FETCH_OP: &str = "auto-fetch";

/// Op name of the fetch an opening fires. Told apart from the interval's
/// because the UI answers it differently: the button turns for both, and
/// only this one leaves the command log where it was — a tab opened on a
/// machine that is offline must not throw the panel up
/// (デザイン規約 §リモートから取り込む).
pub const OPEN_FETCH_OP: &str = "open-fetch";

/// Longest auto-fetch interval the UI offers, in minutes.
pub const AUTO_FETCH_MAX_MINUTES: u32 = 60;

pub const AUTO_FETCH_DEFAULT_MINUTES: u32 = 1;

/// Default cap on the graph window (GitKraken-like initial view). Bounds
/// memory and stream time on 100k+ commit repositories; the UI shows a
/// truncation hint when the cap is hit.
pub const DEFAULT_LOG_LIMIT: u32 = 2000;

/// What a write invalidates once it succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterWrite {
    /// The index and the working tree, and nothing else: staging,
    /// unstaging, discarding, cleaning. No ref moves, no stash appears or
    /// goes, no worktree is added or removed, and what is published is
    /// what it was — so none of those are read again.
    ///
    /// The saving is per press. Staging one line cost fourteen git
    /// invocations on a demo repository, ten of which could not have
    /// changed (2026-08-17 実測, Windows: `for-each-ref` 25ms, two
    /// `rev-list --count` 54ms, `stash list` + `worktree list` 46ms,
    /// `config --get-regexp remote` 25ms). On a repository with fifty
    /// thousand refs the first of those is the whole of the wait.
    ///
    /// **What it gives up**: a write no longer doubles as a poll for ref
    /// moves made outside this window. Those land on the next refresh
    /// instead — and staging is done with the graph off screen, where
    /// there is nothing for a poll to keep current
    /// (2026-08-17 ユーザー判断).
    Tree,
    /// Working tree / index / stash only.
    Snapshots,
    /// History or refs moved, so the graph has to be rebuilt too.
    Graph,
    /// Snapshots plus the author configuration (an identity write).
    Author,
}

/// Receives session events; implementations must be non-blocking.
pub trait SessionSink: Send + Sync + 'static {
    fn event(&self, event: SessionEvent);
}

/// One pending file whose change has something to say about line endings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EolMark {
    pub path: String,
    /// The whole statement, so the row that carries the mark can say the
    /// same sentence the diff pane would.
    pub notice: crate::eol::Notice,
    /// The **index** side is the one with something to say. A commit
    /// carries the index and nothing else, so this is what decides whether
    /// committing now would take the problem with it; a file marked only on
    /// its working-tree side is a warning about the next `git add`, not
    /// about this commit.
    pub staged: bool,
}
