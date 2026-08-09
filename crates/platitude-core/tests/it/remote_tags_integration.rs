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
/// `v-both` is annotated, so the same scenario also holds the peel rule:
/// both sides must read it at the commit, never the tag object.
///
/// Returns the bare origin, the working repository, and (root, head).
fn tag_scenario() -> (TestRepo, TestRepo, String, String) {
    let mut bare = TestRepo::init();
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["config", "core.bare", "true"]);

    let mut work = TestRepo::init();
    let root = work.commit_file("a.txt", "one\n", "root");
    work.git(&["remote", "add", "origin", &bare.file_url()]);
    work.git(&["tag", "-a", "v-both", "-m", "release"]);
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
    assert_eq!(
        at("v-both").as_deref(),
        Some(root.as_str()),
        "the ^{{}} line, which is what a local tag peels to as well"
    );
    assert!(
        tags.iter().any(|t| t.name == "v-both" && t.annotated),
        "the annotated tag is read at the commit, not the tag object"
    );
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

    fn count(&self, pred: impl Fn(&SessionEvent) -> bool) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| pred(e))
            .count()
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
                SessionEvent::RefsLoaded { snapshot } => Some((**snapshot).clone()),
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
fn rows_now(events: &[SessionEvent]) -> HashMap<String, Vec<RefLabel>> {
    crate::support::replay_graph(events)
        .into_values()
        .map(|seen| (seen.oid_hex, seen.labels))
        .collect()
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

/// Waits for a refs snapshot whose tags satisfy `pred`, or reports that
/// none did within the window. Used for a read nobody asked for: there is
/// no event that says "and it did not happen", so the absence is timed.
async fn tags_settle(
    sink: &CaptureSink,
    within: Duration,
    pred: impl Fn(&[TagItem]) -> bool,
) -> bool {
    let deadline = Instant::now() + within;
    loop {
        {
            let evs = sink.events.lock().unwrap();
            let hit = evs.iter().any(|e| match e {
                SessionEvent::RefsLoaded { snapshot } => pred(&snapshot.tags),
                _ => false,
            });
            if hit {
                return true;
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn some_tag_has_a_remote(tags: &[TagItem]) -> bool {
    tags.iter().any(|t| t.has_remote)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interval_that_is_on_is_permission_to_look_without_being_asked() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    assert!(
        tags_settle(&sink, Duration::from_secs(20), some_tag_has_a_remote).await,
        "opening with the interval on reads the remotes' tags on its own"
    );
    // Ten minutes out, so nothing here came from the timer firing.
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::WriteStarted { .. })),
        0,
        "and it does not travel the write queue, which would hold up \
         every write behind a badge"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn with_the_interval_off_nothing_reaches_the_network_unasked() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.set_auto_fetch(None);

    assert!(
        !tags_settle(&sink, Duration::from_secs(3), some_tag_has_a_remote).await,
        "a repository whose owner turned the timer off is not reached \
         into for a badge"
    );
    // Asking is still asking: the fetch reads them as it always did.
    session.fetch(Some("origin".into()));
    assert!(some_tag_has_a_remote(
        &sink.snapshot_after_the_fetch().await.tags
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn tags_out_of_the_walk_are_not_worth_a_round_trip() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.set_include_tags(false);
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    assert!(
        !tags_settle(&sink, Duration::from_secs(3), some_tag_has_a_remote).await,
        "with tags hidden the chips carrying this reading are not on \
         screen, and the interval will fill it in soon enough"
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
        // Which of the two is which cannot be read off the name — they
        // are the same name — so the reading that came from over there
        // says where it came from.
        assert_eq!(theirs.remote, "origin");
        assert_eq!(mine.remote, "", "this one was not read off anything");
        Some(())
    })
    .await;
}
