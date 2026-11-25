//! The reads behind a burst of writes (`session::write`): a write waiting as
//! the one in front answers starts at once where its kind joins a burst
//! (`OperationKind::joins_a_burst`), and the reads are made once, behind the
//! last of them, for all of them.
//!
//! The burst is put together behind a commit its hook holds inside git, so
//! every write behind it is in the queue when the commit answers.

use std::sync::Arc;
use std::time::Duration;

use crate::support::remote::origin_and_clone;
use crate::support::session::{
    CaptureSink, open_with_doors, opened, scenario, write_answer, write_settled,
};
use crate::support::wait::bounded;
use crate::support::{TestRepo, barrier_filter, barrier_hook};
use platitude_core::OperationId;
use platitude_core::commit::CommitOptions;
use platitude_core::session::{
    self, FollowUp, PassStep, Recording, RelaidRow, RepoSession, SessionEvent,
};

/// The file a hook's barrier waits for, where no status reads it.
fn release_of(repo: &TestRepo) -> std::path::PathBuf {
    repo.path.join(".git").join("hook-release")
}

/// `repo` with a file staged for a commit its hook holds until the answered
/// path is written, and `loose` beside it for the writes queued behind.
fn held_at_a_commit(repo: &mut TestRepo, loose: &[&str]) -> std::path::PathBuf {
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    for path in loose {
        repo.write_file(path, "loose\n");
    }
    let release = release_of(repo);
    repo.write_hook("pre-commit", &barrier_hook(&release));
    release
}

/// One commit, and [`held_at_a_commit`] over it.
fn rooted(loose: &[&str]) -> (TestRepo, std::path::PathBuf) {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    let release = held_at_a_commit(&mut repo, loose);
    (repo, release)
}

/// Every read the opening started is over; answers where the events of what
/// follows start.
async fn quiet(sink: &CaptureSink, session: &Arc<RepoSession>) -> usize {
    sink.opening_settled(session).await;
    bounded(
        "the opening's other worktrees",
        session.wait_for_carried_pass(),
    )
    .await;
    bounded("the opening's reads", session.wait_for_snapshot_reads()).await;
    bounded("the opening's graph", session.wait_for_graph_passes()).await;
    sink.events.lock().unwrap().len()
}

fn held_commit(session: &Arc<RepoSession>) -> OperationId {
    session
        .commit("held by the hook".into(), CommitOptions::default())
        .expect("the commit was accepted")
}

fn stage(session: &Arc<RepoSession>, path: &str) -> OperationId {
    session
        .stage_paths(vec![path.to_string()])
        .expect("the stage was accepted")
}

/// Where the first event `wanted` accepts stands.
fn at(events: &[SessionEvent], wanted: impl Fn(&SessionEvent) -> bool) -> usize {
    events
        .iter()
        .position(wanted)
        .expect("the session sent the event")
}

fn started(events: &[SessionEvent], id: OperationId) -> usize {
    at(
        events,
        |e| matches!(e, SessionEvent::WriteStarted { id: got, .. } if *got == id),
    )
}

fn answered(events: &[SessionEvent], id: OperationId) -> usize {
    at(
        events,
        |e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id),
    )
}

fn settled(events: &[SessionEvent], id: OperationId) -> usize {
    at(
        events,
        |e| matches!(e, SessionEvent::WriteSettled { id: got, .. } if *got == id),
    )
}

/// The stamp under which reads speak for what the write under `id` left.
fn reads_from(events: &[SessionEvent], id: OperationId) -> u64 {
    events
        .iter()
        .find_map(|e| match e {
            SessionEvent::WriteFinished {
                id: got,
                reads_from,
                ..
            } if *got == id => Some(*reads_from),
            _ => None,
        })
        .expect("the write answered")
}

/// Where each status was told, and when it looked.
fn statuses(events: &[SessionEvent]) -> Vec<(usize, u64)> {
    events
        .iter()
        .enumerate()
        .filter_map(|(at, e)| match e {
            SessionEvent::StatusLoaded { looked, .. } => Some((at, *looked)),
            _ => None,
        })
        .collect()
}

/// Where the commit under `id` told the status it left — worked out at its
/// answer, not read (`session::foretold`): the one stamped with the write's
/// own end, which no read's stamp is.
fn told_by(events: &[SessionEvent], id: OperationId) -> Option<usize> {
    let end = reads_from(events, id);
    statuses(events)
        .into_iter()
        .find_map(|(at, looked)| (looked == end).then_some(at))
}

/// [`statuses`] without the one the commit under `commit` told at its
/// answer: the statuses that were read.
fn statuses_read(events: &[SessionEvent], commit: OperationId) -> Vec<(usize, u64)> {
    let told = told_by(events, commit);
    statuses(events)
        .into_iter()
        .filter(|(at, _)| Some(*at) != told)
        .collect()
}

/// Three files staged behind a commit: the four writes run back to back,
/// one status is read behind the last — one `git status` spawned, not one a
/// write thrown away — and every one of them settles behind that status,
/// which looked after each of them.
///
/// The commit tells the status it left at its own answer, before the first
/// stage starts and stamped below that stage's end: rows a stage press has
/// moved on the screen stay drawn over it (the app's `StagePress::draws_over`
/// measures the stamp), until the one read behind the burst.
#[tokio::test(flavor = "multi_thread")]
async fn writes_waiting_behind_one_another_are_read_behind_once() {
    let (mut repo, release) = rooted(&["a.txt", "b.txt", "c.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;
    session.set_recording(Recording::WithBackground);

    let commit = held_commit(&session);
    let stages = ["a.txt", "b.txt", "c.txt"].map(|path| stage(&session, path));
    std::fs::write(&release, b"go").expect("release the hook");

    let burst = [commit, stages[0], stages[1], stages[2]];
    for id in burst {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    // A read asked for in a fenced one's place would still be out.
    bounded(
        "the reads behind the burst",
        session.wait_for_snapshot_reads(),
    )
    .await;
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        let first = answered(events, commit);
        let last = answered(events, stages[2]);
        for pair in burst.windows(2) {
            assert!(
                answered(events, pair[0]) < started(events, pair[1]),
                "the writes ran in the order they were asked for"
            );
        }
        assert_eq!(
            events[first..]
                .iter()
                .filter(|e| is_status_spawn(e))
                .count(),
            1,
            "one status was spawned behind the commit's answer"
        );
        let told = told_by(events, commit).expect("the commit told the status it left");
        assert!(
            first < told && told < started(events, stages[0]),
            "at its answer, before the write behind it started"
        );
        assert!(
            reads_from(events, commit) < reads_from(events, stages[0]),
            "stamped below the end of the stage behind it"
        );
        let read = statuses_read(events, commit);
        assert_eq!(
            read.iter()
                .filter(|(at, _)| (first..last).contains(at))
                .count(),
            0,
            "nothing was read between the writes: {read:?}"
        );
        let behind: Vec<(usize, u64)> = read.into_iter().filter(|(at, _)| *at > last).collect();
        assert_eq!(behind.len(), 1, "one status behind the burst: {behind:?}");
        let (published, looked) = behind[0];
        for id in burst {
            assert!(
                settled(events, id) > published,
                "each write settled behind the read that answers for it"
            );
            assert!(
                looked >= reads_from(events, id),
                "and that read looked after the write"
            );
        }
    }
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "held by the hook");
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"])
            .lines()
            .count(),
        3,
        "all three were staged"
    );
    session.close_and_wait().await;
}

/// A write git refuses in the middle of a burst is one of the burst all the
/// same: its answer carries git's words, the writes behind it run, and it
/// settles with them behind the one read.
#[tokio::test(flavor = "multi_thread")]
async fn a_write_refused_inside_a_burst_settles_with_the_burst() {
    let (mut repo, release) = rooted(&["a.txt", "b.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;

    let commit = held_commit(&session);
    let before = stage(&session, "a.txt");
    let refused = stage(&session, "no-such-file.txt");
    let after = stage(&session, "b.txt");
    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(write_answer(&sink, before).await, None);
    assert!(
        write_answer(&sink, refused).await.is_some(),
        "git has no such path to add"
    );
    assert_eq!(write_answer(&sink, after).await, None);
    let burst = [commit, before, refused, after];
    for id in burst {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        let last = answered(events, after);
        let behind: Vec<(usize, u64)> = statuses_read(events, commit)
            .into_iter()
            .filter(|(at, _)| *at > answered(events, commit))
            .collect();
        assert_eq!(behind.len(), 1, "one status read for the burst: {behind:?}");
        assert!(behind[0].0 > last, "read behind the last write");
        for id in burst {
            assert!(settled(events, id) > behind[0].0);
        }
    }
    let staged = repo.git(&["diff", "--cached", "--name-only"]);
    assert_eq!(
        staged.lines().collect::<Vec<_>>(),
        ["a.txt", "b.txt"],
        "the writes either side of the refusal landed"
    );
    session.close_and_wait().await;
}

/// A write that reaches the network joins no burst: what the writes in front
/// of it left is read before it starts, so their result is on screen for
/// as long as the far end takes. The stage behind it joins its burst.
#[tokio::test(flavor = "multi_thread")]
async fn the_reads_are_made_before_a_write_that_reaches_the_network() {
    let (_bare, mut repo) = origin_and_clone();
    let release = held_at_a_commit(&mut repo, &["a.txt", "b.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;

    let commit = held_commit(&session);
    let before = stage(&session, "a.txt");
    let fetch = session.fetch(None).expect("the fetch was accepted");
    let after = stage(&session, "b.txt");
    std::fs::write(&release, b"go").expect("release the hook");

    for id in [commit, before, fetch, after] {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        assert!(
            answered(events, before) < settled(events, commit),
            "the stage joined the commit's burst"
        );
        for id in [commit, before] {
            assert!(
                settled(events, id) < started(events, fetch),
                "what the burst left was read before the fetch started"
            );
        }
        assert!(
            answered(events, after) < settled(events, fetch),
            "and the stage behind the fetch joined the fetch's burst"
        );
    }
    session.close_and_wait().await;
}

/// Past the ceiling the reads go in before the next write starts
/// (`session::unread_ceiling`). Mocked to nothing, every write is past it:
/// each is read behind before the next. mry's mocks are the thread's own,
/// hence the current-thread runtime (rules-refs/core.md「テストの差し替えは
/// mry」). The ceiling's own length is `session::write`'s unit test's, on a
/// paused clock.
#[tokio::test]
#[mry::lock(session::unread_ceiling)]
async fn past_the_ceiling_the_reads_go_in_before_the_next_write() {
    session::mock_unread_ceiling().returns(Duration::ZERO);
    let (repo, release) = rooted(&["a.txt", "b.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;

    let commit = held_commit(&session);
    let first = stage(&session, "a.txt");
    let second = stage(&session, "b.txt");
    std::fs::write(&release, b"go").expect("release the hook");

    for id in [commit, first, second] {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        assert!(settled(events, commit) < started(events, first));
        assert!(settled(events, first) < started(events, second));
    }
    session.close_and_wait().await;
}

/// A read that does not land is named under every write the read answers
/// for: the walk behind the burst fails once, and each write of the burst says
/// the graph did not land.
#[tokio::test(flavor = "multi_thread")]
async fn a_read_that_failed_behind_a_burst_is_named_under_each_of_its_writes() {
    let (repo, release) = rooted(&["a.txt", "b.txt"]);
    let (sink, session, doors) = open_with_doors(&repo);
    // The root and the worktree row.
    sink.opened_graph(&session, 2).await;
    quiet(&sink, &session).await;

    doors.fail_every_pass(PassStep::Swapping);
    let commit = held_commit(&session);
    let stages = [stage(&session, "a.txt"), stage(&session, "b.txt")];
    std::fs::write(&release, b"go").expect("release the hook");

    for id in [commit, stages[0], stages[1]] {
        assert_eq!(
            write_settled(&sink, id).await,
            [FollowUp::Graph],
            "the one walk behind the burst answers for each of its writes"
        );
    }
    session.close_and_wait().await;
}

/// A write of another session standing between two of this one's has the
/// reads made first: the write behind cannot start at once, and what the
/// one in front left would stay off the screen for as long as the other
/// session's write takes. The commit's place is held for those reads: the
/// other session's write starts with the commit settled, read off this
/// session's events from inside the sink call that tells that start.
// `worker_threads = 2`: `CaptureSink::hook_once` asks for it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_another_session_has_in_between_has_the_reads_made_first() {
    let (mut repo, release) = rooted(&["a.txt", "b.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;
    let (other_sink, other) = opened(&repo).await;

    let commit = held_commit(&session);
    let between = stage(&other, "a.txt");
    let behind = stage(&session, "b.txt");
    // Armed with the commit still inside git: the other session's stage
    // cannot start before the commit's place goes back.
    let (counted, at_its_start) = tokio::sync::oneshot::channel();
    let watched = Arc::clone(&sink);
    other_sink.hook_once(
        move |e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == between),
        move || {
            let settled = watched
                .count(|e| matches!(e, SessionEvent::WriteSettled { id, .. } if *id == commit));
            counted.send(settled).expect("the test waits for the count");
        },
    );
    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(
        bounded("the count at the other session's start", at_its_start)
            .await
            .expect("the hook ran"),
        1,
        "the other session's write started under the reads behind the commit"
    );
    assert_eq!(write_settled(&other_sink, between).await, []);
    for id in [commit, behind] {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        assert!(
            settled(events, commit) < started(events, behind),
            "the commit was read behind before the write behind it started"
        );
    }
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"])
            .lines()
            .count(),
        2,
        "both sessions' stages landed"
    );
    other.close_and_wait().await;
    session.close_and_wait().await;
}

/// A branch off main with one commit nothing else holds; its id.
fn off_main(repo: &mut TestRepo, name: &str) -> String {
    repo.git(&["switch", "-c", name]);
    let tip = repo.commit_file_id(&format!("{name}.txt"), "u\n", &format!("{name} work"));
    repo.git(&["switch", "main"]);
    tip
}

/// The rows of the `nth` graph laid out again (`LogRelaid`), by id.
async fn relaid(sink: &CaptureSink, nth: usize) -> Vec<String> {
    sink.wait_for("the graph laid out again", |evs| {
        evs.iter()
            .filter_map(|e| match e {
                SessionEvent::LogRelaid { rows, .. } => Some(rows),
                _ => None,
            })
            .nth(nth)
            .map(|rows| {
                rows.iter()
                    .map(|row| match row {
                        RelaidRow::Moved { oid, .. } => oid.to_hex(),
                        RelaidRow::Made(row) => {
                            row.oid().map(|oid| oid.to_hex()).unwrap_or_default()
                        }
                    })
                    .collect()
            })
    })
    .await
}

/// A delete that took rows off the graph at its press stays out until the
/// walk behind its burst (`session::leaving`): with a stage that joined its
/// burst inside git, held by its filter, a second delete's press lays the
/// graph out again — off a record no walk has been made for since the
/// first landed — and the first one's commit stays off it.
#[tokio::test(flavor = "multi_thread")]
async fn a_delete_in_front_of_a_burst_keeps_its_rows_off_until_the_walk_behind_it() {
    let (mut repo, _head) = scenario();
    let first_tip = off_main(&mut repo, "unmerged");
    let second_tip = off_main(&mut repo, "unmerged-too");
    let filter = repo.path.join(".git").join("filter-release");
    repo.git(&["config", "filter.hold.clean", &barrier_filter(&filter)]);
    repo.git(&["config", "filter.hold.required", "true"]);
    repo.write_file(".gitattributes", "held.txt filter=hold\n");
    repo.write_file("held.txt", "content\n");
    assert_eq!(
        repo.git(&["check-attr", "filter", "--", "held.txt"]),
        "held.txt: filter: hold",
        "the clean filter is wired to the path"
    );
    let hook = release_of(&repo);
    repo.write_hook("reference-transaction", &barrier_hook(&hook));
    let (sink, session) = opened(&repo).await;
    // The scenario's five, the two branches' commits and the worktree row.
    sink.opened_graph(&session, 8).await;
    let from = quiet(&sink, &session).await;

    let first = session
        .delete_branch("unmerged".into(), true)
        .expect("the delete was accepted");
    let behind = stage(&session, "held.txt");
    assert!(!relaid(&sink, 0).await.contains(&first_tip));
    std::fs::write(&hook, b"go").expect("release the hook");
    last_reached_git(&sink, behind).await;

    // The first delete has landed and nothing has walked behind it.
    let second = session
        .delete_branch("unmerged-too".into(), true)
        .expect("the second delete was accepted");
    let rows = relaid(&sink, 1).await;
    assert!(!rows.contains(&second_tip), "{rows:#?}");
    assert!(
        !rows.contains(&first_tip),
        "the first delete's commit was drawn back: {rows:#?}"
    );

    std::fs::write(&filter, b"go").expect("release the filter");
    for id in [first, behind, second] {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        assert!(
            answered(events, behind) < settled(events, first),
            "the stage joined the delete's burst"
        );
        assert!(
            settled(events, first) < started(events, second),
            "and the second delete, which joins none, waited for its reads"
        );
    }
    session.close_and_wait().await;
}

/// Writes `path` into `repo` with a clean filter of its own (`filter`) that
/// holds its stage inside git until the answered path is written.
fn held_at_its_stage(repo: &mut TestRepo, path: &str, filter: &str) -> std::path::PathBuf {
    let release = repo.path.join(".git").join(format!("{filter}-release"));
    repo.git(&[
        "config",
        &format!("filter.{filter}.clean"),
        &barrier_filter(&release),
    ]);
    repo.git(&["config", &format!("filter.{filter}.required"), "true"]);
    let attributes = repo.path.join(".gitattributes");
    let mut lines = std::fs::read_to_string(&attributes).unwrap_or_default();
    lines.push_str(&format!("{path} filter={filter}\n"));
    std::fs::write(&attributes, lines).expect("write the attributes");
    repo.write_file(path, "content\n");
    // Both halves of the barrier are read back: a filter that does not
    // hold would make a test green for the wrong reason.
    assert_eq!(
        repo.git(&["check-attr", "filter", "--", path]),
        format!("{path}: filter: {filter}")
    );
    assert_eq!(
        repo.git(&["config", "--get", &format!("filter.{filter}.clean")]),
        barrier_filter(&release)
    );
    release
}

/// [`rooted`] with `a.txt` loose, and `held.txt` beside it whose stage its
/// clean filter holds until the second path is written: a burst stopped
/// twice, inside its first write and inside its last.
fn held_twice() -> (TestRepo, std::path::PathBuf, std::path::PathBuf) {
    let (mut repo, hook) = rooted(&["a.txt"]);
    let filter = held_at_its_stage(&mut repo, "held.txt", "hold");
    (repo, hook, filter)
}

/// The commit, the stage that joins its burst, and the stage held behind
/// them, queued while the commit is inside git.
fn burst_held_twice(session: &Arc<RepoSession>) -> [OperationId; 3] {
    [
        held_commit(session),
        stage(session, "a.txt"),
        stage(session, "held.txt"),
    ]
}

/// The burst's last write has been handed to git, so the two in front of it
/// have answered.
async fn last_reached_git(sink: &CaptureSink, last: OperationId) {
    sink.wait_for("the last write of the burst reached git", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == last))
            .then_some(())
    })
    .await;
}

fn is_status_spawn(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. } if display.starts_with("git status"))
}

/// The refs listing, told from the walk's narrower one by what it lists.
fn is_refs_spawn(event: &SessionEvent) -> bool {
    matches!(
        event,
        SessionEvent::CommandStarted { display, .. }
            if display.contains("for-each-ref") && display.ends_with("refs/heads refs/remotes refs/tags")
    )
}

/// A read in flight as the writes of a burst answer is not read again under
/// the write behind them: the reads behind the burst are its reading again
/// (`RepoSession::read_status_again`). The read is parked at its spawn
/// while the commit is inside git and let go once the burst's last write is
/// inside git, held by its filter — so one spawned in its place before
/// that write answers would be a read under it.
async fn a_fenced_read_is_left_to_the_burst(
    spawned: fn(&SessionEvent) -> bool,
    ask: fn(&Arc<RepoSession>),
) {
    let (repo, hook, filter) = held_twice();
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;
    session.set_recording(Recording::WithBackground);

    let burst = burst_held_twice(&session);
    // It has looked, and its git has not run.
    let release = sink.park_once("the parked read", spawned);
    ask(&session);
    sink.wait_for("the read reached its spawn", |evs| {
        evs.iter().any(spawned).then_some(())
    })
    .await;
    std::fs::write(&hook, b"go").expect("release the hook");
    last_reached_git(&sink, burst[2]).await;

    // Let go, the read lands fenced off by the commit and the stage. Every
    // read it could have asked for in its place is over once the flights
    // are empty.
    release.send(()).expect("the parked read is waiting");
    bounded(
        "the fenced read and what it asked for",
        session.wait_for_snapshot_reads(),
    )
    .await;
    assert_eq!(
        sink.count(spawned),
        1,
        "nothing was read under the last write: {:?}",
        sink.commands_since(from)
    );

    std::fs::write(&filter, b"go").expect("release the filter");
    for id in burst {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    assert_eq!(sink.count(spawned), 2, "the parked one and the burst's");
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        let told = told_by(events, burst[0]).expect("the commit told the status it left");
        assert!(
            told < started(events, burst[1]),
            "at its answer, the parked read still out"
        );
        let read = statuses_read(events, burst[0]);
        assert_eq!(read.len(), 1, "one status read: {read:?}");
        assert!(
            read[0].1 >= reads_from(events, burst[2]),
            "the one read behind the burst, which looked after its last write"
        );
    }
    session.close_and_wait().await;
}

// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_status_a_burst_fenced_off_is_not_read_again_under_its_writes() {
    a_fenced_read_is_left_to_the_burst(is_status_spawn, |session| session.refresh_status()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refs_read_a_burst_fenced_off_is_not_read_again_under_its_writes() {
    a_fenced_read_is_left_to_the_burst(is_refs_spawn, |session| session.refresh_refs()).await;
}

/// Where the reads behind a burst take no refs, a refs read one of its
/// writes fenced off still asks again for itself — nothing else would read
/// them before the next tick. Two stages, each held inside git by a filter
/// of its own: the read is parked while the first is held and let go with
/// the second held, the burst still unread.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refs_read_fenced_by_a_burst_that_reads_no_refs_asks_again() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    let first_held = held_at_its_stage(&mut repo, "first.txt", "one");
    let second_held = held_at_its_stage(&mut repo, "second.txt", "two");
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;
    session.set_recording(Recording::WithBackground);

    let first = stage(&session, "first.txt");
    let second = stage(&session, "second.txt");
    let release = sink.park_once("the parked refs read", is_refs_spawn);
    session.refresh_refs();
    sink.wait_for("the refs read reached its spawn", |evs| {
        evs.iter().any(is_refs_spawn).then_some(())
    })
    .await;
    std::fs::write(&first_held, b"go").expect("release the first filter");
    last_reached_git(&sink, second).await;

    release.send(()).expect("the parked read is waiting");
    bounded(
        "the fenced read and the one it asked for",
        session.wait_for_snapshot_reads(),
    )
    .await;
    assert_eq!(
        sink.count(is_refs_spawn),
        2,
        "the fenced read asked again for itself: {:?}",
        sink.commands_since(from)
    );
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        let fenced_from = reads_from(events, first);
        assert!(
            events.iter().any(
                |e| matches!(e, SessionEvent::RefsLoaded { looked, .. } if *looked >= fenced_from)
            ),
            "and the refs it read looked after the write that fenced the first"
        );
    }

    std::fs::write(&second_held, b"go").expect("release the second filter");
    for id in [first, second] {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    assert_eq!(
        sink.count(is_refs_spawn),
        2,
        "the reads behind two stages take no refs"
    );
    session.close_and_wait().await;
}

/// What the queue says of its reads to come is taken down behind them: a
/// status a later fetch fences off — one that brings nothing, and so reads
/// no tree behind itself — asks again for itself, the commit read behind
/// before it notwithstanding.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_status_a_fetch_fenced_off_asks_again_behind_a_burst_already_read() {
    let (_bare, mut repo) = origin_and_clone();
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;

    let commit = session
        .commit("read behind in full".into(), CommitOptions::default())
        .expect("the commit was accepted");
    assert_eq!(write_settled(&sink, commit).await, [], "every read landed");
    bounded(
        "the reads behind the commit",
        session.wait_for_snapshot_reads(),
    )
    .await;
    session.set_recording(Recording::WithBackground);
    let spawned = sink.count(is_status_spawn);

    let release = sink.park_once("the parked status", is_status_spawn);
    session.refresh_status();
    sink.wait_for("the status reached its spawn", move |evs| {
        (evs.iter().filter(|e| is_status_spawn(e)).count() > spawned).then_some(())
    })
    .await;
    let fetch = session.fetch(None).expect("the fetch was accepted");
    assert_eq!(write_settled(&sink, fetch).await, [], "every read landed");

    release.send(()).expect("the parked status is waiting");
    bounded(
        "the fenced status and the one it asked for",
        session.wait_for_snapshot_reads(),
    )
    .await;
    assert_eq!(
        sink.count(is_status_spawn) - spawned,
        2,
        "the fenced status asked again for itself: {:?}",
        sink.commands_since(from)
    );
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        let fenced_from = reads_from(events, fetch);
        assert!(
            statuses(events)
                .iter()
                .any(|(_, looked)| *looked >= fenced_from),
            "and the status it read looked after the fetch"
        );
    }
    session.close_and_wait().await;
}

/// A closed session reads nothing behind its writes, so none of them joins
/// a burst: each settles as it answers, and the tree's next holder is told
/// of each. The stage behind the held commit is one that would have joined.
#[tokio::test(flavor = "multi_thread")]
async fn a_closed_session_settles_each_write_as_it_answers() {
    let (mut repo, release) = rooted(&["a.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;

    let commit = held_commit(&session);
    let behind = stage(&session, "a.txt");
    session.close();
    std::fs::write(&release, b"go").expect("release the hook");

    for id in [commit, behind] {
        assert_eq!(write_answer(&sink, id).await, None, "the write landed");
        assert_eq!(write_settled(&sink, id).await, [], "and settled");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        assert!(
            settled(events, commit) < started(events, behind),
            "the commit settled before the stage behind it started"
        );
        assert_eq!(statuses(events), [], "and nothing was read behind either");
    }
    bounded(
        "the write loop ends with the queue drained",
        session.take_write_join().expect("the loop's task"),
    )
    .await
    .expect("the loop ended without panicking");
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]),
        "a.txt",
        "the close lost neither write"
    );
    session.close_and_wait().await;
}

/// A session closed while a burst's last write is inside git reads nothing
/// behind the burst: every write of it settles with nothing read, the ones
/// that answered before the close included. What the commit told at its
/// answer, the session still open, is no read: no `git status` ran.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_closed_inside_a_burst_settles_it_unread() {
    let (mut repo, hook, filter) = held_twice();
    let (sink, session) = opened(&repo).await;
    let from = quiet(&sink, &session).await;
    session.set_recording(Recording::WithBackground);

    let burst = burst_held_twice(&session);
    std::fs::write(&hook, b"go").expect("release the hook");
    last_reached_git(&sink, burst[2]).await;
    session.close();
    std::fs::write(&filter, b"go").expect("release the filter");

    for id in burst {
        assert_eq!(write_answer(&sink, id).await, None, "the write landed");
        assert_eq!(write_settled(&sink, id).await, [], "and settled");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        assert_eq!(
            statuses_read(events, burst[0]),
            [],
            "nothing was read behind a burst whose session closed"
        );
        assert_eq!(
            events.iter().filter(|e| is_status_spawn(e)).count(),
            0,
            "no status was spawned for it either"
        );
        assert!(
            told_by(events, burst[0]).is_some_and(|told| told < started(events, burst[1])),
            "the commit told what it left at its answer, ahead of the close"
        );
    }
    bounded(
        "the write loop ends with the queue drained",
        session.take_write_join().expect("the loop's task"),
    )
    .await
    .expect("the loop ended without panicking");
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"])
            .lines()
            .collect::<Vec<_>>(),
        ["a.txt", "held.txt"],
        "the close lost none of the burst"
    );
    session.close_and_wait().await;
}

/// A write the burst did not take is the next one served, also where the
/// session closes during the reads in front of it: the branch behind the
/// commit was off the queue when the close landed, and still runs.
// `worker_threads = 2`: the hook parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_close_during_the_reads_still_serves_the_write_waiting_behind_them() {
    let (mut repo, release) = rooted(&[]);
    let (sink, session) = opened(&repo).await;
    quiet(&sink, &session).await;

    let commit = held_commit(&session);
    let behind = session
        .create_branch("behind".into(), None, false)
        .expect("the branch was accepted");
    // The first refs listing from here on is the one read behind the commit:
    // nothing else reads while it is inside git. Not the first status, which
    // the commit tells at its answer, ahead of the reads.
    let closing = Arc::clone(&session);
    sink.hook_once(
        |e| matches!(e, SessionEvent::RefsLoaded { .. }),
        move || closing.close(),
    );
    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(write_answer(&sink, commit).await, None);
    assert_eq!(
        write_answer(&sink, behind).await,
        None,
        "the branch ran behind the close"
    );
    bounded(
        "the write loop ends with the queue drained",
        session.take_write_join().expect("the loop's task"),
    )
    .await
    .expect("the loop ended without panicking");
    {
        let events = sink.events.lock().unwrap();
        assert!(
            settled(&events, commit) < started(&events, behind),
            "the branch joined no burst: it was waiting off the queue for the reads"
        );
    }
    assert!(
        repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/behind"]),
        "the branch asked for before the close exists after it"
    );
    session.close_and_wait().await;
}

/// What reading once behind a burst saves where the status is the long read:
/// five files staged over a clean tree, timed from the first press to the
/// last write's settling — the status the Commit button waits for (the
/// app's `StageAhead::unseen`). Each test is one way of reading behind the
/// presses, run in a process of its own on a repository the caller names
/// and lets it write to — its tree is reset between rounds
/// (`_PGG_BURST_REPO`; `_PGG_BURST_PACE_MS` puts that long between two presses,
/// none by default — all five then wait in the queue together):
///
/// ```text
/// <it binary> --ignored --exact --nocapture session_integration::write_burst::measured::<way>
/// ```
///
/// The record: ci/baseline/code-costs-windows-x64.md.
mod measured {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use platitude_core::process::GitCommand;
    use platitude_core::session;

    use super::{quiet, stage};
    use crate::support::session::{opened_at, poll_landed, write_settled};
    use crate::support::wait::bounded;

    const ROUNDS: usize = 6;
    const PRESSES: usize = 5;

    /// The queue as it runs: one status behind the presses that waited
    /// together.
    #[tokio::test]
    #[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
    async fn read_behind_the_burst() {
        five_presses("read_behind_the_burst").await;
    }

    /// The ceiling mocked to nothing, so no write joins a burst: a status
    /// behind every press, as a queue of bursts of one reads. mry's mocks are
    /// the thread's own, hence the current-thread runtime of both.
    #[tokio::test]
    #[mry::lock(session::unread_ceiling)]
    #[ignore = "a measurement for ci/baseline: run by hand, one process per test"]
    async fn read_behind_each() {
        session::mock_unread_ceiling().returns(Duration::ZERO);
        five_presses("read_behind_each").await;
    }

    /// `count` tracked files of `repo`, spread over its listing.
    async fn tracked(repo: &Path, count: usize) -> Vec<String> {
        let (exec, cancel) = crate::support::exec::env();
        let listed = bounded(
            "the tracked files",
            exec.run(GitCommand::new().cwd(repo).args(["ls-files"]), &cancel),
        )
        .await
        .expect("git ls-files");
        let all: Vec<&str> = std::str::from_utf8(&listed.stdout)
            .expect("the paths are text")
            .lines()
            .collect();
        assert!(all.len() >= count, "the repository tracks {count} files");
        let step = all.len() / count;
        (0..count).map(|n| all[n * step].to_string()).collect()
    }

    /// Puts `repo`'s index and tree back to its HEAD.
    async fn clean(repo: &Path) {
        let (exec, cancel) = crate::support::exec::env();
        bounded(
            "the tree put back",
            exec.run(
                GitCommand::new()
                    .cwd(repo)
                    .args(["reset", "--hard", "--quiet"]),
                &cancel,
            ),
        )
        .await
        .expect("git reset --hard");
    }

    #[expect(
        clippy::print_stdout,
        reason = "the measurement's one line is its output"
    )]
    async fn five_presses(way: &str) {
        let repo = PathBuf::from(
            std::env::var_os("_PGG_BURST_REPO").expect("_PGG_BURST_REPO names the repository"),
        );
        let pace = std::env::var("_PGG_BURST_PACE_MS")
            .ok()
            .map(|ms| Duration::from_millis(ms.parse().expect("_PGG_BURST_PACE_MS is a number")))
            .filter(|pace| !pace.is_zero());
        let files = tracked(&repo, PRESSES).await;
        let (sink, session) = opened_at(&repo).await;
        quiet(&sink, &session).await;

        let mut rounds = Vec::new();
        for _ in 0..ROUNDS {
            // Every round over the same clean tree, read once before it is
            // timed: what a write's reads cost grows with the files pending.
            clean(&repo).await;
            poll_landed(&session).await;
            for path in &files {
                let file = repo.join(path);
                let mut text = std::fs::read(&file).expect("a tracked file");
                text.extend_from_slice(b"\nmeasured\n");
                std::fs::write(&file, text).expect("change the file");
            }
            // waits(measured): the round's own time is the measurement and judges nothing
            let started = Instant::now();
            let mut last = None;
            for path in &files {
                if let (Some(pace), Some(_)) = (pace, last) {
                    // waits(measured): the pace between two presses is the round's input and judges nothing
                    tokio::time::sleep(pace).await;
                }
                last = Some(stage(&session, path));
            }
            let failed = write_settled(&sink, last.expect("a round has presses")).await;
            rounds.push(started.elapsed());
            assert_eq!(failed, [], "every read landed");
        }
        session.close_and_wait().await;
        clean(&repo).await;

        rounds.sort();
        let ms = |round: &Duration| round.as_secs_f64() * 1e3;
        println!(
            "write_burst way={way} presses={PRESSES} pace_ms={} rounds={} min_ms={:.0} \
             median_ms={:.0} max_ms={:.0}",
            pace.map_or(0, |pace| pace.as_millis()),
            rounds.len(),
            ms(&rounds[0]),
            ms(&rounds[(rounds.len() - 1) / 2]),
            ms(&rounds[rounds.len() - 1]),
        );
    }
}
