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
mod repo_session;
mod snapshot;
mod state;
mod write;

pub use event::SessionEvent;
use feed::CommandFeed;
pub(crate) use model::LabelIndex;
pub use model::{LabelKind, LogOptions, LogRow, RefLabel};
use print::RowPrint;
pub(crate) use remote_tags::RemoteTagIndex;
pub use repo_session::RepoSession;
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
