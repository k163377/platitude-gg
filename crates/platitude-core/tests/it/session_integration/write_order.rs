//! Two sessions, one working tree (`session::write_order`): the order
//! their writes run in, what each may read while the other is writing,
//! and what a session closed mid-write lets go of.
//!
//! **Every step is held open by a barrier rather than timed** — a write
//! by a hook git waits in, a stage by a clean filter, a read by a parked
//! sink delivery — so "this had provably not happened yet" is arranged
//! rather than raced. Where the barrier *is* the arrangement, it is read
//! back as well: a filter that silently does not hold makes a test green
//! for the wrong reason.

use std::sync::Arc;

use crate::support::session::{opened, pass_of, write_answer, write_settled};
use crate::support::{TestRepo, barrier_filter, barrier_hook};
use platitude_core::session::{RefreshOutcome, RepoSession, SessionEvent};

/// A repository with one commit, a file staged for a second, and a hook
/// that holds that second one until the answered path is written.
fn held_at_the_first_commit() -> (TestRepo, std::path::PathBuf) {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let release = repo.path.join("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));
    (repo, release)
}

/// The order a reopened tab joins is the tree's, not its own.
///
/// The closed session's commit is still inside git when the new session
/// is opened and asks for a branch, so both writes are outstanding at
/// once and nothing but the order can separate them. **The witness is the
/// repository**: a branch cut at HEAD names the HEAD it found, so a
/// branch on the root commit would say the reopened tab went first.
///
/// The two ends of the quit gate are read here too — the closed session's
/// loop is what `Hub::parked_writes` holds, and the pending count on each
/// session is what `Hub::writes_settled` asks.
#[tokio::test(flavor = "multi_thread")]
async fn a_reopened_tab_writes_behind_the_close_it_found_running() {
    let (mut repo, release) = held_at_the_first_commit();
    let root = repo.git(&["rev-parse", "HEAD"]);

    let (closing_sink, closing) = opened(&repo).await;
    let commit = closing
        .commit(
            "held by the hook".into(),
            platitude_core::commit::CommitOptions::default(),
        )
        .expect("the commit was accepted");
    closing_sink
        .wait_for("the commit reached git", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == commit))
                .then_some(())
        })
        .await;

    // The tab goes; the commit does not.
    closing.close();

    // And comes back: a second session on the same index, with a queue of
    // its own and no knowledge of the first.
    let (reopened_sink, reopened) = opened(&repo).await;
    let branch = reopened
        .create_branch("after-the-close".into(), None, false)
        .expect("the branch was accepted");

    // Only now may the commit finish.
    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(
        write_answer(&closing_sink, commit).await,
        None,
        "the commit the close let run on landed"
    );
    assert_eq!(
        write_answer(&reopened_sink, branch).await,
        None,
        "and so did the branch the reopened tab asked for"
    );

    let head = repo.git(&["rev-parse", "HEAD"]);
    assert_ne!(head, root, "the commit moved HEAD");
    assert_eq!(
        repo.git(&["rev-parse", "refs/heads/after-the-close"]),
        head,
        "the reopened tab's branch was cut from the commit the closed one made, \
         so it ran behind it"
    );

    // What the window's quit gate waits for: the reopened session's own
    // write, and the loop the closed one left running.
    write_settled(&reopened_sink, branch).await;
    reopened.close();
    for (whose, session) in [("the closed", &closing), ("the reopened", &reopened)] {
        let ended = crate::support::wait::bounded(
            "the write loop ends with its queue drained",
            session.take_write_join().expect("the loop's task"),
        )
        .await;
        assert!(ended.is_ok(), "{whose} loop ended cleanly: {ended:?}");
        assert_eq!(
            session.local_writes_pending(),
            0,
            "{whose} session has nothing left for the gate to wait on"
        );
    }
}

/// The reopened tab reads the tree only when nobody is writing it — and
/// does not have to wait for a tick of its own clock to read it once the
/// write it was keeping out of lands.
///
/// Without the first half a status read taken between a write's steps is
/// published as where the repository stands, which is how a freshly
/// opened tab offers to abort an operation that is still being done.
/// Without the second half that reading stands until the poll timer comes
/// round again.
#[tokio::test(flavor = "multi_thread")]
async fn a_reopened_tab_keeps_out_of_the_tree_and_is_told_when_it_is_free() {
    let (mut repo, release) = held_at_the_first_commit();

    let (closing_sink, closing) = opened(&repo).await;
    let commit = closing
        .commit(
            "held by the hook".into(),
            platitude_core::commit::CommitOptions::default(),
        )
        .expect("the commit was accepted");
    closing_sink
        .wait_for("the commit reached git", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == commit))
                .then_some(())
        })
        .await;
    closing.close();

    let (reopened_sink, reopened) = opened(&repo).await;
    // Taken before the hook is let go and judged after it: an assertion
    // in between would leave the hook holding git for its whole cap on a
    // failing run, and the suite waiting on it.
    let mid_write = crate::support::wait::bounded(
        "the tick taken over another session's write answers",
        reopened.refresh_poll_tracked().outcome(),
    )
    .await;

    std::fs::write(&release, b"go").expect("release the hook");
    assert_eq!(
        mid_write,
        RefreshOutcome::WriteBusy,
        "a tick taken while another session's write is in the tree reads nothing"
    );
    assert_eq!(
        write_answer(&closing_sink, commit).await,
        None,
        "the commit landed"
    );

    // Nothing here asks the reopened session for anything: the session
    // that finished the write is what tells it to look again, and the
    // commit is one it has never seen.
    let landed = repo.git(&["rev-parse", "HEAD"]);
    reopened_sink
        .wait_for(
            "the reopened tab reads the commit it was kept out of",
            |evs| {
                evs.iter()
                    .filter_map(|e| match e {
                        SessionEvent::HeadObserved { head, .. } => head.oid,
                        _ => None,
                    })
                    .any(|oid| oid.to_string() == landed)
                    .then_some(())
            },
        )
        .await;

    reopened.close();
}

/// What the page drew is given back by the close, not by the write the
/// close let run on.
///
/// A tab released mid-write keeps its session alive to the end of that
/// write (`Hub::park_writes_of`), and on a repository the size of the
/// budget's the drawn graph and the refs snapshot are the largest things
/// in the process — held for as long as git takes, they would make a
/// release cost memory rather than give it back. The measurement is the
/// session's own report, which is what the memory budget is read off.
#[tokio::test(flavor = "multi_thread")]
async fn a_close_lets_go_of_the_screens_copy_while_its_write_runs() {
    let (mut repo, _head) = crate::support::session::scenario();
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let release = repo.path.join("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));

    let (sink, session) = opened(&repo).await;
    // The parts below are what the opening's reads fill, so "let go of
    // it" says nothing until the picture has arrived.
    sink.opening_settled(&session).await;
    sink.wait_for("a graph pass landed", |evs| {
        evs.iter().find_map(pass_of).map(|_| ())
    })
    .await;
    assert!(
        drawn_bytes(&session) > 0,
        "the opening filled what the page draws from: {:?}",
        session.heap_report()
    );

    let commit = session
        .commit(
            "held by the hook".into(),
            platitude_core::commit::CommitOptions::default(),
        )
        .expect("the commit was accepted");
    sink.wait_for("the commit reached git", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == commit))
            .then_some(())
    })
    .await;

    session.close();
    // Read with git still inside the hook, so this is the session the
    // close kept alive rather than one that has already finished.
    let left = session.heap_report();
    assert_eq!(
        drawn_bytes(&session),
        0,
        "the close gave the page's copy back, buckets included: {left:?}"
    );
    assert!(
        left.iter()
            .filter(|part| drawn(part))
            .all(|part| part.count == 0),
        "and nothing it drew is still counted: {left:?}"
    );

    std::fs::write(&release, b"go").expect("release the hook");
    assert_eq!(
        write_answer(&sink, commit).await,
        None,
        "and the write it was holding the session open for still landed"
    );
    let ended = crate::support::wait::bounded(
        "the write loop ends with its queue drained",
        session.take_write_join().expect("the loop's task"),
    )
    .await;
    assert!(ended.is_ok(), "the loop ended cleanly: {ended:?}");
}

/// Whether a part of the memory report holds the page's copy of the
/// repository — what [`RepoSession::close`] gives back.
///
/// Named one by one rather than taken as the whole report: the parts a
/// close deliberately keeps (the pending line-ending marks) would
/// otherwise make this pass or fail on them too.
///
/// **The tag index is counted but not weighed.** Empty, it still costs
/// the `Arc` and the struct behind it, so its bytes never reach zero and
/// only the names it holds say whether a reading was let go.
fn drawn(part: &platitude_core::mem::Part) -> bool {
    matches!(
        part.name,
        "sent-rows"
            | "label-map"
            | "applied"
            | "graph-builder"
            | "publish-marks"
            | "refs-snapshot"
            | "remote-tag-index"
            | "eol-baselines"
    )
}

/// The weight of those parts, the tag index aside.
fn drawn_bytes(session: &Arc<RepoSession>) -> usize {
    session
        .heap_report()
        .iter()
        .filter(|part| drawn(part) && part.name != "remote-tag-index")
        .map(|part| part.bytes)
        .sum()
}

/// A write that lands while another session is already reading is still
/// read by it afterwards.
///
/// The tick a landed write sets off is refused by the single-flight slot
/// whenever that session is mid-read, and the read holding the slot began
/// before the write — so dropping the refused tick leaves the session
/// showing a repository that no longer exists until something else
/// happens to ask. What is asked for by a write is owed, not offered.
#[tokio::test(flavor = "multi_thread")]
async fn a_write_landing_inside_a_read_is_read_again_when_that_read_ends() {
    let (_origin, repo) = crate::support::remote::origin_and_clone();
    let (reader_sink, reader) = opened(&repo).await;
    reader_sink.opening_settled(&reader).await;
    let (writer_sink, writer) = opened(&repo).await;
    writer_sink.opening_settled(&writer).await;

    // The reader is held inside a read of its own, so the tick the write
    // sets off finds the slot taken. Parked from the sink's own delivery
    // — only this test's root future may wait for anything meanwhile.
    let (release, parked) = std::sync::mpsc::channel::<()>();
    reader_sink.hook_once(
        |e| matches!(e, SessionEvent::RefsLoaded { .. }),
        move || {
            let _ = parked.recv();
        },
    );
    reader.refresh_poll();

    let late = writer
        .create_branch("late".into(), None, false)
        .expect("the branch was accepted");
    write_settled(&writer_sink, late).await;
    // Strictly after the branch's own `tell_the_tree`: the loop cannot
    // start a second request until the first has given its place back
    // and told the tree, so the refused tick is provably behind us while
    // the reader is still parked. **A push, because it writes nothing
    // here** — a second local write would tell the tree again once this
    // test let the reader go, and would answer for the very wake the
    // test is here to miss.
    let pushed = writer
        .push(platitude_core::remote::PushSpec {
            remote: "origin".into(),
            local: "late".into(),
            remote_branch: "late".into(),
            set_upstream: false,
            force: platitude_core::remote::PushForce::None,
        })
        .expect("the push was accepted");
    writer_sink
        .wait_for("the writer's loop moved on to the next request", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == pushed))
                .then_some(())
        })
        .await;

    drop(release);
    // Nothing here asks the reader to look: the read it was owed is the
    // only thing that can put the branch on its side.
    reader_sink
        .wait_for(
            "the reader reads the branch made while it was busy",
            |evs| {
                evs.iter()
                    .filter_map(|e| match e {
                        SessionEvent::RefsLoaded { snapshot, .. } => Some(snapshot),
                        _ => None,
                    })
                    .any(|snapshot| snapshot.locals.iter().any(|b| b.short == "late"))
                    .then_some(())
            },
        )
        .await;

    reader.close();
    writer.close();
}

/// A read this session owes is taken when **its own** write ends too, not
/// only when a poll does.
///
/// The tail of a poll is one place a refused read can be served, and on
/// its own it is not enough. A session reading when the news arrives is
/// refused once for being busy, and refused again at that tail if a write
/// of its own has started meanwhile — and after that nothing would take
/// it: the write's own `tell_the_tree` speaks to the *other* sessions,
/// and a write that only moved the index reads no refs behind itself
/// ([`AfterWrite::Tree`]). The branch the other session made would stand
/// missing until something else happened to ask.
///
/// Every step is held open by a barrier rather than timed — the poll by a
/// parked delivery, the stage by a clean filter — and the poll's whole
/// task is waited out through the graph pass it holds, so the second
/// refusal has provably happened before the stage is let go.
#[tokio::test(flavor = "multi_thread")]
async fn a_read_owed_is_taken_when_this_session_s_own_write_ends() {
    let (_origin, mut repo) = crate::support::remote::origin_and_clone();
    let held = repo.path.join("stage-release");
    repo.git(&["config", "filter.hold.clean", &barrier_filter(&held)]);
    repo.git(&["config", "filter.hold.required", "true"]);
    repo.write_file(".gitattributes", "held.txt filter=hold\n");
    repo.write_file("held.txt", "content\n");
    // The barrier is the whole arrangement, so both halves of it are read
    // back: a filter that is not wired makes the stage instant and the
    // test green for the wrong reason.
    assert_eq!(
        repo.git(&["check-attr", "filter", "--", "held.txt"]),
        "held.txt: filter: hold",
        "the clean filter is wired to the path"
    );
    assert_eq!(
        repo.git(&["config", "--get", "filter.hold.clean"]),
        barrier_filter(&held),
        "and the filter itself is the barrier"
    );

    let (reader_sink, reader) = opened(&repo).await;
    reader_sink.opening_settled(&reader).await;
    let (writer_sink, writer) = opened(&repo).await;
    writer_sink.opening_settled(&writer).await;

    // (1) The reader is held inside a read of its own.
    let (release, parked) = std::sync::mpsc::channel::<()>();
    reader_sink.hook_once(
        |e| matches!(e, SessionEvent::RefsLoaded { .. }),
        move || {
            let _ = parked.recv();
        },
    );
    let polled = reader.refresh_poll_tracked();

    // (2) The news arrives and is refused for being busy. The push is the
    // witness that the branch's own `tell_the_tree` is behind us, and it
    // writes nothing here, so it wakes nobody itself.
    let late = writer
        .create_branch("late".into(), None, false)
        .expect("the branch was accepted");
    write_settled(&writer_sink, late).await;
    let pushed = writer
        .push(platitude_core::remote::PushSpec {
            remote: "origin".into(),
            local: "late".into(),
            remote_branch: "late".into(),
            set_upstream: false,
            force: platitude_core::remote::PushForce::None,
        })
        .expect("the push was accepted");
    writer_sink
        .wait_for("the writer's loop moved on to the next request", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == pushed))
                .then_some(())
        })
        .await;

    // (3) A write of the reader's own, held inside git by the filter.
    let staged = reader
        .stage_paths(vec!["held.txt".into()])
        .expect("the stage was accepted");
    reader_sink
        .wait_for("the stage reached git", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == staged))
                .then_some(())
        })
        .await;

    // (4) The poll ends and tries what it owes — refused again, because
    // the stage is provably still inside the filter. Waiting the pass out
    // waits the poll's whole task out, that try included.
    drop(release);
    let parked_poll =
        crate::support::wait::bounded("the parked poll answers", polled.outcome()).await;
    assert!(
        parked_poll.landed(),
        "the poll that was parked is the one that read, not one the slot turned away: {parked_poll:?}"
    );
    crate::support::wait::bounded(
        "the poll's task ends, so the read it was owed has been refused twice",
        reader.wait_for_graph_passes(),
    )
    .await;

    // (5) Only the reader's own write is left to take it. The stage is
    // still inside git at this point — the premise of everything above,
    // so it is read rather than assumed.
    assert_eq!(
        reader_sink.count(|e| matches!(e, SessionEvent::WriteFinished { id, .. } if *id == staged)),
        0,
        "the stage is still held by the filter"
    );
    std::fs::write(&held, b"go").expect("release the filter");
    assert_eq!(
        write_answer(&reader_sink, staged).await,
        None,
        "the stage landed, so the barrier was a barrier and not a broken filter"
    );
    reader_sink
        .wait_for("the reader reads the branch it was twice refused", |evs| {
            evs.iter()
                .filter_map(|e| match e {
                    SessionEvent::RefsLoaded { snapshot, .. } => Some(snapshot),
                    _ => None,
                })
                .any(|snapshot| snapshot.locals.iter().any(|b| b.short == "late"))
                .then_some(())
        })
        .await;

    reader.close();
    writer.close();
}

/// What a close gave back stays given back, even to a read that was
/// already in flight when it happened.
///
/// A refs pass stores its snapshot as it sends `RefsLoaded` and installs
/// its chips **after** — so a close landing between the two would find
/// the chips written back into the `Shared` it had just emptied, and the
/// memory held for as long as the write keeping the session alive takes.
/// The hook is what puts the close exactly there.
#[tokio::test(flavor = "multi_thread")]
async fn a_read_in_flight_at_the_close_does_not_refill_what_it_gave_back() {
    let (mut repo, _head) = crate::support::session::scenario();
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let release = repo.path.join("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));

    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    let commit = session
        .commit(
            "held by the hook".into(),
            platitude_core::commit::CommitOptions::default(),
        )
        .expect("the commit was accepted");
    sink.wait_for("the commit reached git", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == commit))
            .then_some(())
    })
    .await;

    // Closed from inside the delivery of the read's own event, which is
    // the one moment the snapshot is already stored and the chips are
    // not. Nothing is parked here — the hook closes and returns.
    let closing = Arc::clone(&session);
    let closed_inside = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let closed_here = Arc::clone(&closed_inside);
    sink.hook_once(
        |e| matches!(e, SessionEvent::RefsLoaded { .. }),
        move || {
            closing.close();
            closed_here.store(true, std::sync::atomic::Ordering::SeqCst);
        },
    );
    // The refs have to have moved, or the pass answers with the snapshot
    // it already published and returns before it ever reaches the chips
    // (`publish_refs`, the join-key fast path) — a read that stores
    // nothing proves nothing here.
    repo.git(&["branch", "moved-outside"]);
    session.refresh_refs();
    // The read's own completion, not its event: the sink records an event
    // before it runs the hook, so waiting on the event would judge the
    // report before the close inside it has even happened.
    crate::support::wait::bounded(
        "the refs pass that carries the close",
        session.wait_for_snapshot_reads(),
    )
    .await;
    assert!(
        closed_inside.load(std::sync::atomic::Ordering::SeqCst),
        "the close ran inside the read's own delivery, which is the whole arrangement"
    );

    let left = session.heap_report();
    assert_eq!(
        drawn_bytes(&session),
        0,
        "the read that was in flight put nothing back: {left:?}"
    );
    assert!(
        left.iter()
            .filter(|part| drawn(part))
            .all(|part| part.count == 0),
        "and counted nothing back either: {left:?}"
    );

    std::fs::write(&release, b"go").expect("release the hook");
    assert_eq!(
        write_answer(&sink, commit).await,
        None,
        "the write that kept the session alive still landed"
    );
    let ended = crate::support::wait::bounded(
        "the write loop ends with its queue drained",
        session.take_write_join().expect("the loop's task"),
    )
    .await;
    assert!(ended.is_ok(), "the loop ended cleanly: {ended:?}");
    assert_eq!(
        drawn_bytes(&session),
        0,
        "and nothing behind it filled the report again: {:?}",
        session.heap_report()
    );
}

/// A write whose far half reaches the network still takes its turn for
/// the half that is here.
///
/// The composite deletes are supervised as remote writes — a budget, and
/// a token the close cancels — but they delete a ref in this copy before
/// they reach for the other end, and that half must not overtake a
/// rename accepted before it. **The witness is which of the two git
/// refused**: run in the order they were asked, the rename finds its
/// branch and the delete does not.
#[tokio::test(flavor = "multi_thread")]
async fn a_composite_delete_takes_its_turn_for_the_half_that_is_here() {
    let (_origin, mut repo) = crate::support::remote::origin_and_clone();
    repo.git(&["branch", "victim"]);
    repo.git(&["push", "origin", "victim"]);
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let release = repo.path.join("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));

    let (closing_sink, closing) = opened(&repo).await;
    let commit = closing
        .commit(
            "held by the hook".into(),
            platitude_core::commit::CommitOptions::default(),
        )
        .expect("the commit was accepted");
    closing_sink
        .wait_for("the commit reached git", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == commit))
                .then_some(())
        })
        .await;
    // Queued behind the held commit, and the whole of what the close is
    // not allowed to lose.
    let rename = closing
        .rename_branch("victim".into(), "survivor".into(), false)
        .expect("the rename was accepted");
    closing.close();

    let (reopened_sink, reopened) = opened(&repo).await;
    let removal = reopened
        .delete_branch_everywhere("victim".into(), "origin".into(), "victim".into(), false)
        .expect("the delete was accepted");

    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(
        write_answer(&closing_sink, rename).await,
        None,
        "the rename accepted before the close still found the branch it was given"
    );
    assert!(
        write_answer(&reopened_sink, removal).await.is_some(),
        "and the delete asked for afterwards found it already renamed"
    );
    assert!(
        repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/survivor"]),
        "the rename landed"
    );
    assert!(
        !repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/victim"]),
        "under the name it was asked for and not the one the delete named"
    );

    reopened.close();
    for session in [&closing, &reopened] {
        let ended = crate::support::wait::bounded(
            "the write loop ends with its queue drained",
            session.take_write_join().expect("the loop's task"),
        )
        .await;
        assert!(ended.is_ok(), "the loop ended cleanly: {ended:?}");
    }
}
