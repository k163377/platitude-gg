//! Where a push goes when the repository decides it
//! (`remote.pushDefault`), and the remote the repository calls origin —
//! that key with `checkout.defaultRemote` beside it — against real git,
//! with `file://` remotes so nothing leaves the machine (実装計画 §10).
//!
//! Only what git alone can answer: the four reads come back with what git
//! holds — at every scope, and for a branch whose name is a regex — and a
//! push built from them lands where it said without moving where the
//! branch fetches from; where that is another remote than the upstream's,
//! the branch is counted against that remote's own tracking ref, the one
//! git refuses the push by. The order the reads are weighed in is a switch
//! over their values, covered by `remote::push`'s own tests. What git does
//! with the checkout key on its own is in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use crate::support::TestRepo;
use crate::support::exec::{env, logged_global};
use crate::support::oid;
use crate::support::push::{row, row_in, toolbar, toolbar_from};
use crate::support::remote::far;
use crate::support::session::{CaptureSink, opened, poll_landed, write_answer, write_settled};
use platitude_core::GitError;
use platitude_core::process::GitExecutor;
use platitude_core::remote::{self, PushForce};
use platitude_core::session::{Recording, SessionEvent};
use tokio_util::sync::CancellationToken;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// Two bare repositories and a clone that has both as remotes: `origin`,
/// which `main` tracks, and `fork`, which nothing points at yet.
fn origin_fork_and_clone() -> (TestRepo, TestRepo, TestRepo) {
    let mut origin = TestRepo::init();
    let origin_path = origin.path.clone();
    origin.git_in(&origin_path, &["config", "core.bare", "true"]);

    let mut fork = TestRepo::init();
    let fork_path = fork.path.clone();
    fork.git_in(&fork_path, &["config", "core.bare", "true"]);

    let mut work = TestRepo::init();
    work.commit_file("a.txt", "one\n", "root");
    work.git(&["remote", "add", "origin", &origin.file_url()]);
    work.git(&["remote", "add", "fork", &fork.file_url()]);
    work.git(&["push", "--set-upstream", "origin", "main"]);
    (origin, fork, work)
}

/// Both keys go on together and come off together, and the read gives
/// each back — the push's with the level that set it.
#[tokio::test]
async fn the_origin_mark_reads_back_with_both_keys() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    assert_eq!(
        remote::kept_marks(&exec, &repo.path, &cancel)
            .await
            .expect("unset is an answer, not a failure"),
        remote::KeptMarks::default()
    );

    remote::mark_origin(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");
    // Asked of git by name, not through the reader under test.
    assert_eq!(
        repo.git(&["config", "--get", "checkout.defaultRemote"])
            .trim(),
        "fork"
    );
    let marks = remote::kept_marks(&exec, &repo.path, &cancel)
        .await
        .expect("an answer")
        .origin;
    let marked = marks.push_default.expect("a push mark");
    assert_eq!(marked.remote, "fork");
    assert!(marked.local, "this repository's own config set it");
    assert_eq!(marks.checkout_default.as_deref(), Some("fork"));

    remote::clear_origin(&exec, &repo.path, &cancel)
        .await
        .expect("clear");
    assert_eq!(
        remote::kept_marks(&exec, &repo.path, &cancel)
            .await
            .expect("an answer"),
        remote::KeptMarks::default()
    );
    remote::clear_origin(&exec, &repo.path, &cancel)
        .await
        .expect("clearing a mark that is already gone is the state asked for");
}

/// The name goes into the `--get-regexp` pattern escaped — unescaped,
/// `wip.v2+x` answers with `wipAv22x`'s mark — and the send resolves the
/// same way.
#[tokio::test]
async fn a_metacharacter_branch_reads_its_own_mark() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    repo.git(&["switch", "--create", "wip.v2+x"]);
    repo.git(&["config", "branch.wip.v2+x.pushRemote", "fork"]);
    repo.git(&["config", "branch.wipAv22x.pushRemote", "origin"]);

    let marks = remote::push_marks(&exec, &repo.path, "wip.v2+x", &cancel)
        .await
        .expect("an answer");
    assert_eq!(marks.push_remote.as_deref(), Some("fork"));
    let decoy = remote::push_marks(&exec, &repo.path, "wipAv22x", &cancel)
        .await
        .expect("an answer");
    assert_eq!(
        decoy.push_remote.as_deref(),
        Some("origin"),
        "the decoy keeps its own mark"
    );

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(
        plan.target,
        far("fork", "wip.v2+x"),
        "the branch's own name"
    );
}

/// The upstream is read through the same escaped pattern: the decoy's
/// keys are written last, so an unescaped read would take them as this
/// branch's effective values.
#[tokio::test]
async fn a_metacharacter_branch_reads_its_own_upstream() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    repo.git(&["switch", "--create", "wip.v2+x"]);
    repo.git(&["config", "branch.wip.v2+x.remote", "origin"]);
    repo.git(&["config", "branch.wip.v2+x.merge", "refs/heads/far"]);
    repo.git(&["config", "branch.wipAv22x.remote", "fork"]);
    repo.git(&["config", "branch.wipAv22x.merge", "refs/heads/decoy"]);

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(
        plan.target,
        far("origin", "far"),
        "the branch's own upstream name"
    );
    assert!(!plan.set_upstream, "it tracks something already");
}

/// A mark in the user's global configuration, the scope the label can go
/// stale in: the read sees its level, the label spells what the send
/// resolves, and the marks git weighs above it still win.
#[tokio::test]
async fn a_global_mark_is_read_and_sent_alike() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, _log, cancel) = logged_global(repo.global_config());
    let remotes = ["fork", "origin"];

    repo.git(&["config", "--global", "remote.pushDefault", "fork"]);

    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    let marked = marks.push_default.clone().expect("the global mark is read");
    assert_eq!(marked.remote, "fork");
    assert!(
        !marked.local,
        "another level set it; it cannot be unset here"
    );

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    let label = remote::PushRoute::new(
        remote::Upstream::Remote("origin/main"),
        marks.marked(),
        remotes,
    )
    .target("main", "origin");
    assert_eq!(
        label,
        Some(plan.target.to_string()),
        "the label and the send disagree at global scope"
    );

    remote::mark_origin(&exec, &repo.path, "home", &cancel)
        .await
        .expect("mark home locally");
    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    let marked = marks.push_default.expect("a mark");
    assert_eq!(marked.remote, "home");
    assert!(marked.local, "the repository's own config decided");

    repo.git(&["config", "branch.main.pushRemote", "origin"]);
    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    assert_eq!(marks.push_remote.as_deref(), Some("origin"));
    let plan = remote::plan_current_push(&exec, &repo.path, "fork", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.target.remote, "origin");
}

/// [`origin_fork_and_clone`] with the fork's `main` fetched and apart from
/// ours: a commit there ours lacks, ours one there it lacks, and origin
/// simply behind us — then the repository's mark on the fork.
fn fork_diverged() -> (TestRepo, TestRepo, TestRepo) {
    let (origin, fork, mut work) = origin_fork_and_clone();
    work.commit_file("theirs.txt", "theirs\n", "theirs");
    work.git(&["push", "fork", "main"]);
    work.git(&["reset", "--hard", "HEAD~1"]);
    work.commit_file("ours.txt", "ours\n", "ours");
    work.git(&["fetch", "--prune", "--all"]);
    work.git(&["config", "remote.pushDefault", "fork"]);
    (origin, fork, work)
}

/// `main`'s counts against where its push goes.
async fn track_of(
    repo: &Path,
    exec: &GitExecutor,
    cancel: &CancellationToken,
) -> Option<remote::PushTrack> {
    remote::push_track(exec, repo, "main", cancel)
        .await
        .expect("an answer")
}

/// The toolbar's held overwrite, leased to `expect`.
async fn push_leased(
    repo: &Path,
    exec: &GitExecutor,
    expect: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let force = PushForce::WithLease {
        expect: oid(expect),
    };
    let spec = remote::plan_current_push(exec, repo, "origin", force, cancel)
        .await
        .expect("a plan");
    remote::push(exec, repo, &spec, NET, cancel).await
}

/// The push goes to the fork, and git refuses it by the fork's `main`, not
/// by the upstream: counted against origin the branch is only ahead, so a
/// plain push looks like it fits and is turned down.
#[tokio::test(flavor = "multi_thread")]
async fn a_push_elsewhere_is_counted_against_the_destinations_own_ref() {
    let (_origin, fork, mut repo) = fork_diverged();
    let (exec, cancel) = env();

    let status = platitude_core::status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(
        (status.upstream.as_deref(), status.upstream_counts),
        (
            Some("origin/main"),
            Some(platitude_core::AheadBehind {
                ahead: 1,
                behind: 0
            })
        ),
        "against the upstream there is only ours to add"
    );
    assert_eq!(
        track_of(&repo.path, &exec, &cancel).await,
        Some(remote::PushTrack {
            remote: "fork".to_string(),
            tracking: "fork/main".to_string(),
            counts: platitude_core::AheadBehind {
                ahead: 1,
                behind: 1
            },
        })
    );
    let (sink, session) = opened(&repo).await;
    let toolbar = toolbar_from(&sink, 0, "main").await;
    session.close_and_wait().await;
    let destination_tip = repo.git(&["rev-parse", "refs/remotes/fork/main"]);
    assert_eq!(
        (toolbar.standing, toolbar.lease.map(|oid| oid.to_hex())),
        (
            remote::PushStanding::Diverged,
            Some(destination_tip.trim().to_string())
        ),
        "the toolbar leases to the fork's own tip, as it shows it"
    );

    let plain = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    remote::push(&exec, &repo.path, &plain, NET, &cancel)
        .await
        .expect_err("git refuses by the fork's main");

    // Leased where the counts were read: the upstream's tip is not what the
    // fork holds.
    let upstream_tip = repo.git(&["rev-parse", "refs/remotes/origin/main"]);
    push_leased(&repo.path, &exec, &upstream_tip, &cancel)
        .await
        .expect_err("a lease on the upstream's tip is stale over there");
    push_leased(&repo.path, &exec, &destination_tip, &cancel)
        .await
        .expect("a lease on the fork's own tip lands");

    let fork_path = fork.path.clone();
    assert_eq!(
        repo.git_in(&fork_path, &["rev-parse", "main"]),
        repo.git(&["rev-parse", "HEAD"]),
        "the fork now holds ours"
    );
}

/// The status tick reads the destination's counts in the one arrangement
/// they speak for — a mark on another remote than the upstream's — and
/// pays no process for them anywhere else.
#[tokio::test(flavor = "multi_thread")]
async fn the_status_carries_the_destinations_counts_only_where_the_push_goes_elsewhere() {
    let (_origin, _fork, mut repo) = fork_diverged();
    repo.git(&["config", "--unset", "remote.pushDefault"]);
    let (sink, session) = opened(&repo).await;
    session.set_recording(Recording::WithBackground);
    let statuses = |sink: &CaptureSink| -> Vec<Option<remote::PushTrack>> {
        sink.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                SessionEvent::StatusLoaded { push, .. } => Some(push.track.clone()),
                _ => None,
            })
            .collect()
    };

    let asked = |sink: &CaptureSink| {
        sink.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| {
                matches!(e, SessionEvent::CommandStarted { display, .. }
                         if display.contains("%(push:track)"))
            })
            .count()
    };

    let quiet = |sink: &CaptureSink, why: &str| {
        let read = statuses(sink);
        assert!(
            !read.is_empty() && read.iter().all(Option::is_none),
            "{why}: {read:?}"
        );
        assert_eq!(
            asked(sink),
            0,
            "{why}: nothing reads the counts no one needs"
        );
    };

    poll_landed(&session).await;
    quiet(&sink, "nothing is marked");
    repo.git(&["config", "remote.pushDefault", "origin"]);
    poll_landed(&session).await;
    quiet(&sink, "the mark is the upstream's own remote");

    repo.git(&["config", "remote.pushDefault", "fork"]);
    poll_landed(&session).await;
    let track = sink
        .wait_for("a status with the fork's counts", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::StatusLoaded {
                    push:
                        remote::StatusPush {
                            track: Some(track), ..
                        },
                    ..
                } => Some(track.clone()),
                _ => None,
            })
        })
        .await;
    assert_eq!(
        track,
        remote::PushTrack {
            remote: "fork".to_string(),
            tracking: "fork/main".to_string(),
            counts: platitude_core::AheadBehind {
                ahead: 1,
                behind: 1
            },
        }
    );
    assert!(
        asked(&sink) > 0,
        "the count above is of the command that read these"
    );
    session.close_and_wait().await;
}

/// The fork fetched into `refs/remotes/copy/` — a name no remote has: the
/// counts there are still the fork's, so the toolbar reads them where the
/// mark sends the push, and a branch's row does too.
#[tokio::test(flavor = "multi_thread")]
async fn counts_fetched_under_another_name_still_speak_for_the_marked_remote() {
    let (_origin, _fork, mut repo) = fork_diverged();
    repo.git(&[
        "config",
        "remote.fork.fetch",
        "+refs/heads/*:refs/remotes/copy/*",
    ]);
    repo.git(&["fetch", "-q", "fork"]);
    let (exec, cancel) = env();
    let track = track_of(&repo.path, &exec, &cancel).await;
    assert_eq!(
        track
            .as_ref()
            .map(|track| (track.remote.as_str(), track.tracking.as_str())),
        Some(("fork", "copy/main"))
    );
    let copy_tip = repo.git(&["rev-parse", "refs/remotes/copy/main"]);
    let (sink, session) = opened(&repo).await;
    let facts = toolbar_from(&sink, 0, "main").await;
    session.close_and_wait().await;
    assert_eq!(
        (
            facts.standing,
            facts.target.as_deref(),
            facts.lease.map(|oid| oid.to_hex())
        ),
        (
            remote::PushStanding::Diverged,
            Some("fork/main"),
            Some(copy_tip.trim().to_string())
        )
    );

    repo.git(&["switch", "-c", "aside"]);
    let (sink, session) = opened(&repo).await;
    let row = row(&sink, "main").await;
    assert_eq!(
        (
            row.standing,
            row.target.as_deref(),
            row.lease.map(|oid| oid.to_hex())
        ),
        (
            remote::PushStanding::Diverged,
            Some("fork/main"),
            Some(copy_tip.trim().to_string())
        )
    );
    session.close_and_wait().await;
}

/// The repository's mark as the toolbar weighs it: whole, as the status
/// read it — `.` and a name no remote has alike —, the way the send reads
/// it, never as if unmarked (`RepoTab.pushDefault` drops both, for the
/// remote the repository calls origin). Marked to `.`, the push stays here
/// and the send refuses; marked to a name no remote has, the push goes
/// there under the branch's own name, and so does the send.
#[tokio::test(flavor = "multi_thread")]
async fn the_toolbar_weighs_the_repositorys_mark_as_the_send_does() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    repo.commit_file("b.txt", "two\n", "ours");
    repo.git(&["config", "remote.pushDefault", "."]);
    let (exec, cancel) = env();
    let (sink, session) = opened(&repo).await;
    let here = toolbar_from(&sink, 0, "main").await;
    assert_eq!(
        (here.standing, here.target),
        (remote::PushStanding::StaysHere, None)
    );
    let sent = session
        .push_current("origin".into(), PushForce::None)
        .expect("the write is accepted");
    let refused = write_answer(&sink, sent).await;
    assert!(
        refused
            .as_deref()
            .is_some_and(|said| said.contains("this repository")),
        "the send: {refused:?}"
    );
    write_settled(&sink, sent).await;

    repo.git(&["config", "remote.pushDefault", "gone"]);
    let from = sink.events.lock().unwrap().len();
    poll_landed(&session).await;
    let gone = toolbar_from(&sink, from, "main").await;
    session.close_and_wait().await;
    assert_eq!(
        (gone.standing, gone.target.as_deref()),
        (remote::PushStanding::Elsewhere, Some("gone/main"))
    );
    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.target, far("gone", "main"), "where the send goes");
}

/// Every answer the toolbar passes through from event `from` on, the one
/// it held there first, each once in a row.
fn toolbar_passes(sink: &CaptureSink, from: usize) -> Vec<(remote::PushStanding, Option<String>)> {
    type Held<'a> = (
        Option<(
            &'a platitude_core::status::WorktreeStatus,
            &'a remote::StatusPush,
        )>,
        Option<&'a platitude_core::session::RefsSnapshot>,
    );
    fn note(held: Held<'_>, passes: &mut Vec<(remote::PushStanding, Option<String>)>) {
        if let (Some((status, push)), Some(snapshot)) = held {
            let facts = toolbar(status, push, snapshot);
            let answer = (facts.standing, facts.target);
            if passes.last() != Some(&answer) {
                passes.push(answer);
            }
        }
    }
    let events = sink.events.lock().unwrap();
    let mut held: Held<'_> = (None, None);
    let mut passes = Vec::new();
    for (at, event) in events.iter().enumerate() {
        if at == from {
            note(held, &mut passes);
        }
        let moved = match event {
            SessionEvent::StatusLoaded { status, push, .. } => {
                held.0 = Some((status.as_ref(), push));
                true
            }
            SessionEvent::RefsLoaded { snapshot, .. } => {
                held.1 = Some(snapshot.as_ref());
                true
            }
            _ => false,
        };
        if moved && at >= from {
            note(held, &mut passes);
        }
    }
    passes
}

/// Waits for a status, and the refs with the repository's mark on
/// `marked`, among the events from `from` on.
async fn both_read(sink: &CaptureSink, from: usize, marked: Option<&str>) {
    sink.wait_for("a status and the refs after the mark moved", |evs| {
        let refs = evs[from..].iter().any(|e| {
            matches!(e, SessionEvent::RefsLoaded { snapshot, .. }
                if snapshot.marks.origin.push_default.as_ref().map(|m| m.remote.as_str())
                    == marked)
        });
        let status = evs[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { .. }));
        (refs && status).then_some(())
    })
    .await;
}

/// A mark moved outside, through the toolbar: each answer it passes
/// through is the old destination's or the new one's, never the fork with
/// nothing counted (`Elsewhere`) — a plain `push` over a diverged fork.
///
/// Measured: unset, the status lands before the refs read the move sends
/// out (`RepoSession::note_push_default`); set again, the refs land first,
/// the status tick reading the fork's counts one process more. Held side by
/// side, either order pairs a mark with counts read under another.
#[tokio::test(flavor = "multi_thread")]
async fn a_mark_moved_outside_reaches_the_toolbar_whole() {
    let (_origin, _fork, mut repo) = fork_diverged();
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    poll_landed(&session).await;
    let diverged = (
        remote::PushStanding::Diverged,
        Some("fork/main".to_string()),
    );
    let ready = (remote::PushStanding::Ready, Some("origin/main".to_string()));

    let from = sink.events.lock().unwrap().len();
    repo.git(&["config", "--unset", "remote.pushDefault"]);
    poll_landed(&session).await;
    both_read(&sink, from, None).await;
    assert_eq!(
        toolbar_passes(&sink, from),
        [diverged.clone(), ready.clone()]
    );

    let from = sink.events.lock().unwrap().len();
    repo.git(&["config", "remote.pushDefault", "fork"]);
    poll_landed(&session).await;
    both_read(&sink, from, Some("fork")).await;
    assert_eq!(toolbar_passes(&sink, from), [ready, diverged]);
    session.close_and_wait().await;
}

/// A branch the worktree is not on: the refs read counts it against where
/// its mark sends it, and a branch row's push goes there — refused plain,
/// landed under a lease on the tip that read showed.
#[tokio::test(flavor = "multi_thread")]
async fn a_branch_aside_is_counted_and_sent_where_its_mark_says() {
    let (_origin, fork, mut repo) = fork_diverged();
    repo.git(&["config", "--unset", "remote.pushDefault"]);
    repo.git(&["config", "branch.main.pushRemote", "fork"]);
    repo.git(&["switch", "-c", "aside"]);
    let (sink, session) = opened(&repo).await;

    let facts = row(&sink, "main").await;
    let fork_tip = repo.git(&["rev-parse", "refs/remotes/fork/main"]);
    assert_eq!(
        (
            facts.standing,
            facts.target.as_deref(),
            facts.lease.map(|oid| oid.to_hex()),
        ),
        (
            remote::PushStanding::Diverged,
            Some("fork/main"),
            Some(fork_tip.trim().to_string()),
        ),
        "only ahead of the upstream, diverged from where the push goes, leased to its tip there"
    );

    let plain = session
        .push_named("main".into(), "origin".into(), PushForce::None)
        .expect("the write is accepted");
    assert!(
        write_answer(&sink, plain).await.is_some(),
        "git refuses by the fork's main"
    );
    let leased = session
        .push_named(
            "main".into(),
            "origin".into(),
            PushForce::WithLease {
                expect: oid(&fork_tip),
            },
        )
        .expect("the write is accepted");
    assert_eq!(write_answer(&sink, leased).await, None);
    let fork_path = fork.path.clone();
    assert_eq!(
        repo.git_in(&fork_path, &["rev-parse", "main"]),
        repo.git(&["rev-parse", "main"]),
        "the fork now holds main"
    );
    assert_eq!(
        repo.git(&["symbolic-ref", "--short", "HEAD"]),
        "aside",
        "the worktree stays where it was"
    );
    session.close_and_wait().await;
}

/// A branch measured against one here (`remote = .`) that its own mark
/// sends to a remote: counted against the tracking ref there, and sent
/// there under its own name, its upstream left as it was.
#[tokio::test(flavor = "multi_thread")]
async fn a_branch_measured_against_one_here_goes_where_its_mark_says() {
    let (_origin, fork, mut repo) = origin_fork_and_clone();
    repo.git(&["switch", "--track", "-c", "stacked", "main"]);
    repo.git(&["push", "fork", "stacked"]);
    repo.commit_file("b.txt", "two\n", "stacked work");
    repo.git(&["switch", "main"]);
    repo.git(&["config", "branch.stacked.pushRemote", "fork"]);
    let (sink, session) = opened(&repo).await;

    let facts = row(&sink, "stacked").await;
    let fork_tip = repo.git(&["rev-parse", "refs/remotes/fork/stacked"]);
    assert_eq!(
        (
            facts.standing,
            facts.target.as_deref(),
            facts.lease.map(|oid| oid.to_hex()),
        ),
        (
            remote::PushStanding::Ready,
            Some("fork/stacked"),
            Some(fork_tip.trim().to_string()),
        ),
        "ahead of where the push goes, leased to its tip there"
    );

    let plain = session
        .push_named("stacked".into(), "origin".into(), PushForce::None)
        .expect("the write is accepted");
    assert_eq!(write_answer(&sink, plain).await, None);
    let fork_path = fork.path.clone();
    assert_eq!(
        repo.git_in(&fork_path, &["rev-parse", "stacked"]),
        repo.git(&["rev-parse", "stacked"]),
        "the fork now holds stacked"
    );
    assert_eq!(
        repo.git(&["config", "branch.stacked.remote"]).trim(),
        ".",
        "the upstream is left as it was"
    );
    session.close_and_wait().await;
}

/// A read of those counts that cannot tell repeats the last answer, as the
/// status tick's does: the diverged row stays the held overwrite instead of
/// falling to a plain push git would turn down. The read answers once and
/// then fails (`remote::mock_push_tracks` — on this thread's runtime, the
/// one the session's reads run on: the mock is the thread's).
#[tokio::test]
#[mry::lock(remote::push_tracks)]
async fn a_count_read_that_cannot_tell_repeats_the_last_answer() {
    let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = std::sync::Arc::clone(&reads);
    remote::mock_push_tracks().returns_with(move || {
        if counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            Ok(vec![(
                "main".to_string(),
                remote::PushTrack {
                    remote: "fork".to_string(),
                    tracking: "fork/main".to_string(),
                    counts: platitude_core::AheadBehind {
                        ahead: 1,
                        behind: 1,
                    },
                },
            )])
        } else {
            Err(GitError::UnexpectedOutput {
                command: "git for-each-ref".to_string(),
                message: "made to fail".to_string(),
            })
        }
    });
    let (_origin, _fork, mut repo) = fork_diverged();
    repo.git(&["config", "--unset", "remote.pushDefault"]);
    repo.git(&["config", "branch.main.pushRemote", "fork"]);
    repo.git(&["switch", "-c", "aside"]);
    let (sink, session) = opened(&repo).await;
    // The standing of `main` in the last snapshot that lists `branch`.
    let main_beside = |branch: &'static str| {
        sink.wait_for("a snapshot listing the branch", move |evs| {
            evs.iter().rev().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot, .. }
                    if snapshot.local_named(branch).is_some() =>
                {
                    Some(row_in(snapshot, "main"))
                }
                _ => None,
            })
        })
    };
    assert_eq!(
        main_beside("aside").await.standing,
        remote::PushStanding::Diverged
    );

    // A ref moved: the snapshot is built again, and this read fails.
    repo.git(&["branch", "later"]);
    poll_landed(&session).await;
    let facts = main_beside("later").await;
    let fork_tip = repo.git(&["rev-parse", "refs/remotes/fork/main"]);
    assert_eq!(
        (facts.standing, facts.lease.map(|oid| oid.to_hex())),
        (
            remote::PushStanding::Diverged,
            Some(fork_tip.trim().to_string())
        )
    );
    assert!(
        reads.load(std::sync::atomic::Ordering::SeqCst) >= 2,
        "the second snapshot asked again and was not told"
    );
    session.close_and_wait().await;
}

/// A status tick whose marks read cannot tell repeats the branch's last
/// push reading whole — the marks with the counts read under them: an
/// empty one would weigh the toolbar's push by the upstream alone, a plain
/// push over the fork it diverged from. The read answers until the test
/// says, then fails (`remote::mock_push_marks`, on this thread's runtime
/// as above).
#[tokio::test]
#[mry::lock(remote::push_marks)]
async fn a_marks_read_that_cannot_tell_repeats_the_last_push_reading() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let failed = std::sync::Arc::new(AtomicUsize::new(0));
    let failing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (count, told) = (
        std::sync::Arc::clone(&failed),
        std::sync::Arc::clone(&failing),
    );
    remote::mock_push_marks().returns_with(move || {
        if told.load(Ordering::SeqCst) {
            count.fetch_add(1, Ordering::SeqCst);
            return Err(GitError::UnexpectedOutput {
                command: "git config".to_string(),
                message: "made to fail".to_string(),
            });
        }
        Ok(remote::PushMarks {
            push_default: Some(remote::PushDefault {
                remote: "fork".to_string(),
                local: true,
            }),
            tracks: Some("origin".to_string()),
            merge: Some("refs/heads/main".to_string()),
            ..remote::PushMarks::default()
        })
    });
    let (_origin, _fork, repo) = fork_diverged();
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    let last_push = |from: usize| {
        sink.wait_for("a status", move |evs| {
            evs[from..].iter().rev().find_map(|e| match e {
                SessionEvent::StatusLoaded { status, push, .. } => {
                    Some((status.items.len(), push.clone()))
                }
                _ => None,
            })
        })
    };
    let (_, read) = last_push(0).await;
    assert_eq!(
        (
            read.push_default.as_deref(),
            read.track.as_ref().map(|track| track.tracking.as_str())
        ),
        (Some("fork"), Some("fork/main"))
    );

    failing.store(true, Ordering::SeqCst);
    let from = sink.events.lock().unwrap().len();
    std::fs::write(
        repo.path.join("note.txt"),
        "a change for the tick to read\n",
    )
    .expect("write a file");
    poll_landed(&session).await;
    let (files, repeated) = last_push(from).await;
    assert_eq!(files, 1, "the tick after the change");
    assert_eq!(repeated, read);
    assert!(
        failed.load(Ordering::SeqCst) >= 1,
        "that tick asked for the marks and was not told"
    );
    session.close_and_wait().await;
}

/// The refs read counts branches against where a mark sends them in the
/// one arrangement those counts speak for — a mark on another remote than
/// the upstream's — and spends no process on them anywhere else. The
/// worktree is on a branch that tracks nothing, so the status tick reads
/// no counts of its own.
#[tokio::test(flavor = "multi_thread")]
async fn the_refs_read_counts_only_the_branches_a_mark_sends_elsewhere() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    repo.git(&["switch", "-c", "aside"]);
    let (sink, session) = opened(&repo).await;
    session.set_recording(Recording::WithBackground);
    let counted = |sink: &CaptureSink| {
        sink.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| {
                matches!(e, SessionEvent::CommandStarted { display, .. }
                         if display.contains("%(push:track)"))
            })
            .count()
    };
    // The snapshot built on the mark `marked` names, and what it says of
    // `main`: only a snapshot built after the mark moved carries it.
    let main_under = |marked: &'static str| {
        sink.wait_for("a snapshot built on the mark", move |evs| {
            evs.iter().rev().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot, .. }
                    if snapshot
                        .marks
                        .origin
                        .push_default
                        .as_ref()
                        .is_some_and(|mark| mark.remote == marked) =>
                {
                    Some(row_in(snapshot, "main"))
                }
                _ => None,
            })
        })
    };

    repo.git(&["config", "remote.pushDefault", "origin"]);
    poll_landed(&session).await;
    assert_eq!(
        main_under("origin").await.standing,
        remote::PushStanding::Clean
    );
    assert_eq!(
        counted(&sink),
        0,
        "the mark is the upstream's own remote: nothing reads the counts"
    );

    repo.git(&["config", "remote.pushDefault", "fork"]);
    poll_landed(&session).await;
    assert_eq!(
        main_under("fork").await.standing,
        remote::PushStanding::Elsewhere,
        "the fork was never fetched, so nothing here counts against it"
    );
    assert!(
        counted(&sink) > 0,
        "the answer above is of the command that read the counts"
    );
    session.close_and_wait().await;
}

/// What git does with the checkout key once it is written. Writing it and
/// reading the two keys apart are held above and by `remote::marks`' own
/// tests, so only the full gate runs these.
mod periodic {
    use super::*;

    /// What the second key is for: `git switch` refuses a name more than one
    /// remote carries until a remote is marked, then tracks the marked one.
    #[tokio::test]
    #[ignore = "git's own use of checkout.defaultRemote: not worth the pre-merge run"]
    async fn a_marked_remote_is_where_an_ambiguous_switch_takes_its_branch() {
        let (_origin, _fork, mut repo) = origin_fork_and_clone();
        let (exec, cancel) = env();

        repo.git(&["push", "origin", "main:topic"]);
        repo.git(&["push", "fork", "main:topic"]);
        repo.git(&["fetch", "fork"]);
        repo.git_expect_failure(&["switch", "topic"]);

        remote::mark_origin(&exec, &repo.path, "fork", &cancel)
            .await
            .expect("mark fork");
        repo.git(&["switch", "topic"]);
        assert_eq!(
            repo.git(&["rev-parse", "--abbrev-ref", "topic@{upstream}"])
                .trim(),
            "fork/topic"
        );
    }

    /// Why the read keeps the two keys apart: a remote renamed in a terminal
    /// takes the push's key along and leaves the checkout one on the old name.
    #[tokio::test]
    #[ignore = "what git's remote rename rewrites: not worth the pre-merge run"]
    async fn a_remote_renamed_in_a_terminal_leaves_the_checkout_key_behind() {
        let (_origin, _fork, mut repo) = origin_fork_and_clone();
        let (exec, cancel) = env();

        remote::mark_origin(&exec, &repo.path, "fork", &cancel)
            .await
            .expect("mark fork");
        repo.git(&["remote", "rename", "fork", "home"]);

        let marks = remote::kept_marks(&exec, &repo.path, &cancel)
            .await
            .expect("an answer")
            .origin;
        assert_eq!(marks.push_default.expect("a push mark").remote, "home");
        assert_eq!(marks.checkout_default.as_deref(), Some("fork"));
    }

    /// The branch's own mark sends it the same way, and git resolves the same
    /// destination for it.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "git's %(push) resolving branch.<name>.pushRemote: not worth the pre-merge run"]
    async fn a_branchs_own_mark_is_counted_against_the_same_ref() {
        let (_origin, _fork, mut repo) = fork_diverged();
        let (exec, cancel) = env();

        repo.git(&["config", "--unset", "remote.pushDefault"]);
        repo.git(&["config", "branch.main.pushRemote", "fork"]);
        assert_eq!(
            track_of(&repo.path, &exec, &cancel)
                .await
                .map(|track| track.tracking)
                .as_deref(),
            Some("fork/main")
        );
        let (sink, session) = opened(&repo).await;
        assert_eq!(
            toolbar_from(&sink, 0, "main").await.standing,
            remote::PushStanding::Diverged
        );
        session.close_and_wait().await;
    }

    /// Where nothing here tracks the branch over there, no count speaks and
    /// the push stays plain.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "what git's %(push) fields print for a ref nothing tracks: not worth the pre-merge run"]
    async fn a_destination_nothing_here_tracks_has_no_counts() {
        let (_origin, _fork, mut repo) = fork_diverged();
        let (exec, cancel) = env();

        // Named by the fetch refspec, but never fetched (`[gone]`).
        let tip = repo.git(&["rev-parse", "refs/remotes/fork/main"]);
        repo.git(&["update-ref", "-d", "refs/remotes/fork/main"]);
        assert_eq!(track_of(&repo.path, &exec, &cancel).await, None);
        repo.git(&["update-ref", "refs/remotes/fork/main", tip.trim()]);

        // A push refspec that renames the branch: `%(push)` names a ref the
        // send (an explicit refspec) never goes to.
        repo.git(&["push", "fork", "HEAD:refs/heads/wip"]);
        repo.git(&[
            "config",
            "remote.fork.push",
            "refs/heads/main:refs/heads/wip",
        ]);
        assert_eq!(track_of(&repo.path, &exec, &cancel).await, None);
        repo.git(&["config", "--unset", "remote.fork.push"]);

        // No fetch refspec: `%(push)` has nothing to name.
        repo.git(&["config", "--unset-all", "remote.fork.fetch"]);
        assert_eq!(track_of(&repo.path, &exec, &cancel).await, None);
        let (sink, session) = opened(&repo).await;
        assert_eq!(
            toolbar_from(&sink, 0, "main").await.standing,
            remote::PushStanding::Elsewhere
        );
        session.close_and_wait().await;
    }
}
