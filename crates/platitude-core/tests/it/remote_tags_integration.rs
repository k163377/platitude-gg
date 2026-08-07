//! What a tag says about the remote, end to end and entirely offline: the
//! remote is a `file://` URL of a second local repository (実装計画 §11.3).
//!
//! Every state here was measured on real git before it was written down
//! (git 2.51):
//!
//! - `fetch --prune` re-creates a local tag that was deleted, as long as it
//!   can reach the commit — so "only on the remote" is not simply "deleted
//!   here". It lasts when the commit is out of the fetch's reach, which is
//!   what the scenario below builds.
//! - A tag that points somewhere else on the remote is never moved by a
//!   fetch. `--prune` leaves it silently; `--prune-tags` refuses it with
//!   "would clobber existing tag" and exits 1. Nothing resolves the
//!   disagreement on its own, which is why it has to keep showing.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::support::TestRepo;
use platitude_core::process::GitExecutor;
use platitude_core::remote;
use platitude_core::session::{
    LabelKind, RefLabel, RefsSnapshot, RepoSession, SessionEvent, SessionSink, TagItem,
};
use tokio_util::sync::CancellationToken;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// The four tag states the design has to tell apart, in one repository:
///
/// | tag        | here | on the remote                     |
/// |------------|------|-----------------------------------|
/// | `v-both`   | root | root — the two agree              |
/// | `v-local`  | head | nowhere                           |
/// | `v-drift`  | head | root — moved here, not over there |
/// | `v-remote` | —    | a commit no branch there reaches  |
///
/// `v-remote` is pushed by a third repository and its branch deleted, so
/// the commit under it is never downloaded here: that is what stops the
/// fetch's tag auto-following from quietly turning it into a local tag.
///
/// Returns the bare origin, the working repository, and (root, head).
fn tag_scenario() -> (TestRepo, TestRepo, String, String) {
    let mut bare = TestRepo::init();
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["config", "core.bare", "true"]);

    let mut work = TestRepo::init();
    let root = work.commit_file("a.txt", "one\n", "root");
    work.git(&["remote", "add", "origin", &bare.file_url()]);
    work.git(&["tag", "v-both"]);
    work.git(&["tag", "v-drift"]);
    work.git(&["push", "origin", "main", "v-both", "v-drift"]);

    let head = work.commit_file("a.txt", "two\n", "second");
    work.git(&["push", "origin", "main"]);
    // Made here and never pushed…
    work.git(&["tag", "v-local"]);
    // …and one the remote still knows one commit back.
    work.git(&["tag", "-f", "v-drift", &head]);

    // Someone else's tag, on a commit this repository never receives.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-b", "gone", "origin/main"]);
    other.commit_file("b.txt", "theirs\n", "work kept off the branches");
    other.git(&["tag", "v-remote"]);
    other.git(&["push", "origin", "gone", "v-remote"]);
    other.git(&["push", "origin", "--delete", "gone"]);

    (bare, work, root, head)
}

#[tokio::test]
async fn a_remote_is_read_at_the_commits_its_tags_peel_to() {
    let (_bare, work, root, head) = tag_scenario();
    let exec = GitExecutor::new();
    let cancel = CancellationToken::new();

    let tags = remote::list_tags(&exec, &work.path, "origin", NET, &cancel)
        .await
        .expect("ls-remote --tags");

    let at = |name: &str| {
        tags.iter()
            .find(|t| t.name == name)
            .map(|t| t.commit.to_hex())
    };
    assert_eq!(at("v-both").as_deref(), Some(root.as_str()));
    assert_eq!(
        at("v-drift").as_deref(),
        Some(root.as_str()),
        "the remote still has the tag where it was pushed"
    );
    assert!(at("v-remote").is_some(), "advertised even with no branch");
    assert_eq!(at("v-local"), None, "never pushed");
    assert_ne!(
        head, root,
        "the scenario needs two commits to drift between"
    );
}

#[tokio::test]
async fn an_annotated_tag_comes_back_as_the_commit_not_the_tag_object() {
    let mut bare = TestRepo::init();
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["config", "core.bare", "true"]);
    let mut work = TestRepo::init();
    let root = work.commit_file("a.txt", "one\n", "root");
    work.git(&["remote", "add", "origin", &bare.file_url()]);
    work.git(&["tag", "-a", "v1", "-m", "release"]);
    work.git(&["push", "origin", "main", "v1"]);

    let tags = remote::list_tags(
        &GitExecutor::new(),
        &work.path,
        "origin",
        NET,
        &CancellationToken::new(),
    )
    .await
    .expect("ls-remote --tags");

    let v1 = tags.iter().find(|t| t.name == "v1").expect("v1 advertised");
    assert!(v1.annotated);
    assert_eq!(
        v1.commit.to_hex(),
        root,
        "the ^{{}} line, which is what a local tag peels to as well"
    );
}

// --- through the session ------------------------------------------------

struct CaptureSink {
    events: Mutex<Vec<SessionEvent>>,
}

impl SessionSink for CaptureSink {
    fn event(&self, event: SessionEvent) {
        self.events.lock().unwrap().push(event);
    }
}

impl CaptureSink {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
        })
    }

    async fn wait_for<T>(&self, what: &str, pred: impl Fn(&[SessionEvent]) -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(v) = pred(&self.events.lock().unwrap()) {
                return v;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}; events so far: {:?}",
                self.events.lock().unwrap()
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// The refs snapshot published after the fetch landed. Opening
    /// publishes one too, and that one predates any answer from the
    /// remote — waiting for it instead would test the empty index.
    async fn snapshot_after_the_fetch(&self) -> RefsSnapshot {
        self.wait_for("RefsLoaded after the fetch", |evs| {
            let done = evs.iter().position(|e| {
                matches!(e, SessionEvent::WriteFinished { op, error } if *op == "fetch" && error.is_none())
            })?;
            evs[done..].iter().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot } => Some(snapshot.clone()),
                _ => None,
            })
        })
        .await
    }
}

/// Opens the session and waits until it is open. A write queued before
/// that is dropped for having no repository to run in, so the fetch has to
/// come after.
async fn opened(work: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        work.path.clone(),
        Arc::clone(&sink) as Arc<dyn SessionSink>,
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;
    (sink, session)
}

/// The chips each row carries right now, keyed by commit.
///
/// Rows arrive with the walk and their chips are corrected afterwards:
/// a fetch that moves no ref this repository holds rebuilds nothing, so
/// what the remote turned out to have reaches the graph as `LabelsChanged`
/// against rows already on screen. Reading only the walk would miss it.
fn rows_now(events: &[SessionEvent]) -> HashMap<String, Vec<RefLabel>> {
    let mut by_row: HashMap<u32, (String, Vec<RefLabel>)> = HashMap::new();
    for e in events {
        match e {
            SessionEvent::LogStarted { .. } => by_row.clear(),
            SessionEvent::LogReplaced { rows, .. } => {
                by_row.clear();
                for r in rows {
                    by_row.insert(r.row, (r.oid_hex.clone(), r.labels.clone()));
                }
            }
            SessionEvent::LogChunk { rows, .. } => {
                for r in rows {
                    by_row.insert(r.row, (r.oid_hex.clone(), r.labels.clone()));
                }
            }
            SessionEvent::LabelsChanged { rows } => {
                for (row, labels) in rows {
                    if let Some(entry) = by_row.get_mut(row) {
                        entry.1 = labels.clone();
                    }
                }
            }
            _ => {}
        }
    }
    by_row.into_values().collect()
}

fn tag<'a>(snapshot: &'a RefsSnapshot, name: &str) -> &'a TagItem {
    snapshot
        .tags
        .iter()
        .find(|t| t.short == name)
        .unwrap_or_else(|| panic!("{name} missing from {:?}", snapshot.tags))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fetch_is_what_tells_a_tag_whether_a_remote_has_it_too() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;

    // Before anything reaches the remote there is nothing to say: no local
    // ref records where a remote keeps its tags, so every one of them is
    // one this repository alone has.
    let opening = sink
        .wait_for("the first RefsLoaded", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot } if !snapshot.tags.is_empty() => {
                    Some(snapshot.clone())
                }
                _ => None,
            })
        })
        .await;
    assert!(
        opening.tags.iter().all(|t| !t.has_remote),
        "a badge before asking would be a guess: {:?}",
        opening.tags
    );
    assert!(
        opening.tags.iter().all(|t| t.here),
        "and nothing is listed that this repository does not hold"
    );

    session.fetch(Some("origin".into()));
    let after = sink.snapshot_after_the_fetch().await;

    assert!(tag(&after, "v-both").has_remote);
    assert!(tag(&after, "v-both").here);
    assert!(
        !tag(&after, "v-local").has_remote,
        "made here and never pushed"
    );
    assert!(
        tag(&after, "v-drift").has_remote,
        "the remote has the name, just not where this one has it"
    );

    let only_there = tag(&after, "v-remote");
    assert!(!only_there.here, "no local ref holds it");
    assert!(only_there.has_remote);
    assert_eq!(
        only_there.created_unix, 0,
        "an advertisement carries no date to sort by"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drifted_tag_puts_its_name_on_both_rows() {
    let (_bare, work, root, head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.fetch(Some("origin".into()));
    sink.snapshot_after_the_fetch().await;

    // Both sides of the drift are on main, so both have a row to stand on,
    // and the name is on each of them — the whole of that signal.
    sink.wait_for("v-drift on both rows", |evs| {
        let rows = rows_now(evs);
        let drift = |oid: &str| rows.get(oid)?.iter().find(|l| l.text == "v-drift").cloned();
        let (mine, theirs) = (drift(&head)?, drift(&root)?);
        if !mine.here || theirs.here {
            return None;
        }
        assert_eq!(mine.kind, LabelKind::Tag);
        assert_eq!(theirs.kind, LabelKind::Tag);
        assert!(
            mine.has_remote && theirs.has_remote,
            "the remote has the name on both readings"
        );
        Some(())
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tag_standing_where_both_sides_agree_is_one_chip_not_two() {
    let (_bare, work, root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.fetch(Some("origin".into()));
    sink.snapshot_after_the_fetch().await;

    sink.wait_for("the root row's chips", |evs| {
        let rows = rows_now(evs);
        let both: Vec<_> = rows
            .get(&root)?
            .iter()
            .filter(|l| l.text == "v-both")
            .collect();
        if both.len() != 1 || !both[0].has_remote {
            return None;
        }
        assert!(both[0].here, "the local tag is the one that speaks");
        Some(())
    })
    .await;
}
