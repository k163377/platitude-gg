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
    LabelKind, Recording, RefLabel, RefsSnapshot, RemoteTagRefreshOutcome, RepoSession,
    SessionEvent, SessionSink, TagItem,
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
/// both sides must read it at the commit.
///
/// Returns the bare origin, the working repository, and (root, head).
fn tag_scenario() -> (TestRepo, TestRepo, String, String) {
    let mut bare = TestRepo::init();
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["config", "core.bare", "true"]);

    let mut work = TestRepo::init();
    let root = work.commit_file_id("a.txt", "one\n", "root");
    work.git(&["remote", "add", "origin", &bare.file_url()]);
    work.git(&["tag", "-a", "v-both", "-m", "release"]);
    work.git(&["tag", "v-drift"]);
    work.git(&["push", "origin", "main", "v-both", "v-drift"]);

    let head = work.commit_file_id("a.txt", "two\n", "second");
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

/// The two forms of the menu's push row, against the remote they are
/// about. **A plain push cannot move a name the remote already has
/// somewhere else** — git refuses it outright — which is the whole reason
/// the drifted row comes up as the leased overwrite instead
/// (デザイン規約 §相手の履歴を置き換える).
#[tokio::test]
async fn a_tag_the_remote_has_not_got_goes_plainly_and_a_drifted_one_needs_the_lease() {
    let (_bare, mut work, root, head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();

    remote::push_tag(&exec, &path, "origin", "v-local", "", NET, &cancel)
        .await
        .expect("a name the remote has not got needs nothing");
    assert_eq!(
        at_origin(&mut work, "v-local"),
        head,
        "it is over there now"
    );

    let refused = remote::push_tag(&exec, &path, "origin", "v-drift", "", NET, &cancel).await;
    assert!(
        refused.is_err(),
        "a plain push will not move a tag the remote has elsewhere"
    );
    assert_eq!(
        at_origin(&mut work, "v-drift"),
        root,
        "and nothing over there moved"
    );

    remote::push_tag(&exec, &path, "origin", "v-drift", &root, NET, &cancel)
        .await
        .expect("the lease names the commit the remote is actually on");
    assert_eq!(
        at_origin(&mut work, "v-drift"),
        head,
        "the name follows this repository"
    );
}

/// What the remote holds one tag on, straight from the remote.
fn at_origin(work: &mut TestRepo, name: &str) -> String {
    let out = work.git(&["ls-remote", "origin", &format!("refs/tags/{name}")]);
    out.split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Taking a tag off a remote, and the two measurements that decide how
/// it has to be spelled.
///
/// **The name is qualified because a bare one is ambiguous.** A remote
/// carrying a branch and a tag of the same name refuses `--delete <name>`
/// outright — `error: dst refspec … matches more than one` — and deletes
/// neither (measured, git 2.55). `refs/tags/<name>` takes the tag and leaves
/// the branch where it is.
#[tokio::test]
async fn a_qualified_delete_takes_the_tag_and_leaves_the_branch_of_the_same_name() {
    let (_bare, mut work, root, _head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    // A branch over there under the name a tag already has.
    work.git(&["push", "origin", &format!("{root}:refs/heads/v-both")]);

    remote::delete_remote_tag(&exec, &path, "origin", "v-both", NET, &cancel)
        .await
        .expect("the qualified refspec names one ref");

    assert_eq!(at_origin(&mut work, "v-both"), "", "the tag is gone");
    assert_eq!(
        work.git(&["ls-remote", "origin", "refs/heads/v-both"])
            .split_whitespace()
            .next()
            .unwrap_or_default(),
        root,
        "and the branch of the same name was not touched"
    );
    assert_eq!(
        work.git(&["rev-list", "-n", "1", "v-both"]),
        root,
        "nor was the one here"
    );
}

/// **git takes a name the remote has not got, in this
/// spelling.** The qualified form needs no resolution over there, so the
/// answer is `warning: deleting a non-existent ref` and exit 0 (measured) —
/// where a bare name would have failed. Whether there is anything to
/// delete is the menu's to know before it asks (`offers::TagSides`).
#[tokio::test]
async fn deleting_a_tag_the_remote_has_not_got_is_not_an_error() {
    let (_bare, work, _root, _head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();

    remote::delete_remote_tag(&exec, &work.path, "origin", "v-local", NET, &cancel)
        .await
        .expect("git answers with a warning, not a refusal");
}

/// The lease is what makes the hold safe to offer without a dialog: it
/// is pinned to the commit the reader was being shown, so a remote that
/// has moved since is refused.
#[tokio::test]
async fn a_lease_pinned_to_a_commit_the_remote_has_left_is_refused() {
    let (_bare, mut work, root, _head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();

    // Somebody else moves it while the menu stands, and to a third
    // commit: a move to where this repository has it would leave the
    // push with nothing to send and exit 0 on those grounds instead.
    let third = work.commit_file_id("c.txt", "three\n", "third");
    work.git(&[
        "push",
        "--force",
        "origin",
        &format!("{third}:refs/tags/v-drift"),
    ]);

    let refused = remote::push_tag(&exec, &path, "origin", "v-drift", &root, NET, &cancel).await;

    assert!(refused.is_err(), "the lease is stale, so git turns it down");
    assert_eq!(
        at_origin(&mut work, "v-drift"),
        third,
        "and what the other push left is still there"
    );
}

// --- through the session ------------------------------------------------

/// The refs snapshot published after the fetch landed. Opening
/// publishes one too, and that one predates any answer from the
/// remote — waiting for it instead would test the empty index.
///
/// **A fetch that failed ends the wait.**
/// Nothing else here publishes the snapshot this is about, so a broken
/// environment would otherwise spend the whole patience budget and come
/// back as a timeout that names the wait instead of the failure inside it.
async fn snapshot_after_the_fetch(sink: &CaptureSink) -> RefsSnapshot {
    sink.wait_for("RefsLoaded after the fetch", |evs| {
        let done = evs.iter().position(|e| match e {
            SessionEvent::WriteFinished {
                kind: platitude_core::OperationKind::Fetch,
                error,
                ..
            } => {
                assert!(error.is_none(), "the fetch failed: {error:?}");
                true
            }
            _ => false,
        })?;
        evs[done..].iter().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot, .. } => Some((**snapshot).clone()),
            _ => None,
        })
    })
    .await
}

/// Opens the session and waits until it is open. A write queued before
/// that is dropped for having no repository to run in, so the fetch has to
/// come after.
async fn opened_recording(work: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let sink = CaptureSink::new();
    // Recording is part of how this session is created: graph
    // baselines here count the opening's own walks, and the opening
    // spawns them the moment the path is accepted — a flag set from
    // this thread afterwards would keep however many of them the
    // scheduler had not reached yet.
    let session = RepoSession::open_recording(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        work.path.clone(),
        Arc::clone(&sink) as Arc<dyn SessionSink>,
        None,
        Recording::WithBackground,
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
    let (sink, session) = opened_recording(&work).await;

    // Before anything reaches the remote there is nothing to say: no local
    // ref records where a remote keeps its tags, so every one of them is
    // one this repository alone has.
    let opening = sink
        .wait_for("the first RefsLoaded", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot, .. } if !snapshot.tags.is_empty() => {
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

/// Asks for a tracked remote-tag read until one runs.
///
/// A `Busy` is a read of somebody else's holding the single-flight slot —
/// the slot books no repeat, so the ask is dropped — and its ack is sent
/// once that read has let the slot go (`RemoteTagRefreshOutcome::Busy`),
/// so the ask after it finds the slot free unless another reader got
/// there first. Only two can: the opening's catch-up and the interval's.
/// A third refusal is a slot nobody lets go, and is said by name.
async fn asked_past_busy(session: &Arc<RepoSession>) -> RemoteTagRefreshOutcome {
    let outcome = asked_past_busy_answered(session).await;
    // A remote that would not answer publishes nothing, and every wait
    // after this one would spend its whole budget on that silence. Said
    // here, where the session has just answered for it.
    assert_ne!(
        outcome,
        RemoteTagRefreshOutcome::Unanswered,
        "a remote these repositories own did not answer its `ls-remote` — the machine's git \
         cannot reach a path beside them, so nothing downstream can publish a badge"
    );
    outcome
}

/// The same, for the one test whose subject is a remote that answers
/// nothing.
async fn asked_past_busy_answered(session: &Arc<RepoSession>) -> RemoteTagRefreshOutcome {
    for _ in 0..3 {
        let outcome = crate::support::wait::bounded(
            "the tracked remote-tag read",
            session.refresh_remote_tags_tracked().outcome(),
        )
        .await;
        if outcome != RemoteTagRefreshOutcome::Busy {
            return outcome;
        }
    }
    panic!("the remote-tag slot was never let go: three asks in a row found it held");
}

/// Waits for the snapshot that causally carries the remote-tag answer.
async fn tags_loaded(sink: &CaptureSink, pred: impl Fn(&[TagItem]) -> bool) {
    sink.wait_for("the remote-tag snapshot", |events| {
        events.iter().any(|event| {
            matches!(event, SessionEvent::RefsLoaded { snapshot, .. } if pred(&snapshot.tags))
        }).then_some(())
    })
    .await;
}

fn some_tag_has_a_remote(tags: &[TagItem]) -> bool {
    tags.iter().any(|t| t.has_remote)
}

/// A remote that would not answer is said by name, so a caller has
/// something to fail on.
///
/// **The two are the same downstream** — no event, no snapshot, no badge
/// — so a caller waiting for the badge to change has nothing to fail on
/// and spends its whole budget on a machine whose git cannot reach the
/// remote. Nothing is failed for it: the readings the remotes that did
/// answer gave are kept, which is what the badge is drawn from.
#[tokio::test(flavor = "multi_thread")]
async fn a_remote_that_would_not_answer_is_named_rather_than_left_silent() {
    let (_bare, mut work, _root, _head) = tag_scenario();
    work.git(&[
        "remote",
        "add",
        "nowhere",
        "file:///nowhere/there-is-no-such-repository.git",
    ]);
    let (sink, session) = opened_recording(&work).await;
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    assert_eq!(
        asked_past_busy_answered(&session).await,
        RemoteTagRefreshOutcome::Unanswered
    );
    // And the remote that did answer is still read: what one remote
    // would not say is not a reason to drop what the others did.
    tags_loaded(&sink, some_tag_has_a_remote).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interval_that_is_on_is_permission_to_look_without_being_asked() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (sink, session) = opened_recording(&work).await;
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

/// Learning what the remotes carry repaints chips and leaves the
/// graph standing.
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
    let (sink, session) = opened_recording(&work).await;
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
    // The whole opening baseline, in the order that closes it: both
    // snapshots landed, their readers handed the slots back, a tracked
    // refresh owns the graph, the passes it displaced have stopped, and
    // the graph it asked for is installed. A tracked refresh on its own
    // is not that — it *cancels* the opening's tag-inclusive pass, which
    // stops when it next looks, and the walk that pass had already begun
    // lands after the count as a walk this test's subject appears to
    // have asked for.
    sink.opened_graph(&session, 2).await;
    let (settled_walks, settled_swaps, settled_chips) = (walks(&sink), swaps(&sink), chips(&sink));
    // Which rests on the opening's own reads being recorded (`opened`
    // asks for that when it creates the session): a walk nobody wrote down
    // is one the wait reads as silence, and the baseline goes back to
    // being taken mid-opening. A zero here is that regression, said
    // out loud.
    assert!(
        settled_walks > 0,
        "the opening walks, so the baseline has to have one to show for it"
    );

    // Somebody tags the commit this repository is already sitting on,
    // over there. **No local ref moves**: what changed is only what the
    // remote says it carries.
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["tag", "v-later", &root]);
    // Asked through the tracked form, past the opening's own catch-up
    // where that still holds the single-flight slot.
    asked_past_busy(&session).await;
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
    // …and the graph they arrive on is the one already there.
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
    let (sink, session) = opened_recording(&work).await;
    session.set_auto_fetch(None);

    assert!(
        session.auto_fetch_ticker().is_none(),
        "with the interval off there is no autonomous source that can ask for tags"
    );
    assert_eq!(
        crate::support::wait::bounded(
            "the tracked remote-tag refresh",
            session.refresh_remote_tags_tracked().outcome()
        )
        .await,
        RemoteTagRefreshOutcome::Disabled,
        "the catch-up path itself causally declines the unasked network read"
    );
    // The opening refs read has to land first: `snapshot_after_the_fetch`
    // takes the first snapshot after the write, and an opening read still
    // in flight can deliver its badge-less listing into that window.
    sink.opening_settled(&session).await;
    // Asking is still asking: the fetch reads them as it always did.
    session.fetch(Some("origin".into()));
    assert!(some_tag_has_a_remote(
        &snapshot_after_the_fetch(&sink).await.tags
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn tags_out_of_the_walk_are_not_worth_a_round_trip() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (_sink, session) = opened_recording(&work).await;
    session.set_include_tags(false);
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    assert!(
        session.auto_fetch_ticker().is_some(),
        "the interval remains available; its later fetch, rather than an \
         eager badge read, is what may fill hidden tags in"
    );
    assert_eq!(
        crate::support::wait::bounded(
            "the tracked remote-tag refresh",
            session.refresh_remote_tags_tracked().outcome()
        )
        .await,
        RemoteTagRefreshOutcome::Hidden,
        "the eager catch-up explicitly declines tags that cannot be shown"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_remote_tag_completion_returns_its_single_flight_slot() {
    let (_bare, work, _root, _head) = tag_scenario();
    let (_sink, session) = opened_recording(&work).await;
    session.set_auto_fetch(Some(Duration::from_secs(600)));

    // `Opened` is delivered before its eager catch-up is started, so that
    // untracked read is allowed to own the slot first. This test's own
    // flight is the first ask past it. The assertion below takes
    // whatever comes back: it is the ack whose ownership
    // boundary is under test.
    let completed = asked_past_busy(&session).await;
    assert!(
        matches!(
            completed,
            RemoteTagRefreshOutcome::Changed | RemoteTagRefreshOutcome::Unchanged
        ),
        "the first remote-tag read completed: {completed:?}"
    );
    let following = crate::support::wait::bounded(
        "the tracked remote-tag refresh",
        session.refresh_remote_tags_tracked().outcome(),
    )
    .await;
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
    let (sink, session) = opened_recording(&work).await;
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
        // And the two rows are the whole of it: on the graph the cloud
        // says the remote's copy is on *this* row, so where the two sides
        // disagree neither row wears one. The sidebar keeps the wider
        // reading, which is asserted where the snapshot is read.
        assert!(
            !mine.has_remote && !theirs.has_remote,
            "a badge here would answer for the row the other one is on"
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

/// The same disagreement, in the form the menu reads it: which remote,
/// and the commit the lease has to be pinned to. Off its own run — the
/// sidebar lists a name that is here once, so a drift leaves no row of
/// its own to read it from.
#[tokio::test]
async fn a_drift_is_listed_by_remote_with_the_commit_a_lease_would_name() {
    let (_bare, work, root, _head) = tag_scenario();
    let (sink, session) = opened_recording(&work).await;
    session.fetch(Some("origin".into()));
    let snapshot = snapshot_after_the_fetch(&sink).await;

    let drifts: Vec<(&str, &str, String)> = snapshot
        .tag_drifts
        .iter()
        .map(|d| (d.name.as_str(), d.remote.as_str(), d.commit.to_hex()))
        .collect();
    assert_eq!(
        drifts,
        vec![("v-drift", "origin", root.clone())],
        "only the name the two sides disagree about, and only where the \
         remote actually has it"
    );
    assert!(
        snapshot.tags.iter().any(|t| t.short == "v-drift" && t.here),
        "the drifted tag still gets the one row it has always had"
    );
    assert!(
        !snapshot
            .tags
            .iter()
            .any(|t| t.short == "v-local" && !t.here),
        "a name only this repository has is nobody's drift"
    );
}
