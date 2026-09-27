//! RepoSession: one open repository = one session (実装計画 §2).
//!
//! Owns all git activity for a repository on a tokio runtime; results go
//! to the UI through a [`SessionSink`].
//!
//! Reads run concurrently; writes go through one queue, since order is
//! part of the contract (実装計画 §2). Every write refreshes afterwards —
//! a failed one too, since a command that stops halfway (a conflicted
//! merge) has still changed the repository.
//!
//! One repository can have more than one session at a time — a tab closed
//! mid-write outlives its page, and the tab reopened over it is a second
//! session on the same index. The queue is the session's; the *order* it
//! serves in belongs to the working tree (`session::write_order`).

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
use crate::process::{CommandEnd, GitCommand, GitExecutor, Kept};
use crate::publish;
use crate::reachable;
use crate::rebase_plan;
use crate::refs::{self, HeadState, RefEntry, RefKind};
use crate::remote;
use crate::repo::RepoInfo;
use crate::report;
use crate::sequencer;
use crate::stage;
use crate::stash::{self, StashEntry};
use crate::status::{self, WorkTreeStatus};
use crate::tag;

mod author;
mod auto_fetch;
mod build;
mod carried;
mod chips;
mod details_read;
mod eol;
mod event;
mod feed;
mod graph_refresh;
mod head_reach;
mod heap;
#[cfg(test)]
mod join_tests;
mod joins;
mod latest;
mod log;
mod model;
mod open;
mod ops_integrate;
mod ops_refs;
mod ops_remote;
mod ops_remote_tags;
mod ops_stash;
mod ops_tree;
mod pass_watch;
mod print;
mod published;
mod query;
mod read_flight;
mod refresh;
mod refresh_op;
mod relay;
#[cfg(test)]
mod relay_tests;
mod remote_tags;
mod repo_session;
mod rows;
mod snapshot;
mod standing;
mod stash_round;
mod state;
#[cfg(test)]
mod state_tests;
mod walk;
mod write;
mod write_order;

pub use auto_fetch::OpenFetch;
pub use carried::{
    COPIES_INTERVAL_DEFAULT_SECS, COPIES_INTERVAL_MAX_SECS, COPIES_INTERVAL_MIN_SECS, Carried,
    CarriedOutcome, CarriedPass, copies_interval_secs,
};
pub use details_read::{DetailsOutcome, DetailsTask, SelectionRead};
pub use event::SessionEvent;
use feed::CommandFeed;
pub use feed::Recording;
use graph_refresh::{GraphPasses, GraphRun};
pub use graph_refresh::{
    RefreshOutcome, RefreshTask, RemoteTagRefreshOutcome, RemoteTagRefreshTask,
};
pub use joins::same_path_key;
use latest::Latest;
pub(crate) use model::LabelIndex;
pub use model::{LabelKind, LogOptions, LogRow, RefLabel};
pub use open::{DrawnGraph, FirstPass};
use pass_watch::PassWatch;
pub use pass_watch::{PassHooks, PassStep};
use print::RowPrint;
use published::{PublishMarks, RemoteTips};
pub use query::{DiffReadOutcome, DiffRefreshTask};
use read_flight::{ReadFlight, Stamp};
pub use remote_tags::RemoteTagIndex;
pub use repo_session::RepoSession;
pub use snapshot::{BranchItem, RefsSnapshot, TagDeleteHeld, TagDrift, TagItem, TagMenuFacts};
use standing::{HeadHold, HeadOffer, HeadPublished, Standing};
pub use state::AutoFetchTicker;
use state::{
    AutoFetch, ConfigStamp, Derived, EndingContext, Footer, OpenFetchState, Operation, Reread,
    Shared, WorktreeRead, WriteRequest,
};
use walk::{Building, WalkInputs};
use write_order::WriteOrder;

use crate::operation::{Lane, OperationId, OperationKind};

/// First chunk is small so the first paint happens as early as possible.
const FIRST_CHUNK_ROWS: usize = 512;
const CHUNK_ROWS: usize = 4096;

/// Longest auto-fetch interval, in minutes — past an hour the automatic
/// fetch has no point left. Applied through [`auto_fetch_minutes`].
pub const AUTO_FETCH_MAX_MINUTES: u32 = 60;

pub const AUTO_FETCH_DEFAULT_MINUTES: u32 = 1;

/// The interval that will actually run for a number a person asked for:
/// zero is off, and past the ceiling the ceiling is meant. The one place
/// it is applied — the settings screen and a hand-edited `settings.toml`
/// both come through here (rules-refs/core.md「上限の適用点を 1 つ持ち」).
#[must_use]
pub fn auto_fetch_minutes(minutes: u32) -> u32 {
    minutes.min(AUTO_FETCH_MAX_MINUTES)
}

/// Default cap on the graph window; bounds memory and stream time on 100k+
/// commit repositories.
pub const DEFAULT_LOG_LIMIT: u32 = 2000;

/// Fewest commits a graph can be told to open with. The step a press adds
/// is a quarter of it ([`log_window_step`]), so any lower buys a graph
/// that has to be pressed before it says anything.
pub const MIN_LOG_LIMIT: u32 = 500;

/// The window that will actually open for a number a person asked for:
/// below the floor, the floor is meant. The one place the floor is
/// applied, as [`auto_fetch_minutes`] is for its ceiling.
///
/// No ceiling: the walk is nearly flat in the count (mostly fixed frontier
/// setup; ci/baseline/code-costs-windows-x64.md §git のプロセス代), so a
/// number typed on purpose is affordable. No window at all is `None` and
/// does not come through here.
#[must_use]
pub const fn log_limit(limit: u32) -> u32 {
    if limit < MIN_LOG_LIMIT {
        MIN_LOG_LIMIT
    } else {
        limit
    }
}

/// What one press of the graph's tail adds, for a window that opened at
/// `initial` commits: a quarter of it, so the step follows the setting
/// (`settings::Defaults::initial_commits`).
#[must_use]
pub const fn log_window_step(initial: u32) -> u32 {
    if initial < 4 { 1 } else { initial / 4 }
}

/// One of the reads a write's settling waits for — what
/// [`SessionEvent::WriteSettled`] names when one of them did not put the
/// write's result on screen. The label is the one the failure itself went
/// out under ([`SessionEvent::OpFailed`]), so the two name the same read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowUp {
    /// The working tree and the standing operation.
    Status,
    /// The refs listing, the snapshot and the chips.
    Refs,
    /// The graph rebuild, asked for by the write or by a read it made.
    Graph,
    Stashes,
    Worktrees,
    /// The author identity, after an identity write.
    Author,
}

impl FollowUp {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Refs => "refs",
            Self::Graph => "log",
            Self::Stashes => "stash",
            Self::Worktrees => "worktrees",
            Self::Author => "identity",
        }
    }
}

/// What a write invalidates once it succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterWrite {
    /// The index and the working tree, and nothing else: staging,
    /// unstaging, discarding, cleaning — so refs, stashes and worktrees
    /// are not read again (on a large repository the refs listing is most
    /// of a full refresh's wait). What it gives up: the write does not
    /// double as a poll for ref moves made outside this window.
    Tree,
    /// Refs and nothing else (a fetch): reads the refs first and the
    /// status only where they moved — the one status field such a write
    /// can move, `# branch.ab`, follows the upstream's remote-tracking ref
    /// (`joins::refs_keys`). Saves a `git status -uall`, the longest read
    /// in the app, on almost every auto-fetch tick
    /// (ci/baseline/code-costs-windows-x64.md §git のプロセス代).
    ///
    /// Only for a write that touches refs alone: the status event also
    /// carries the push marks and the merge tool, `git config` reads the
    /// refs key does not see.
    Refs,
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

/// Takes a lock, poisoned or not. Every lock in the session guards plain
/// data that stays usable after a holder panicked, so the guard is
/// recovered.
pub(crate) fn relock<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// One pending file whose change has something to say about line endings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EolMark {
    pub path: String,
    /// The whole statement, so the row says the same sentence the diff
    /// pane would.
    pub notice: crate::eol::Notice,
    /// The index side is the one with something to say — whether
    /// committing now takes the problem with it. Marked only on the
    /// working-tree side, it warns about the next `git add`.
    pub staged: bool,
}
