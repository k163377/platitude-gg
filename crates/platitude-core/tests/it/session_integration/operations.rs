//! One id from the press to the last publish: a write answers under the
//! id the queue handed back when it accepted it, whatever ran before or
//! after it, and everything said about it on the way carries the same id
//! (`platitude_core::operation`).

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{
    CaptureSink, PassDoors, open_with_doors, opened, write_answer, write_settled,
};
use crate::support::wait::bounded;
use platitude_core::commit::CommitOptions;
use platitude_core::identity::ConfigScope;
use platitude_core::session::{FollowUp, PassStep, RepoSession, SessionEvent};
use platitude_core::{OperationId, OperationKind};

/// A clean tree with one entry on the stash: a change to `f.txt`.
fn holding_one_stash() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.write_file("f.txt", "changed\n");
    repo.git(&["stash", "push", "-m", "held"]);
    repo
}

/// Where the first event `wanted` accepts stands.
fn index_of(events: &[SessionEvent], wanted: impl Fn(&SessionEvent) -> bool) -> usize {
    events
        .iter()
        .position(wanted)
        .expect("the session sent the event")
}

/// Every write event about `id`, in the order the session sent them.
fn lifecycle_of(events: &[SessionEvent], id: OperationId) -> Vec<&'static str> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::WriteStarted { id: got, .. } if *got == id => Some("started"),
            SessionEvent::WriteStopped { id: got, .. } if *got == id => Some("stopped"),
            SessionEvent::WriteFinished { id: got, .. } if *got == id => Some("finished"),
            SessionEvent::WriteSettled { id: got, .. } if *got == id => Some("settled"),
            _ => None,
        })
        .collect()
}

/// An apply pressed just before a pop of the same entry. Both answer
/// under the same kind; the apply lands, and the pop is refused, because
/// git will not restore over the change the apply just made (measured:
/// `Your local changes … would be overwritten by merge`, exit 1, entry
/// kept, nothing unmerged). Counted by turn, the apply's landing would be
/// taken for the pop's and the pop's refusal for somebody else's; by id,
/// each answer is its own, and nothing about the order is inferred.
#[tokio::test(flavor = "multi_thread")]
async fn a_pop_behind_an_apply_answers_under_its_own_id() {
    let mut repo = holding_one_stash();
    let (sink, session) = opened(&repo).await;

    let apply = session
        .stash_apply("stash@{0}".into())
        .expect("the session is open");
    let pop = session
        .stash_pop("stash@{0}".into())
        .expect("the session is open");
    assert_ne!(apply, pop, "two presses, two ids");

    assert_eq!(write_answer(&sink, apply).await, None, "the apply landed");
    let refusal = write_answer(&sink, pop).await.expect("the pop was refused");
    assert!(
        refusal.contains("would be overwritten"),
        "git's own reason travels under the pop's id: {refusal}"
    );
    assert!(
        write_settled(&sink, pop).await.is_empty(),
        "every read behind the pop landed"
    );

    let events = sink.events.lock().unwrap();
    let kinds: Vec<OperationKind> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::WriteFinished { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [OperationKind::Stash, OperationKind::Stash],
        "the kind alone could not have told the two answers apart"
    );
    drop(events);
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "the refused pop kept the entry"
    );
    session.close();
}

/// The boundaries come in order under each id — the start, git's
/// answer, and the settling of the reads it invalidated — and the queue
/// takes the next write only once the one in front is settled: the
/// stash listing the pop moved has published before the pop is settled,
/// and the apply was settled before the pop's git ran.
#[tokio::test(flavor = "multi_thread")]
async fn each_write_is_settled_before_the_next_one_starts() {
    let repo = holding_one_stash();
    let (sink, session) = opened(&repo).await;

    let apply = session
        .stash_apply("stash@{0}".into())
        .expect("the session is open");
    let pop = session
        .stash_pop("stash@{0}".into())
        .expect("the session is open");
    assert!(
        write_settled(&sink, pop).await.is_empty(),
        "every read behind the pop landed"
    );

    let events = sink.events.lock().unwrap();
    assert_eq!(
        lifecycle_of(&events, apply),
        ["started", "finished", "settled"]
    );
    assert_eq!(
        lifecycle_of(&events, pop),
        ["started", "finished", "settled"]
    );

    let apply_settled = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteSettled { id, .. } if *id == apply),
    );
    let pop_started = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == pop),
    );
    assert!(
        apply_settled < pop_started,
        "the queue is serial across the settled boundary"
    );

    let pop_finished = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteFinished { id, .. } if *id == pop),
    );
    let pop_settled = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteSettled { id, .. } if *id == pop),
    );
    assert!(
        events[pop_finished..pop_settled]
            .iter()
            .any(|e| matches!(e, SessionEvent::StashesLoaded { .. })),
        "the stash listing is one of the reads the settling waits for"
    );
    drop(events);
    session.close();
}

/// A pop is several commands — the status read that decides what a
/// non-zero exit meant, the pop itself, the status read that judges the
/// refusal — and every one of them stands in the log under the pop's id,
/// the apply's under the apply's. The log holds only what the user asked
/// for, and every such command names its write.
///
/// **The last assertion is the one the window is built on**: a command
/// the reader asked for that ran under no write would be a failure
/// nothing on screen answers for, and the page raises the log off the
/// operation's answer alone (rules-refs/app-ui.md — `CommandsModel`'s
/// failure touches no panel).
#[tokio::test(flavor = "multi_thread")]
async fn every_command_of_a_compound_write_carries_its_id() {
    let repo = holding_one_stash();
    let (sink, session) = opened(&repo).await;

    let apply = session
        .stash_apply("stash@{0}".into())
        .expect("the session is open");
    let pop = session
        .stash_pop("stash@{0}".into())
        .expect("the session is open");
    assert!(
        write_settled(&sink, pop).await.is_empty(),
        "every read behind the pop landed"
    );

    let events = sink.events.lock().unwrap();
    let pop_started = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == pop),
    );
    let pop_finished = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteFinished { id, .. } if *id == pop),
    );
    let under_the_pop: Vec<Option<OperationId>> = events[pop_started..pop_finished]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { operation, .. } => Some(*operation),
            _ => None,
        })
        .collect();
    assert!(
        under_the_pop.len() >= 2,
        "a pop is at least the status read and the pop: {under_the_pop:?}"
    );
    assert!(
        under_the_pop.iter().all(|op| *op == Some(pop)),
        "every command between the pop's boundaries is the pop's: {under_the_pop:?}"
    );
    let under = |id: OperationId| {
        events
            .iter()
            .filter(|e| {
                matches!(e, SessionEvent::CommandStarted { operation, .. } if *operation == Some(id))
            })
            .count()
    };
    assert!(under(apply) >= 2, "the apply is several commands too");
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e,
                SessionEvent::CommandStarted {
                    operation: None,
                    ..
                }
            ))
            .count(),
        0,
        "nothing in the log ran under no write"
    );
    drop(events);
    session.close();
}

/// The reads a listing asks for are the write's reads too. A working
/// copy taken in a terminal, standing on no branch, reaches the graph
/// only through the walk the worktree listing asks for — and the write
/// whose listing found it is not settled until that walk has answered.
/// The walk is parked from inside (`PassDoors`), so "not settled yet"
/// is read off a pass provably still out.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_is_not_settled_until_the_reads_its_listings_asked_for_have_landed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    let (sink, session, doors) = open_with_doors(&repo);
    sink.opened_graph(&session, 1).await;

    // Taken outside the window, with a commit of its own: no ref moves,
    // and the commit is in the graph only if the listing's walk names it.
    let elsewhere = tempfile::tempdir().expect("a folder for the copy");
    let spike = elsewhere.path().join("spike");
    repo.git(&["worktree", "add", "--detach", &spike.to_string_lossy()]);
    std::fs::write(spike.join("idea.txt"), "an idea\n").expect("write the spike's file");
    repo.git_in(&spike, &["add", "idea.txt"]);
    repo.git_in(&spike, &["commit", "-m", "spike: try the idea"]);

    // The next off-screen pass parks on its own task until let go — the
    // worker under it is why two are declared (`CaptureSink::hook_once`
    // says the same).
    let (parked, at_the_door) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    doors.run_inside_next_pass(PassStep::Swapping, move || {
        parked
            .send(())
            .expect("the test is waiting to hear the pass parked");
        held.recv().expect("the test lets the pass go");
    });

    // A write that moves nothing the graph draws: the tree and the refs
    // read the same, so the only walk behind it is the listing's.
    let id = session
        .set_identity(
            "Spike".into(),
            "spike@example.com".into(),
            ConfigScope::Local,
        )
        .expect("the session is open");
    bounded("the listing's walk reached the door", at_the_door)
        .await
        .expect("the pass reached the door");
    {
        let events = sink.events.lock().unwrap();
        let answered = index_of(
            &events,
            |e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id),
        );
        assert!(
            events[answered..]
                .iter()
                .any(|e| matches!(e, SessionEvent::WorktreesLoaded { .. })),
            "the listing has published behind the answer"
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, SessionEvent::WriteSettled { id: got, .. } if *got == id))
                .count(),
            0,
            "not settled while the walk the listing asked for is still out"
        );
    }
    release.send(()).expect("the pass is waiting");
    assert!(
        write_settled(&sink, id).await.is_empty(),
        "settled once the walk landed, with every read landed"
    );
    session.close();
}

/// A read the write asked for that fell over is the write's own news:
/// the settling names it under the write's id, where the failure itself
/// (`OpFailed`) carries none. The graph rebuild a commit asks for is made
/// to fail from inside (`PassDoors::fail_every_pass`).
#[tokio::test(flavor = "multi_thread")]
async fn a_read_that_failed_is_named_under_the_writes_own_id() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.write_file("f.txt", "changed\n");
    repo.git(&["add", "f.txt"]);
    let (sink, session, doors) = open_with_doors(&repo);
    // The root and the working-tree row.
    sink.opened_graph(&session, 2).await;

    doors.fail_every_pass(PassStep::Swapping);
    let id = session
        .commit("record it".into(), CommitOptions::default())
        .expect("the session is open");
    assert_eq!(
        write_answer(&sink, id).await,
        None,
        "git recorded the commit"
    );
    assert_eq!(
        write_settled(&sink, id).await,
        [FollowUp::Graph],
        "the rebuild the commit asked for did not land, and the write says so by id"
    );
    assert!(
        sink.count(|e| matches!(e, SessionEvent::OpFailed { op: "log", .. })) >= 1,
        "the failure itself still reaches the error surface"
    );
    session.close();
}

/// A repository with one commit and a change staged on top, opened with
/// the doors into its graph passes and the opening's own passes closed —
/// so the next pass to reach a door is the one the write asks for.
///
/// The commit those tests make moves both halves: the tree goes clean
/// and a ref moves, so the rebuild behind it is one the graph really
/// changes under.
async fn staged_with_doors() -> (TestRepo, Arc<CaptureSink>, Arc<RepoSession>, Arc<PassDoors>) {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.write_file("f.txt", "changed\n");
    repo.git(&["add", "f.txt"]);
    let (sink, session, doors) = open_with_doors(&repo);
    // The root and the working-tree row.
    sink.opened_graph(&session, 2).await;
    (repo, sink, session, doors)
}

/// Leaves a hook on the next graph pass to reach `at` that takes the
/// stream over from inside it — which is what makes the handover an
/// ordering: the hook runs on that pass's own task, so the pass it
/// displaces is exactly the one that was running.
///
/// `take_over` is what asks for the newer graph, and whatever it leaves
/// at the doors is left before the ask.
fn taken_over_from_inside(
    doors: &Arc<PassDoors>,
    session: &Arc<RepoSession>,
    at: PassStep,
    take_over: impl FnOnce(&Arc<PassDoors>, &Arc<RepoSession>) + Send + 'static,
) {
    // Weak, both of them: the session holds the doors, the doors hold
    // this hook, and a strong handle either way would be a ring nothing
    // ever drops.
    let doors_later = Arc::downgrade(doors);
    let session_later = Arc::downgrade(session);
    doors.run_inside_next_pass(at, move || {
        if let (Some(doors), Some(session)) = (doors_later.upgrade(), session_later.upgrade()) {
            take_over(&doors, &session);
        }
    });
}

/// A rebuild another ask took over is not an answer: the write behind it
/// is not settled until the ask that took it over has answered for the
/// graph.
///
/// **The one that took over is parked, and the test is what lets it
/// go** — so the picture it publishes cannot exist before the release,
/// and a settling recorded ahead of that picture is one that did not
/// wait for it. Read off the order the two were recorded in: when a
/// look taken while the pass is parked lands is a race with the reads
/// a settling makes either way
/// (core.md §「もう起きない」は完了後の件数・状態で証明する).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_waits_for_the_rebuild_that_took_its_own_over() {
    let (_repo, sink, session, doors) = staged_with_doors().await;
    let (reached, at_the_door) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    taken_over_from_inside(
        &doors,
        &session,
        PassStep::Swapping,
        move |doors, session| {
            // Parked where this pass is standing, so the ask that displaces
            // it cannot answer until the test lets it.
            doors.run_inside_next_pass(PassStep::Swapping, move || {
                reached
                    .send(())
                    .expect("the test is waiting for the replacement");
                held.recv().expect("the test lets the replacement go");
            });
            session.refresh_log();
        },
    );

    let id = session
        .commit("record it".into(), CommitOptions::default())
        .expect("the session is open");
    assert_eq!(
        write_answer(&sink, id).await,
        None,
        "git recorded the commit"
    );
    bounded("the replacement reached the door", at_the_door)
        .await
        .expect("the replacement parked");

    release.send(()).expect("the replacement is waiting");
    // Both halves are waited for before either is read, so what the
    // order below says is the order they happened in, whichever the
    // test looked for first.
    sink.wait_for("the graph the replacement walked", |evs| {
        let answered = evs
            .iter()
            .position(|e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id))?;
        evs[answered..]
            .iter()
            .any(|e| matches!(e, SessionEvent::LogReplaced { .. }))
            .then_some(())
    })
    .await;
    assert!(
        write_settled(&sink, id).await.is_empty(),
        "the replacement landed, and the write reads its answer as its own"
    );

    let events = sink.events.lock().unwrap();
    // The write's own pass published nothing — it was taken over — so
    // the one picture after its answer is the replacement's, and the
    // replacement stood at the door until the release above.
    let answered = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id),
    );
    let replaced = answered
        + index_of(&events[answered..], |e| {
            matches!(e, SessionEvent::LogReplaced { .. })
        });
    let settled = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteSettled { id: got, .. } if *got == id),
    );
    assert!(
        replaced < settled,
        "the write settled only once the parked rebuild had published, \
         which nothing before the release could have done"
    );
    drop(events);
    session.close();
}

/// What the write waits for is the queue's promise as much as its own
/// answer: the request behind it does not start until the rebuild that
/// took its own over has landed.
///
/// Parked and released the way the test above is, and for the same
/// reason — the picture the replacement publishes cannot exist before
/// the release, so a write that started ahead of it is one the queue
/// let through on a handover.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_queue_holds_the_next_write_until_the_replacement_has_landed() {
    let (_repo, sink, session, doors) = staged_with_doors().await;
    let (reached, at_the_door) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    taken_over_from_inside(
        &doors,
        &session,
        PassStep::Swapping,
        move |doors, session| {
            doors.run_inside_next_pass(PassStep::Swapping, move || {
                reached
                    .send(())
                    .expect("the test is waiting for the replacement");
                held.recv().expect("the test lets the replacement go");
            });
            session.refresh_log();
        },
    );

    let id = session
        .commit("record it".into(), CommitOptions::default())
        .expect("the session is open");
    // Queued behind the commit, before anything of the commit's has
    // settled: what the queue has to hold.
    let behind = session.stage_all().expect("the session is open");
    assert_eq!(
        write_answer(&sink, id).await,
        None,
        "git recorded the commit"
    );
    bounded("the replacement reached the door", at_the_door)
        .await
        .expect("the replacement parked");

    release.send(()).expect("the replacement is waiting");
    assert!(
        write_settled(&sink, id).await.is_empty(),
        "the replacement landed, and that is the commit's answer"
    );
    assert_eq!(
        write_answer(&sink, behind).await,
        None,
        "and the write behind it ran once it could"
    );

    let events = sink.events.lock().unwrap();
    let answered = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id),
    );
    let replaced = answered
        + index_of(&events[answered..], |e| {
            matches!(e, SessionEvent::LogReplaced { .. })
        });
    let started = index_of(
        &events,
        |e| matches!(e, SessionEvent::WriteStarted { id: got, .. } if *got == behind),
    );
    assert!(
        replaced < started,
        "the queue took the next write only once the rebuild the commit \
         was waiting on had published"
    );
    drop(events);
    session.close();
}

/// A replacement that failed is the write's own news: the graph never
/// caught up with the commit, and the settling says so under the
/// write's id.
///
/// The one that takes over restarts the stream, so the fault it walks
/// into is one the write's own pass cannot read: a rebuild asks for a
/// fault at `Swapping`, and only a stream asks at `Streaming`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_whose_replacement_failed_says_the_graph_did_not_land() {
    let (_repo, sink, session, doors) = staged_with_doors().await;
    taken_over_from_inside(&doors, &session, PassStep::Swapping, |doors, session| {
        doors.fail_every_pass(PassStep::Streaming);
        session.restart_log();
    });

    let id = session
        .commit("record it".into(), CommitOptions::default())
        .expect("the session is open");
    assert_eq!(
        write_answer(&sink, id).await,
        None,
        "git recorded the commit"
    );
    assert_eq!(
        write_settled(&sink, id).await,
        [FollowUp::Graph],
        "the rebuild that replaced the write's own failed, and the write says so by id"
    );
    session.close();
}

/// The acceptance boundary refuses: once the session is closed and
/// its write loop has ended, a request gets no id, because nothing
/// would ever answer one.
#[tokio::test(flavor = "multi_thread")]
async fn a_closed_session_accepts_nothing() {
    let repo = holding_one_stash();
    let (sink, session) = opened(&repo).await;
    let drop = session
        .stash_drop("stash@{0}".into())
        .expect("an open session accepts");
    assert!(write_settled(&sink, drop).await.is_empty());

    session.close();
    let ended = bounded(
        "the write loop ends",
        session.take_write_join().expect("the loop's task"),
    )
    .await;
    assert!(ended.is_ok(), "the loop ended without panicking: {ended:?}");

    assert_eq!(
        session.stash_drop("stash@{0}".into()),
        None,
        "nothing is accepted once the loop has ended"
    );
    assert_eq!(
        session.local_writes_pending(),
        0,
        "and nothing was counted for the request that was not taken"
    );
}
