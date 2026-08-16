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
mod event;
mod feed;
mod log;
mod model;
mod ops_integrate;
mod ops_remote;
mod ops_tree;
mod print;
mod query;
mod refresh;
mod remote_tags;
mod snapshot;
mod state;
mod write;

pub use event::SessionEvent;
use feed::CommandFeed;
pub(crate) use model::LabelIndex;
pub use model::{LabelKind, LogOptions, LogRow, RefLabel};
use print::RowPrint;
pub(crate) use remote_tags::RemoteTagIndex;
pub use snapshot::{BranchItem, RefsSnapshot, TagItem};
pub use state::AutoFetchTicker;
use state::{
    AutoFetch, Derived, EndingContext, Footer, HeadHold, OpGate, ReadSlot, Shared, SlotHeld,
    WriteRequest,
};

/// First chunk is small so the first paint happens as early as possible.
const FIRST_CHUNK_ROWS: usize = 512;
const CHUNK_ROWS: usize = 4096;

/// Op name of the interval-driven fetch. The UI keeps this one off the
/// shared error surface, so both sides have to agree on the spelling.
pub const AUTO_FETCH_OP: &str = "auto-fetch";

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
    /// Which diff read is the current one. Bumped by every
    /// [`RepoSession::load_diff`], and read again just before the colours
    /// for that diff would be worked out: a reader going down a commit's
    /// file list starts a read per row, and colouring costs enough (see
    /// [`SessionEvent::DiffColoured`]) that the ones nobody is waiting for
    /// any more are worth not doing at all. The rows are unaffected —
    /// those are cheap, and a stale one is dropped by the pane on arrival.
    diff_epoch: AtomicU64,
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
    /// Whether git normalises line endings here (`core.autocrlf`).
    eol_normalises: Derived<bool>,
    /// What remotes are configured. Read on every refs listing before
    /// this, which is a process per poll tick for an answer that only a
    /// write moves.
    remotes: Derived<Vec<remote::Remote>>,
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
    /// The same for everything the joins read, which is more than the
    /// listing: `refs_key` moving means the history is walked again, this
    /// moving means only that the snapshot and the chips are rebuilt.
    join_key: Mutex<Option<u64>>,
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
    /// One in flight per snapshot, for the reads a repository can be asked
    /// for from more than one place at once (see [`ReadSlot`]).
    refs_read: ReadSlot,
    status_read: ReadSlot,
    stash_read: ReadSlot,
    worktrees_read: ReadSlot,
    /// Submission end of the write queue (see the module docs).
    write_tx: tokio::sync::mpsc::UnboundedSender<WriteRequest>,
    /// Time budget for fetch / push. Persisted as the settings key
    /// `network_timeout_secs`; only the settings dialog's input field is
    /// missing (実装計画 §7).
    network_timeout: Mutex<std::time::Duration>,
    /// What the remotes last advertised under `refs/tags/`, merged into
    /// the shape the joins read. Empty until a fetch has been through:
    /// asking costs the network, so it rides the one command the user
    /// already meant to spend it on, and before that every tag reads as
    /// one this repository alone has. Rebuilt only
    /// when a remote has spoken. Shared because every refs read wants it
    /// and none of them changes it: merging tens of thousands of names on
    /// every poll tick copies the whole tag list for nothing
    /// (`JetBrains/kotlin`: 45,782 names).
    remote_tag_index: Mutex<Arc<RemoteTagIndex>>,
    /// Bumped whenever the index above became different readings, so the
    /// refs key can cover it without walking 45,909 entries.
    remote_tag_gen: AtomicU64,
    /// One permit for the background read of the above, so a second
    /// permission-granting call cannot stack another on top of it.
    remote_tags_slot: Arc<tokio::sync::Semaphore>,
    /// What the last refs read saw of the branch tip, so the walk behind
    /// [`reachable`] can be started without reading the listing again.
    head_hold: Mutex<Option<HeadHold>>,
    /// The same read's answer to "what commit is HEAD on", kept where the
    /// graph walk can reach it: `None` until a refs read has landed,
    /// `Some(None)` for a branch with no commits yet.
    ///
    /// Two levels because the walk has to tell "nobody has looked" from
    /// "looked, and there is nothing there" — they take opposite actions.
    /// Separate from [`Self::head_hold`], whose own `None` already means
    /// the second of those.
    head_tip: Mutex<Option<Option<Oid>>>,
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
            diff_epoch: AtomicU64::new(0),
            log_cancel: Mutex::new(None),
            wip_dirty: std::sync::atomic::AtomicBool::new(false),
            merge_tool_wanted: std::sync::atomic::AtomicBool::new(false),
            merge_tool_seen: Mutex::new(String::new()),
            eol_baselines: Mutex::new(HashMap::new()),
            eol_normalises: Derived::default(),
            remotes: Derived::default(),
            eol_marks: Mutex::new(Arc::new(Vec::new())),
            eol_marks_stale: std::sync::atomic::AtomicBool::new(true),
            status_key: Mutex::new(None),
            refs_key: Mutex::new(None),
            join_key: Mutex::new(None),
            last_snapshot: Mutex::new(None),
            write_busy: std::sync::atomic::AtomicBool::new(false),
            poll_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            refs_read: ReadSlot::default(),
            status_read: ReadSlot::default(),
            stash_read: ReadSlot::default(),
            worktrees_read: ReadSlot::default(),
            write_tx,
            network_timeout: Mutex::new(remote::DEFAULT_NETWORK_TIMEOUT),
            remote_tag_index: Mutex::new(Arc::new(RemoteTagIndex::default())),
            remote_tag_gen: AtomicU64::new(0),
            remote_tags_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            head_hold: Mutex::new(None),
            head_tip: Mutex::new(None),
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

    /// What this session is holding on to, part by part.
    ///
    /// Everything named here outlives the operation that filled it: it is
    /// still there when the window is idle, which is what the memory budget
    /// is about. `refs-snapshot` is an `Arc` the sidebar models hold too —
    /// counted in full on both sides, and the app's report says so.
    pub fn heap_report(&self) -> Vec<crate::mem::Part> {
        use crate::mem::{Footprint as _, Part};
        let shared = self.lock_shared();
        let mut parts = vec![
            Part::of("sent-rows", &shared.sent_rows),
            Part::new(
                "label-map",
                shared.label_map.heap_bytes(),
                shared.label_map.len(),
            ),
            Part::new("applied", shared.applied.heap_bytes(), shared.applied.len()),
            Part::new(
                "graph-builder",
                shared.builder.heap_bytes(),
                shared.builder.tracked_oids(),
            ),
        ];
        drop(shared);

        let snapshot = match self.last_snapshot.lock() {
            Ok(g) => g.clone(),
            Err(e) => e.into_inner().clone(),
        };
        let (snap_bytes, snap_refs) = match &snapshot {
            Some(s) => (
                s.heap_bytes(),
                s.locals.len() + s.remotes.len() + s.tags.len(),
            ),
            None => (0, 0),
        };
        parts.push(Part::new("refs-snapshot", snap_bytes, snap_refs));

        let index = self.remote_tag_index();
        parts.push(Part::new(
            "remote-tag-index",
            index.heap_bytes(),
            index.len(),
        ));

        let marks = match self.eol_marks.lock() {
            Ok(g) => Arc::clone(&g),
            Err(e) => Arc::clone(&e.into_inner()),
        };
        parts.push(Part::new("eol-marks", marks.heap_bytes(), marks.len()));

        let (base_bytes, base_count) = match self.eol_baselines.lock() {
            Ok(g) => (g.heap_bytes(), g.len()),
            Err(e) => {
                let g = e.into_inner();
                (g.heap_bytes(), g.len())
            }
        };
        parts.push(Part::new("eol-baselines", base_bytes, base_count));
        parts
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
            name: crate::Name::from(format!("refs/tags/{name}")),
            short: crate::Name::from(name),
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
        RemoteTagIndex::build(
            tags.into_iter()
                .map(|t| (t.name, t.commit, t.annotated, crate::Name::from(remote))),
        )
    }

    /// A tag both sides agree on is one tag: the local label carries the
    /// cloud and the remote's reading adds no second chip.
    #[test]
    fn an_agreed_tag_gets_one_label() {
        let refs = vec![tag("v1", oid(1), false)];
        let remote_tags = index_of(
            "origin",
            vec![RemoteTag {
                name: crate::Name::const_new("v1"),
                commit: oid(1),
                annotated: false,
            }],
        );
        let joins = RefJoins::new(&refs);
        let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
        let labels = map.labels_of(&oid(1));
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
                name: crate::Name::const_new("v1"),
                commit: oid(2),
                annotated: false,
            }],
        );
        let joins = RefJoins::new(&refs);
        let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
        assert!(map.labels_of(&oid(1))[0].here);
        let theirs = map.labels_of(&oid(2));
        assert!(!theirs.is_empty(), "the remote's reading");
        assert!(!theirs[0].here);
        assert_eq!(theirs[0].remote, "origin");

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
                name: crate::Name::const_new("v9"),
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
