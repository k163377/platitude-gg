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
use std::sync::Arc;
use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::CaptureSink;
use platitude_core::remote;
use platitude_core::session::{
    LabelKind, RefLabel, RefsSnapshot, RemoteTagRefreshOutcome, RepoSession, SessionEvent,
    SessionSink, TagItem,
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
    let exec = crate::support::exec::isolated();
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

/// The refs snapshot published after the fetch landed. Opening
/// publishes one too, and that one predates any answer from the
/// remote — waiting for it instead would test the empty index.
async fn snapshot_after_the_fetch(sink: &CaptureSink) -> RefsSnapshot {
    sink.wait_for("RefsLoaded after the fetch", |evs| {
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

/// Opens the session and waits until it is open. A write queued before
/// that is dropped for having no repository to run in, so the fetch has to
/// come after.
async fn opened(work: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        work.path.clone(),
        Arc::clone(&sink) as Arc<dyn SessionSink>,
    );
    // Here and not in the test that reads them: graph baselines count the
    // opening's own walks, and by the time `Opened` has been delivered the
    // first one is already on its way. Switched on before the wait below,
    // this is ahead of everything the session spawns off `Opened`.
    session.set_record_background(true);
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
    let after = snapshot_after_the_fetch(&sink).await;

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

/// Waits for the snapshot that causally carries the remote-tag answer.
async fn tags_loaded(sink: &CaptureSink, pred: impl Fn(&[TagItem]) -> bool) {
    sink.wait_for("the remote-tag snapshot", |events| {
        events.iter().any(|event| {
            matches!(event, SessionEvent::RefsLoaded { snapshot } if pred(&snapshot.tags))
        }).then_some(())
    })
    .await;
}

fn some_tag_has_a_remote(tags: &[TagItem]) -> bool {
    tags.iter().any(|t| t.has_remote)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interval_that_is_on_is_permission_to_look_without_being_asked() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    tags_loaded(&sink, some_tag_has_a_remote).await;
    // Ten minutes out, so nothing here came from the timer firing.
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::WriteStarted { .. })),
        0,
        "and it does not travel the write queue, which would hold up \
         every write behind a badge"
    );
}

/// Learning what the remotes carry repaints chips. It does not rebuild
/// the graph.
///
/// No ref moved — the commits and where they sit are exactly what they
/// were — so the rows on screen are still the right rows and only their
/// badges are stale. Swapping the graph for that resets the view: the
/// scroll anchor and the selection are re-resolved against a model the
/// consumer is handed whole. The chip diff exists so this case does not
/// have to (`session::apply_refs`).
#[tokio::test(flavor = "multi_thread")]
async fn learning_what_the_remotes_carry_repaints_chips_without_swapping_the_graph() {
    let (mut bare, work, root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.set_auto_fetch(Some(Duration::from_secs(600)));
    // The first look settles the badges. Counting from before it would
    // race the interval this session opened with.
    tags_loaded(&sink, some_tag_has_a_remote).await;
    let named = |tags: &[TagItem]| tags.iter().any(|t| t.short == "v-later");
    let swaps = |sink: &CaptureSink| sink.count(|e| matches!(e, SessionEvent::LogReplaced { .. }));
    let chips =
        |sink: &CaptureSink| sink.count(|e| matches!(e, SessionEvent::LabelsChanged { .. }));
    let walks = |sink: &CaptureSink| {
        sink.count(
            |e| matches!(e, SessionEvent::CommandStarted { display, .. } if display.contains("log -z")),
        )
    };
    // The tracked refresh is an explicit completion boundary for all graph
    // work the test can count. A quiet interval would only guess that an
    // opening pass was not merely delayed.
    assert!(matches!(
        session.refresh_log_tracked().outcome().await,
        platitude_core::session::RefreshOutcome::Changed
            | platitude_core::session::RefreshOutcome::Unchanged
    ));
    let (settled_walks, settled_swaps, settled_chips) = (walks(&sink), swaps(&sink), chips(&sink));
    // Which rests on the opening's own reads being recorded (`opened`): a
    // walk nobody wrote down is one the wait reads as silence, and the
    // baseline goes back to being taken mid-opening. A zero here is that
    // regression, said out loud rather than left to come back as a flake.
    assert!(
        settled_walks > 0,
        "the opening walks, so the baseline has to have one to show for it"
    );

    // Somebody tags the commit this repository is already sitting on,
    // over there. **No local ref moves**: what changed is only what the
    // remote says it carries.
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["tag", "v-later", &root]);
    session.set_auto_fetch(Some(Duration::from_secs(600)));
    tags_loaded(&sink, named).await;
    // The chips have to arrive by their own event…
    sink.wait_for("the chips", move |evs| {
        (evs.iter()
            .filter(|e| matches!(e, SessionEvent::LabelsChanged { .. }))
            .count()
            > settled_chips)
            .then_some(())
    })
    .await;
    // …and nothing may have replaced the graph to deliver them.
    assert_eq!(
        swaps(&sink),
        settled_swaps,
        "badges are not a reason to hand the consumer a new graph"
    );
    // Nor to walk one. A walk that ends in "the same picture" costs the
    // whole walk to find that out — on a repository the size of the
    // reference one, that is the most expensive read there is.
    assert_eq!(
        walks(&sink),
        settled_walks,
        "and not a reason to re-walk the history either (chips {settled_chips} -> {})",
        chips(&sink)
    );
    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn with_the_interval_off_nothing_reaches_the_network_unasked() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.set_auto_fetch(None);

    assert!(
        session.auto_fetch_ticker().is_none(),
        "with the interval off there is no autonomous source that can ask for tags"
    );
    assert_eq!(
        session.refresh_remote_tags_tracked().outcome().await,
        RemoteTagRefreshOutcome::Disabled,
        "the catch-up path itself causally declines the unasked network read"
    );
    // Asking is still asking: the fetch reads them as it always did.
    session.fetch(Some("origin".into()));
    assert!(some_tag_has_a_remote(
        &snapshot_after_the_fetch(&sink).await.tags
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn tags_out_of_the_walk_are_not_worth_a_round_trip() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (_sink, session) = opened(&work).await;
    session.set_include_tags(false);
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    assert!(
        session.auto_fetch_ticker().is_some(),
        "the interval remains available; its later fetch, rather than an \
         eager badge read, is what may fill hidden tags in"
    );
    assert_eq!(
        session.refresh_remote_tags_tracked().outcome().await,
        RemoteTagRefreshOutcome::Hidden,
        "the eager catch-up explicitly declines tags that cannot be shown"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_remote_tag_completion_returns_its_single_flight_slot() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (_sink, session) = opened(&work).await;
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    // `Opened` is delivered before its eager catch-up is started, so that
    // untracked read is allowed to own the slot first.  Acquire this test's
    // flight by its explicit Busy state, without guessing how long the
    // opening work needs.  The assertion below deliberately does *not* use
    // this loop: it is the ack whose ownership boundary we are testing.
    let completed = tokio::time::timeout(crate::support::wait::OVERALL_BUDGET, async {
        loop {
            let outcome = session.refresh_remote_tags_tracked().outcome().await;
            if outcome != RemoteTagRefreshOutcome::Busy {
                break outcome;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the opening remote-tag flight returns its slot");
    assert!(
        matches!(
            completed,
            RemoteTagRefreshOutcome::Changed | RemoteTagRefreshOutcome::Unchanged
        ),
        "the first remote-tag read completed: {completed:?}"
    );
    let following = session.refresh_remote_tags_tracked().outcome().await;
    assert!(
        matches!(
            following,
            RemoteTagRefreshOutcome::Changed | RemoteTagRefreshOutcome::Unchanged
        ),
        "its ack returned the permit before the next request: {following:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drifted_tag_puts_its_name_on_both_rows() {
    let (_bare, work, root, head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.fetch(Some("origin".into()));
    snapshot_after_the_fetch(&sink).await;

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
