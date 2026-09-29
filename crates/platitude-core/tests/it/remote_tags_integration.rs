//! What a tag says about the remote, end to end and entirely offline: the
//! remote is a `file://` URL of a second local repository (実装計画 §11.3).
//!
//! - `fetch --prune` re-creates a deleted local tag whose commit it can
//!   reach, so "only on the remote" needs a commit out of the fetch's reach
//!   (the scenario below builds one).
//! - A fetch never moves a tag that points elsewhere on the remote:
//!   `--prune` leaves it silently, `--prune-tags` exits 1 with "would
//!   clobber existing tag". Nothing resolves it, so it has to keep showing.
//!
//! What git answers on its own terms, and the graph rows the join's unit
//! tests already hold, are in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, write_answer};
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
/// `v-remote`'s branch is deleted over there, so its commit is never
/// downloaded here and tag auto-following cannot make it a local tag.
/// `v-both` is annotated, so the scenario also holds the peel rule: both
/// sides must read it at the commit.
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
    work.git(&["tag", "v-local"]);
    work.git(&["tag", "-f", "v-drift", &head]);

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

/// A plain push cannot move a name the remote has elsewhere — git refuses
/// it — so the drifted row offers the leased overwrite
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
    assert_eq!(
        refused
            .as_ref()
            .err()
            .and_then(platitude_core::GitError::report)
            .map(|report| report.kind),
        Some(platitude_core::ReportKind::TagElsewhere),
        "a plain push will not move a tag the remote has on another commit: {refused:?}"
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

/// git refuses a plain push onto a name the remote holds as another object
/// even on the same commit (`already exists` — a lightweight tag over there
/// for the annotated one here). The screen shows one commit on both sides,
/// so it is answered as sent, and the remote's object stays.
#[tokio::test]
async fn a_tag_the_remote_holds_on_the_same_commit_is_answered_as_sent() {
    let (_bare, mut work, root, _head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    work.git(&[
        "push",
        "--force",
        "origin",
        &format!("{root}:refs/tags/v-both"),
    ]);

    remote::push_tag(&exec, &path, "origin", "v-both", "", NET, &cancel)
        .await
        .expect("the same commit over there is no collision");
    assert_eq!(
        at_origin(&mut work, "v-both"),
        root,
        "the lightweight tag over there was left as it is"
    );
}

/// The same-commit check reads where the push goes: with a `pushurl` naming
/// another repository, the fetch URL holding the name on this commit says
/// nothing about the one the send reaches.
#[tokio::test]
async fn the_same_commit_is_weighed_where_the_push_goes() {
    let (bare, mut work, _root, head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    let mut pushed_to = TestRepo::init();
    let dest = pushed_to.path.clone();
    pushed_to.git_in(&dest, &["config", "core.bare", "true"]);
    pushed_to.git_in(&dest, &["fetch", "-q", &bare.file_url(), "+refs/*:refs/*"]);
    pushed_to.git_in(&dest, &["update-ref", "refs/tags/v-both", &head]);
    work.git(&[
        "remote",
        "set-url",
        "--push",
        "origin",
        &pushed_to.file_url(),
    ]);

    let refused = remote::push_tag(&exec, &path, "origin", "v-both", "", NET, &cancel).await;
    assert_eq!(
        refused
            .as_ref()
            .err()
            .and_then(platitude_core::GitError::report)
            .map(|report| report.kind),
        Some(platitude_core::ReportKind::TagElsewhere),
        "the fetch URL agrees with this commit, the push URL does not: {refused:?}"
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

/// Over a branch and a tag of the same name, a bare `--delete <name>` fails
/// (`dst refspec … matches more than one`) and deletes neither;
/// `refs/tags/<name>` takes only the tag.
#[tokio::test]
async fn a_qualified_delete_takes_the_tag_and_leaves_the_branch_of_the_same_name() {
    let (_bare, mut work, root, _head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    work.git(&["push", "origin", &format!("{root}:refs/heads/v-both")]);

    remote::delete_remote_tag(&exec, &path, "origin", "v-both", &root, NET, &cancel)
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

/// The local rename runs first in the application
/// (デザイン規約 §手元の改名の後のリモート), so the name pushed is the one
/// here — pushing the wrong side would leave a name over there on a commit
/// nobody asked about.
#[tokio::test]
async fn replacing_a_tag_on_a_remote_pushes_the_new_name_and_deletes_the_old() {
    let (_bare, mut work, root, _head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    // The local rename, on the annotated tag: what travels is the tag
    // object, and the peel under it has to line up.
    work.git(&["tag", "v-moved", "v-both"]);
    work.git(&["tag", "--delete", "v-both"]);
    let object = work.git(&["rev-parse", "v-moved"]);

    remote::replace_remote_tag(
        &exec, &path, "origin", "v-both", "v-moved", &root, NET, &cancel,
    )
    .await
    .expect("the push and the delete both go through");

    assert_eq!(at_origin(&mut work, "v-both"), "", "the old name is gone");
    assert_eq!(
        at_origin(&mut work, "v-moved"),
        object,
        "and the new one carries the very object this repository has"
    );
    assert_eq!(
        work.git(&["rev-list", "-n", "1", "v-moved"]),
        root,
        "which is still the commit the old name marked"
    );
}

/// The push goes first, so a refused name has taken nothing away. The
/// application never asks this (`RepoPage.armRenameTagRemote`); what is
/// pinned is that the order makes a refusal harmless.
#[tokio::test]
async fn a_replace_the_remote_refuses_leaves_the_old_name_standing() {
    let (_bare, mut work, root, _head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    let both = at_origin(&mut work, "v-both");
    let drift = at_origin(&mut work, "v-drift");

    // `v-drift` stands on another commit over there, so its push is refused.
    let refused = remote::replace_remote_tag(
        &exec, &path, "origin", "v-both", "v-drift", &root, NET, &cancel,
    )
    .await;

    assert!(
        refused.is_err(),
        "git will not move a tag with a plain push"
    );
    assert_eq!(
        at_origin(&mut work, "v-both"),
        both,
        "and the name the replace was leaving is still over there"
    );
    assert_eq!(
        at_origin(&mut work, "v-drift"),
        drift,
        "with the one it was refused for where it was"
    );
}

/// `v-both` moved over there to `to` since it was read — somebody else's
/// push of a lightweight tag over the annotated one.
fn move_v_both_over_there(work: &mut TestRepo, to: &str) {
    work.git(&[
        "push",
        "--force",
        "origin",
        &format!("{to}:refs/tags/v-both"),
    ]);
}

/// A delete leased to the commit the screen showed: a remote that moved
/// the name since refuses it and keeps it; read again, the delete goes.
/// `a_qualified_delete_…` above holds the unmoved annotated tag, whose
/// lease is its object and not the commit shown (`remote::tags::lease_on`).
#[tokio::test]
async fn a_tag_the_remote_moved_since_it_was_read_is_not_deleted() {
    let (_bare, mut work, root, head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    move_v_both_over_there(&mut work, &head);

    let refused = remote::delete_remote_tag(&exec, &path, "origin", "v-both", &root, NET, &cancel)
        .await
        .expect_err("the name is not where the screen showed it");
    let Some(report) = refused.report() else {
        panic!("a stale lease on a tag reads as a branch's does: {refused}");
    };
    assert_eq!(report.kind, platitude_core::ReportKind::MovedDelete);
    assert_eq!(
        (report.remote.as_str(), report.name.as_str()),
        ("origin", "v-both")
    );
    assert_eq!(
        at_origin(&mut work, "v-both"),
        head,
        "and it stays over there"
    );

    remote::delete_remote_tag(&exec, &path, "origin", "v-both", &head, NET, &cancel)
        .await
        .expect("leased to what is there, the delete goes");
    assert_eq!(at_origin(&mut work, "v-both"), "");
}

/// Replace is a push and then the leased delete: refused, both names stay
/// over there.
#[tokio::test]
async fn a_replace_whose_delete_is_refused_leaves_both_names() {
    let (_bare, mut work, root, head) = tag_scenario();
    let exec = crate::support::exec::isolated();
    let cancel = CancellationToken::new();
    let path = work.path.clone();
    move_v_both_over_there(&mut work, &head);
    work.git(&["tag", "v-moved", "v-both"]);
    work.git(&["tag", "--delete", "v-both"]);
    let object = work.git(&["rev-parse", "v-moved"]);

    remote::replace_remote_tag(
        &exec, &path, "origin", "v-both", "v-moved", &root, NET, &cancel,
    )
    .await
    .expect_err("the old name moved since the screen showed it");

    assert_eq!(at_origin(&mut work, "v-both"), head, "the old name stays");
    assert_eq!(
        at_origin(&mut work, "v-moved"),
        object,
        "beside the new one"
    );
}

/// `Delete both` of a tag runs the remote half first
/// (`session::delete_tag_everywhere`): a refused lease leaves the name
/// here as well as there. Leased to what is there, both go.
#[tokio::test(flavor = "multi_thread")]
async fn delete_both_of_a_tag_refused_by_the_lease_touches_nothing() {
    let (_bare, mut work, root, head) = tag_scenario();
    move_v_both_over_there(&mut work, &head);
    let (sink, session) = opened(&work).await;

    let id = session
        .delete_tag_everywhere("v-both".into(), "origin".into(), root.clone())
        .expect("accepted");
    assert!(
        write_answer(&sink, id).await.is_some(),
        "the lease is refused"
    );
    assert_eq!(
        work.git(&["rev-list", "-n", "1", "v-both"]),
        root,
        "the name is still here"
    );
    assert_eq!(at_origin(&mut work, "v-both"), head, "and over there");
    // The catch-up a branch gets from a fetch: the remote's tags read again.
    let snapshot = refs_after_write(&sink, id).await;
    assert!(
        snapshot
            .tag_drifts
            .iter()
            .any(|d| d.name.as_str() == "v-both" && d.commit.to_hex() == head),
        "read again, the name shows where the remote moved it: {:?}",
        snapshot.tag_drifts
    );

    work.git(&["push", "origin", "v-local"]);
    let id = session
        .delete_tag_everywhere("v-local".into(), "origin".into(), head.clone())
        .expect("accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert!(!work.git_ok(&["rev-parse", "--verify", "refs/tags/v-local"]));
    assert_eq!(at_origin(&mut work, "v-local"), "");
}

/// A plain push onto a name the remote holds on another commit is read
/// again on its way out, as a stale lease is: the drift the screen had not
/// read is what the answer leaves on it.
#[tokio::test(flavor = "multi_thread")]
async fn a_tag_the_remote_holds_elsewhere_is_read_again_on_the_refusal() {
    let (_bare, work, root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;

    let id = session
        .push_tag("origin".into(), "v-drift".into(), String::new())
        .expect("accepted");
    assert!(write_answer(&sink, id).await.is_some(), "git refuses it");
    let snapshot = refs_after_write(&sink, id).await;
    assert!(
        snapshot
            .tag_drifts
            .iter()
            .any(|d| d.name.as_str() == "v-drift" && d.commit.to_hex() == root),
        "read again, the name shows where the remote has it: {:?}",
        snapshot.tag_drifts
    );
}

/// The remote half landed and the local one did not (its ref is locked
/// here): the answer is the local refusal, and the remote's tags are read
/// all the same, so the badge does not claim a name the remote dropped.
#[tokio::test(flavor = "multi_thread")]
async fn delete_both_of_a_tag_reads_the_remote_again_where_the_local_half_fails() {
    let (_bare, mut work, root, _head) = tag_scenario();
    let (sink, session) = opened(&work).await;
    session.fetch(Some("origin".into()));
    let before = snapshot_after_the_fetch(&sink).await;
    assert!(
        before
            .tags
            .iter()
            .any(|t| t.short == "v-both" && t.has_remote),
        "the badge is up to begin with"
    );
    let lock = work
        .path
        .join(".git")
        .join("refs")
        .join("tags")
        .join("v-both.lock");
    std::fs::write(&lock, b"").expect("hold the tag's ref lock");

    let id = session
        .delete_tag_everywhere("v-both".into(), "origin".into(), root.clone())
        .expect("accepted");
    let answer = write_answer(&sink, id).await;

    assert!(
        answer
            .as_deref()
            .is_some_and(|said| said.contains("v-both.lock")),
        "the local half is what refused: {answer:?}"
    );
    assert_eq!(at_origin(&mut work, "v-both"), "", "the remote half landed");
    let after = refs_after_write(&sink, id).await;
    assert!(
        after
            .tags
            .iter()
            .any(|t| t.short == "v-both" && t.here && !t.has_remote),
        "and the name stands here without the remote's badge: {:?}",
        after.tags.iter().find(|t| t.short == "v-both")
    );
    std::fs::remove_file(&lock).expect("let go of the lock");
}

/// The refs snapshot published after the write accepted under `id` —
/// the one that carries what the write read on its way out.
async fn refs_after_write(sink: &CaptureSink, id: platitude_core::OperationId) -> RefsSnapshot {
    sink.wait_for("the refs published after the write", |evs| {
        let done = evs
            .iter()
            .position(|e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id))?;
        evs[done..].iter().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot, .. } => Some((**snapshot).clone()),
            _ => None,
        })
    })
    .await
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

    // To a third commit: a move to where this repository has it would
    // leave the push nothing to send, and it would exit 0.
    let third = work.commit_file_id("c.txt", "three\n", "third");
    work.git(&[
        "push",
        "--force",
        "origin",
        &format!("{third}:refs/tags/v-drift"),
    ]);

    let refused = remote::push_tag(&exec, &path, "origin", "v-drift", &root, NET, &cancel).await;

    assert_eq!(
        refused
            .as_ref()
            .err()
            .and_then(platitude_core::GitError::report)
            .map(|report| report.kind),
        Some(platitude_core::ReportKind::Moved),
        "the lease is stale, so git turns it down — the name moved: {refused:?}"
    );
    assert_eq!(
        at_origin(&mut work, "v-drift"),
        third,
        "and what the other push left is still there"
    );
}

// --- through the session ------------------------------------------------

/// The refs snapshot published after the fetch landed — the opening's
/// predates any answer from the remote. A failed fetch ends the wait, so a
/// broken environment fails on its error rather than timing out.
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

/// Opens the session and waits for `Opened`: a write queued before it is
/// dropped for having no repository to run in.
async fn opened_recording(work: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let sink = CaptureSink::new();
    // Recording from creation: baselines count the opening's own walks,
    // spawned the moment the path is accepted, so a flag set afterwards
    // would miss some.
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

    // No local ref records where a remote keeps its tags, so before the
    // remote is asked every tag is this repository's alone.
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
/// A `Busy` is acked once the other read has let the slot go, so the next
/// ask finds it free unless another reader got there first — only the
/// opening's catch-up and the interval's can. A third `Busy` is a slot
/// nobody lets go.
async fn asked_past_busy(session: &Arc<RepoSession>) -> RemoteTagRefreshOutcome {
    let outcome = asked_past_busy_answered(session).await;
    // An unanswered remote publishes nothing, so every later wait would
    // spend its whole budget on the silence.
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

/// Downstream, a remote that would not answer looks like no change — no
/// event, no snapshot — so it is named for a caller to fail on. The
/// readings of the remotes that did answer are kept.
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

/// No ref moved, so only the badges are stale. Swapping the graph would
/// reset the view (scroll anchor and selection re-resolve against a new
/// model); the chip diff exists to avoid that (`session::chips`).
#[tokio::test(flavor = "multi_thread")]
async fn learning_what_the_remotes_carry_repaints_chips_without_swapping_the_graph() {
    let (mut bare, work, root, _head) = tag_scenario();
    let (sink, session) = opened_recording(&work).await;
    session.set_auto_fetch(Some(Duration::from_secs(600)));
    // Counting before the first look settles the badges would race the
    // interval this session opened with.
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
    sink.opened_graph(&session, 2).await;
    let (settled_walks, settled_swaps, settled_chips) = (walks(&sink), swaps(&sink), chips(&sink));
    // A zero here means the opening's walks went unrecorded
    // (`opened_recording`), so the baseline was taken mid-opening.
    assert!(
        settled_walks > 0,
        "the opening walks, so the baseline has to have one to show for it"
    );

    // Tagged over there on a commit already here: no local ref moves.
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["tag", "v-later", &root]);
    asked_past_busy(&session).await;
    tags_loaded(&sink, named).await;
    sink.wait_for("the chips", move |evs| {
        (evs.iter()
            .filter(|e| matches!(e, SessionEvent::LabelsChanged { .. }))
            .count()
            > settled_chips)
            .then_some(())
    })
    .await;
    assert_eq!(
        swaps(&sink),
        settled_swaps,
        "badges are not a reason to hand the consumer a new graph"
    );
    // A walk that ends in "the same picture" is, on a reference-size
    // repository, the most expensive read there is.
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
    // An opening read still in flight can deliver its badge-less listing
    // after the write, where `snapshot_after_the_fetch` looks.
    sink.opening_settled(&session).await;
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

    // The opening's untracked catch-up may own the slot first; the ack of
    // the first ask past it is the ownership boundary under test.
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

/// A drift as the menu reads it: which remote, and the commit the lease
/// pins. It is its own list because the sidebar lists a name that is here
/// once, leaving a drift no row to read it from.
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

/// git's answer to deleting a name the remote has not got, behind the
/// command line the delete above holds; and the drifted tag's two graph
/// rows through a whole session, whose labels are
/// `session::join_tests::a_drifted_tag_stands_on_both_rows`. Only the full
/// gate runs these.
mod periodic {
    use super::*;

    /// Leased to a commit, the qualified form refuses a name the remote
    /// has not got as `[rejected] (stale info)` — unleased, it exits 0 with
    /// `warning: deleting a non-existent ref`. The screen showed it there,
    /// so the remote dropped it since: someone else's move, not ours to
    /// call done.
    #[tokio::test]
    #[ignore = "git's answer to a ref that is not there: not worth the pre-merge run"]
    async fn deleting_a_tag_the_remote_has_not_got_is_refused_by_the_lease() {
        let (_bare, work, _root, head) = tag_scenario();
        let exec = crate::support::exec::isolated();
        let cancel = CancellationToken::new();

        let err =
            remote::delete_remote_tag(&exec, &work.path, "origin", "v-local", &head, NET, &cancel)
                .await
                .expect_err("nothing is there to hold the lease");
        assert!(
            err.is_outdated(),
            "read as any stale lease on a tag (`remote::push_tag`): {err:?}"
        );
    }

    /// The chips each row carries right now, keyed by commit.
    fn rows_now(events: &[SessionEvent]) -> HashMap<String, Vec<RefLabel>> {
        crate::support::replay_graph(events)
            .into_values()
            .map(|seen| (seen.oid_hex, seen.labels))
            .collect()
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "duplicates a_drifted_tag_stands_on_both_rows: not worth the pre-merge run"]
    async fn a_drifted_tag_puts_its_name_on_both_rows() {
        let (_bare, work, root, head) = tag_scenario();
        let (sink, session) = opened_recording(&work).await;
        session.fetch(Some("origin".into()));
        snapshot_after_the_fetch(&sink).await;

        // Both sides of the drift are on main, so each row carries the name.
        sink.wait_for("v-drift on both rows", |evs| {
            let rows = rows_now(evs);
            let drift = |oid: &str| rows.get(oid)?.iter().find(|l| l.text == "v-drift").cloned();
            let (mine, theirs) = (drift(&head)?, drift(&root)?);
            if !mine.here || theirs.here {
                return None;
            }
            assert_eq!(mine.kind, LabelKind::Tag);
            assert_eq!(theirs.kind, LabelKind::Tag);
            // On the graph the cloud says the remote's copy is on *this*
            // row, so neither drifted row wears one (the sidebar keeps the
            // wider reading).
            assert!(
                !mine.has_remote && !theirs.has_remote,
                "a badge here would answer for the row the other one is on"
            );
            // The name is the same on both, so the far side's reading names
            // its remote.
            assert_eq!(theirs.remote, "origin");
            assert_eq!(mine.remote, "", "this one was not read off anything");
            Some(())
        })
        .await;
    }
}
