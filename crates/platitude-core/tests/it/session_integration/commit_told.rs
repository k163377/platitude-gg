//! What a commit leaves, told as git answers (`session::foretold`): the
//! status worked out from the one told before it, ahead of the status read
//! behind the write, and the walk that goes ahead of that read with it.
//!
//! The tests that have to stand between the two hold the status read
//! behind the commit off with a read of their own, parked at its spawn
//! ([`status_parked`]): it keeps the status flight, so the queue's read
//! waits behind it while the queue's own task goes on with the rest.

use std::sync::Arc;

use crate::support::remote::origin_and_clone;
use crate::support::session::{
    CaptureSink, install_todo_editor, open_with_doors, opened, poll_landed, write_answer,
    write_settled,
};
use crate::support::wait::bounded;
use crate::support::{TestRepo, oid};
use platitude_core::OperationId;
use platitude_core::commit::CommitOptions;
use platitude_core::session::{FollowUp, PassStep, Recording, RepoSession, SessionEvent};
use platitude_core::status::{StatusItem, WorktreeStatus};

/// One status as the session told it.
struct Told {
    /// Where it stands among the events.
    at: usize,
    looked: u64,
    head_seq: u64,
    status: Arc<WorktreeStatus>,
}

impl Told {
    /// Its rows as `<staged><unstaged> <path>`, `??` for an untracked file.
    fn rows(&self) -> Vec<String> {
        self.status
            .items
            .iter()
            .map(|item| match item {
                StatusItem::Tracked {
                    staged,
                    unstaged,
                    path,
                    ..
                } => format!("{staged}{unstaged} {path}"),
                StatusItem::Unmerged { path, .. } => format!("UU {path}"),
                StatusItem::Untracked { path } => format!("?? {path}"),
            })
            .collect()
    }
}

/// Every status told, in order.
fn statuses(events: &[SessionEvent]) -> Vec<Told> {
    events
        .iter()
        .enumerate()
        .filter_map(|(at, e)| match e {
            SessionEvent::StatusLoaded {
                status,
                head_seq,
                looked,
                ..
            } => Some(Told {
                at,
                looked: *looked,
                head_seq: *head_seq,
                status: Arc::clone(status),
            }),
            _ => None,
        })
        .collect()
}

/// git's answer to the write under `id`: where it stands, and the two
/// numbers it carries.
struct Answer {
    at: usize,
    head_seq: u64,
    reads_from: u64,
}

fn answer(events: &[SessionEvent], id: OperationId) -> Answer {
    events
        .iter()
        .enumerate()
        .find_map(|(at, e)| match e {
            SessionEvent::WriteFinished {
                id: got,
                head_seq,
                reads_from,
                ..
            } if *got == id => Some(Answer {
                at,
                head_seq: *head_seq,
                reads_from: *reads_from,
            }),
            _ => None,
        })
        .expect("the write answered")
}

fn settled_at(events: &[SessionEvent], id: OperationId) -> usize {
    events
        .iter()
        .position(|e| matches!(e, SessionEvent::WriteSettled { id: got, .. } if *got == id))
        .expect("the write settled")
}

/// The statuses told behind the answer of the write under `id`.
fn told_behind(events: &[SessionEvent], id: OperationId) -> Vec<Told> {
    let from = answer(events, id).at;
    statuses(events)
        .into_iter()
        .filter(|told| told.at > from)
        .collect()
}

/// The graphs swapped in from event `from` on: where each stands, and its
/// rows' ids, the worktree row's being all zeros.
fn graphs(events: &[SessionEvent], from: usize) -> Vec<(usize, Vec<String>)> {
    events
        .iter()
        .enumerate()
        .skip(from)
        .filter_map(|(at, e)| match e {
            SessionEvent::LogReplaced { rows, .. } => {
                Some((at, rows.iter().map(|row| row.oid_hex.clone()).collect()))
            }
            _ => None,
        })
        .collect()
}

fn is_worktree_row(oid: &str) -> bool {
    oid.bytes().all(|b| b == b'0')
}

fn is_status_spawn(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. } if display.starts_with("git status"))
}

/// The pending diffs a status pass reads the line-ending marks off.
fn is_marks_read(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. } if display.starts_with("git diff"))
}

/// The refs listing, told from the walk's narrower one by what it lists.
fn is_refs_spawn(event: &SessionEvent) -> bool {
    matches!(
        event,
        SessionEvent::CommandStarted { display, .. }
            if display.contains("for-each-ref") && display.ends_with("refs/heads refs/remotes refs/tags")
    )
}

/// Every read the opening started is over, and the reads from here on are
/// in the command log.
async fn quiet(sink: &CaptureSink, session: &Arc<RepoSession>) {
    sink.opening_settled(session).await;
    bounded(
        "the opening's other worktrees",
        session.wait_for_carried_pass(),
    )
    .await;
    bounded("the opening's reads", session.wait_for_snapshot_reads()).await;
    bounded("the opening's graph", session.wait_for_graph_passes()).await;
    session.set_recording(Recording::WithBackground);
}

/// A status read asked for and parked at its spawn: it has looked, and its
/// git has not run. Answers the release (module doc).
async fn status_parked(
    sink: &Arc<CaptureSink>,
    session: &Arc<RepoSession>,
) -> std::sync::mpsc::Sender<()> {
    let spawned = sink.count(is_status_spawn);
    let release = sink.park_once("the parked status", is_status_spawn);
    session.refresh_status();
    sink.wait_for("the status reached its spawn", move |evs| {
        (evs.iter().filter(|e| is_status_spawn(e)).count() > spawned).then_some(())
    })
    .await;
    release
}

fn commit(session: &Arc<RepoSession>, message: &str, options: CommitOptions) -> OperationId {
    session
        .commit(message.into(), options)
        .expect("the commit was accepted")
}

fn amend() -> CommitOptions {
    CommitOptions {
        amend: true,
        ..CommitOptions::default()
    }
}

/// One commit, `new.txt` staged over it, and `root.txt` edited beside it
/// with `loose.txt` untracked: a commit leaves the last two.
fn with_changes_left_over() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    repo.write_file("root.txt", "edited\n");
    repo.write_file("loose.txt", "loose\n");
    repo
}

/// One commit and `new.txt` staged over it: a commit leaves the tree clean.
fn with_everything_staged() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    repo
}

/// The status a commit left is told right behind git's answer, under the
/// answer's own two numbers, before any `git status` is spawned: the staged
/// row gone, the rest as it was. The read behind the write then tells the
/// same rows under a later stamp and the new commit, and the history is
/// walked once.
#[tokio::test(flavor = "multi_thread")]
async fn a_commit_tells_the_status_it_left_at_gits_answer() {
    let mut repo = with_changes_left_over();
    let before = repo.git(&["rev-parse", "HEAD"]);
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let id = commit(&session, "the staged file", CommitOptions::default());
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    let head = repo.git(&["rev-parse", "HEAD"]);
    {
        let events = sink.events.lock().unwrap();
        let answered = answer(&events, id);
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 2, "the one worked out, then the one read");
        let (ahead, read) = (&told[0], &told[1]);
        let spawned = events
            .iter()
            .enumerate()
            .skip(answered.at)
            .filter(|(_, e)| is_status_spawn(e))
            .map(|(at, _)| at)
            .collect::<Vec<_>>();
        assert_eq!(spawned.len(), 1, "one status was read behind the commit");
        assert!(
            ahead.at < spawned[0],
            "told before the status behind the write was spawned"
        );
        assert_eq!(
            (ahead.looked, ahead.head_seq),
            (answered.reads_from, answered.head_seq),
            "under the two numbers the write's answer carries"
        );
        assert_eq!(ahead.rows(), [".M root.txt", "?? loose.txt"]);
        assert_eq!(
            ahead.status.branch_oid.map(|oid| oid.to_hex()),
            Some(before),
            "the branch lines are not worked out"
        );

        assert!(read.looked > answered.reads_from && read.head_seq >= answered.head_seq);
        assert_eq!(read.rows(), ahead.rows(), "git's own word is the same");
        assert_eq!(read.status.branch_oid.map(|oid| oid.to_hex()), Some(head));
        assert_eq!(
            graphs(&events, answered.at)
                .iter()
                .filter(|(at, _)| *at < settled_at(&events, id))
                .count(),
            1,
            "a read that finds what was told asks for no second walk"
        );
    }
    session.close_and_wait().await;
}

/// A commit of everything: the worktree row is off the graph and the
/// commit's row on it while the status behind the write has not been read
/// — it is held off here — and that read, finding the tree as told, walks
/// nothing more.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_of_everything_is_on_the_graph_before_its_status_is_read() {
    let mut repo = with_everything_staged();
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let release = status_parked(&sink, &session).await;
    let id = commit(&session, "everything", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    let from = answer(&sink.events.lock().unwrap(), id).at;
    let rows = sink
        .wait_for("the graph behind the commit", |evs| {
            graphs(evs, from).into_iter().next().map(|(_, rows)| rows)
        })
        .await;
    let head = repo.git(&["rev-parse", "HEAD"]);
    assert_eq!(rows.first(), Some(&head), "the commit's row leads");
    assert!(
        !rows.iter().any(|oid| is_worktree_row(oid)),
        "and the worktree row is gone: {rows:?}"
    );
    {
        let events = sink.events.lock().unwrap();
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 1, "no status has been read behind the commit");
        assert_eq!(told[0].rows(), [] as [&str; 0]);
    }

    release.send(()).expect("the parked status is waiting");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        let answered = answer(&events, id);
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 2, "the one worked out, then the one read");
        assert!(told[1].looked > answered.reads_from);
        assert_eq!(told[1].rows(), [] as [&str; 0]);
        assert_eq!(
            graphs(&events, answered.at).len(),
            1,
            "the status read found the tree as told: one walk"
        );
    }
    session.close_and_wait().await;
}

/// A hook that leaves a file behind the commit makes what was told wrong:
/// the first walk laid the graph with no worktree row, the status read
/// behind the write tells the file, and the history is walked again with
/// the row back.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_status_told_wrong_is_put_right_by_the_read_and_walked_again() {
    let repo = with_everything_staged();
    repo.write_hook("post-commit", "echo left > left-by-the-hook.txt\n");
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let release = status_parked(&sink, &session).await;
    let id = commit(&session, "everything", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    let from = answer(&sink.events.lock().unwrap(), id).at;
    let first = sink
        .wait_for("the graph laid from what was told", |evs| {
            graphs(evs, from).into_iter().next().map(|(_, rows)| rows)
        })
        .await;
    assert!(
        !first.iter().any(|oid| is_worktree_row(oid)),
        "told clean, walked without the worktree row: {first:?}"
    );

    release.send(()).expect("the parked status is waiting");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 2);
        assert_eq!(told[0].rows(), [] as [&str; 0]);
        assert_eq!(told[1].rows(), ["?? left-by-the-hook.txt"]);
        let walked = graphs(&events, from);
        assert_eq!(walked.len(), 2, "walked again for the row: {walked:?}");
        assert!(
            walked[1].1.first().is_some_and(|oid| is_worktree_row(oid)),
            "the worktree row is back: {:?}",
            walked[1].1
        );
        assert!(walked[1].0 < settled_at(&events, id));
    }
    session.close_and_wait().await;
}

/// Of the two walks behind such a commit the later one says whether the
/// repository reached the screen: the first falling over is put right by a
/// second that lands, and a first that landed does not answer for a second
/// that fell over.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_later_of_two_walks_answers_for_the_commit() {
    let mut repo = with_everything_staged();
    repo.write_hook("post-commit", "echo left >> left-by-the-hook.txt\n");
    let (sink, session, doors) = open_with_doors(&repo);
    // The root and the worktree row.
    sink.opened_graph(&session, 2).await;
    quiet(&sink, &session).await;

    doors.fail_every_pass(PassStep::Swapping);
    let release = status_parked(&sink, &session).await;
    let id = commit(&session, "first", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    let from = answer(&sink.events.lock().unwrap(), id).at;
    sink.wait_for("the walk ahead of the status fell over", |evs| {
        evs[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::LogStale { stale: true }))
            .then_some(())
    })
    .await;
    doors.stop_failing();
    release.send(()).expect("the parked status is waiting");
    assert_eq!(
        write_settled(&sink, id).await,
        [],
        "the walk behind the status read landed"
    );

    // The hook's file is committed, and the hook leaves it changed again:
    // told clean, read dirty, as before.
    repo.git(&["add", "left-by-the-hook.txt"]);
    poll_landed(&session).await;
    let release = status_parked(&sink, &session).await;
    let id = commit(&session, "second", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    let from = answer(&sink.events.lock().unwrap(), id).at;
    sink.wait_for("the walk ahead of the status landed", |evs| {
        graphs(evs, from).into_iter().next().map(|_| ())
    })
    .await;
    doors.fail_every_pass(PassStep::Swapping);
    release.send(()).expect("the parked status is waiting");
    assert_eq!(
        write_settled(&sink, id).await,
        [FollowUp::Graph],
        "the walk behind the status read fell over"
    );
    session.close_and_wait().await;
}

/// A remote-tracking ref on an object git cannot unpack: the refs listing
/// reads every ref's type and fails on it, where the status and the stash
/// and worktree listings read nothing under `refs/remotes`.
const BROKEN_REF: &str = "refs/remotes/origin/broken";

/// Fails the next refs listing and nothing else: [`BROKEN_REF`] is planted
/// at the listing's ask, before its git runs, and taken away where the read
/// says it failed (`OpFailed`) — ahead of any walk, which lists the
/// remote-tracking refs too.
fn fail_the_next_refs_read(sink: &Arc<CaptureSink>, repo: &TestRepo) {
    let git = repo.path.join(".git");
    let name = "cd".repeat(19);
    let object = git.join("objects").join("ab").join(&name);
    let reference = git.join(BROKEN_REF);
    let then = Arc::clone(sink);
    sink.hook_once(is_refs_spawn, move || {
        std::fs::create_dir_all(object.parent().unwrap()).unwrap();
        std::fs::write(&object, b"not a zlib stream").unwrap();
        std::fs::create_dir_all(reference.parent().unwrap()).unwrap();
        std::fs::write(&reference, format!("ab{name}\n")).unwrap();
        then.hook_once(
            |e| matches!(e, SessionEvent::OpFailed { op, .. } if *op == FollowUp::Refs.label()),
            move || {
                std::fs::remove_file(&reference).unwrap();
                std::fs::remove_file(&object).unwrap();
            },
        );
    });
}

/// The refs read behind a commit failed, so the record still holds HEAD
/// where it was before the commit: a walk ahead of the status read would
/// hang the worktree row on the commit before, and the status read, finding
/// the row as told, would walk nothing more. The walk waits for that read,
/// which puts HEAD where the commit left it — one walk, the worktree row on
/// the commit's lane.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_whose_refs_read_failed_is_walked_behind_its_status() {
    let mut repo = with_changes_left_over();
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let release = status_parked(&sink, &session).await;
    fail_the_next_refs_read(&sink, &repo);
    let id = commit(&session, "the staged file", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    sink.wait_for("the refs read behind the commit failed", |evs| {
        evs.iter()
            .any(
                |e| matches!(e, SessionEvent::OpFailed { op, .. } if *op == FollowUp::Refs.label()),
            )
            .then_some(())
    })
    .await;
    release.send(()).expect("the parked status is waiting");
    assert_eq!(write_settled(&sink, id).await, [FollowUp::Refs]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    {
        let events = sink.events.lock().unwrap();
        let answered = answer(&events, id);
        let read = told_behind(&events, id)
            .into_iter()
            .find(|told| told.looked > answered.reads_from)
            .expect("the status read behind the commit");
        let settled = settled_at(&events, id);
        let walked: Vec<(usize, Vec<(String, u16)>)> = events
            .iter()
            .enumerate()
            .take(settled)
            .skip(answered.at)
            .filter_map(|(at, e)| match e {
                SessionEvent::LogReplaced { rows, .. } => Some((
                    at,
                    rows.iter()
                        .map(|row| (row.oid_hex.clone(), row.node_lane))
                        .collect(),
                )),
                _ => None,
            })
            .collect();
        assert_eq!(walked.len(), 1, "one walk behind the commit: {walked:?}");
        let (at, rows) = &walked[0];
        assert!(
            rows.len() >= 2 && is_worktree_row(&rows[0].0) && rows[1].0 == head,
            "the worktree row over the commit: {rows:?}"
        );
        assert_eq!(rows[1].1, rows[0].1, "and on its lane: {rows:?}");
        assert!(*at > read.at, "walked behind the status read");
    }
    session.close_and_wait().await;
}

/// An amend leaves the index HEAD's as a commit does, and is told the same
/// way — also where it took nothing but the message, and the rows stand as
/// they were.
#[tokio::test(flavor = "multi_thread")]
async fn an_amend_is_told_as_a_commit_is() {
    let mut repo = with_changes_left_over();
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let id = commit(&session, "root, and the staged file", amend());
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    let message_only = commit(&session, "root, reworded", amend());
    assert_eq!(write_settled(&sink, message_only).await, []);
    {
        let events = sink.events.lock().unwrap();
        for (write, what) in [(id, "the staged file"), (message_only, "the message alone")] {
            let answered = answer(&events, write);
            let told = told_behind(&events, write);
            let ahead = told.first().expect("a status behind the amend");
            assert_eq!(
                (ahead.looked, ahead.head_seq),
                (answered.reads_from, answered.head_seq),
                "the amend of {what} told what it left at its answer"
            );
            assert_eq!(ahead.rows(), [".M root.txt", "?? loose.txt"], "{what}");
            let read = told
                .iter()
                .find(|told| told.looked > answered.reads_from)
                .expect("the status read behind the amend");
            assert_eq!(read.rows(), ahead.rows(), "{what}");
        }
    }
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "root, reworded");
    session.close_and_wait().await;
}

/// Whether the write under `id` told a status at its answer: one stamped
/// with the write's own end, which no read's stamp is.
fn told_ahead(events: &[SessionEvent], id: OperationId) -> bool {
    let end = answer(events, id).reads_from;
    statuses(events).iter().any(|told| told.looked == end)
}

/// Saving HEAD's message is the same `git commit --amend` over the index —
/// what is staged goes into HEAD with the words — and is told as a commit
/// is: the staged row gone at git's answer, the rest as it was.
#[tokio::test(flavor = "multi_thread")]
async fn a_reword_of_head_is_told_as_a_commit_is() {
    let mut repo = with_changes_left_over();
    let head = repo.git(&["rev-parse", "HEAD"]);
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let id = session
        .reword(oid(&head), "root, reworded".into())
        .expect("the reword was accepted");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        let answered = answer(&events, id);
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 2, "the one worked out, then the one read");
        assert_eq!(
            (told[0].looked, told[0].head_seq),
            (answered.reads_from, answered.head_seq),
            "told at its answer, under the two numbers it carries"
        );
        assert_eq!(told[0].rows(), [".M root.txt", "?? loose.txt"]);
        assert!(told[1].looked > answered.reads_from);
        assert_eq!(told[1].rows(), told[0].rows(), "git's own word is the same");
        assert_eq!(
            graphs(&events, answered.at)
                .iter()
                .filter(|(at, _)| *at < settled_at(&events, id))
                .count(),
            1,
            "a read that finds what was told asks for no second walk"
        );
    }
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "root, reworded");
    assert_eq!(
        repo.git(&["show", "--name-only", "--format=", "HEAD"])
            .lines()
            .collect::<Vec<_>>(),
        ["new.txt", "root.txt"],
        "the staged file went into HEAD with the words"
    );
    session.close_and_wait().await;
}

/// A reword further down the history replays the commits above it, which
/// no row foretells: nothing is told ahead of the reads behind it.
#[tokio::test(flavor = "multi_thread")]
async fn a_reword_that_replays_tells_nothing_ahead() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    let first = repo.commit_file_id("root.txt", "0\n", "root");
    repo.commit_file("above.txt", "1\n", "above");
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let id = session
        .reword(oid(&first), "root, reworded".into())
        .expect("the reword was accepted");
    assert_eq!(write_answer(&sink, id).await, None, "the replay landed");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        assert!(!told_ahead(&events, id), "a replay tells nothing ahead");
    }
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", "HEAD~1"]),
        "root, reworded"
    );
    session.close_and_wait().await;
}

/// The commit that ends a merge takes the merge with it, which no row says:
/// nothing is told ahead, and the read behind the write tells the tree and
/// the operation gone.
#[tokio::test(flavor = "multi_thread")]
async fn the_commit_that_ends_a_merge_waits_for_its_status() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.git(&["switch", "-c", "side"]);
    repo.commit_file("side.txt", "s\n", "side work");
    repo.git(&["switch", "main"]);
    repo.commit_file("main.txt", "m\n", "main work");
    repo.git(&["merge", "--no-ff", "--no-commit", "side"]);
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let id = commit(&session, "merge side", CommitOptions::default());
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        assert!(!told_ahead(&events, id), "nothing follows from a merge");
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 1, "the one status read behind the commit");
        assert_eq!(told[0].rows(), [] as [&str; 0]);
    }
    session.close_and_wait().await;
}

/// A commit git refused left nothing: nothing is told ahead of the read.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_commit_tells_nothing_ahead() {
    let repo = with_everything_staged();
    repo.write_hook("pre-commit", "exit 1\n");
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let id = commit(&session, "refused by the hook", CommitOptions::default());
    assert!(write_answer(&sink, id).await.is_some(), "the hook said no");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        assert!(!told_ahead(&events, id));
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 1, "the one status read behind the refusal");
        assert_eq!(told[0].rows(), ["A. new.txt"]);
    }
    session.close_and_wait().await;
}

/// The status read behind a stage failed, so the status told last does not
/// say what the stage staged: worked out from it, the commit behind would
/// tell the committed file back as unstaged. Nothing is told ahead, and the
/// read behind the commit tells the tree.
///
/// `git status` refuses a `status.aheadBehind` it cannot read as a boolean;
/// `git add` reads none of it.
#[tokio::test(flavor = "multi_thread")]
async fn a_commit_behind_a_stage_whose_status_failed_waits_for_its_status() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("root.txt", "edited\n");
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    repo.git(&["config", "status.aheadBehind", "bogus"]);
    let stage = session
        .stage_paths(vec!["root.txt".to_string()])
        .expect("the stage was accepted");
    assert_eq!(
        write_settled(&sink, stage).await,
        [FollowUp::Status],
        "the status behind the stage did not land"
    );
    repo.git(&["config", "--unset", "status.aheadBehind"]);

    let id = commit(&session, "the staged edit", CommitOptions::default());
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        assert!(
            !told_ahead(&events, id),
            "the status told last looked before the stage ended"
        );
        let told = told_behind(&events, id);
        assert_eq!(told.len(), 1, "the one status read behind the commit");
        assert_eq!(told[0].rows(), [] as [&str; 0]);
    }
    session.close_and_wait().await;
}

/// A stage another session on the same worktree made — a second tab — does
/// not fence this one's reads, and the status this session told last does
/// not say what it staged: worked out from it, the commit behind would tell
/// the committed file back as unstaged. The read the other session's
/// landing asks of this one is held at its spawn, so that status is still
/// the last told when the commit answers. Nothing is told ahead
/// (`RepoSession::read_again`), and the read behind the commit tells the
/// tree.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_behind_another_sessions_stage_waits_for_its_status() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("root.txt", "edited\n");
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;
    let (other_sink, other) = opened(&repo).await;
    quiet(&other_sink, &other).await;

    let release = sink.park_once("the read the other's stage asks for", is_status_spawn);
    let stage = other
        .stage_paths(vec!["root.txt".to_string()])
        .expect("the stage was accepted");
    assert_eq!(write_settled(&other_sink, stage).await, []);
    sink.wait_for("this session was asked to read again", |evs| {
        evs.iter().any(is_status_spawn).then_some(())
    })
    .await;

    let id = commit(&session, "the other's stage", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    release.send(()).expect("the parked read is waiting");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        assert!(
            !told_ahead(&events, id),
            "the status told last looked before the other session's stage"
        );
        let told = told_behind(&events, id);
        assert_eq!(
            told.last().map(Told::rows),
            Some(Vec::new()),
            "the read behind the commit"
        );
    }
    other.close_and_wait().await;
    session.close_and_wait().await;
}

/// A fetch that brought nothing reads no tree behind itself, so the status
/// told last is older than its end — and still says what is staged: the
/// commit behind it is told as any.
#[tokio::test(flavor = "multi_thread")]
async fn a_commit_behind_a_fetch_that_brought_nothing_is_still_told() {
    let (_bare, mut repo) = origin_and_clone();
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let fetch = session.fetch(None).expect("the fetch was accepted");
    assert_eq!(write_settled(&sink, fetch).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        assert_eq!(
            told_behind(&events, fetch).len(),
            0,
            "no status was read behind the fetch"
        );
    }

    let id = commit(&session, "behind the fetch", CommitOptions::default());
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        assert!(
            told_ahead(&events, id),
            "worked out from the status told before the fetch"
        );
    }
    session.close_and_wait().await;
}

/// A status read that looked before the commit ended and comes back after
/// it is not told behind the status the commit left — it would draw the
/// committed row back. The read is parked past the last place it asks
/// whether a write ended behind it, reading the line-ending marks of a file
/// saved over since the last status.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_status_that_looked_before_the_commit_is_not_told_behind_what_it_left() {
    let repo = with_everything_staged();
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    repo.write_file("root.txt", "saved over\n");
    let release = sink.park_once("the marks of the parked status", is_marks_read);
    session.refresh_status();
    sink.wait_for("the status read reached its marks", move |evs| {
        evs.iter().any(is_marks_read).then_some(())
    })
    .await;

    let id = commit(&session, "under the parked read", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    release.send(()).expect("the parked read is waiting");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    bounded(
        "the parked read and the one behind the commit",
        session.wait_for_snapshot_reads(),
    )
    .await;
    {
        let events = sink.events.lock().unwrap();
        let answered = answer(&events, id);
        let told = told_behind(&events, id);
        assert!(
            told.iter().all(|told| told.looked >= answered.reads_from),
            "a status from before the commit's end was told behind it: {:?}",
            told.iter()
                .map(|told| (told.looked, told.rows()))
                .collect::<Vec<_>>()
        );
        assert_eq!(told.len(), 2, "the one worked out, then the one read");
        assert_eq!(told[0].rows(), [] as [&str; 0]);
        assert_eq!(told[1].rows(), [".M root.txt"]);
    }
    session.close_and_wait().await;
}

/// A status read past its last look at the writes before a commit ended,
/// held where it reports HEAD — ahead of what it records beside it, the
/// worktree row among them — while the commit tells what it left. Recorded
/// after the commit's, the row from before the commit would stand until the
/// read behind the commit turned it back and walked a second time. Nothing
/// is recorded by a read a write has ended behind (`publish_status`): the
/// one walk behind the commit stands.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_status_that_looked_before_the_commit_records_nothing_over_what_it_left() {
    let mut repo = with_everything_staged();
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    // HEAD moved outside onto a commit of the same tree, rows and all as
    // they were: the next status read reports it from inside its pass.
    let tree = repo.git(&["rev-parse", "HEAD^{tree}"]);
    let outside = repo.git(&["commit-tree", &tree, "-p", "HEAD", "-m", "outside"]);
    repo.git(&["update-ref", "refs/heads/main", &outside]);
    let reported = move |e: &SessionEvent| {
        matches!(e, SessionEvent::HeadObserved { head, .. }
            if head.oid.map(|oid| oid.to_hex()) == Some(outside.clone()))
    };
    let release = sink.park_once("the status read at its HEAD", reported.clone());
    session.refresh_status();
    sink.wait_for("the status read reported HEAD", move |evs| {
        evs.iter().any(&reported).then_some(())
    })
    .await;

    let id = commit(&session, "everything", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    sink.wait_for("the commit told what it left", |evs| {
        told_ahead(evs, id).then_some(())
    })
    .await;
    release.send(()).expect("the parked read is waiting");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    bounded(
        "the parked read and the one behind the commit",
        session.wait_for_snapshot_reads(),
    )
    .await;
    {
        let events = sink.events.lock().unwrap();
        let answered = answer(&events, id);
        let settled = settled_at(&events, id);
        // Counted by the walk's spawns: a walk that lays the same graph
        // swaps nothing in.
        let walks = events[answered.at..settled]
            .iter()
            .filter(|e| is_walk_spawn(e))
            .count();
        assert_eq!(walks, 1, "one walk behind the commit");
        let walked = graphs(&events, answered.at);
        assert!(
            walked
                .iter()
                .all(|(_, rows)| !rows.iter().any(|oid| is_worktree_row(oid))),
            "and no worktree row on the graph: {walked:?}"
        );
    }
    session.close_and_wait().await;
}

/// The graph's walk, told from the other `git log`s by its order.
fn is_walk_spawn(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. } if display.starts_with("git log -z --date-order"))
}

/// A stage pressed over the rows a commit left, while the reads behind the
/// commit are still out: the status those reads bring holds the same rows
/// as the one worked out, and both are stamped below the stage's end — a
/// consumer that moved the row at the press goes on drawing it moved over
/// each of them (the app's `StagePress::draws_over`). The status read
/// behind the stage is the first stamped at its end or above.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stage_behind_a_commit_is_answered_by_no_status_of_the_commits() {
    let repo = with_changes_left_over();
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let release = status_parked(&sink, &session).await;
    let id = commit(&session, "the staged file", CommitOptions::default());
    assert_eq!(write_answer(&sink, id).await, None);
    // The reads behind the commit have begun, so the stage joins no burst
    // of its: it waits for them.
    let from = answer(&sink.events.lock().unwrap(), id).at;
    sink.wait_for("the reads behind the commit began", move |evs| {
        evs[from..].iter().any(is_refs_spawn).then_some(())
    })
    .await;
    let stage = session
        .stage_paths(vec!["root.txt".to_string()])
        .expect("the stage was accepted");
    release.send(()).expect("the parked status is waiting");
    assert_eq!(write_settled(&sink, id).await, []);
    assert_eq!(write_settled(&sink, stage).await, [], "every read landed");
    {
        let events = sink.events.lock().unwrap();
        let (committed, staged) = (answer(&events, id), answer(&events, stage));
        let told = told_behind(&events, id);
        let rows: Vec<Vec<String>> = told.iter().map(Told::rows).collect();
        assert_eq!(
            rows,
            [
                vec![".M root.txt", "?? loose.txt"],
                vec![".M root.txt", "?? loose.txt"],
                vec!["M. root.txt", "?? loose.txt"],
            ],
            "worked out, read behind the commit, read behind the stage"
        );
        assert_eq!(told[0].looked, committed.reads_from);
        assert!(
            told[1].looked > told[0].looked && told[1].looked < staged.reads_from,
            "git's own word behind the commit is newer, and still below the stage's end"
        );
        assert!(told[2].looked >= staged.reads_from);
    }
    session.close_and_wait().await;
}
