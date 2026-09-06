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
use crate::process::{CommandEnd, GitCommand, GitExecutor, Kept};
use crate::publish;
use crate::reachable;
use crate::rebase_plan;
use crate::refs::{self, HeadState, RefEntry, RefKind};
use crate::remote;
use crate::repo::RepoInfo;
use crate::sequencer;
use crate::stage;
use crate::stash::{self, StashEntry};
use crate::status::{self, WorkTreeStatus};
use crate::tag;

mod author;
mod auto_fetch;
mod build;
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

pub use auto_fetch::OpenFetch;
pub use details_read::{DetailsOutcome, DetailsTask, SelectionRead};
pub use event::SessionEvent;
use feed::CommandFeed;
pub use feed::Recording;
use graph_refresh::GraphPasses;
pub use graph_refresh::{
    RefreshOutcome, RefreshTask, RemoteTagRefreshOutcome, RemoteTagRefreshTask,
};
use latest::Latest;
pub(crate) use model::LabelIndex;
pub use model::{LabelKind, LogOptions, LogRow, RefLabel};
use pass_watch::PassWatch;
pub use pass_watch::{PassHooks, PassStep};
use print::RowPrint;
use published::PublishMarks;
pub use query::{DiffRefreshOutcome, DiffRefreshTask};
use read_flight::{ReadFlight, Stamp};
pub(crate) use remote_tags::RemoteTagIndex;
pub use repo_session::RepoSession;
pub use snapshot::{BranchItem, RefsSnapshot, TagDrift, TagItem};
use standing::{HeadHold, HeadOffer, HeadPublished, Standing};
pub use state::AutoFetchTicker;
use state::{
    AutoFetch, ConfigStamp, Derived, EndingContext, Footer, OpenFetchState, Shared, WriteRequest,
};
pub use write::{remote_paced, replays_history};

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

/// Longest auto-fetch interval there is, in minutes: past an hour the
/// automatic fetch has no point left. The settings input offers up to
/// this, and [`auto_fetch_minutes`] is what everything else goes through.
pub const AUTO_FETCH_MAX_MINUTES: u32 = 60;

pub const AUTO_FETCH_DEFAULT_MINUTES: u32 = 1;

/// The interval that will actually run, for a number a person asked for.
/// Zero is off; past the ceiling, the ceiling is what a larger number
/// means — nearer to what was asked for than the default is.
///
/// The one place the ceiling is applied. A number typed into the settings
/// screen and a number written into `settings.toml` by hand reach the same
/// field, so anything either door decides on its own is a difference
/// nothing on screen would show.
#[must_use]
pub fn auto_fetch_minutes(minutes: u32) -> u32 {
    minutes.min(AUTO_FETCH_MAX_MINUTES)
}

/// Default cap on the graph window (GitKraken-like initial view). Bounds
/// memory and stream time on 100k+ commit repositories; the UI shows the
/// cut, and offers the next step of history, when the cap is hit.
pub const DEFAULT_LOG_LIMIT: u32 = 2000;

/// Fewest commits a graph can be told to open with.
///
/// Below this the window stops being one worth having: the step a press
/// adds is a quarter of it ([`log_window_step`]), so a floor any lower
/// buys a graph that has to be pressed before it says anything.
pub const MIN_LOG_LIMIT: u32 = 500;

/// The window that will actually open, for a number a person asked for.
/// Below the floor, the floor is what a smaller number means — nearer to
/// what was asked for than the default is.
///
/// The one place the floor is applied, for the reason
/// [`auto_fetch_minutes`] is the one place its ceiling is: the settings
/// screen and a hand-written `settings.toml` write the same field, so
/// anything either door decided on its own would be a difference nothing
/// on screen would show.
///
/// **There is no ceiling — the type is the ceiling.** What a wider
/// window costs is the walk, and the walk is nearly flat in the count
/// (measured on the baseline repository: mostly fixed frontier setup,
/// milliseconds per extra thousand commits), so a number typed on
/// purpose is one this can afford to answer. Asking for no window at all
/// is `None` rather than a large number, and does not come through here.
#[must_use]
pub const fn log_limit(limit: u32) -> u32 {
    if limit < MIN_LOG_LIMIT {
        MIN_LOG_LIMIT
    } else {
        limit
    }
}

/// What one press of the graph's tail adds, for a window that opened at
/// `initial` commits: a quarter of it.
///
/// **A fraction of the initial window rather than a number of its own**
/// — the initial count is a setting
/// (`settings::Defaults::initial_commits`), and the step moves with it.
///
/// **What a press costs is the walk's frontier setup, not the commits**
/// (measured: mostly fixed, milliseconds per extra thousand), so the
/// step is a question of how much a reader wants at once, not of what
/// the walk can afford — and a quarter of the default window is still
/// hundreds of commits.
#[must_use]
pub const fn log_window_step(initial: u32) -> u32 {
    if initial < 4 { 1 } else { initial / 4 }
}

/// What a write invalidates once it succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterWrite {
    /// The index and the working tree, and nothing else: staging,
    /// unstaging, discarding, cleaning. No ref moves, no stash appears or
    /// goes, no worktree is added or removed, and what is published is
    /// what it was — so none of those are read again.
    ///
    /// The saving is per press: most of the invocations a full refresh
    /// makes could not have changed (measured — on a repository with
    /// tens of thousands of refs, `for-each-ref` alone is the whole of
    /// the wait).
    ///
    /// **What it gives up**: a write does not double as a poll for ref
    /// moves made outside this window. Those land on the next refresh
    /// instead — and staging is done with the graph off screen, where
    /// there is nothing for a poll to keep current.
    Tree,
    /// Refs and nothing else: a fetch moves `refs/remotes/*` and writes
    /// `FETCH_HEAD`, and leaves the index and the working tree exactly
    /// where they were.
    ///
    /// So this one reads the refs first and the status only where they
    /// moved. Nothing else a status reports can have changed under such a
    /// write — the file lists, the standing operation, the merge tool,
    /// the line-ending marks — and the one thing that can, porcelain v2's
    /// `# branch.ab`, moves only when the upstream's remote-tracking ref
    /// does, which is the question the refs read already answers
    /// (`joins::refs_key`).
    ///
    /// The saving is per tick, on the longest read in the app: automatic
    /// fetching runs by the minute, almost every tick brings nothing
    /// down, and one `git status -uall` walks every tracked and every
    /// ignored path there is (measured on the 200,000-commit synthetic
    /// corpus — 109,652 tracked, 78,000 ignored: 2.9s wall, 2.3 CPU
    /// seconds spread over the preload-index threads).
    ///
    /// **Only for a write that reaches neither the index, the working
    /// tree, nor the configuration.** The status event carries the push
    /// marks and the merge tool beside the files, and those are `git
    /// config` reads that `refs_key` does not answer for: a write that
    /// edited config without moving a ref would have its mark go unread
    /// until the next poll. A commit, a merge, a rebase, a switch move
    /// history and the tree together, and read them together
    /// ([`AfterWrite::Graph`]).
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
/// recovered rather than every later reader failing too.
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
