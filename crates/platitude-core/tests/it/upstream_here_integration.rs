//! A branch measured against another branch here (`branch.<n>.remote =
//! .`), against real git with a `file://` remote so nothing leaves the
//! machine (実装計画 §10).
//!
//! The branch here is called `origin/base` beside origin's own `base`:
//! `git status` shortens it to the very name a remote branch would have,
//! and shortens origin's own to `remotes/origin/base` for a branch
//! tracking that, so only configuration tells the two apart and names
//! either. A mark sends the branch to its remote under its own name,
//! counted against the tracking ref there and leaving the upstream as it
//! was; marked to none, or to `.`, neither door sends — a send to `.`
//! would move the branch here. Gone, it is a first push like any gone
//! upstream. How the counts get read without a process where nothing
//! needs them is `push_default_integration`'s.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::{far, origin_and_clone};
use crate::support::session::{CaptureSink, opened, write_answer};
use platitude_core::process::GitExecutor;
use platitude_core::remote::{self, PushForce, PushStanding, PushTrack};
use platitude_core::session::{BranchPushFacts, SessionEvent};
use tokio_util::sync::CancellationToken;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

const REMOTES: [&str; 1] = ["origin"];

/// `topic` two commits ahead of the branch here called `origin/base`,
/// which it tracks, beside origin's own `base`, fetched; origin has
/// `topic` too, level with it, and `branch.topic.pushRemote` marks origin.
fn topic_against_a_branch_here() -> (TestRepo, TestRepo) {
    let (bare, mut work) = origin_and_clone();
    work.git(&["push", "origin", "main:base"]);
    work.git(&["branch", "origin/base", "main"]);
    work.git(&["switch", "--track", "-c", "topic", "refs/heads/origin/base"]);
    work.commit_file("t.txt", "one\n", "topic work");
    work.commit_file("t.txt", "two\n", "more topic work");
    work.git(&["push", "origin", "refs/heads/topic:refs/heads/topic"]);
    work.git(&["fetch", "origin"]);
    work.git(&["config", "branch.topic.pushRemote", "origin"]);
    // Asked of git: the arrangement has to hold before anything leans on it.
    assert_eq!(work.git(&["config", "branch.topic.remote"]), ".");
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/origin/base"
    );
    (bare, work)
}

/// What the toolbar makes of the branch HEAD is on — where its push goes
/// and how it stands — off what the status tick reads: the status, its
/// upstream named from the marks as the tick names it, and the
/// destination's counts where the push goes elsewhere.
async fn toolbar(
    repo: &Path,
    exec: &GitExecutor,
    cancel: &CancellationToken,
) -> (String, PushStanding) {
    let status = platitude_core::status::load(exec, repo, cancel)
        .await
        .expect("status");
    let branch = status.branch_head.clone().expect("a branch");
    let marks = remote::push_marks(exec, repo, &branch, cancel)
        .await
        .expect("marks");
    let named = match (&status.upstream, marks.upstream_name()) {
        (Some(_), Some(named)) => named,
        (Some(git), None) => git.clone(),
        (None, _) => String::new(),
    };
    let upstream = remote::Upstream::of(&named, marks.upstream_is_here());
    let push_remote = marks.push_remote.as_deref().unwrap_or_default();
    let push_default = marks
        .push_default
        .as_ref()
        .map_or("", |marked| marked.remote.as_str());
    let track = if remote::pushes_elsewhere(upstream, push_remote, push_default, REMOTES) {
        remote::push_track(exec, repo, &branch, cancel)
            .await
            .expect("the destination's counts")
    } else {
        PushTrack::default()
    };
    let label = remote::push_target(
        &branch,
        upstream,
        push_remote,
        push_default,
        "origin",
        REMOTES,
    );
    let standing = remote::push_standing(
        false,
        false,
        &branch,
        upstream,
        status.upstream_tracked,
        status.ahead,
        status.behind,
        push_remote,
        push_default,
        REMOTES,
        &track,
    );
    (label, standing)
}

/// What the last refs snapshot says of `branch`'s row.
async fn row_of(sink: &CaptureSink, branch: &'static str) -> BranchPushFacts {
    sink.wait_for("a snapshot listing the branch", move |evs| {
        evs.iter().rev().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot, .. } if snapshot.local_named(branch).is_some() => {
                Some(snapshot.branch_push(branch, "origin"))
            }
            _ => None,
        })
    })
    .await
}

/// The commits the far side holds under `topic`, and the branch here.
fn tips(bare: &mut TestRepo, work: &mut TestRepo) -> (String, String) {
    let path = bare.path.clone();
    (
        bare.git_in(&path, &["rev-parse", "refs/heads/topic"]),
        work.git(&["rev-parse", "refs/heads/origin/base"]),
    )
}

/// Two ahead of the branch here, level with origin/topic: nothing to
/// send. One more commit, and the send goes to origin/topic, the upstream
/// left as it was.
#[tokio::test]
async fn the_mark_decides_where_the_branch_goes_and_what_it_is_counted_against() {
    let (mut bare, mut work) = topic_against_a_branch_here();
    let (exec, cancel) = env();

    let status = platitude_core::status::load(&exec, &work.path, &cancel)
        .await
        .expect("status");
    assert_eq!(
        (status.upstream.as_deref(), status.ahead, status.behind),
        (Some("origin/base"), 2, 0),
        "git's short name for the branch here is a remote branch's"
    );
    assert_eq!(
        toolbar(&work.path, &exec, &cancel).await,
        ("origin/topic".to_string(), PushStanding::Clean)
    );

    work.commit_file("t.txt", "three\n", "still more topic work");
    assert_eq!(
        toolbar(&work.path, &exec, &cancel).await,
        ("origin/topic".to_string(), PushStanding::Ready)
    );
    let plan = remote::plan_current_push(&exec, &work.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(
        (&plan.target, plan.set_upstream),
        (&far("origin", "topic"), false)
    );
    let base = work.git(&["rev-parse", "refs/heads/origin/base"]);
    remote::push(&exec, &work.path, &plan, NET, &cancel)
        .await
        .expect("the push lands");
    assert_eq!(
        tips(&mut bare, &mut work),
        (work.git(&["rev-parse", "refs/heads/topic"]), base),
        "origin has topic's tip, and the branch here has not moved"
    );
    assert_eq!(
        (
            work.git(&["config", "branch.topic.remote"]),
            work.git(&["config", "branch.topic.merge"])
        ),
        (".".to_string(), "refs/heads/origin/base".to_string()),
        "the upstream is left as it was"
    );
}

/// A branch tracking origin's own `base`, beside the branch here that took
/// the name `origin/base`: `git status` shortens that upstream to
/// `remotes/origin/base`. The tick names it from configuration, as the
/// send names its destination, so with origin marked the label is the
/// upstream, its counts are the ones that speak, and the lease's ref is
/// in the listing.
#[tokio::test(flavor = "multi_thread")]
async fn a_remote_upstream_is_named_as_the_send_names_it() {
    let (_bare, mut work) = topic_against_a_branch_here();
    work.git(&[
        "switch",
        "--create",
        "beside",
        "--track",
        "refs/remotes/origin/base",
    ]);
    work.commit_file("b.txt", "beside\n", "beside work");
    work.git(&["config", "remote.pushDefault", "origin"]);
    let (exec, cancel) = env();
    assert_eq!(
        platitude_core::status::load(&exec, &work.path, &cancel)
            .await
            .expect("status")
            .upstream
            .as_deref(),
        Some("remotes/origin/base"),
        "git's own short name for it"
    );

    let (sink, session) = opened(&work).await;
    let (upstream, push_track) = sink
        .wait_for("a status of beside", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::StatusLoaded {
                    status, push_track, ..
                } if status.branch_head.as_deref() == Some("beside") => {
                    Some((status.upstream.clone(), push_track.clone()))
                }
                _ => None,
            })
        })
        .await;
    session.close_and_wait().await;
    let upstream = upstream.expect("an upstream");
    assert_eq!(
        (upstream.as_str(), push_track),
        ("origin/base", PushTrack::default()),
        "the mark is the upstream's own remote, so no counts of its own are read"
    );
    let plan = remote::plan_current_push(&exec, &work.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(
        remote::push_target(
            "beside",
            remote::Upstream::of(&upstream, false),
            "",
            "origin",
            "origin",
            REMOTES
        ),
        format!("{}/{}", plan.target.remote, plan.target.branch),
        "the label and the send"
    );
    assert!(
        !work
            .git(&["for-each-ref", "refs/remotes/origin/base"])
            .is_empty(),
        "the lease's ref, as the remote branches list it"
    );
}

/// A branch tracking a pull request's head, fetched under a refspec of its
/// own (`+refs/pull/*/head:refs/remotes/origin/pr/*`) and configured the
/// way `gh pr checkout` writes it: `branch.<n>.merge` is
/// `refs/pull/12/head`, and only the remote's fetch refspecs say its
/// tracking ref is `origin/pr/12` — the name the listing has, and the one
/// the tick has to give.
#[tokio::test(flavor = "multi_thread")]
async fn an_upstream_under_a_refspec_of_its_own_is_named_through_it() {
    let (_bare, mut work) = topic_against_a_branch_here();
    work.git(&["push", "origin", "refs/heads/topic:refs/pull/12/head"]);
    work.git(&[
        "config",
        "--add",
        "remote.origin.fetch",
        "+refs/pull/*/head:refs/remotes/origin/pr/*",
    ]);
    work.git(&["fetch", "origin"]);
    work.git(&[
        "switch",
        "--create",
        "review",
        "--no-track",
        "refs/remotes/origin/pr/12",
    ]);
    work.git(&["config", "branch.review.remote", "origin"]);
    work.git(&["config", "branch.review.merge", "refs/pull/12/head"]);
    let (exec, cancel) = env();
    assert_eq!(
        platitude_core::status::load(&exec, &work.path, &cancel)
            .await
            .expect("status")
            .upstream
            .as_deref(),
        Some("origin/pr/12"),
        "git maps the merge ref through the refspec"
    );

    let (sink, session) = opened(&work).await;
    let upstream = sink
        .wait_for("a status of review", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::StatusLoaded { status, .. }
                    if status.branch_head.as_deref() == Some("review") =>
                {
                    Some(status.upstream.clone())
                }
                _ => None,
            })
        })
        .await;
    session.close_and_wait().await;
    assert_eq!(upstream.as_deref(), Some("origin/pr/12"));
}

/// The status tick says the upstream is here, and counts against where
/// the mark sends the branch.
#[tokio::test(flavor = "multi_thread")]
async fn the_status_says_the_upstream_is_here() {
    let (_bare, work) = topic_against_a_branch_here();
    let (sink, session) = opened(&work).await;

    let told = sink
        .wait_for("a status of topic", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::StatusLoaded {
                    status,
                    upstream_here,
                    push_track,
                    ..
                } if status.branch_head.as_deref() == Some("topic") => {
                    Some((*upstream_here, push_track.clone()))
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(
        told,
        (
            true,
            PushTrack {
                tracking: "origin/topic".to_string(),
                ahead: 0,
                behind: 0,
            }
        )
    );
    session.close_and_wait().await;
}

/// Neither door sends `topic`, its push having nowhere but here to go: the
/// toolbar (HEAD on it) and its row (HEAD on main) both stand on
/// `standing`, naming no destination, both pushes are refused with words
/// holding `why`, and nothing moves here or over there.
async fn neither_door_sends(
    bare: &mut TestRepo,
    work: &mut TestRepo,
    standing: PushStanding,
    why: &str,
) {
    let (exec, cancel) = env();
    assert_eq!(
        toolbar(&work.path, &exec, &cancel).await,
        (String::new(), standing)
    );
    let before = tips(bare, work);

    let (sink, session) = opened(work).await;
    let toolbar_push = session
        .push_current("origin".into(), PushForce::None)
        .expect("the write is accepted");
    let refused = write_answer(&sink, toolbar_push).await;
    assert!(
        refused.as_deref().is_some_and(|said| said.contains(why)),
        "the toolbar's: {refused:?}"
    );
    session.close_and_wait().await;

    work.git(&["switch", "main"]);
    let (sink, session) = opened(work).await;
    let row = row_of(&sink, "topic").await;
    assert_eq!(
        (row.standing, row.target.as_str(), row.lease),
        (standing, "", None)
    );
    let row_push = session
        .push_named("topic".into(), "origin".into(), PushForce::None)
        .expect("the write is accepted");
    let refused = write_answer(&sink, row_push).await;
    assert!(
        refused.as_deref().is_some_and(|said| said.contains(why)),
        "the row's: {refused:?}"
    );
    session.close_and_wait().await;

    assert_eq!(
        tips(bare, work),
        before,
        "nothing moved, here or over there"
    );
}

/// Marked to no remote: nothing names one to send to.
#[tokio::test(flavor = "multi_thread")]
async fn marked_to_no_remote_neither_door_sends() {
    let (mut bare, mut work) = topic_against_a_branch_here();
    work.git(&["config", "--unset", "branch.topic.pushRemote"]);
    neither_door_sends(
        &mut bare,
        &mut work,
        PushStanding::StaysHere,
        "is a local branch",
    )
    .await;
}

/// The branch here deleted: git's `[gone]`, a first push for the toolbar
/// and the row alike — and the row's question opens on no remote, the
/// gone name being the branch here's.
#[tokio::test(flavor = "multi_thread")]
async fn gone_it_asks_where_the_branch_goes() {
    let (_bare, mut work) = topic_against_a_branch_here();
    work.git(&["config", "--unset", "branch.topic.pushRemote"]);
    work.git(&["branch", "--delete", "--force", "origin/base"]);
    let (exec, cancel) = env();
    assert_eq!(
        toolbar(&work.path, &exec, &cancel).await.1,
        PushStanding::Publish
    );

    work.git(&["switch", "main"]);
    let (sink, session) = opened(&work).await;
    let row = row_of(&sink, "topic").await;
    assert_eq!(
        (row.standing, row.upstream.as_str()),
        (PushStanding::Publish, "")
    );
    session.close_and_wait().await;
}
