//! Writes go through the session's queue: one at a time, and a failure is
//! reported and refreshed like any other.

use crate::support::TestRepo;
use crate::support::session::{opened, write_result};
use platitude_core::session::SessionEvent;

/// Writes are serialized per session: a burst of concurrent stage requests
/// must all land. Without the lock they race on `.git/index.lock` and some
/// silently fail.
///
/// Six, not more: two would already race the lock, and each write runs a
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

/// A failing write reports git's own message and still refreshes, because a
/// command that stops halfway has already changed the repository.
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_write_reports_and_refreshes() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;

    session.delete_branch("does-not-exist".into(), false);
    let error = sink
        .wait_for("WriteFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::WriteFinished { error, .. } => Some(error.clone()),
                _ => None,
            })
        })
        .await;
    let error = error.expect("the write failed");
    assert!(
        error.contains("does-not-exist"),
        "git's wording is passed through: {error}"
    );
    session.close();
}

/// The full local round trip through the session: stage, commit, branch.
///
/// Order is the point: committing before staging, or branching before
/// committing, would produce a different repository. Serialization alone
/// does not give this — the queue does.
#[tokio::test(flavor = "multi_thread")]
async fn stage_commit_and_branch_through_the_session() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");

    let (sink, session) = opened(&repo).await;

    session.stage_paths(vec!["new.txt".into()]);
    session.commit(
        "add new file".into(),
        platitude_core::commit::CommitOptions::default(),
    );
    session.create_branch("feature".into(), None, true);

    let done = sink
        .wait_for("three writes finished", |evs| {
            let done: Vec<(&str, Option<String>)> = evs
                .iter()
                .filter_map(|e| match e {
                    SessionEvent::WriteFinished { op, error, .. } => Some((*op, error.clone())),
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
        done.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
        vec!["stage", "commit", "branch"],
        "they ran in the order they were asked for"
    );

    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "add new file");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "feature");
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
    assert_eq!(write_result(&sink, "reset").await, None);

    assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
    assert_eq!(
        repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "main",
        "the branch moved, not just HEAD"
    );
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
                    op: "rebase",
                    error,
                    ..
                } => Some(error.clone()),
                _ => None,
            })
        })
        .await;
    // Not a failure: git stopped and left the rebase standing, which is a
    // landing of its own. The event that says so
    // has already been published — it goes out between the write's start
    // and the answer just waited for.
    assert_eq!(error, None, "a stop is not a failed write");
    assert!(
        crate::support::session::write_stopped(&sink, "rebase"),
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
        let aborted = evs
            .iter()
            .position(|e| matches!(e, SessionEvent::WriteFinished { op: "resolve", .. }))?;
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
    assert_eq!(write_result(&sink, "stage").await, None, "the write landed");

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
