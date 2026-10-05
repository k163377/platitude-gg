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
use crate::support::session::{CaptureSink, opened};
use crate::support::wait::bounded;
use platitude_core::GitError;
use platitude_core::process::GitExecutor;
use platitude_core::remote::{self, PushForce};
use platitude_core::session::{Recording, RefreshOutcome, SessionEvent};
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

/// The upstream is under another name on purpose: with the same name on
/// both sides, a plan that never read `branch.<name>.merge` would pass too.
#[tokio::test]
async fn nothing_marked_sends_a_branch_where_it_tracks() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    repo.git(&["push", "--set-upstream", "origin", "main:elsewhere"]);
    // Asked of git: the value the plan must carry has to come from outside it.
    assert_eq!(
        repo.git(&["config", "--get", "branch.main.merge"]).trim(),
        "refs/heads/elsewhere",
        "the push recorded the name on the far side"
    );

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "origin");
    assert_eq!(
        plan.remote_branch, "elsewhere",
        "the upstream's own name, read out of branch.main.merge"
    );
    assert!(!plan.set_upstream, "the branch already tracks something");
}

/// A branch that tracks `origin/main` pushes to the marked remote under its
/// own name, and keeps tracking origin.
#[tokio::test]
async fn a_marked_remote_takes_the_push_from_the_upstream() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    remote::mark_origin(&exec, &repo.path, "fork", &cancel)
        .await
        .expect("mark fork");

    let plan = remote::plan_current_push(&exec, &repo.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    assert_eq!(plan.remote, "fork");
    assert_eq!(plan.remote_branch, "main", "the branch's own name");
    assert!(
        !plan.set_upstream,
        "a branch that tracks origin must go on tracking it"
    );

    remote::push(&exec, &repo.path, &plan, NET, &cancel)
        .await
        .expect("the push lands");
    assert_eq!(
        repo.git(&["config", "--get", "branch.main.remote"]).trim(),
        "origin",
        "where the branch fetches from is untouched"
    );
}

/// Both keys go on together and come off together, and the read gives
/// each back — the push's with the level that set it.
#[tokio::test]
async fn the_origin_mark_reads_back_with_both_keys() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    assert_eq!(
        remote::origin_marks(&exec, &repo.path, &cancel)
            .await
            .expect("unset is an answer, not a failure"),
        remote::OriginMarks::default()
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
    let marks = remote::origin_marks(&exec, &repo.path, &cancel)
        .await
        .expect("an answer");
    let marked = marks.push_default.expect("a push mark");
    assert_eq!(marked.remote, "fork");
    assert!(marked.local, "this repository's own config set it");
    assert_eq!(marks.checkout_default.as_deref(), Some("fork"));

    remote::clear_origin(&exec, &repo.path, &cancel)
        .await
        .expect("clear");
    assert_eq!(
        remote::origin_marks(&exec, &repo.path, &cancel)
            .await
            .expect("an answer"),
        remote::OriginMarks::default()
    );
    remote::clear_origin(&exec, &repo.path, &cancel)
        .await
        .expect("clearing a mark that is already gone is the state asked for");
}

/// The keys the snapshot reads, through the one reader the send shares.
#[tokio::test]
async fn the_marks_read_back_and_unset_is_an_answer() {
    let (_origin, _fork, mut repo) = origin_fork_and_clone();
    let (exec, cancel) = env();

    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    assert!(
        marks.push_remote.is_none() && marks.push_default.is_none(),
        "unset is an answer, not a failure"
    );

    repo.git(&["config", "branch.main.pushRemote", "fork"]);
    let marks = remote::push_marks(&exec, &repo.path, "main", &cancel)
        .await
        .expect("an answer");
    assert_eq!(marks.push_remote.as_deref(), Some("fork"));

    assert!(
        remote::push_marks(&exec, &repo.path, "topic", &cancel)
            .await
            .expect("an answer")
            .push_remote
            .is_none(),
        "another branch's mark is not this one's"
    );
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
    assert_eq!(plan.remote, "fork");
    assert_eq!(plan.remote_branch, "wip.v2+x", "the branch's own name");
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
    let label = remote::push_target(
        "main",
        "origin/main",
        marks.push_remote.as_deref().unwrap_or(""),
        &marked.remote,
        "origin",
        remotes,
    );
    assert_eq!(
        label,
        format!("{}/{}", plan.remote, plan.remote_branch),
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
    assert_eq!(plan.remote, "origin");
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
) -> remote::PushTrack {
    remote::push_track(exec, repo, "main", cancel)
        .await
        .expect("an answer")
}

/// The standing the toolbar reads, off what the status tick reads.
async fn standing_of(
    repo: &Path,
    exec: &GitExecutor,
    cancel: &CancellationToken,
) -> remote::PushStanding {
    let status = platitude_core::status::load(exec, repo, cancel)
        .await
        .expect("status");
    let marks = remote::push_marks(exec, repo, "main", cancel)
        .await
        .expect("marks");
    let track = track_of(repo, exec, cancel).await;
    remote::push_standing(
        false,
        false,
        "main",
        status.upstream.as_deref().unwrap_or_default(),
        status.upstream_tracked,
        status.ahead,
        status.behind,
        marks.push_remote.as_deref().unwrap_or_default(),
        marks
            .push_default
            .as_ref()
            .map_or("", |marked| marked.remote.as_str()),
        ["fork", "origin"],
        &track,
    )
}

/// The toolbar's held overwrite, leased to `expect`.
async fn push_leased(
    repo: &Path,
    exec: &GitExecutor,
    expect: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let force = PushForce::WithLease {
        expect: Some(expect.trim().to_string()),
    };
    let spec = remote::plan_current_push(exec, repo, "origin", force, cancel)
        .await
        .expect("a plan");
    remote::push(exec, repo, &spec, NET, cancel).await
}

/// The push goes to the fork, and git refuses it by the fork's `main`, not
/// by the upstream: counted against origin the branch is only ahead, so a
/// plain push looks like it fits and is turned down.
#[tokio::test]
async fn a_push_elsewhere_is_counted_against_the_destinations_own_ref() {
    let (_origin, fork, mut repo) = fork_diverged();
    let (exec, cancel) = env();

    let status = platitude_core::status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(
        (status.upstream.as_deref(), status.ahead, status.behind),
        (Some("origin/main"), 1, 0),
        "against the upstream there is only ours to add"
    );
    assert_eq!(
        track_of(&repo.path, &exec, &cancel).await,
        remote::PushTrack {
            tracking: "fork/main".to_string(),
            ahead: 1,
            behind: 1,
        }
    );
    assert_eq!(
        standing_of(&repo.path, &exec, &cancel).await,
        remote::PushStanding::Diverged
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
    let destination_tip = repo.git(&["rev-parse", "refs/remotes/fork/main"]);
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

/// The branch's own mark sends it the same way, and git resolves the same
/// destination for it.
#[tokio::test]
async fn a_branchs_own_mark_is_counted_against_the_same_ref() {
    let (_origin, _fork, mut repo) = fork_diverged();
    let (exec, cancel) = env();

    repo.git(&["config", "--unset", "remote.pushDefault"]);
    repo.git(&["config", "branch.main.pushRemote", "fork"]);
    assert_eq!(
        track_of(&repo.path, &exec, &cancel).await.tracking,
        "fork/main"
    );
    assert_eq!(
        standing_of(&repo.path, &exec, &cancel).await,
        remote::PushStanding::Diverged
    );
}

/// Where nothing here tracks the branch over there, no count speaks and
/// the push stays plain.
#[tokio::test]
async fn a_destination_nothing_here_tracks_has_no_counts() {
    let (_origin, _fork, mut repo) = fork_diverged();
    let (exec, cancel) = env();
    let none = remote::PushTrack::default();

    // Named by the fetch refspec, but never fetched (`[gone]`).
    let tip = repo.git(&["rev-parse", "refs/remotes/fork/main"]);
    repo.git(&["update-ref", "-d", "refs/remotes/fork/main"]);
    assert_eq!(track_of(&repo.path, &exec, &cancel).await, none);
    repo.git(&["update-ref", "refs/remotes/fork/main", tip.trim()]);

    // A push refspec that renames the branch: `%(push)` names a ref the
    // send (an explicit refspec) never goes to.
    repo.git(&["push", "fork", "HEAD:refs/heads/wip"]);
    repo.git(&[
        "config",
        "remote.fork.push",
        "refs/heads/main:refs/heads/wip",
    ]);
    assert_eq!(track_of(&repo.path, &exec, &cancel).await, none);
    repo.git(&["config", "--unset", "remote.fork.push"]);

    // No fetch refspec: `%(push)` has nothing to name.
    repo.git(&["config", "--unset-all", "remote.fork.fetch"]);
    assert_eq!(track_of(&repo.path, &exec, &cancel).await, none);
    assert_eq!(
        standing_of(&repo.path, &exec, &cancel).await,
        remote::PushStanding::Elsewhere
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
    let poll = || async {
        let polled = bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
        assert!(
            matches!(polled, RefreshOutcome::Changed | RefreshOutcome::Unchanged),
            "the poll ran: {polled:?}"
        );
    };
    let statuses = |sink: &CaptureSink| -> Vec<remote::PushTrack> {
        sink.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                SessionEvent::StatusLoaded { push_track, .. } => Some(push_track.clone()),
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
            !read.is_empty()
                && read
                    .iter()
                    .all(|track| *track == remote::PushTrack::default()),
            "{why}: {read:?}"
        );
        assert_eq!(
            asked(sink),
            0,
            "{why}: nothing reads the counts no one needs"
        );
    };

    poll().await;
    quiet(&sink, "nothing is marked");
    repo.git(&["config", "remote.pushDefault", "origin"]);
    poll().await;
    quiet(&sink, "the mark is the upstream's own remote");

    repo.git(&["config", "remote.pushDefault", "fork"]);
    poll().await;
    let track = sink
        .wait_for("a status with the fork's counts", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::StatusLoaded { push_track, .. }
                    if !push_track.tracking.is_empty() =>
                {
                    Some(push_track.clone())
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(
        track,
        remote::PushTrack {
            tracking: "fork/main".to_string(),
            ahead: 1,
            behind: 1,
        }
    );
    assert!(
        asked(&sink) > 0,
        "the count above is of the command that read these"
    );
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

        let marks = remote::origin_marks(&exec, &repo.path, &cancel)
            .await
            .expect("an answer");
        assert_eq!(marks.push_default.expect("a push mark").remote, "home");
        assert_eq!(marks.checkout_default.as_deref(), Some("fork"));
    }
}
