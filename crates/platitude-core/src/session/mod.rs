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
use crate::repo::{self, RepoInfo};
use crate::sequencer;
use crate::stage;
use crate::stash::{self, StashEntry};
use crate::status::{self, WorkTreeStatus};
use crate::tag;

mod auto_fetch;
mod build;
mod eol;
mod log;
mod ops_integrate;
mod ops_remote;
mod ops_tree;
mod query;
mod refresh;
mod write;

/// First chunk is small so the first paint happens as early as possible.
const FIRST_CHUNK_ROWS: usize = 512;
const CHUNK_ROWS: usize = 4096;

/// Op name of the interval-driven fetch. The UI keeps this one off the
/// shared error surface, so both sides have to agree on the spelling.
pub const AUTO_FETCH_OP: &str = "auto-fetch";

/// Longest auto-fetch interval the UI offers, in minutes. Beyond an hour
/// the point of an automatic fetch is gone; use the manual one.
pub const AUTO_FETCH_MAX_MINUTES: u32 = 60;

/// Interval auto fetch starts at when nothing says otherwise.
pub const AUTO_FETCH_DEFAULT_MINUTES: u32 = 1;

/// Default cap on the graph window (GitKraken-like initial view). Bounds
/// memory and stream time on 100k+ commit repositories; the UI shows a
/// truncation hint when the cap is hit.
pub const DEFAULT_LOG_LIMIT: u32 = 2000;

/// What the log stream walks.
///
/// Tags are shown by default (product decision). On tag-heavy repositories
/// they dominate the walk's frontier setup (JetBrains/kotlin: 44k tags cost
/// ~1.7s extra before the first byte even with a commit-graph), which is
/// why the toggle exists.
#[derive(Debug, Clone, Copy)]
pub struct LogOptions {
    pub include_tags: bool,
    /// `None` walks the full history.
    pub limit: Option<u32>,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            include_tags: true,
            limit: Some(DEFAULT_LOG_LIMIT),
        }
    }
}

/// Kind of a row label chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LabelKind {
    /// Detached-HEAD marker (synthetic `HEAD` chip).
    Head,
    LocalBranch,
    RemoteBranch,
    Tag,
}

/// One label chip on a graph row (branch / tag / HEAD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLabel {
    pub text: String,
    pub kind: LabelKind,
    /// Cloud badge: this name is on a remote as well. The PR dimension is
    /// wired in Phase 4.
    pub has_remote: bool,
    /// True when HEAD is on this branch (bold chip).
    pub is_head: bool,
    /// Whether this repository holds the ref — the whereabouts the chip
    /// writes in the name's colour (デザイン規約 §ref の種別). False for a
    /// remote branch, and for a tag that is only over there or that points
    /// somewhere this one does not.
    pub here: bool,
    /// Whose reading this is, when it is not this repository's: the remote
    /// names carrying the tag, comma-separated. Empty for everything else.
    ///
    /// A remote branch says it in its own name (`origin/main`); a tag has
    /// no such namespace to say it in, and a drifted one puts the same
    /// bare name on two rows. The hover card is where those two meet, and
    /// this is what tells them apart there.
    pub remote: String,
}

/// Display-ready row of the commit graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRow {
    pub row: u32,
    pub oid_hex: String,
    pub short_sha: String,
    pub author: String,
    /// The author's address, lowercased and mailmapped — what a locally
    /// assigned picture is filed under. Empty on the WIP row, which has
    /// no author until it is committed.
    pub author_email: String,
    /// Whoever the message credits alongside the author, in its order.
    /// Empty on the WIP and stash rows.
    pub co_authors: Vec<crate::details::CoAuthor>,
    /// Author time (unix seconds); formatting is presentation.
    pub time: i64,
    pub subject: String,
    /// Everything after the subject, minus the co-author trailers — what
    /// the row's hover reads out. Empty on the WIP and stash rows.
    pub body: String,
    pub node_lane: u16,
    pub node_color: u8,
    pub width: u16,
    pub segments: Vec<Segment>,
    pub labels: Vec<RefLabel>,
    /// Reflog selector (`stash@{n}`) when this row is a stash; empty for
    /// ordinary commits and the WIP row.
    pub stash_ref: String,
}

/// What the remotes last said they carry under `refs/tags/`: tag name →
/// every commit some remote has it on, and what is known about it there.
///
/// Two commits under one name means the remotes disagree, which reads on
/// screen exactly like a tag that drifted from the one here — the name
/// standing on more than one row.
type RemoteTagIndex = BTreeMap<String, BTreeMap<Oid, RemoteTagPlace>>;

/// One reading of a tag: what the remotes holding it there call themselves,
/// and whether it is annotated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RemoteTagPlace {
    annotated: bool,
    /// Sorted, and more than one when several remotes agree on the commit.
    remotes: Vec<String>,
}

/// The same thing before it is merged, kept per remote so one that could
/// not be reached keeps its last answer instead of dropping every badge it
/// accounted for. `refs/remotes/` does this for branches; a tag has no such
/// local record, so the session holds it.
type RemoteTagsByRemote = BTreeMap<String, Vec<remote::RemoteTag>>;

/// Sidebar-ready refs snapshot (sorted).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefsSnapshot {
    pub locals: Vec<BranchItem>,
    pub remotes: Vec<BranchItem>,
    pub tags: Vec<TagItem>,
    pub head: Option<HeadState>,
    /// Names of the configured remotes, sorted. A branch with no upstream
    /// has to be told where to go, and this is the list to offer.
    pub remote_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchItem {
    pub short: String,
    pub full: String,
    pub oid_hex: String,
    pub has_remote: bool,
    pub is_head: bool,
    /// For a local branch, the remote branch it speaks for (`origin/main`),
    /// wherever the two stand — the one its badge is about, and the one a
    /// rename offers to carry over. Empty when it speaks for none.
    pub upstream: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagItem {
    pub short: String,
    /// Peeled commit id (what the graph row is keyed on).
    pub oid_hex: String,
    pub annotated: bool,
    /// Creator date (unix seconds); the sidebar sorts tags newest-first.
    /// Zero for a tag only a remote has: an advertisement carries the name
    /// and the commit, and no date to sort by.
    pub created_unix: i64,
    /// A remote carries this name too (cloud badge).
    pub has_remote: bool,
    /// Whether this repository holds the tag. False lists a name only a
    /// remote has — the sidebar is where it can be read at all, since no
    /// local ref puts it on a graph row.
    pub here: bool,
}

/// Everything the session can tell the UI.
#[derive(Debug)]
pub enum SessionEvent {
    Opened {
        info: RepoInfo,
    },
    /// The repository would not open. The path is the one the session was
    /// asked for: only some errors carry it themselves, and the screen
    /// names the folder whichever error it was.
    OpenFailed {
        path: PathBuf,
        error: GitError,
    },
    /// A (re)load of the graph began; the model must reset.
    LogStarted {
        generation: u64,
    },
    LogChunk {
        generation: u64,
        rows: Vec<LogRow>,
    },
    LogFinished {
        generation: u64,
        total: u32,
        elapsed_ms: u64,
        /// Commits the walk emitted — what `--max-count` caps, so this
        /// equals the window limit whenever `truncated`. The truncation
        /// footer's number: `total` counts shown rows, which the WIP row
        /// and sifted stash parents move off any round figure.
        walked: u32,
        /// True when the stream stopped at the configured window limit
        /// (older history exists but is not shown).
        truncated: bool,
    },
    LogFailed {
        generation: u64,
        error: String,
    },
    /// A background rebuild finished and replaces the whole graph in one
    /// step. Deliberately one event rather than Started/Chunk/Finished:
    /// those travel as separate queued messages, and a consumer that
    /// drains between them paints an empty model for a frame — visible
    /// as a white flash whenever a background refresh finds changes.
    LogReplaced {
        generation: u64,
        rows: Vec<LogRow>,
        elapsed_ms: u64,
        /// As on [`SessionEvent::LogFinished`].
        walked: u32,
        truncated: bool,
    },
    /// Labels of already-delivered rows changed (refs arrived/refreshed).
    /// `generation` names the graph the row numbers were read from, so a
    /// consumer showing another one drops them instead of putting chips
    /// on whatever commit now sits at those numbers.
    LabelsChanged {
        generation: u64,
        rows: Vec<(u32, Vec<RefLabel>)>,
    },
    /// Shared rather than owned: every sidebar section is handed the whole
    /// snapshot and reads its own part of it, and a deep copy per section
    /// is tens of thousands of strings duplicated for nobody
    /// (`JetBrains/kotlin`: 45,782 tags).
    RefsLoaded {
        snapshot: Arc<RefsSnapshot>,
    },
    StatusLoaded {
        status: WorkTreeStatus,
        op_state: OpState,
        /// "commit N of M" while a rebase is stepping through commits.
        progress: Option<conflict::Progress>,
        /// What to call the two sides of a conflict, for whatever is
        /// stopped. Default while nothing is.
        sides: conflict::Sides,
        /// The merge tool git would launch, empty when none is configured
        /// or when nothing is conflicted. Display only — the launch reads
        /// the config again, so a stale name here cannot start anything.
        merge_tool: String,
        /// Pending paths whose change has something to say about line
        /// endings. Shared rather than copied: most status reads repeat the
        /// previous answer unchanged, and every open tab does it.
        eol_marks: Arc<Vec<EolMark>>,
    },
    /// Answer to [`RepoSession::ask_merge_tools`]: names the settings field
    /// can offer, deliberate ones first. Empty is a valid answer.
    ///
    /// Sent twice where config named anything: once with those, so the
    /// eight-second read does not hold back an answer that took
    /// milliseconds, and once with everything. `settled` marks the last.
    MergeToolsLoaded {
        names: Vec<String>,
        settled: bool,
    },
    /// Answer to [`RepoSession::check_remote_branch`]: whether the remote
    /// already carries that exact name, as of this moment rather than as of
    /// the last fetch. The question is echoed back because the box that
    /// asked it may have moved on to another name by the time this lands.
    RemoteBranchChecked {
        remote: String,
        branch: String,
        /// What a push under that name would meet: taken or not, and where
        /// taken, whether git would carry it or turn it down. Silence from
        /// the remote is one of the answers rather than a missing one —
        /// reading it as "nothing is there" would send on the assumption
        /// that git refuses what it does not.
        state: remote::RemoteBranchState,
        /// The commit the remote advertised, hex, empty where it named
        /// none. What an overwrite would have to lease against, so the
        /// question can offer one pinned to what it showed.
        tip: String,
        /// How many commits that tip reaches that this history does not —
        /// what an overwrite would take off that branch. Zero unless the
        /// answer was `Refused`, which is the only state where both the
        /// commit and the walk are here.
        theirs: u32,
    },
    /// Answer to [`RepoSession::check_branch_delete`]: whether the branch
    /// is merged into the reference point `branch --delete` measures
    /// against (its upstream, or HEAD without one). The branch is echoed
    /// back because the menu that asked may be open over another row by
    /// the time this lands. No event is sent where the reads fail —
    /// silence leaves the delete row on its plain form, and a refusal,
    /// if any, lands the way it always has.
    BranchDeleteChecked {
        branch: String,
        merged: bool,
    },
    /// Answer to [`RepoSession::check_publish`].
    PublishChecked {
        range: String,
        state: publish::PublishState,
    },
    /// Answer to [`RepoSession::check_in_history`].
    InHistoryChecked {
        oid: String,
        in_history: bool,
    },
    /// Whether anything besides the branch HEAD is on still reaches its
    /// tip, so the rows that rewrite history can tell a rewrite that
    /// leaves the old commits drawn from one that leaves them to the
    /// reflog (see [`crate::reachable`]). Sent when the answer moves.
    HeadReachChecked {
        reached_elsewhere: bool,
    },
    /// Answer to [`RepoSession::check_signature`].
    SignatureChecked {
        oid: String,
        signature: identity::Signature,
    },
    /// Answer to [`RepoSession::load_head_commit`] — what an amend starts
    /// from: HEAD's message, and whose commit it is about to replace. All
    /// empty on an unborn branch.
    HeadCommitLoaded {
        head: commit::HeadCommit,
    },
    /// Author identity and signing configuration. Emitted on open so the
    /// UI can ask for an identity before the first commit fails.
    AuthorLoaded {
        config: identity::AuthorConfig,
    },
    StashesLoaded {
        stashes: Vec<StashEntry>,
    },
    WorktreesLoaded {
        worktrees: Vec<crate::worktrees::WorktreeEntry>,
    },
    DetailsLoaded {
        details: CommitDetails,
    },
    DiffLoaded {
        target: DiffTarget,
        patches: Vec<FilePatch>,
        /// Image bytes / binary sizes when the text diff is not the whole
        /// story (`None` for ordinary text files).
        preview: Option<FilePreview>,
        /// Fingerprint of the bytes `patches` was parsed from. Hunk/line
        /// selections carry it back, so a partial write can refuse a diff
        /// that drifted under the selection (`stage::apply_partial`).
        fingerprint: u64,
        /// What the same bytes said about line endings, if anything. Rides
        /// with the diff rather than following it: a notice that appears
        /// after the reader has started is worse than none.
        endings: Option<crate::eol::Notice>,
    },
    /// A background refresh/query failed (op is a stable identifier).
    OpFailed {
        op: &'static str,
        error: GitError,
    },
    /// A branch move would leave commits unreachable, so it was not made.
    /// Nothing changed; the UI asks before running it for real
    /// ([`RepoSession::checkout`] with [`CheckoutTarget::ForceCreate`]).
    MoveNeedsAsk {
        local: String,
        start: String,
    },
    /// A write operation started; the UI can show it as in flight.
    WriteStarted {
        op: &'static str,
    },
    /// A write operation ended. `error` carries git's own message.
    WriteFinished {
        op: &'static str,
        error: Option<String>,
    },
    /// A git subprocess was spawned (command log). Only what the user
    /// asked for, unless background reads were switched on.
    CommandStarted {
        id: u64,
        /// The command as a log line shows it.
        display: String,
        /// The same command with the always-applied configuration and
        /// environment spelled out, for copying.
        full: String,
        /// Wall clock at the spawn, milliseconds since the epoch.
        at_ms: i64,
    },
    /// The command with this id ended.
    CommandFinished {
        id: u64,
        end: CommandEnd,
        elapsed_ms: u64,
        /// git's own output on the way out (stderr), or the reason it
        /// never ran. Empty when it said nothing.
        message: String,
    },
}

/// Turns invocations into session events, so the command log travels the
/// same path as everything else the UI shows.
struct CommandFeed {
    sink: Arc<dyn SessionSink>,
    next_id: AtomicU64,
    /// Off by default: a poll tick runs five commands and would bury the
    /// operations the user actually performed.
    record_background: std::sync::atomic::AtomicBool,
}

/// Id of a command that is not being recorded; its end is dropped too.
const UNRECORDED: u64 = 0;

impl CommandFeed {
    fn new(sink: Arc<dyn SessionSink>) -> Self {
        Self {
            sink,
            next_id: AtomicU64::new(UNRECORDED + 1),
            record_background: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl crate::process::CommandObserver for CommandFeed {
    fn records(&self, user: bool) -> bool {
        user || self.record_background.load(Ordering::Relaxed)
    }

    fn started(&self, display: &str, full: &str, user: bool) -> u64 {
        if !self.records(user) {
            return UNRECORDED;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default();
        self.sink.event(SessionEvent::CommandStarted {
            id,
            display: display.to_string(),
            full: full.to_string(),
            at_ms,
        });
        id
    }

    fn finished(&self, id: u64, end: CommandEnd, elapsed_ms: u64, message: &str) {
        if id == UNRECORDED {
            return;
        }
        self.sink.event(SessionEvent::CommandFinished {
            id,
            end,
            elapsed_ms,
            message: message.to_string(),
        });
    }
}

/// What the refs read last saw of the branch tip, which is everything the
/// reachability walk needs to start (see [`crate::reachable`]).
#[derive(Debug, Clone, PartialEq, Eq)]
struct HeadHold {
    /// The commit HEAD is on.
    tip: Oid,
    /// Short name of the branch HEAD is on; empty when detached.
    branch: String,
    /// Some other ref already sits exactly on `tip`, which the listing
    /// answers on its own — no walk needed.
    on_a_ref: bool,
}

/// What a write invalidates once it succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterWrite {
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

/// One tick handed to the timer by hand rather than by the clock; the
/// timer answers on it once it has acted (see [`AutoFetchTicker`]).
type AutoFetchTick = tokio::sync::oneshot::Sender<()>;

/// The auto-fetch timer that is running: what stops it, and the way in for
/// a tick that does not come from the clock.
struct AutoFetch {
    cancel: CancellationToken,
    ticks: tokio::sync::mpsc::UnboundedSender<AutoFetchTick>,
}

/// Steps one auto-fetch timer by hand, in place of waiting out its
/// interval.
///
/// Handed out by [`RepoSession::auto_fetch_ticker`] and bound to the timer
/// that was running when it was taken, so a tick reports back whether that
/// timer is still there to take it. That is what makes "this timer fetches
/// nothing any more" something to wait for rather than a wall-clock margin
/// to guess at, which is all the tests have to go on otherwise: a fetch
/// queued a moment before the stop can start much later on a loaded
/// machine, and no length of quiet proves the next one is not coming. The
/// app only ever sets an interval.
pub struct AutoFetchTicker(tokio::sync::mpsc::UnboundedSender<AutoFetchTick>);

impl AutoFetchTicker {
    /// Fires one tick and resolves once the timer has acted on it: `true`
    /// when it took the tick, `false` once that timer has stopped — turned
    /// off, replaced by another interval, or gone with the session. A
    /// stopped timer never takes a tick, not even one that raced its own
    /// cancellation, so `false` is the last word on it.
    pub async fn tick(&self) -> bool {
        let (ack, taken) = tokio::sync::oneshot::channel();
        if self.0.send(ack).is_err() {
            return false;
        }
        taken.await.is_ok()
    }
}

/// A queued write: what to run, what it invalidates, what to call it.
struct WriteRequest {
    op: &'static str,
    after: AfterWrite,
    #[expect(
        clippy::type_complexity,
        reason = "a boxed async job needs its shape spelled out"
    )]
    run: Box<
        dyn FnOnce(
                GitExecutor,
                RepoInfo,
                CancellationToken,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), GitError>> + Send>,
            > + Send,
    >,
}

/// What a pass reported *under* its graph rather than in it: how far the
/// walk got, and whether it stopped because the window ran out.
///
/// Kept beside the rows because it does not follow from them. The walk
/// count drifts from the shown count in both directions (the WIP row is
/// shown but never walked, sifted stash parents are walked but never
/// shown), and truncation is a property of the walk alone — so two passes
/// can draw the very same graph and still owe the consumer different
/// answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Footer {
    walked: u32,
    truncated: bool,
}

/// Shared mutable state between the log task and refs joins.
#[derive(Default)]
struct Shared {
    builder: GraphBuilder,
    /// The graph the consumer is on — the last pass that reached it, in
    /// the row numbers `builder`, `applied` and `sent_rows` speak in.
    /// Not `log_gen`: that counter is bumped before a pass takes this
    /// lock and bumped by passes that send nothing at all, and either way
    /// everything here still belongs to the walk that was shown.
    generation: u64,
    /// Label chips per commit id, derived from the last refs snapshot.
    label_map: HashMap<Oid, Vec<RefLabel>>,
    /// Labels currently shown per row (for diffing on refs refresh).
    applied: HashMap<u32, Vec<RefLabel>>,
    /// Rows exactly as delivered to the UI (labels included), kept so a
    /// background rebuild can tell "same picture" from "changed" and skip
    /// the swap entirely — an unchanged repository must not repaint.
    sent_rows: Vec<LogRow>,
    /// The footer delivered with them, compared alongside the rows for the
    /// same decision: a window change can leave every row where it is and
    /// still change the answer beneath them (two commits through a window
    /// of two are cut; through a window of three they are not), and that
    /// change reaches the consumer through a rebuild whenever one overtakes
    /// the stream the change asked for. `None` = no pass has answered for
    /// this graph yet, so the next one to finish has something to say.
    sent_footer: Option<Footer>,
}

/// Guards snapshot-replacing ops against out-of-order completion.
#[derive(Default)]
struct OpGate(AtomicU64);

impl OpGate {
    fn begin(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
    fn is_current(&self, generation: u64) -> bool {
        self.0.load(Ordering::SeqCst) == generation
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

/// What is known about a path's line endings before its patch is read.
enum EndingContext {
    /// git calls the path something other than text, so nothing is said
    /// about it. Also where a failed reading lands: not knowing whether a
    /// path is binary is a reason to stay quiet, not to guess.
    Excluded,
    /// Worth reading, with the neighbours' opinion if one was needed and
    /// could be had.
    Open(Option<crate::eol::Baseline>),
}

pub struct RepoSession {
    /// Reads and refreshes: recorded in the command log only while
    /// background recording is on.
    executor: GitExecutor,
    /// The queue's handle: everything run through it is something the
    /// user asked for, and is always recorded.
    exec_user: GitExecutor,
    commands: Arc<CommandFeed>,
    runtime: tokio::runtime::Handle,
    sink: Arc<dyn SessionSink>,
    /// Cancelled when the session closes; all ops derive from it.
    root_cancel: CancellationToken,
    info: Mutex<Option<RepoInfo>>,
    shared: Arc<Mutex<Shared>>,
    log_options: Mutex<LogOptions>,
    log_gen: AtomicU64,
    log_cancel: Mutex<Option<CancellationToken>>,
    /// Dirty working tree → the log stream prepends a synthetic WIP row.
    wip_dirty: std::sync::atomic::AtomicBool,
    /// Set by [`RepoSession::ask_merge_tool`] to have the next status read
    /// name the merge tool even with nothing conflicted. Cleared by that
    /// read: two `git config` spawns on every poll of every open tab is
    /// not a price the common case should pay for a settings field.
    merge_tool_wanted: std::sync::atomic::AtomicBool,
    /// The last answer, repeated by refreshes that did not read it.
    merge_tool_seen: Mutex<String>,
    /// Line-ending baselines already sampled, keyed by (directory,
    /// extension) — what a house style is scoped to, and what makes the
    /// second file opened in a directory cost nothing.
    ///
    /// `None` is a cached "unknown", which is worth keeping: not knowing
    /// costs the same reads as knowing. Emptied whenever a write lands or
    /// refs move, since either can bring a new `.gitattributes` or change
    /// what the neighbours look like.
    eol_baselines: Mutex<HashMap<(String, String), Option<crate::eol::Baseline>>>,
    /// Pending paths whose change has something to say about line endings,
    /// repeated by every status read until something asks for them again.
    eol_marks: Mutex<Arc<Vec<EolMark>>>,
    /// Set when the marks are worth re-reading whatever status says: a
    /// write landed, or the tab has only just opened.
    eol_marks_stale: std::sync::atomic::AtomicBool,
    /// Fingerprint of the last status read, so a tick that finds the same
    /// files in the same states reads no diffs at all.
    status_key: Mutex<Option<u64>>,
    /// Fingerprint of the last refs read (see [`refs_key`]), so a refresh
    /// can tell an external commit / fetch / switch from a quiet re-read.
    /// `None` until the first read: opening already streams the graph.
    refs_key: Mutex<Option<u64>>,
    /// The snapshot last published. A read that finds nothing moved hands
    /// this one out again rather than an equal copy, so the sidebar can
    /// tell "the same" from "equal" by pointer — a repository with tens of
    /// thousands of tags must not rebuild every section, on the Qt thread,
    /// to discover that a poll tick changed nothing.
    last_snapshot: Mutex<Option<Arc<RefsSnapshot>>>,
    /// Set while the write queue runs a request, so the poll can stay out
    /// of a repository that is mid-operation.
    write_busy: std::sync::atomic::AtomicBool,
    /// One permit, held by a running poll: a tick that arrives while the
    /// previous one is still reading is dropped rather than queued.
    poll_slot: Arc<tokio::sync::Semaphore>,
    /// Submission end of the write queue (see the module docs).
    write_tx: tokio::sync::mpsc::UnboundedSender<WriteRequest>,
    /// Time budget for fetch / push. Persisted as the settings key
    /// `network_timeout_secs`; only the settings dialog's input field is
    /// missing (実装計画 §7).
    network_timeout: Mutex<std::time::Duration>,
    /// What each remote last advertised under `refs/tags/`. Empty until a
    /// fetch has been through: asking costs the network, so it rides the
    /// one command the user already meant to spend it on, and before that
    /// every tag reads as one this repository alone has.
    remote_tags: Mutex<RemoteTagsByRemote>,
    /// The same answers merged into the shape the joins read, rebuilt only
    /// when a remote has spoken. Shared because every refs read wants it
    /// and none of them changes it: merging tens of thousands of names on
    /// every poll tick copies the whole tag list for nothing
    /// (`JetBrains/kotlin`: 45,782 names).
    remote_tag_index: Mutex<Arc<RemoteTagIndex>>,
    /// One permit for the background read of the above, so a second
    /// permission-granting call cannot stack another on top of it.
    remote_tags_slot: Arc<tokio::sync::Semaphore>,
    /// What the last refs read saw of the branch tip, so the walk behind
    /// [`reachable`] can be started without reading the listing again.
    head_hold: Mutex<Option<HeadHold>>,
    /// The last answer sent, so a re-check landing on the same one says
    /// nothing.
    head_reach_seen: Mutex<Option<bool>>,
    /// One permit: the walk is the only part of a refresh that scales with
    /// the history rather than the refs, and a tick arriving mid-walk is
    /// dropped rather than stacked.
    head_reach_slot: Arc<tokio::sync::Semaphore>,
    /// One merge-tool candidate read at a time. Opening settings twice in
    /// a row must not start a second eight-second walk of the registry.
    merge_tools_slot: Arc<tokio::sync::Semaphore>,
    /// The running auto-fetch timer, if any.
    auto_fetch: Mutex<Option<AutoFetch>>,
    /// The interval the timer was last *asked* for, kept while it is
    /// suspended so there is something to put back.
    auto_fetch_interval: Mutex<Option<std::time::Duration>>,
    /// One permit: an auto fetch that is still queued or running holds it,
    /// so a tick that arrives meanwhile is skipped instead of stacking up.
    /// A permit moved into a dropped request is released with it.
    auto_fetch_slot: Arc<tokio::sync::Semaphore>,
    refs_gate: OpGate,
    status_gate: OpGate,
    stash_gate: OpGate,
    worktrees_gate: OpGate,
}

impl RepoSession {
    /// Creates the session and starts opening `path` in the background.
    /// On success everything loads: log stream, refs, status, stashes.
    pub fn open(
        executor: GitExecutor,
        runtime: tokio::runtime::Handle,
        path: PathBuf,
        sink: Arc<dyn SessionSink>,
    ) -> Arc<Self> {
        let (write_tx, write_rx) = tokio::sync::mpsc::unbounded_channel();
        let commands = Arc::new(CommandFeed::new(Arc::clone(&sink)));
        let observer: Arc<dyn crate::process::CommandObserver> = Arc::clone(&commands) as _;
        let session = Arc::new(Self {
            executor: executor.observed(Arc::clone(&observer), false),
            exec_user: executor.observed(observer, true),
            commands,
            runtime: runtime.clone(),
            sink,
            root_cancel: CancellationToken::new(),
            info: Mutex::new(None),
            shared: Arc::new(Mutex::new(Shared::default())),
            log_options: Mutex::new(LogOptions::default()),
            log_gen: AtomicU64::new(0),
            log_cancel: Mutex::new(None),
            wip_dirty: std::sync::atomic::AtomicBool::new(false),
            merge_tool_wanted: std::sync::atomic::AtomicBool::new(false),
            merge_tool_seen: Mutex::new(String::new()),
            eol_baselines: Mutex::new(HashMap::new()),
            eol_marks: Mutex::new(Arc::new(Vec::new())),
            eol_marks_stale: std::sync::atomic::AtomicBool::new(true),
            status_key: Mutex::new(None),
            refs_key: Mutex::new(None),
            last_snapshot: Mutex::new(None),
            write_busy: std::sync::atomic::AtomicBool::new(false),
            poll_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            write_tx,
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            remote_tags: Mutex::new(RemoteTagsByRemote::new()),
            remote_tag_index: Mutex::new(Arc::new(RemoteTagIndex::new())),
            remote_tags_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            head_hold: Mutex::new(None),
            head_reach_seen: Mutex::new(None),
            head_reach_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            merge_tools_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            auto_fetch: Mutex::new(None),
            auto_fetch_interval: Mutex::new(None),
            auto_fetch_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            refs_gate: OpGate::default(),
            status_gate: OpGate::default(),
            stash_gate: OpGate::default(),
            worktrees_gate: OpGate::default(),
        });
        runtime.spawn(Arc::clone(&session).write_loop(write_rx));

        let s = Arc::clone(&session);
        runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match repo::open(&s.executor, &path, &cancel).await {
                Ok(info) => {
                    s.set_info(info.clone());
                    s.sink.event(SessionEvent::Opened { info });
                    // Before anything else: a missing identity turns the
                    // first commit into a wall of git text, and the UI can
                    // ask for one instead.
                    s.refresh_author();
                    s.restart_log();
                    s.refresh_quick();
                    // Not from `set_auto_fetch`, which the application
                    // calls the instant this session is handed over —
                    // there is no workdir to read from until the line
                    // above, and the interval it installs is what grants
                    // permission to look at all.
                    s.catch_up_remote_tags();
                }
                Err(error) => {
                    if !error.is_cancelled() {
                        s.sink.event(SessionEvent::OpenFailed { path, error });
                    }
                }
            }
        });
        session
    }

    /// Workdir of the opened repository (None until `Opened`).
    pub fn workdir(&self) -> Option<PathBuf> {
        self.lock_info().as_ref().map(|i| i.workdir.clone())
    }

    /// Resolved repository paths (None until `Opened`).
    pub fn repo_info(&self) -> Option<RepoInfo> {
        self.lock_info().clone()
    }

    /// Time budget for fetch / push.
    pub fn network_timeout(&self) -> std::time::Duration {
        match self.network_timeout.lock() {
            Ok(g) => *g,
            Err(e) => *e.into_inner(),
        }
    }

    /// Raises or lowers the fetch / push time budget. Zero is ignored — a
    /// network command must always have a backstop.
    pub fn set_network_timeout(&self, timeout: std::time::Duration) {
        if timeout.is_zero() {
            return;
        }
        if let Ok(mut guard) = self.network_timeout.lock() {
            *guard = timeout;
        }
    }

    /// Whether the command log also records the reads this session makes
    /// on its own (polling, refreshes, details). Off by default; it
    /// applies to commands spawned from here on, not retroactively.
    pub fn set_record_background(&self, on: bool) {
        self.commands.record_background.store(on, Ordering::Relaxed);
    }

    /// Cancels everything this session is doing. Idempotent.
    pub fn close(&self) {
        self.root_cancel.cancel();
    }

    // --- internals ------------------------------------------------------

    fn fail(&self, op: &'static str, error: GitError) {
        if error.is_cancelled() {
            return;
        }
        tracing::warn!(op, error = %error, "session operation failed");
        self.sink.event(SessionEvent::OpFailed { op, error });
    }

    fn set_info(&self, info: RepoInfo) {
        if let Ok(mut guard) = self.info.lock() {
            *guard = Some(info);
        }
    }

    fn lock_info(&self) -> std::sync::MutexGuard<'_, Option<RepoInfo>> {
        // A poisoned lock only happens if a holder panicked; the data is a
        // plain snapshot, safe to keep using.
        match self.info.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    fn lock_shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        match self.shared.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }
}

impl Drop for RepoSession {
    fn drop(&mut self) {
        self.root_cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::print_stdout,
        reason = "the scale measurement reports its number to whoever ran it"
    )]

    use super::build::{RefJoins, build_label_map, build_snapshot};
    use super::*;
    use crate::remote::RemoteTag;

    fn tag(name: &str, commit: Oid, annotated: bool) -> RefEntry {
        RefEntry {
            name: format!("refs/tags/{name}"),
            short: name.to_string(),
            kind: RefKind::Tag,
            target: commit,
            peeled: annotated.then_some(commit),
            upstream: None,
            is_head: false,
            created_unix: 0,
        }
    }

    fn oid(byte: u8) -> Oid {
        Oid::from_hex_str(&format!("{byte:02x}").repeat(20)).expect("valid sha")
    }

    fn head_at(commit: Oid) -> HeadState {
        HeadState {
            branch: Some("main".to_string()),
            oid: Some(commit),
            detached: false,
        }
    }

    fn index_of(remote: &str, tags: Vec<RemoteTag>) -> RemoteTagIndex {
        let mut index = RemoteTagIndex::new();
        for t in tags {
            let place = index
                .entry(t.name)
                .or_default()
                .entry(t.commit)
                .or_default();
            place.annotated |= t.annotated;
            place.remotes.push(remote.to_string());
        }
        index
    }

    /// A tag both sides agree on is one tag: the local label carries the
    /// cloud and the remote's reading adds no second chip.
    #[test]
    fn an_agreed_tag_gets_one_label() {
        let refs = vec![tag("v1", oid(1), false)];
        let remote_tags = index_of(
            "origin",
            vec![RemoteTag {
                name: "v1".to_string(),
                commit: oid(1),
                annotated: false,
            }],
        );
        let joins = RefJoins::new(&refs);
        let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
        let labels = map.get(&oid(1)).expect("a label on the commit");
        assert_eq!(labels.len(), 1, "one name, one chip: {labels:?}");
        assert!(labels[0].has_remote, "the cloud says the remote has it");
        assert!(labels[0].here);

        let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
        assert_eq!(snapshot.tags.len(), 1, "the sidebar lists the name once");
        assert!(snapshot.tags[0].here && snapshot.tags[0].has_remote);
    }

    /// A tag the remote puts somewhere else stands on both rows, and the
    /// one that is not here says whose reading it is.
    #[test]
    fn a_drifted_tag_stands_on_both_rows() {
        let refs = vec![tag("v1", oid(1), false)];
        let remote_tags = index_of(
            "origin",
            vec![RemoteTag {
                name: "v1".to_string(),
                commit: oid(2),
                annotated: false,
            }],
        );
        let joins = RefJoins::new(&refs);
        let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
        assert!(map.get(&oid(1)).is_some_and(|l| l[0].here));
        let theirs = map.get(&oid(2)).expect("the remote's reading");
        assert!(!theirs[0].here);
        assert_eq!(theirs[0].remote, "origin");

        // One name is one row in the sidebar, wherever the two point.
        let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
        assert_eq!(snapshot.tags.len(), 1);
        assert!(snapshot.tags[0].here);
    }

    /// A name only a remote has reaches no graph row, so the sidebar is
    /// where it can be read at all.
    #[test]
    fn a_tag_only_a_remote_has_is_listed_and_marked() {
        let refs = vec![tag("v1", oid(1), false)];
        let remote_tags = index_of(
            "origin",
            vec![RemoteTag {
                name: "v9".to_string(),
                commit: oid(9),
                annotated: true,
            }],
        );
        let joins = RefJoins::new(&refs);
        let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
        let v9 = snapshot
            .tags
            .iter()
            .find(|t| t.short == "v9")
            .expect("listed");
        assert!(!v9.here && v9.has_remote && v9.annotated);
        assert_eq!(v9.created_unix, 0, "an advertisement carries no date");
    }

    /// Ignored: it needs a repository worth measuring. Run it with
    /// `PG_PERF_REPO=<path> cargo test -p platitude-core --release
    /// refs_join_at_scale -- --ignored --nocapture`.
    ///
    /// What it times is one `publish_refs` join — the work every poll tick
    /// does on top of the two git reads. Recorded in
    /// `ci/baseline/refs-join-windows-x64.md`.
    #[test]
    #[ignore = "needs PG_PERF_REPO pointed at a large repository"]
    fn refs_join_at_scale() {
        let repo = std::env::var("PG_PERF_REPO").unwrap_or_else(|_| ".".to_string());
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args([
                "for-each-ref",
                refs::REFS_FORMAT_ARG,
                "refs/heads",
                "refs/remotes",
                "refs/tags",
            ])
            .output()
            .expect("git for-each-ref");
        let refs = refs::parse_refs(&out.stdout);
        let head = head_at(refs.first().map(RefEntry::commit_oid).unwrap_or(oid(0)));

        // What a remote carrying every tag this repository has would
        // advertise — the steady state once a fetch has been through.
        let remote_tags = index_of(
            "origin",
            refs.iter()
                .filter(|r| r.kind == RefKind::Tag)
                .map(|r| RemoteTag {
                    name: r.short.clone(),
                    commit: r.commit_oid(),
                    annotated: r.peeled.is_some(),
                })
                .collect(),
        );

        let started = Instant::now();
        let joins = RefJoins::new(&refs);
        let snapshot = build_snapshot(&refs, &head, &remote_tags, &joins);
        let labels = build_label_map(&refs, &head, &remote_tags, &joins);
        println!(
            "refs={} tags={} remote_branches={} | one publish_refs join: {:.1} ms \
             ({} sidebar tags, {} labelled commits)",
            refs.len(),
            refs.iter().filter(|r| r.kind == RefKind::Tag).count(),
            refs.iter()
                .filter(|r| r.kind == RefKind::RemoteBranch)
                .count(),
            started.elapsed().as_secs_f64() * 1000.0,
            snapshot.tags.len(),
            labels.len(),
        );
    }
}
