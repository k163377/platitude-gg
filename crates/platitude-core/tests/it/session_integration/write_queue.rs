//! Writes go through the session's queue: one at a time, a stop answered
//! as a landing, and a close that waits the queue out. The periodic part
//! walks single writes whose remaining claim is git's.

use crate::support::session::{opened, write_answer, write_result};
use crate::support::{TestRepo, barrier_hook};
use platitude_core::OperationKind;
use platitude_core::commit::CommitOptions;
use platitude_core::session::SessionEvent;

/// Writes are serialized per session: a burst of concurrent stage requests
/// must all land. Without the lock they race on `.git/index.lock` and some
/// silently fail.
///
/// Six is enough: two would already race the lock, and each write runs a
/// status read behind it, so the count is what this test costs under a
/// loaded suite — the longest test of the binary is this one, linearly.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_writes_are_serialized() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    const COUNT: usize = 6;
    for n in 0..COUNT {
        repo.write_file(&format!("f{n}.txt"), "content\n");
    }

    let (sink, session) = opened(&repo).await;

    for n in 0..COUNT {
        session.stage_paths(vec![format!("f{n}.txt")]);
    }

    let finished = sink
        .wait_for("all writes finished", |evs| {
            let done: Vec<Option<String>> = evs
                .iter()
                .filter_map(|e| match e {
                    SessionEvent::WriteFinished { error, .. } => Some(error.clone()),
                    _ => None,
                })
                .collect();
            (done.len() == COUNT).then_some(done)
        })
        .await;
    assert!(
        finished.iter().all(Option::is_none),
        "every write succeeded: {finished:?}"
    );
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::WriteStarted { .. })),
        COUNT,
        "one start per write"
    );

    let staged = repo.git(&["diff", "--cached", "--name-only"]);
    assert_eq!(staged.lines().count(), COUNT, "all files staged: {staged}");
    session.close();
}

/// A conflicting rebase driven through the session: the failure is
/// reported, the status refresh carries the step counter, and the abort
/// lands through the same write path.
#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_rebase_reports_progress_and_aborts_through_the_session() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("f.txt", "topic\n", "topic change");
    repo.commit_file("g.txt", "extra\n", "topic extra");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo.git(&["checkout", "topic"]);

    let (sink, session) = opened(&repo).await;

    session.rebase(
        "main".into(),
        platitude_core::integrate::RebaseOptions::default(),
    );

    let error = sink
        .wait_for("rebase reported", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::WriteFinished {
                    kind: OperationKind::Rebase,
                    error,
                    ..
                } => Some(error.clone()),
                _ => None,
            })
        })
        .await;
    // A landing of its own: git stopped and left the rebase
    // standing. The event that says so has already been published —
    // it goes out between the write's start and the answer just
    // waited for.
    assert_eq!(error, None, "a stop is not a failed write");
    assert!(
        crate::support::session::write_stopped(&sink, OperationKind::Rebase),
        "the landing is said out loud, because the answer cannot say it"
    );

    // The refresh that follows carries the step counter.
    let progress = sink
        .wait_for("progress in a status refresh", |evs| {
            evs.iter().rev().find_map(|e| match e {
                SessionEvent::StatusLoaded {
                    progress: Some(p),
                    op_state,
                    ..
                } if op_state.rebasing => Some(*p),
                _ => None,
            })
        })
        .await;
    assert_eq!((progress.current, progress.total), (1, 2));

    session.resolve_current(platitude_core::integrate::Continuation::Abort);
    // The clean status has to be one from *after* the abort. `wait_for`
    // polls the whole event list and never drains it, so a bare "any clean
    // StatusLoaded" also matches the one this repository emitted when it
    // opened — the wait then returns before the abort has run and the
    // assertion below races it. Windows loses that race slowly enough to
    // pass; Linux does not (measured).
    sink.wait_for("clean again", |evs| {
        let aborted = evs.iter().position(|e| {
            matches!(
                e,
                SessionEvent::WriteFinished {
                    kind: OperationKind::Resolve,
                    ..
                }
            )
        })?;
        evs[aborted..].iter().rev().find_map(|e| match e {
            SessionEvent::StatusLoaded {
                op_state, progress, ..
            } if !op_state.any() && progress.is_none() => Some(()),
            _ => None,
        })
    })
    .await;
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "topic extra");
    session.close();
}

/// `stage_conflicted` is `git add` over the unmerged paths and nothing
/// else. A merge stopped on all four kinds of conflict at once (`UU`,
/// `AA`, `DU`, `UD`) settles whole, and a modification standing beside it
/// stays out of the index — which is the whole difference between this
/// and `git add --all`.
#[tokio::test(flavor = "multi_thread")]
async fn marking_the_conflicts_resolved_leaves_the_rest_of_the_tree_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "base\n", "root");
    repo.commit_file("ours-del.txt", "base\n", "one we will drop");
    repo.commit_file("theirs-del.txt", "base\n", "one they will drop");
    repo.commit_file("bystander.txt", "base\n", "one nobody touches");

    repo.git(&["checkout", "-b", "topic"]);
    repo.write_file("both.txt", "topic side\n");
    repo.write_file("ours-del.txt", "topic keeps editing\n");
    repo.write_file("added.txt", "topic's new file\n");
    std::fs::remove_file(repo.path.join("theirs-del.txt")).expect("the topic side drops it");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-m", "the topic side of all four"]);

    repo.git(&["checkout", "main"]);
    repo.write_file("both.txt", "main side\n");
    repo.write_file("theirs-del.txt", "main keeps editing\n");
    repo.write_file("added.txt", "main's new file\n");
    std::fs::remove_file(repo.path.join("ours-del.txt")).expect("the main side drops it");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-m", "the main side of all four"]);

    assert!(
        !repo.git_ok(&["merge", "--no-edit", "topic"]),
        "the merge stopped on the conflicts, which is the point of it"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"])
            .lines()
            .count(),
        4,
        "all four kinds are standing"
    );
    // Work of the reader's own, in the unstaged bucket beside the
    // conflicts. Written after the merge so nothing could carry it into
    // one.
    repo.write_file("bystander.txt", "read while settling this\n");

    let (sink, session) = opened(&repo).await;
    session.stage_conflicted();
    assert_eq!(
        write_result(&sink, OperationKind::Stage).await,
        None,
        "the write landed"
    );

    assert_eq!(
        repo.git(&["ls-files", "--unmerged"]),
        "",
        "nothing is unmerged any more"
    );
    let staged = repo.git(&["diff", "--cached", "--name-only"]);
    assert!(
        staged.lines().any(|l| l == "added.txt"),
        "the conflicted paths went into the index: {staged}"
    );
    assert!(
        !staged.lines().any(|l| l == "bystander.txt"),
        "the file beside them did not: {staged}"
    );
    assert!(
        repo.git(&["diff", "--name-only"])
            .lines()
            .any(|l| l == "bystander.txt"),
        "and is still where it was"
    );
    session.close();
}

/// A close loses nothing the queue was already asked for. The running
/// commit outlives it — the token handed to git is the write's own and no
/// stock budget binds the local lane (`operation::Lane::Local`), so
/// a tab going down, or the whole application, waits it out: killed
/// mid-write, the commit is simply gone (measured with a short stock
/// budget over the local lane; a cancel loses it the same way). And the
/// branch queued behind it still lands: the asked order is the queue's
/// promise, and a close only stops intake — the same tail the quit gate
/// holds the window for, so the pending count the gate reads drains to
/// zero exactly when the loop ends.
#[tokio::test(flavor = "multi_thread")]
async fn a_close_waits_out_the_running_write_and_the_queue() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let release = repo.path.join("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));

    let (sink, session) = opened(&repo).await;
    session.commit(
        "survives the close".into(),
        platitude_core::commit::CommitOptions::default(),
    );
    sink.wait_for("the commit started", |evs| {
        evs.iter()
            .any(|e| {
                matches!(
                    e,
                    SessionEvent::WriteStarted {
                        kind: OperationKind::Commit,
                        ..
                    }
                )
            })
            .then_some(())
    })
    .await;
    // Queued while the commit is still held by the hook: the close
    // below keeps this branch — they asked for it.
    session.create_branch("queued-behind".into(), None, false);

    session.close();
    // Released only after the close: the hook is still waiting for this
    // file, so git was provably alive when the cancel landed.
    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(
        write_result(&sink, OperationKind::Commit).await,
        None,
        "the commit landed"
    );
    assert_eq!(
        write_result(&sink, OperationKind::Branch).await,
        None,
        "the queued branch landed behind it"
    );
    let done = crate::support::wait::bounded(
        "the write loop ends with the queue drained",
        session.take_write_join().expect("the loop's task"),
    )
    .await;
    assert!(done.is_ok(), "the loop ended without panicking: {done:?}");

    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "survives the close"
    );
    assert!(
        repo.git_ok(&[
            "rev-parse",
            "--verify",
            "--quiet",
            "refs/heads/queued-behind"
        ]),
        "the branch asked for before the close exists after it"
    );
    assert_eq!(
        session.local_writes_pending(),
        0,
        "the pending count drained with the loop"
    );
}

/// **The id the queue hands back is the caller's own**, and the events
/// about that write are found by it and by nothing else.
///
/// Which is the only thing that can be: every event afterwards carries an
/// id, and none of them says which of them is yours. There is no shared
/// "the last one accepted" to read instead — a reader that went looking
/// for one would be reading whatever the *next* caller put there, since
/// nothing holds still between two asks.
///
/// **It is an identity.** `OperationId::next()` and the lock the queue
/// is entered under would be two moments if the number were taken
/// outside it, and a request that stalled in between would be numbered
/// ahead of one that went in first. Taken under that lock, the
/// numbering and the order agree — and **readers match**
/// ([`OperationId`] answers to equality alone), so nothing depends on
/// the agreement.
///
/// Two asks are outstanding at once here, the first held inside git by
/// its hook, which is what makes "which id is mine" a real
/// question.
#[tokio::test(flavor = "multi_thread")]
async fn each_ask_is_answered_under_the_id_it_was_given() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let release = repo.path.join("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));

    let (sink, session) = opened(&repo).await;
    let held = session
        .commit("held by the hook".into(), CommitOptions::default())
        .expect("the commit was accepted");
    let mine = session
        .create_branch("topic".into(), None, false)
        .expect("the branch was accepted");
    assert_ne!(held, mine, "two asks, two ids");
    assert!(
        held.as_u64() < mine.as_u64(),
        "numbered under the lock they queue under: {held:?} then {mine:?}"
    );

    // Both are out at once; each answer comes back under the id its own
    // ask was given, which is what lets a reader holding one tell the
    // other's answer from its own.
    std::fs::write(&release, b"go").expect("the hook is released");
    assert_eq!(write_answer(&sink, held).await, None, "the commit landed");
    assert_eq!(write_answer(&sink, mine).await, None, "and the branch did");
    session.close();
}

/// Taking the branch back a commit runs as a queued write of its own,
/// under the name the page keys its follow-up off: a reset rewrites the
/// working tree the diff on screen was read from.
#[tokio::test(flavor = "multi_thread")]
async fn a_reset_moves_the_branch_through_the_write_queue() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "second");

    let (sink, session) = opened(&repo).await;
    session.reset(root.clone(), platitude_core::branch::ResetMode::Mixed);
    assert_eq!(write_result(&sink, OperationKind::Reset).await, None);

    assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
    assert_eq!(
        repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "main",
        "the branch moved, not just HEAD"
    );
    session.close();
}

/// **What the pre-merge run leaves out**: single writes whose only claim
/// beyond the queue is what git does behind a fixed command line. The
/// queue's order is held by `operations` and by the close above, and the
/// `switch --create` command line by `branch::tests`; what a commit or a
/// `switch --create` leaves in the repository is git's. Run by the full
/// gate (`-- --ignored ::periodic::`) rather than by every change.
mod periodic {
    use super::*;

    /// The full local round trip through the session: stage, commit, branch.
    ///
    /// Order is the point: committing before staging, or branching before
    /// committing, would produce a different repository. The queue is
    /// what gives this.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "duplicates operations::each_write_is_settled_before_the_next_one_starts: not worth the pre-merge run"]
    async fn stage_commit_and_branch_through_the_session() {
        let mut repo = TestRepo::init();
        repo.commit_file("root.txt", "0\n", "root");
        repo.write_file("new.txt", "content\n");

        let (sink, session) = opened(&repo).await;

        session.stage_paths(vec!["new.txt".into()]);
        session.commit("add new file".into(), CommitOptions::default());
        session.create_branch("feature".into(), None, true);

        let done = sink
            .wait_for("three writes finished", |evs| {
                let done: Vec<(OperationKind, Option<String>)> = evs
                    .iter()
                    .filter_map(|e| match e {
                        SessionEvent::WriteFinished { kind, error, .. } => {
                            Some((*kind, error.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                (done.len() == 3).then_some(done)
            })
            .await;
        assert!(
            done.iter().all(|(_, error)| error.is_none()),
            "all succeeded: {done:?}"
        );
        assert_eq!(
            done.iter().map(|(kind, _)| *kind).collect::<Vec<_>>(),
            vec![
                OperationKind::Stage,
                OperationKind::Commit,
                OperationKind::Branch
            ],
            "they ran in the order they were asked for"
        );

        assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "add new file");
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "feature");
        session.close();
    }
}
