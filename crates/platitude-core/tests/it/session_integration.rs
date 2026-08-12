//! End-to-end RepoSession tests on real temp repositories.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::support::{Patience, TestRepo};
use platitude_core::details::DiffTarget;
use platitude_core::identity::SignatureStatus;
use platitude_core::session::{RepoSession, SessionEvent, SessionSink};
use platitude_core::{GitExecutor, Oid};

/// Picks the event a [`CaptureSink`] hook fires on.
type When = Box<dyn Fn(&SessionEvent) -> bool + Send>;
/// What runs inside that event's delivery.
type Then = Box<dyn FnOnce() + Send>;

/// What a graph pass reports when it lands, whichever of the two shapes
/// it landed in.
#[derive(Debug, Clone, Copy)]
struct Pass {
    generation: u64,
    /// Rows the pass put on screen (`LogFinished`'s `total`).
    total: u32,
    walked: u32,
    truncated: bool,
}

/// Reads a landed pass out of the one event that ends it.
///
/// A stream (`LogStarted` → chunks → `LogFinished`) and an atomic
/// replacement (`LogReplaced`) are two shapes of the same thing, and
/// which one carries a given ask is a scheduling accident. `restart_log`
/// asks for a stream, but that pass drops without a word the moment a
/// background rebuild is asked for over it — `run_direct_pass` returns on
/// a cancelled token, and a walk cancelled mid-stream reports neither
/// `LogFinished` nor `LogFailed` — and the rebuild behind it replaces
/// instead, carrying the very options the ask just changed (both entry
/// points read them after taking the token, so the winner is always the
/// one holding the new ones).
///
/// A test that waits for one shape is waiting on that race. 実測: a
/// window change on a loaded machine landed as `LogReplaced { generation:
/// 4 }` — the open sequence's dirty-flip `refresh_log` overtook the
/// stream at generation 3 — and the wait sat out its whole budget.
fn pass_of(event: &SessionEvent) -> Option<Pass> {
    match event {
        SessionEvent::LogFinished {
            generation,
            total,
            walked,
            truncated,
            ..
        } => Some(Pass {
            generation: *generation,
            total: *total,
            walked: *walked,
            truncated: *truncated,
        }),
        SessionEvent::LogReplaced {
            generation,
            rows,
            walked,
            truncated,
            ..
        } => Some(Pass {
            generation: *generation,
            total: rows.len() as u32,
            walked: *walked,
            truncated: *truncated,
        }),
        _ => None,
    }
}

struct CaptureSink {
    events: Mutex<Vec<SessionEvent>>,
    hook: Mutex<Option<(When, Then)>>,
}

impl CaptureSink {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
            hook: Mutex::new(None),
        })
    }

    /// Runs `run` once, from inside the sink call that delivers the first
    /// event `when` accepts — the only place a test can stand in the
    /// middle of a read. Everything the session sends comes through here,
    /// so a hook that parks holds the reader there while the test drives
    /// the rest.
    ///
    /// A parked hook blocks the worker thread its reader runs on, and
    /// tokio leaves a task queued there queued: whatever has to run
    /// meanwhile must be started from the test's own thread, not from
    /// inside the hook.
    fn hook_once(
        &self,
        when: impl Fn(&SessionEvent) -> bool + Send + 'static,
        run: impl FnOnce() + Send + 'static,
    ) {
        *self.hook.lock().unwrap() = Some((Box::new(when), Box::new(run)));
    }

    /// Number of recorded events matching `pred`.
    fn count(&self, pred: impl Fn(&SessionEvent) -> bool) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| pred(e))
            .count()
    }

    /// Waits until log streaming settles: a pass ending with `total`
    /// rows (a `LogFinished`, or a `LogReplaced` carrying that many
    /// rows) exists and no further stream event arrives for a beat,
    /// then returns the newest matching generation. Acting on the
    /// *first* matching pass instead would race the passes still in
    /// flight (the tag swap, the dirty-flip replacement), which finish
    /// afterwards with higher generations and would be mistaken for the
    /// reaction to whatever the test does next.
    async fn settled_stream_gen(&self, total: u32) -> u64 {
        fn stream_events(evs: &[SessionEvent]) -> usize {
            evs.iter().filter(|e| is_stream_event(e)).count()
        }
        let what = format!("the stream to settle at {total} rows");
        let mut patience = Patience::new();
        loop {
            let (newest, seen) = {
                let evs = self.events.lock().unwrap();
                let newest = evs
                    .iter()
                    .filter_map(pass_of)
                    .filter(|p| p.total == total)
                    .map(|p| p.generation)
                    .max();
                patience.note(evs.len());
                (newest, stream_events(&evs))
            };
            if let Some(g) = newest {
                tokio::time::sleep(Duration::from_millis(400)).await;
                if stream_events(&self.events.lock().unwrap()) == seen {
                    return g;
                }
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            patience.check(&what, &self.events);
        }
    }

    /// Waits for the first graph pass after generation `after` to land,
    /// in whichever shape it landed in (see [`pass_of`]).
    ///
    /// "The first one after" is the reaction to whatever the test asked
    /// for last, and nothing else: a pass that finds the graph unchanged
    /// swaps nothing and says nothing (`run_swap_pass`), so the only
    /// passes that speak are the ones an ask produced. That also makes it
    /// safe for `after` to be older than the newest settled pass — a
    /// duplicate of a graph already on screen could not have spoken.
    ///
    /// Which means the caller owes one thing: **ask for something the
    /// graph on screen differs from** — in its rows, or in the footer
    /// under them (`run_swap_pass` compares both, so a window that only
    /// moves `walked`/`truncated` does speak). A change that leaves the
    /// two exactly as they are has nothing to announce if a rebuild
    /// overtakes the stream, and no wait can conjure an event nobody sent.
    async fn pass_after(&self, what: &str, after: u64) -> Pass {
        self.wait_for(what, |evs| {
            evs.iter()
                .filter_map(pass_of)
                .find(|p| p.generation > after)
        })
        .await
    }

    /// Polls until `pred` over the event list returns `Some`, giving up
    /// only once the session has gone quiet on it (see [`Patience`]).
    async fn wait_for<T>(&self, what: &str, pred: impl Fn(&[SessionEvent]) -> Option<T>) -> T {
        let mut patience = Patience::new();
        loop {
            {
                let events = self.events.lock().unwrap();
                if let Some(v) = pred(&events) {
                    return v;
                }
                patience.note(events.len());
            }
            patience.check(what, &self.events);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl SessionSink for CaptureSink {
    fn event(&self, event: SessionEvent) {
        let run = {
            let mut slot = self.hook.lock().unwrap();
            let fires = slot.as_ref().is_some_and(|(when, _)| when(&event));
            fires.then(|| slot.take().map(|(_, run)| run)).flatten()
        };
        self.events.lock().unwrap().push(event);
        // Outside both locks: a parked hook must not hold the recording
        // shut, or the events it is waiting on could never be written.
        if let Some(run) = run {
            run();
        }
    }
}

/// main: root ─ a ─ merge ← side, tag v1 on merge target, one stash.
fn scenario() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("g.txt", "s\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "1\n", "main work");
    repo.git(&["merge", "--no-ff", "-m", "merge side", "side"]);
    repo.git(&["tag", "v1"]);
    repo.write_file("f.txt", "wip\n");
    repo.git(&["stash", "push", "-m", "wip stash"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    (repo, head)
}

#[tokio::test(flavor = "multi_thread")]
async fn open_streams_the_full_pipeline() {
    let (repo, head) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    sink.wait_for("Opened", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::Opened { info } => Some(info.workdir.clone()),
            _ => None,
        })
    })
    .await;

    let total = sink
        .wait_for("LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { total, .. } => Some(*total),
                _ => None,
            })
        })
        .await;
    assert_eq!(total, 5, "root + side + main + merge + stash row");

    // All rows delivered, topo-consistent; the stash (newest child of the
    // merge) streams first, the head commit right after.
    let rows = sink
        .wait_for("chunk rows", |evs| {
            let mut rows = Vec::new();
            for e in evs {
                if let SessionEvent::LogChunk { rows: r, .. } = e {
                    rows.extend(r.iter().cloned());
                }
            }
            (rows.len() == 5).then_some(rows)
        })
        .await;
    assert_eq!(rows[0].stash_ref, "stash@{0}", "stash row leads");
    assert!(
        rows[0].subject.contains("wip stash"),
        "stash subject is its reflog message: {:?}",
        rows[0].subject
    );
    assert_eq!(rows[1].oid_hex, head, "merge commit is the newest commit");
    assert_eq!(rows[1].row, 1);
    assert!(rows.iter().skip(1).all(|r| r.stash_ref.is_empty()));
    assert!(rows.iter().all(|r| !r.subject.is_empty()));
    assert!(rows.iter().all(|r| r.author == "Test User"));

    // Labels: the head row must end up carrying main (+ v1 tag), either
    // inline or via a LabelsChanged update.
    sink.wait_for("labels on head row", |evs| {
        let seen = crate::support::replay_graph(evs);
        let latest: Vec<&str> = seen
            .get(&1)
            .map(|row| row.labels.iter().map(|l| l.text.as_str()).collect())
            .unwrap_or_default();
        (latest.contains(&"main") && latest.contains(&"v1")).then_some(())
    })
    .await;

    sink.wait_for("RefsLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot } => {
                let locals: Vec<&str> = snapshot.locals.iter().map(|b| b.short.as_str()).collect();
                assert_eq!(locals, vec!["main", "side"], "sorted locals");
                assert!(snapshot.locals[0].is_head);
                assert_eq!(snapshot.tags.len(), 1);
                Some(())
            }
            _ => None,
        })
    })
    .await;

    sink.wait_for("StatusLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::StatusLoaded {
                status, op_state, ..
            } => {
                assert_eq!(status.branch_head.as_deref(), Some("main"));
                assert!(!op_state.any());
                Some(())
            }
            _ => None,
        })
    })
    .await;

    sink.wait_for("StashesLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::StashesLoaded { stashes } => {
                assert_eq!(stashes.len(), 1);
                assert!(stashes[0].message.contains("wip stash"));
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_log_delivers_a_new_generation() {
    let (repo, _) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    let first_gen = sink
        .wait_for("first LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { generation, .. } => Some(*generation),
                _ => None,
            })
        })
        .await;

    session.restart_log();

    let second_gen = sink
        .wait_for("second LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { generation, .. } if *generation > first_gen => {
                    Some(*generation)
                }
                _ => None,
            })
        })
        .await;
    assert!(second_gen > first_gen);

    // The restarted stream re-delivers all rows under the new generation
    // (4 commits + the stash row).
    sink.wait_for("second-generation rows", |evs| {
        let count: usize = evs
            .iter()
            .filter_map(|e| match e {
                SessionEvent::LogChunk { generation, rows } if *generation == second_gen => {
                    Some(rows.len())
                }
                _ => None,
            })
            .sum();
        (count == 5).then_some(())
    })
    .await;

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn details_and_diff_round_trip_through_the_session() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\n", "add f");
    repo.write_file("f.txt", "one\ntwo changed\n");
    repo.git(&["commit", "-am", "edit f"]);
    let head = repo.git(&["rev-parse", "HEAD"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

    let oid = Oid::from_hex_str(&head).unwrap();
    session.load_details(oid);
    sink.wait_for("DetailsLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::DetailsLoaded { details } => {
                assert_eq!(details.oid, oid);
                assert_eq!(details.message, "edit f");
                assert_eq!(details.files.len(), 1);
                assert_eq!(details.files[0].path, "f.txt");
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.load_diff(DiffTarget::Commit {
        oid,
        parent: Some(Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD^"])).unwrap()),
        path: "f.txt".to_string(),
        orig_path: None,
    });
    sink.wait_for("DiffLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::DiffLoaded { patches, .. } => {
                assert_eq!(patches.len(), 1);
                assert!(!patches[0].hunks.is_empty());
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

/// The signature question is asked and answered on its own, apart from
/// the details it belongs beside: verifying may run gpg, and the details
/// pane cannot wait for that.
#[tokio::test(flavor = "multi_thread")]
async fn a_signature_answer_names_the_commit_it_is_about() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "add f");
    let head = repo.git(&["rev-parse", "HEAD"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

    let oid = Oid::from_hex_str(&head).unwrap();
    session.check_signature(oid);
    sink.wait_for("SignatureChecked", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::SignatureChecked {
                oid: asked,
                signature,
            } => {
                assert_eq!(asked, &head, "the answer says which commit it is about");
                assert_eq!(signature.status, SignatureStatus::Absent);
                assert!(!signature.status.is_signed());
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn tag_only_commits_follow_the_include_tags_option() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "base");
    // A commit reachable only through a tag (detached, then back to main).
    repo.git(&["checkout", "--detach", "HEAD"]);
    repo.commit_file("g.txt", "t\n", "tag only work");
    repo.git(&["tag", "islet"]);
    repo.git(&["checkout", "main"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    // Tags are walked by default → the tag-only commit has a row. The
    // tag-inclusive pass differs from the fast pass here, so it arrives
    // as an atomic replacement.
    let first_gen = sink
        .wait_for("tags-on LogReplaced", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogReplaced {
                    generation, rows, ..
                } if rows.len() == 2 => Some(*generation),
                _ => None,
            })
        })
        .await;

    // Two-phase streaming: a fast tag-less pass must have painted first.
    {
        let events = sink.events.lock().unwrap();
        let fast_pass = events.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                generation, total, ..
            } if *total == 1 => Some(*generation),
            _ => None,
        });
        assert!(
            fast_pass.is_some_and(|g| g < first_gen),
            "expected a tag-less fast pass before the tag-inclusive swap"
        );
    }

    session.set_include_tags(false);
    let off = sink.pass_after("the tag-less graph", first_gen).await;
    assert_eq!(off.total, 1, "the tag-only commit left the walk");

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn log_limit_truncates_the_window() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.commit_file("f.txt", "3\n", "three");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    sink.wait_for("full LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                total, truncated, ..
            } if *total == 3 => {
                assert!(!truncated, "3 commits fit in the default window");
                Some(())
            }
            _ => None,
        })
    })
    .await;
    let first_gen = sink.settled_stream_gen(3).await;

    session.set_log_limit(Some(2));
    let limited = sink.pass_after("the limited window", first_gen).await;
    assert_eq!(limited.total, 2);
    assert_eq!(limited.walked, 2, "the footer's number is the limit itself");
    assert!(limited.truncated);

    session.close();
}

/// A stash's synthetic index parent is walked but sifted out of the
/// shown rows, so the shown count sits below the window limit even when
/// the walk was cut — truncation must follow the walk, not the rows.
#[tokio::test(flavor = "multi_thread")]
async fn truncation_follows_the_walk_not_the_shown_rows() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.commit_file("f.txt", "3\n", "three");
    repo.write_file("f.txt", "wip\n");
    repo.git(&["stash", "push", "-m", "wip stash"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    // Full pass first: stash row + three commits, nothing truncated.
    sink.wait_for("full LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                total, truncated, ..
            } if *total == 4 => {
                assert!(!truncated);
                Some(())
            }
            _ => None,
        })
    })
    .await;
    let first_gen = sink.settled_stream_gen(4).await;

    // The walk emits 4 rows (stash, its index parent, "three", "two") and
    // is cut before "one"; the sifted index parent leaves 3 shown rows.
    session.set_log_limit(Some(4));
    let limited = sink.pass_after("the limited window", first_gen).await;
    assert_eq!(limited.total, 3, "stash + three + two, index parent sifted");
    assert_eq!(limited.walked, 4, "the walk count stays on the limit");
    assert!(limited.truncated, "the walk was cut before the root commit");

    session.close();
}

/// The synthetic WIP row is shown but never walked: a window that holds
/// the whole history must not report truncation just because the WIP row
/// pushes the shown count up to the limit.
///
/// Reached by widening a window that really was cut, rather than by
/// opening the wide one straight away. Both say the same thing about the
/// WIP row, but only the widening changes the graph — and a change is
/// what makes the answer arrive at all. Going straight to the wide window
/// leaves the rows *and the footer* exactly as the opening pass left them
/// (the default window holds this history whole either way), so a rebuild
/// that overtakes the stream (this repository opens dirty, and the status
/// read that notices it asks for one) finds nothing to swap and says
/// nothing, and the test waits for an event that was never sent.
#[tokio::test(flavor = "multi_thread")]
async fn the_wip_row_does_not_trigger_truncation() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.write_file("f.txt", "wip\n"); // dirty → synthetic WIP row

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    // Wait until the dirty state is reflected and the stream settles, so
    // the next pass is the reaction to the limit change.
    let first_gen = sink.settled_stream_gen(3).await;

    // One commit through a window of one: cut, and the WIP row rides on
    // top of it regardless.
    session.set_log_limit(Some(1));
    let cut = sink.pass_after("the cut window", first_gen).await;
    assert_eq!(cut.total, 2, "WIP row + the one commit walked");
    assert_eq!(cut.walked, 1, "the walk stopped on the limit");
    assert!(cut.truncated, "older history exists and is not shown");

    // Two commits walk through a window of three; the WIP row makes three
    // shown rows, which is not a truncated window.
    session.set_log_limit(Some(3));
    let whole = sink.pass_after("the widened window", cut.generation).await;
    assert_eq!(whole.total, 3, "WIP row + two commits");
    assert_eq!(whole.walked, 2, "the WIP row is shown but never walked");
    assert!(!whole.truncated, "the whole history fits the window");

    session.close();
}

/// A window change asks for a stream, but only until somebody else asks
/// for the graph: the stream drops without a word when a background
/// rebuild supersedes it, and that rebuild — reading the options the
/// change just wrote — lands the new window as an atomic replacement
/// instead. The two are the same answer, and which one arrives is a
/// scheduling accident, so waiting for one shape is waiting on a race.
///
/// Held in the swap that adds the WIP row, the interleaving is exact: the
/// stream cannot reach its cancel check until the rebuild behind it has
/// taken its place. Left to the scheduler it is rare — it turned up as a
/// flake on a machine running three other builds, not as a test.
#[tokio::test(flavor = "multi_thread")]
async fn a_window_change_a_rebuild_overtakes_still_lands_the_new_window() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(2).await;

    // Park in the swap that adds the WIP row: it sends under the graph
    // lock, so every pass asked for from here waits at that door.
    let (arrived, at_the_window) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3),
        move || {
            let _ = arrived.send(());
            let _ = held.recv_timeout(Duration::from_secs(20));
        },
    );
    repo.write_file("f.txt", "wip\n");
    session.refresh_status();
    at_the_window.await.expect("the rebuild reached the window");
    // The sink records before it runs the hook, so the graph the change
    // is measured against is already readable from where it is parked.
    let wip_gen = sink
        .wait_for("the WIP row's generation", |evs| {
            evs.iter()
                .filter_map(pass_of)
                .find(|p| p.total == 3)
                .map(|p| p.generation)
        })
        .await;

    // The change's stream is stopped at that door; the rebuild asked for
    // behind it takes its place before the stream gets through.
    session.set_log_limit(Some(1));
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    let cut = sink.pass_after("the new window", wip_gen).await;
    assert_eq!(cut.total, 2, "WIP row + the one commit walked");
    assert_eq!(cut.walked, 1, "the walk stopped on the limit");
    assert!(cut.truncated, "older history exists and is not shown");
    assert_eq!(
        sink.count(
            |e| matches!(e, SessionEvent::LogFinished { generation, .. } if *generation > wip_gen)
        ),
        0,
        "the superseded stream stayed silent, so the replacement is the \
         only thing that could have carried the window: {:?}",
        sink.events.lock().unwrap()
    );
    session.close();
}

/// The same interleaving over a window change that moves nothing but the
/// footer: two commits through a window of two are reported cut (the walk
/// stopped on the limit, which is all truncation can mean), and widening
/// to three leaves every row exactly where it was. The rebuild that
/// overtakes the stream carries the new window, so it is the only thing
/// that can say the history is no longer cut — and a comparison that only
/// looks at rows finds nothing to do, leaving the notice claiming history
/// the user just asked to see.
#[tokio::test(flavor = "multi_thread")]
async fn a_window_change_only_the_footer_notices_still_lands() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    let first_gen = sink.settled_stream_gen(2).await;

    // A window exactly as wide as the history: every commit is shown, and
    // the walk stopping on the limit is what makes it cut all the same.
    session.set_log_limit(Some(2));
    let cut = sink.pass_after("the window on the limit", first_gen).await;
    assert_eq!(cut.total, 2, "both commits fit");
    assert_eq!(cut.walked, 2, "the walk stopped on the limit");
    assert!(cut.truncated, "which is all the footer knows");

    // Park in the swap that adds the WIP row (see the test above): from
    // here every pass waits at the graph lock.
    let (arrived, at_the_window) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3),
        move || {
            let _ = arrived.send(());
            let _ = held.recv_timeout(Duration::from_secs(20));
        },
    );
    repo.write_file("f.txt", "wip\n");
    session.refresh_status();
    at_the_window.await.expect("the rebuild reached the window");
    let wip = sink
        .wait_for("the WIP row's pass", |evs| {
            evs.iter().filter_map(pass_of).find(|p| p.total == 3)
        })
        .await;
    assert!(wip.truncated, "the window is still sitting on the limit");

    // Widen past the end of the history. The stream this asks for is
    // stopped at the door and dropped; the rebuild behind it walks the
    // same two commits, draws the same three rows, and carries the only
    // thing that did change.
    session.set_log_limit(Some(3));
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    let whole = sink.pass_after("the widened window", wip.generation).await;
    assert_eq!(whole.total, 3, "WIP row + both commits, exactly as before");
    assert_eq!(whole.walked, 2, "the walk ran out of history");
    assert!(!whole.truncated, "so nothing is being kept from the user");
    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn unborn_repository_finishes_with_zero_rows() {
    let repo = TestRepo::init();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    let total = sink
        .wait_for("LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { total, .. } => Some(*total),
                _ => None,
            })
        })
        .await;
    assert_eq!(total, 0);

    sink.wait_for("StatusLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::StatusLoaded { status, .. } => {
                assert_eq!(status.branch_oid, None, "unborn branch");
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_failure_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let sink = CaptureSink::new();
    let _session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        dir.path().to_path_buf(),
        sink.clone(),
    );
    sink.wait_for("OpenFailed", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::OpenFailed { .. }))
            .then_some(())
    })
    .await;
}

/// Writes are serialized per session: a burst of concurrent stage requests
/// must all land. Without the lock they race on `.git/index.lock` and some
/// silently fail.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_writes_are_serialized() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    const COUNT: usize = 12;
    for n in 0..COUNT {
        repo.write_file(&format!("f{n}.txt"), "content\n");
    }

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

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
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

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

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

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
                    SessionEvent::WriteFinished { op, error } => Some((*op, error.clone())),
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

/// Opens a session and waits until the repository is loaded.
async fn opened(repo: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;
    (sink, session)
}

/// Opening a repository asks for a read, and so does the window becoming
/// active a moment later; on a large repository that pair was two
/// `for-each-ref` and two `status -uall` for one answer. The second
/// caller now books a repeat instead of starting its own — and the point
/// of booking rather than dropping is that the repeat still sees what
/// happened in between.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_made_while_a_read_runs_gets_a_read_of_its_own() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.wait_for("the opening status read", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { .. }))
            .then_some(())
    })
    .await;

    // Park a read on its way out, which is where a reader stands after it
    // has seen the repository and before it asks whether to go round
    // again. Everything below happens inside that window.
    let (arrived, at_the_window) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::StatusLoaded { status, .. } if !status.is_dirty()),
        move || {
            let _ = arrived.send(());
            let _ = held.recv_timeout(Duration::from_secs(20));
        },
    );
    session.refresh_status();
    at_the_window.await.expect("the read reached the window");

    // The tree turns dirty behind the parked read — so what it is holding
    // is already out of date — and somebody asks again. Dropping that ask
    // for being a duplicate is what this is here to catch: the only read
    // that could answer it is the one that already looked.
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    release.send(()).expect("let the read finish");

    sink.wait_for("a status read that sees the dirty tree", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { status, .. } if status.is_dirty()))
            .then_some(())
    })
    .await;
    session.close();
}

/// Every command the session recorded, in order.
fn commands_of(sink: &CaptureSink) -> Vec<String> {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .collect()
}

/// The refs listing already marks the branch HEAD is on, so a refs read
/// does not ask a second and third process where HEAD is.
#[tokio::test(flavor = "multi_thread")]
async fn a_refs_read_takes_head_out_of_the_listing_it_already_has() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    // Let the opening pipeline finish first: it walks the graph, and that
    // walk asks where HEAD is by a path of its own.
    sink.settled_stream_gen(1).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    session.refresh_refs();
    sink.wait_for("the listing", |evs| {
        evs.iter()
            .any(|e| {
                matches!(e, SessionEvent::CommandStarted { display, .. }
                              if display.contains("for-each-ref"))
            })
            .then_some(())
    })
    .await;
    // Give the two it used to spawn every chance to turn up.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let seen = commands_of(&sink);
    assert!(
        !seen.iter().any(|c| c.contains("symbolic-ref")),
        "HEAD came out of the listing: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c.contains("rev-parse --verify")),
        "and so did the commit it is on: {seen:?}"
    );
    session.close();
}

/// Detached HEAD is the case the listing cannot answer — no ref is marked
/// — and it still gets a right answer, by asking.
#[tokio::test(flavor = "multi_thread")]
async fn a_detached_head_is_still_read_correctly() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("g.txt", "1\n", "second");
    repo.git(&["checkout", "--detach", &root]);

    let (sink, session) = opened(&repo).await;
    let head = sink
        .wait_for("the refs snapshot", |evs| {
            evs.iter().rev().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot } => snapshot.head.clone(),
                _ => None,
            })
        })
        .await;
    assert!(head.detached, "{head:?}");
    assert_eq!(head.branch, None);
    assert_eq!(head.oid.map(|o| o.to_hex()), Some(root));
    session.close();
}

/// A read that finds nothing moved publishes the snapshot it published
/// last — the same one, by pointer — instead of building an equal one.
///
/// It used to sort every ref into a snapshot and a label map on every
/// tick and then compare the result with the last to be told nothing had
/// changed: 39ms of a core against `JetBrains/kotlin`, ten seconds apart,
/// for an answer the key already had.
#[tokio::test(flavor = "multi_thread")]
async fn an_unmoved_repository_republishes_the_snapshot_it_already_built() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.git(&["branch", "side"]);
    let (sink, session) = opened(&repo).await;

    let latest = |sink: &CaptureSink| {
        sink.events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot } => Some(Arc::clone(snapshot)),
                _ => None,
            })
            .expect("a snapshot")
    };
    let settle = async |want: usize| {
        sink.wait_for("a refs snapshot", move |evs| {
            (evs.iter()
                .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
                .count()
                >= want)
                .then_some(())
        })
        .await
    };

    settle(1).await;
    let first = latest(&sink);
    let before = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));

    session.refresh_refs();
    settle(before + 1).await;
    let again = latest(&sink);
    assert!(
        Arc::ptr_eq(&first, &again),
        "the same snapshot, not an equal one"
    );

    // A ref really moving still rebuilds, and the sidebar is told.
    repo.git(&["branch", "-D", "side"]);
    let before = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));
    session.refresh_refs();
    settle(before + 1).await;
    let after = latest(&sink);
    assert!(
        !Arc::ptr_eq(&first, &after),
        "a moved ref is a new snapshot"
    );
    assert_eq!(
        after.locals.len(),
        1,
        "and it is the repository as it stands: {:?}",
        after.locals
    );
    session.close();
}

/// Once a refs read has said where HEAD is, the walk stops asking.
///
/// It used to spawn `symbolic-ref` and `rev-parse` before every rebuild —
/// two processes in front of the first chunk, on the path a commit or a
/// fetch takes to reach the screen.
#[tokio::test(flavor = "multi_thread")]
async fn the_walk_reads_head_from_the_refs_read_that_already_landed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.settled_stream_gen(1).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    // An external commit moves the refs, which is what rebuilds the graph.
    repo.commit_file("g.txt", "1\n", "second");
    session.refresh_refs();
    sink.settled_stream_gen(2).await;

    let seen = commands_of(&sink);
    assert!(
        seen.iter().any(|c| c.contains("log -z")),
        "the walk did run: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c.contains("symbolic-ref")),
        "and did not ask where HEAD is: {seen:?}"
    );
    session.close();
}

/// The remotes are read once per refs listing no more: a poll tick that
/// finds nothing moved spawns no `git config` to re-read them. A write
/// puts the question back, because a write is what can add one.
#[tokio::test(flavor = "multi_thread")]
async fn the_remotes_are_read_once_until_something_could_have_changed_them() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.settled_stream_gen(1).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    let reads = |sink: &CaptureSink| {
        commands_of(sink)
            .iter()
            .filter(|c| c.contains("remote\\..*\\.(url|pushurl)"))
            .count()
    };
    // Counted in snapshots delivered, not in elapsed time: a read that is
    // merely slow must not read as a read that did not happen.
    let snapshots =
        |sink: &CaptureSink| sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. })) as u32;
    let settle = async |want: u32| {
        sink.wait_for("a refs snapshot", move |evs| {
            (evs.iter()
                .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
                .count() as u32
                >= want)
                .then_some(())
        })
        .await
    };

    // One read warms it — the opening listing found the refs where it had
    // never seen them before, which counts as a move and drops what was
    // read once.
    session.refresh_refs();
    settle(1).await;
    assert_eq!(reads(&sink), 1, "{:?}", commands_of(&sink));

    // Now nothing moves, and the listings that follow ask git nothing.
    let from = snapshots(&sink);
    for n in 1..=3 {
        session.refresh_refs();
        settle(from + n).await;
    }
    assert_eq!(reads(&sink), 1, "still the one: {:?}", commands_of(&sink));

    // A write can add one, so the answer is dropped and asked again.
    session.create_branch("side".into(), None, false);
    write_result(&sink, "branch").await;
    sink.wait_for("the remotes read again", |evs| {
        evs.iter()
            .any(|e| {
                matches!(e, SessionEvent::CommandStarted { display, .. }
                              if display.contains("remote\\..*\\.(url|pushurl)"))
            })
            .then_some(())
    })
    .await;
    session.close();
}

/// Whether git normalises line endings is repository configuration, so it
/// is read once however many diffs are opened.
#[tokio::test(flavor = "multi_thread")]
async fn the_line_ending_setting_is_read_once_for_the_repository() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let (sink, session) = opened(&repo).await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    let head = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    let parent = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD^"])).unwrap();
    for path in ["a.txt", "b.txt"] {
        session.load_diff(DiffTarget::Commit {
            oid: head,
            parent: Some(parent),
            path: path.to_string(),
            orig_path: None,
        });
        sink.wait_for("the diff", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::DiffLoaded { target, .. }
                                  if matches!(target, DiffTarget::Commit { path: p, .. } if p == path)))
                .then_some(())
        })
        .await;
    }

    let reads = commands_of(&sink)
        .iter()
        .filter(|c| c.contains("autocrlf"))
        .count();
    assert_eq!(reads, 1, "{:?}", commands_of(&sink));
    session.close();
}

/// Waits for the write named `op` to finish and returns git's error, if any.
async fn write_result(sink: &CaptureSink, op: &'static str) -> Option<String> {
    sink.wait_for(op, |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteFinished { op: got, error } if *got == op => Some(error.clone()),
            _ => None,
        })
    })
    .await
}

/// A move that fails for a reason a stash cannot help with — a name git
/// rejects — stops there: nothing is stashed, so the uncommitted work is
/// still in the tree where its owner left it.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_fails_outright_stashes_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.write_file("f.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "no-such-branch".into(),
    });
    assert!(
        write_result(&sink, "checkout").await.is_some(),
        "git rejected the branch name"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "uncommitted\n",
        "the work never left the tree"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "no entry left behind");
    session.close();
}

/// A move still refused once the tree has been emptied: whatever is
/// holding it is not something a stash gets past, so the work goes back
/// where it was and git's refusal is what comes out.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_nothing_can_unblock_puts_the_stashed_work_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("x.txt", "base\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("x.txt", "theirs\n", "other");
    repo.git(&["switch", "main"]);
    // Hidden from status and from the stash, but still in the move's way:
    // git will not write over what it was told to stop looking at.
    repo.write_file("x.txt", "mine\n");
    repo.git(&["update-index", "--skip-worktree", "--", "x.txt"]);
    repo.write_file("left.txt", "mine too\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    let error = write_result(&sink, "checkout").await;
    assert!(
        error.is_some_and(|e| e.contains("would be overwritten")),
        "git's first refusal is the one worth reporting"
    );

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert!(
        repo.git(&["stash", "list"]).is_empty(),
        "the entry went back where it came from"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("left.txt")).unwrap(),
        "mine too\n",
        "the work is where it was before the refused move"
    );
    session.close();
}

/// Two branches that disagree about `both.txt`, HEAD on `main`.
fn colliding_branches() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "base\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "theirs\n", "other");
    repo.git(&["switch", "main"]);
    repo
}

/// The move git will not make itself is made the long way round instead,
/// with nothing asked: stash, switch, put back — the sequence a person
/// would type (デザイン規約 §未コミット変更がある状態での移動).
#[tokio::test(flavor = "multi_thread")]
async fn a_move_git_refuses_goes_round_through_a_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "l1\nl2\nl3\nl4\nl5\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "l1-THEIRS\nl2\nl3\nl4\nl5\n", "other");
    repo.git(&["switch", "main"]);
    // Collides with `other` (the file differs there), so the plain switch
    // is refused — but on another line, so the restore merges it cleanly.
    repo.write_file("both.txt", "l1\nl2\nl3\nl4\nl5-MINE\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, "checkout").await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("both.txt")).unwrap(),
        "l1-THEIRS\nl2\nl3\nl4\nl5-MINE\n",
        "both sides of the file survived"
    );
    assert!(
        repo.git(&["stash", "list"]).is_empty(),
        "a clean restore takes the stash with it"
    );
    session.close();
}

/// Going round through the stash is what keeps the staged/unstaged split
/// — the reason the move is not `switch --merge`, which refuses outright
/// while anything is staged.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_goes_round_keeps_what_was_staged_staged() {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "l1\nl2\nl3\nl4\nl5\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "l1-THEIRS\nl2\nl3\nl4\nl5\n", "other");
    repo.git(&["switch", "main"]);
    repo.write_file("both.txt", "l1\nl2\nl3\nl4\nl5-MINE\n");
    repo.write_file("staged.txt", "staged\n");
    repo.git(&["add", "--", "staged.txt"]);

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, "checkout").await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("both.txt")).unwrap(),
        "l1-THEIRS\nl2\nl3\nl4\nl5-MINE\n",
        "both sides of the file survived"
    );
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]),
        "staged.txt",
        "what was staged is staged still"
    );
    session.close();
}

/// The same move when the sides cannot be combined: git leaves the markers
/// and keeps the stash, and neither is a failure to report — the work is
/// across, waiting to be settled, and still recoverable from the stash.
/// This is the display a person typing the three commands would land on.
#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_carry_leaves_the_stash_as_the_way_back() {
    let mut repo = colliding_branches();
    repo.write_file("both.txt", "mine\n");
    repo.git(&["add", "--", "both.txt"]);

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(
        write_result(&sink, "checkout").await,
        None,
        "a conflict is the outcome that was asked for, not an error"
    );

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    let both = std::fs::read_to_string(repo.path.join("both.txt")).unwrap();
    assert!(both.contains("<<<<<<<") && both.contains("mine"), "{both}");
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "left unmerged for the merge tool"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "the entry stays, so the work exists outside the marked-up file"
    );
    session.close();
}

/// Somebody else's entry sits at `stash@{0}` when the move begins. The one
/// this makes goes on top and is the only one it may put back — pop the
/// wrong one and work nobody asked about lands in the tree.
#[tokio::test(flavor = "multi_thread")]
async fn a_carry_leaves_other_stashes_alone() {
    let mut repo = colliding_branches();
    // Somebody's earlier work, parked before any of this.
    repo.write_file("both.txt", "parked work\n");
    repo.git(&["stash", "push", "-m", "parked"]);
    repo.write_file("both.txt", "mine\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, "checkout").await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    let list = repo.git(&["stash", "list"]);
    assert!(
        list.contains("parked"),
        "the parked entry was not ours to pop: {list}"
    );
    session.close();
}

/// A restore that really cannot land still reports. The untracked file the
/// target tracks has nowhere to go — but every tracked change travels
/// anyway, and the entry stays as the way back to the one that did not.
#[tokio::test(flavor = "multi_thread")]
async fn a_carry_that_cannot_restore_reports_gits_message() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("theirs.txt", "only over there\n", "other");
    repo.git(&["switch", "main"]);
    repo.write_file("theirs.txt", "mine, uncommitted\n");
    repo.write_file("seed.txt", "seed\nand a tracked edit\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    let error = write_result(&sink, "checkout").await;
    assert!(
        error.is_some_and(|e| e.contains("untracked")),
        "git's own wording goes through"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("seed.txt")).unwrap(),
        "seed\nand a tracked edit\n",
        "the tracked edit came across regardless"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "the entry is the way back to the file that stayed behind"
    );
    session.close();
}

/// A pop whose restore conflicts lands exactly where an apply would have:
/// git keeps the entry, and the conflict is the outcome that was asked
/// for, not a failure to report (デザイン規約 §変更を退避する).
#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_pop_keeps_the_entry_and_is_not_a_failure() {
    let mut repo = colliding_branches();
    repo.write_file("both.txt", "mine\n");
    repo.git(&["stash", "push", "-u"]);
    repo.git(&["switch", "other"]);

    let (sink, session) = opened(&repo).await;
    session.stash_pop("stash@{0}".into());
    assert_eq!(
        write_result(&sink, "stash").await,
        None,
        "the restore landed; it just needs settling"
    );

    let both = std::fs::read_to_string(repo.path.join("both.txt")).unwrap();
    assert!(both.contains("<<<<<<<") && both.contains("mine"), "{both}");
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "left unmerged to be settled"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "kept, the way an apply would have kept it"
    );
    session.close();
}

/// The same reading must not swallow a pop that did nothing. git refuses
/// to restore onto an index that already has unmerged paths, and the
/// conflicts standing there afterwards are the old ones — so the tree
/// alone cannot judge it, and what it was before decides.
#[tokio::test(flavor = "multi_thread")]
async fn a_pop_refused_by_a_conflicted_tree_is_still_a_failure() {
    let mut repo = colliding_branches();
    repo.git(&["switch", "-c", "mine", "main"]);
    repo.commit_file("both.txt", "ours\n", "mine");
    // An entry that has nothing to do with the conflict below.
    repo.write_file("spare.txt", "parked\n");
    repo.git(&["stash", "push", "-u"]);
    // A conflicting merge exits 1, which is the point of it.
    repo.git_expect_failure(&["merge", "other"]);
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "the merge stopped on a conflict"
    );

    let (sink, session) = opened(&repo).await;
    session.stash_pop("stash@{0}".into());
    let error = write_result(&sink, "stash").await;
    assert!(
        error.is_some(),
        "the refusal is not read as a landed conflict"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "nothing was restored, so nothing was dropped"
    );
    assert!(!repo.path.join("spare.txt").exists(), "still in the entry");
    session.close();
}

/// Puts the todo-editor helper where the session looks for it — beside the
/// running executable, which for a test is the test binary's own directory.
/// Packaging carries the same obligation for the application.
///
/// Once per process, and the file is published by `rename` rather than
/// written where it stands. Both halves are about the same thing: on Linux
/// a file somebody holds open for writing cannot be executed at all
/// (`ETXTBSY`), and all five replaying tests call this and then hand the
/// path to git. Copying straight onto it put one test's write fd under
/// another's exec — `pg-todo-editor: Text file busy`, reported by the `sh`
/// git runs `GIT_SEQUENCE_EDITOR` through, on 6 runs out of 8 with a thread
/// per core (実測 24 cores; 規約 §テストが差し込む実行ファイルは rename で置く).
fn install_todo_editor() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    // Every caller waits for the one copy, so no test reaches git while it
    // is in flight; the rename covers the rest — another process sharing
    // this `target/` never sees a partly-written helper either.
    INSTALLED.call_once(|| {
        let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"));
        let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        else {
            return;
        };
        let _ = publish_helper(&built, &dir);
    });
}

/// Copies `built` into `dir` under the name the application looks for,
/// through a staging name of its own so the live path is never opened for
/// writing. Returns where it put it.
fn publish_helper(
    built: &std::path::Path,
    dir: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    let name = format!(
        "{}{}",
        platitude_core::sequencer::HELPER_NAME,
        std::env::consts::EXE_SUFFIX
    );
    let beside = dir.join(&name);
    if beside == built {
        return Ok(beside);
    }
    // One staging name per process is enough: `INSTALLED` means one copy
    // runs at a time, and a second process gets a name of its own.
    let staged = dir.join(format!("{name}.{}.staged", std::process::id()));
    let published = std::fs::copy(built, &staged).and_then(|_| std::fs::rename(&staged, &beside));
    if let Err(error) = published {
        // Windows locks a running executable, so the rename can lose to a
        // helper another run left behind — the one that is there will do.
        let _ = std::fs::remove_file(&staged);
        if !beside.is_file() {
            return Err(error);
        }
    }
    Ok(beside)
}

/// Runs a helper this suite published, retrying while the kernel answers
/// that somebody still holds the file open for writing (`ETXTBSY`).
///
/// `rename` keeps the inode, so what gets published is the very file
/// `fs::copy` had open for writing a moment earlier — and that write
/// reference can outlive the copy. `i_writecount` is counted per open
/// file description, and a thread that forks git mid-copy hands the child
/// a reference to the same one; `CLOEXEC` closes it, but not before the
/// child's `execve`. Until then the file cannot be executed at all. The
/// forking thread never sees its own window — `spawn` returns when the
/// child's `execve` closes the error pipe, so it is already past — but
/// with a thread per core the suite is forking git constantly and every
/// neighbour sees it. It belongs to another process's scheduling, and
/// this one cannot time it (実測: a child made to sleep 300ms between
/// fork and `execve` refuses a neighbour's exec for exactly that long).
///
/// Hence a retry on the error, not a wait for the window: every attempt
/// is the real run, and the first answer that is not "busy" is the answer
/// — a busy one at the end of the budget included, which reaches the
/// caller as the failure it is. (cargo and rustup carry the same loop for
/// the same reason, around the binaries they have just written.)
#[cfg(unix)]
fn run_published_helper(path: &std::path::Path) -> std::io::Result<std::process::Output> {
    // A second in all (40 × 25ms), which spans a fork→exec on a loaded
    // machine many times over, and is paid only while it really is busy.
    let mut retries = 40;
    loop {
        let answer = std::process::Command::new(path).output();
        let busy = matches!(
            &answer,
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy
        );
        if !busy || retries == 0 {
            return answer;
        }
        retries -= 1;
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// The install must replace the helper's directory entry, never write
/// through it: a replay running the old one has it open for execution, and
/// on Linux that makes it unwritable (`ETXTBSY`) in one direction and
/// unexecutable in the other. A new inode under the same name settles both
/// — whoever is mid-exec keeps the file they started, and the next replay
/// gets the fresh one.
#[test]
#[cfg(unix)]
fn the_helper_is_replaced_rather_than_written_over() {
    use std::os::unix::fs::MetadataExt;

    let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"));
    let dir = tempfile::tempdir().expect("tempdir");
    let first = publish_helper(&built, dir.path()).expect("install");
    let before = std::fs::metadata(&first).expect("stat").ino();

    let again = publish_helper(&built, dir.path()).expect("install over the first");
    assert_eq!(again, first, "the same name both times");
    assert_ne!(
        std::fs::metadata(&again).expect("stat").ino(),
        before,
        "the second install wrote through the live path"
    );

    // And what landed is still the helper: `fs::copy` carries the mode, so
    // the file it publishes is one git can execute.
    let out = run_published_helper(&again).expect("run the installed helper");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("usage: pg-todo-editor"),
        "the installed file is not the helper: {out:?}"
    );
}

/// The other half of that: publishing by `rename` settles who wins a race
/// between two copies, and settles nothing about a write handle already
/// open on the inode it publishes. Under load the suite hits that as a
/// forked git holding the copy's fd until its own `execve` — a window of
/// somebody else's making, too short to catch on purpose. Held open here
/// on purpose instead, since what the runner has to survive is the error,
/// not the fork: one attempt is refused outright, and the run that keeps
/// asking gets its answer as soon as the handle goes.
///
/// Linux rather than every unix, because POSIX only says `execve` *may*
/// refuse a file open for writing — this asserts that it does, which is
/// a promise Linux makes and the container is the machine that keeps it.
#[test]
#[cfg(target_os = "linux")]
fn a_helper_held_open_for_writing_is_run_once_the_handle_goes() {
    let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"));
    let dir = tempfile::tempdir().expect("tempdir");
    let published = publish_helper(&built, dir.path()).expect("install");

    // Opened, not truncated: the file stays the helper throughout.
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(&published)
        .expect("hold the published helper open for writing");
    let refused = std::process::Command::new(&published)
        .output()
        .expect_err("a file open for writing is not executable on linux");
    assert_eq!(refused.kind(), std::io::ErrorKind::ExecutableFileBusy);

    let letting_go = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        drop(handle);
    });
    let out = run_published_helper(&published).expect("run the installed helper");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("usage: pg-todo-editor"),
        "the installed file is not the helper: {out:?}"
    );
    letting_go.join().expect("the holder thread");
}

#[tokio::test(flavor = "multi_thread")]
async fn squash_and_reword_run_through_the_write_queue() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "keep me");
    let fold = repo.commit_file("c.txt", "three\n", "fold me in");

    let (sink, session) = opened(&repo).await;
    session.squash_into_parent(fold);
    assert_eq!(write_result(&sink, "squash").await, None);
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");

    // Rewording HEAD takes the amend path: no replay, same parent.
    let parent = repo.git(&["rev-parse", "HEAD~1"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    session.reword(head, "reworded head\n".into());
    assert_eq!(write_result(&sink, "reword").await, None);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "reworded head");
    assert_eq!(repo.git(&["rev-parse", "HEAD~1"]), parent);
    session.close();
}

/// The commands a rewrite issued, named coarsely enough to read as the
/// route it took rather than as an argument list.
///
/// Matched from the front of the command, not anywhere inside it: the
/// status refresh that follows a stopped rebase reads its progress with
/// `rev-parse --git-path rebase-merge/msgnum`, which a plain `contains`
/// counts as a fourth rebase (実測).
fn rewrite_route(sink: &CaptureSink) -> Vec<&'static str> {
    let mut out = Vec::new();
    for event in sink.events.lock().unwrap().iter() {
        let SessionEvent::CommandStarted { display, .. } = event else {
            continue;
        };
        // Longest first: "stash pop --index" also starts with "stash
        // pop", and every interactive rebase with "rebase".
        for step in [
            "rebase --interactive",
            "stash push",
            "stash pop --index",
            "stash pop",
            "rebase",
        ] {
            if display.starts_with(&format!("git {step}")) {
                out.push(step);
                break;
            }
        }
    }
    out
}

/// Whether git is part-way through something.
fn stopped_part_way(repo: &TestRepo) -> bool {
    repo.path.join(".git").join("rebase-merge").exists()
}

/// A squash fired over a dirty tree neither stops nor asks: git refuses to
/// replay while the work is in the tree, so the session goes round the way
/// a person typing the three commands would (デザイン規約
/// §未コミット変更がある状態で履歴を書き換える). What lands is what `stash` →
/// `squash` → `stash pop --index` leaves — **the staged and unstaged
/// halves still told apart**, which is the one thing `--autostash` cannot
/// do: it restores with a plain apply and everything comes back unstaged
/// (実測 2.55).
#[tokio::test(flavor = "multi_thread")]
async fn a_squash_over_a_dirty_tree_carries_the_work_across() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "keep me");
    let fold = repo.commit_file("c.txt", "three\n", "fold me in");
    repo.write_file("a.txt", "staged edit\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("b.txt", "unstaged edit\n");
    repo.write_file("u.txt", "untracked\n");

    let (sink, session) = opened(&repo).await;
    session.squash_into_parent(fold);
    assert_eq!(write_result(&sink, "squash").await, None);

    assert_eq!(
        rewrite_route(&sink),
        vec![
            // Refused, without touching anything.
            "rebase --interactive",
            "stash push",
            "rebase --interactive",
            "stash pop --index",
        ]
    );
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(
        repo.git(&["diff", "--name-only", "--cached"]),
        "a.txt",
        "the staged half is still staged"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only"]),
        "b.txt",
        "and the unstaged half still is not"
    );
    assert_eq!(
        repo.git(&["ls-files", "--others", "--exclude-standard"]),
        "u.txt"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
    session.close();
}

/// What a `rebase <current> onto it` fires with: the flags the two menu
/// rows pass, which no longer include an autostash knob to pass.
fn rebase_onto() -> platitude_core::integrate::RebaseOptions {
    platitude_core::integrate::RebaseOptions {
        update_refs: true,
        ..Default::default()
    }
}

/// A whole branch moved onto a new base goes round the very same way, so
/// the answer to "does my staging survive a history rewrite" does not
/// depend on which menu row was clicked. This is the operation that used
/// to be handed to `--autostash`, which restores with a plain apply and
/// brings **everything back unstaged** — the split below is exactly what
/// that flag cannot keep (実測 2.55).
#[tokio::test(flavor = "multi_thread")]
async fn a_rebase_onto_over_a_dirty_tree_carries_the_work_across() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["switch", "-c", "topic"]);
    repo.commit_file("t.txt", "topic\n", "topic one");
    repo.git(&["switch", "main"]);
    repo.commit_file("m.txt", "main\n", "main moved");
    repo.git(&["switch", "topic"]);
    repo.write_file("a.txt", "staged edit\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("t.txt", "unstaged edit\n");
    repo.write_file("u.txt", "untracked\n");

    let (sink, session) = opened(&repo).await;
    session.rebase("main".into(), rebase_onto());
    assert_eq!(write_result(&sink, "rebase").await, None);

    assert_eq!(
        rewrite_route(&sink),
        vec![
            // Refused, without touching anything.
            "rebase",
            "stash push",
            "rebase",
            "stash pop --index",
        ]
    );
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "3");
    assert_eq!(
        repo.git(&["diff", "--name-only", "--cached"]),
        "a.txt",
        "the staged half is still staged"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only"]),
        "t.txt",
        "and the unstaged half still is not"
    );
    assert_eq!(
        repo.git(&["ls-files", "--others", "--exclude-standard"]),
        "u.txt"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
    session.close();
}

/// And when that rebase stops on a conflict, the work waits in the stash
/// exactly as a stopped replay's does — no restore is attempted over a
/// tree git is still holding. This is what the alignment gives up:
/// `--autostash` would have put the work back itself after `--continue`
/// or `--abort` (実測), whereas this entry is the person's to pop.
#[tokio::test(flavor = "multi_thread")]
async fn a_rebase_onto_that_stops_leaves_the_work_in_the_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.commit_file("keep.txt", "keep\n", "second");
    repo.git(&["switch", "-c", "topic"]);
    repo.commit_file("f.txt", "topic's line\n", "topic one");
    repo.git(&["switch", "main"]);
    repo.commit_file("f.txt", "main's line\n", "main moved");
    repo.git(&["switch", "topic"]);
    repo.write_file("keep.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.rebase("main".into(), rebase_onto());
    assert!(
        write_result(&sink, "rebase").await.is_some(),
        "git's own message about where it stopped goes through"
    );

    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase", "stash push", "rebase"],
        "no restore is attempted over a tree git is still holding"
    );
    assert!(stopped_part_way(&repo), "the operation is waiting");
    assert_eq!(repo.git(&["stash", "list"]).lines().count(), 1);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("keep.txt")).expect("read"),
        "keep\n",
        "the uncommitted edit is in the entry, not in the tree"
    );
    session.close();
}

/// Untracked files are not in the way of a replay at all (実測: git takes
/// the plan and leaves them where they are), so no stash is taken for
/// them. The route is the whole assertion — a needless stash would still
/// have ended with the same working tree.
#[tokio::test(flavor = "multi_thread")]
async fn untracked_files_alone_are_replayed_straight_over() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let gone = repo.commit_file("b.txt", "two\n", "drop me");
    repo.commit_file("c.txt", "three\n", "keep me");
    repo.write_file("u.txt", "untracked\n");

    let (sink, session) = opened(&repo).await;
    session.drop_commit(gone);
    assert_eq!(write_result(&sink, "drop").await, None);

    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase --interactive"],
        "one command, no stash"
    );
    assert!(!repo.path.join("b.txt").exists(), "the commit went");
    assert_eq!(
        repo.git(&["ls-files", "--others", "--exclude-standard"]),
        "u.txt"
    );
    session.close();
}

/// The replay goes through and the *restore* is what collides. The
/// landing is the one a move already has (規約 §未コミット変更がある状態
/// での移動): markers in the files, and the stash entry kept so the work
/// still exists somewhere other than a marked-up file. Not a failed
/// write — nothing failed.
#[tokio::test(flavor = "multi_thread")]
async fn a_restore_that_collides_lands_in_the_files_and_keeps_the_entry() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("file.txt", "a\nb\nc\n", "root");
    let gone = repo.commit_file("file.txt", "a\nmiddle\nc\n", "drop me");
    repo.commit_file("other.txt", "side\n", "keep me");
    // Touches the same line the dropped commit did, and nothing the
    // replay itself has to apply.
    repo.write_file("file.txt", "a\nlocal\nc\n");

    let (sink, session) = opened(&repo).await;
    session.drop_commit(gone);
    assert_eq!(write_result(&sink, "drop").await, None, "nothing failed");

    assert_eq!(
        rewrite_route(&sink),
        vec![
            "rebase --interactive",
            "stash push",
            "rebase --interactive",
            "stash pop --index",
        ]
    );
    assert!(!stopped_part_way(&repo), "the replay itself finished");
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"]),
        "file.txt",
        "the collision is in the file, waiting to be settled"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "and the work is still in the stash as well"
    );
    session.close();
}

/// The replay stops part-way, and then the restore is not attempted: git
/// will not write into an index that already holds unmerged paths, so a
/// pop there does nothing while reporting the collision it walked into
/// (実測 `could not write index` / `needs merge` — 規約 §`stash pop` の
/// 非ゼロを conflict と読んでよいのは). The work waits in the stash,
/// drawn as its own row in the graph, until the operation is over.
#[tokio::test(flavor = "multi_thread")]
async fn a_replay_that_stops_part_way_leaves_the_work_in_the_stash() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "base\n", "root");
    repo.commit_file("file.txt", "one\n", "one");
    let gone = repo.commit_file("file.txt", "one\ntwo\n", "drop me");
    repo.commit_file("file.txt", "one\ntwo\nthree\n", "needs the one before");
    repo.write_file("base.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.drop_commit(gone);
    assert!(
        write_result(&sink, "drop").await.is_some(),
        "git's own message about where it stopped goes through"
    );

    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase --interactive", "stash push", "rebase --interactive"],
        "no restore is attempted over a tree git is still holding"
    );
    assert!(stopped_part_way(&repo), "the operation is waiting");
    assert_eq!(repo.git(&["stash", "list"]).lines().count(), 1);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("base.txt")).expect("read"),
        "base\n",
        "the uncommitted edit is in the entry, not in the tree"
    );
    session.close();
}

/// Taking the branch back a commit runs as a queued write of its own,
/// under the name the page keys its follow-up off: a reset rewrites the
/// working tree the diff on screen was read from.
#[tokio::test(flavor = "multi_thread")]
async fn a_reset_moves_the_branch_through_the_write_queue() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("f.txt", "0\n", "root");
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

/// Waits for the `nth` automatic fetch to finish and returns git's error,
/// if any. Telling them apart is the point: only the second one can be laid
/// at the clock's door.
async fn auto_fetch_done(sink: &CaptureSink, nth: usize) -> Option<String> {
    sink.wait_for("an automatic fetch", |evs| {
        evs.iter()
            .filter_map(|e| match e {
                SessionEvent::WriteFinished { op, error }
                    if *op == platitude_core::session::AUTO_FETCH_OP =>
                {
                    Some(error.clone())
                }
                _ => None,
            })
            .nth(nth - 1)
    })
    .await
}

/// The auto-fetch timer runs the fetch it promises, and turns off again.
///
/// Both halves are read off the timer — a tick it takes, a tick it refuses
/// once stopped — rather than off a stretch of quiet clock: a fetch queued
/// a moment before the stop starts whenever the write queue reaches it,
/// which on a loaded machine is long after any margin worth waiting, so no
/// amount of silence tells "stopped" from "slow".
#[tokio::test(flavor = "multi_thread")]
async fn auto_fetch_runs_on_its_interval_and_stops() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);

    let (sink, session) = opened(&clone).await;
    // An hour, so nothing but the tick below can fire this one and the
    // fetch that follows is that tick's doing and nothing else's.
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    let hourly = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(hourly.tick().await, "the running timer took the tick");
    assert_eq!(
        auto_fetch_done(&sink, 1).await,
        None,
        "the file:// remote fetched cleanly"
    );
    assert_eq!(
        clone.git(&["rev-parse", "origin/main"]),
        origin.git(&["rev-parse", "main"]),
    );

    // Now hand it to the clock. The hourly timer is replaced, so a second
    // fetch can only be the new interval's.
    session.set_auto_fetch(Some(Duration::from_millis(120)));
    assert!(
        !hourly.tick().await,
        "setting an interval stops the timer it replaces"
    );
    let ticking = session.auto_fetch_ticker().expect("auto fetch is on");
    assert_eq!(
        auto_fetch_done(&sink, 2).await,
        None,
        "the interval came round and fetched on its own"
    );

    session.set_auto_fetch(None);
    assert!(
        !ticking.tick().await,
        "turning it off stops the timer, so no further fetch can start"
    );
    session.close();
}

/// Suspending stops the timer without forgetting what it was set to, so
/// resuming needs no one to say the interval again — and a repository that
/// never had one says so, which is how the caller tells "stopped because it
/// kept failing" from "never fetched on its own in the first place".
#[tokio::test(flavor = "multi_thread")]
async fn a_suspended_timer_comes_back_on_the_interval_it_had() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (sink, session) = opened(&clone).await;
    assert!(
        !session.suspend_auto_fetch(),
        "nothing to suspend before an interval is ever set"
    );
    // Resuming what was never on leaves it off.
    session.resume_auto_fetch();
    assert!(
        session.auto_fetch_ticker().is_none(),
        "resume does not invent an interval of its own"
    );

    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    let before = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(session.suspend_auto_fetch(), "there was a timer to stop");
    assert!(
        !before.tick().await,
        "the suspended timer refuses the tick it would have taken"
    );
    assert!(
        session.auto_fetch_ticker().is_none(),
        "and there is no timer to reach while it is suspended"
    );
    assert!(
        !session.suspend_auto_fetch(),
        "suspending twice has nothing left to stop"
    );

    session.resume_auto_fetch();
    let after = session
        .auto_fetch_ticker()
        .expect("resume put the interval back");
    assert!(after.tick().await, "the timer that came back takes a tick");
    assert_eq!(
        auto_fetch_done(&sink, 1).await,
        None,
        "and the fetch it queued ran"
    );
    session.close();
}

/// A push refused for looking at an older remote is followed by a fetch,
/// so what the remote actually holds is on screen before anything else is
/// decided. The push itself still fails and nothing is retried.
#[tokio::test(flavor = "multi_thread")]
async fn a_push_refused_as_out_of_date_fetches_what_it_was_missing() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    origin.git(&["config", "core.bare", "true"]);

    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["reset", "--hard", "origin/main"]);
    clone.git(&["branch", "--set-upstream-to=origin/main", "main"]);
    let known = clone.git(&["rev-parse", "origin/main"]);

    // Someone else pushes while this clone is not looking.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &origin.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-B", "main", "origin/main"]);
    other.commit_file("theirs.txt", "t\n", "their work");
    other.git(&["push", "origin", "main"]);

    // Ours goes its own way, still believing the remote is where it was.
    clone.commit_file("mine.txt", "m\n", "my work");

    let (sink, session) = opened(&clone).await;
    session.push_current(String::new(), platitude_core::remote::PushForce::None);

    let error = sink
        .wait_for("the push to be refused", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::WriteFinished { op, error } if *op == "push" => Some(error.clone()),
                _ => None,
            })
        })
        .await;
    assert!(
        error.is_some_and(|e| e.contains("rejected")),
        "the refusal is reported as it stands"
    );

    sink.wait_for("the fetch that answers it", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::WriteFinished { op, error: None } if *op == "fetch"))
            .then_some(())
    })
    .await;
    assert_ne!(
        clone.git(&["rev-parse", "origin/main"]),
        known,
        "the tracking ref caught up, so the graph can show what would be overwritten"
    );
    assert_eq!(
        origin.git(&["log", "-1", "--format=%s", "main"]),
        "their work",
        "nothing was retried: the remote still holds only their commit"
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

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

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
                } => Some(error.clone()),
                _ => None,
            })
        })
        .await;
    assert!(error.is_some(), "the conflict is reported as a failure");

    // The refresh that follows a failed write carries the step counter.
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
    // pass; Linux does not (実測).
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

/// The publish check answers through its own event, so a UI can warn
/// before rewriting history a remote already has.
#[tokio::test(flavor = "multi_thread")]
async fn publish_check_answers_through_the_session() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

    session.check_publish("HEAD~1..HEAD".into());
    let state = sink
        .wait_for("PublishChecked", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::PublishChecked { range, state } if range == "HEAD~1..HEAD" => {
                    Some(*state)
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(state.total, 1);
    assert!(!state.rewrites_published(), "nothing is on a remote");
    session.close();
}

/// A write rebuilds the graph exactly once. Committing turns a dirty tree
/// clean, which removes the WIP row; reacting to that separately from the
/// write itself would stream the whole graph twice for one action.
#[tokio::test(flavor = "multi_thread")]
async fn a_write_rebuilds_the_graph_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    // Wait until the WIP row is on screen (root + WIP = 2 rows) and the
    // stream has settled, so the commit below is the transition that
    // removes it and every later stream event is a reaction to a write.
    sink.settled_stream_gen(2).await;

    session.stage_all();
    session.commit(
        "add new file".into(),
        platitude_core::commit::CommitOptions::default(),
    );

    // Counting from where each write finished ignores whatever the open
    // sequence was still doing, which a wall-clock delay would not.
    sink.wait_for("the commit's rebuild finished", |evs| {
        let commit_at = position_of(evs, "commit")?;
        evs[commit_at..]
            .iter()
            .any(|e| matches!(e, SessionEvent::LogReplaced { .. }))
            .then_some(())
    })
    .await;
    // Give a trailing second rebuild (the regression this guards against)
    // time to show up before counting.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let events = sink.events.lock().unwrap();
    let stage_at = position_of(&events, "stage").expect("stage finished");
    let commit_at = position_of(&events, "commit").expect("commit finished");
    assert_eq!(
        log_starts(&events[stage_at..commit_at]),
        0,
        "staging left the tree dirty, so the graph did not change"
    );
    assert_eq!(
        log_starts(&events[commit_at..]),
        0,
        "the rebuild replaces atomically; it never resets and re-streams"
    );
    assert_eq!(
        events[commit_at..]
            .iter()
            .filter(|e| matches!(e, SessionEvent::LogReplaced { .. }))
            .count(),
        1,
        "the commit replaced the rebuilt graph in exactly once"
    );
    drop(events);
    session.close();
}

/// Every event a log stream can emit — what "the graph stayed silent"
/// counts.
fn is_stream_event(e: &SessionEvent) -> bool {
    matches!(
        e,
        SessionEvent::LogStarted { .. }
            | SessionEvent::LogChunk { .. }
            | SessionEvent::LogFinished { .. }
            | SessionEvent::LogReplaced { .. }
            | SessionEvent::LogFailed { .. }
    )
}

/// Opens a session over `scenario()`, settles the first 5-row graph, then
/// holds `refresh` to silence: a background pass over an unchanged
/// repository must not emit a single stream event ("nothing happens" can
/// only be observed by giving the pass ample time to run). Returns the
/// stream-event count to measure "after" against. The quiet half of both
/// refresh entry points is the same promise, so it is written once.
async fn settled_and_silent(
    refresh: impl Fn(&Arc<RepoSession>),
) -> (TestRepo, Arc<CaptureSink>, Arc<RepoSession>, usize) {
    let (repo, _) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(5).await;
    let baseline = sink.count(is_stream_event);

    refresh(&session);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        sink.count(is_stream_event),
        baseline,
        "an unchanged rebuild stayed silent: {:?}",
        sink.events.lock().unwrap()
    );
    (repo, sink, session, baseline)
}

/// A background rebuild that finds nothing changed must stay silent — no
/// reset, no chunk, no repaint. This is what keeps a quiet auto-fetch
/// interval (or any other background refresh) from flickering the graph.
#[tokio::test(flavor = "multi_thread")]
async fn background_refresh_swaps_only_on_change() {
    let (mut repo, sink, session, baseline) = settled_and_silent(|s| s.refresh_log()).await;

    // History moved outside the session: the same call now delivers one
    // atomic replacement — a single LogReplaced carrying every row, so
    // the consumer never holds an empty model in between.
    repo.commit_file("h.txt", "x\n", "outside commit");
    session.refresh_log();
    let swap_gen = sink.settled_stream_gen(6).await;
    let events = sink.events.lock().unwrap();
    let after: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| is_stream_event(e))
        .skip(baseline)
        .collect();
    assert_eq!(after.len(), 1, "one event for the whole change: {after:?}");
    match after[0] {
        SessionEvent::LogReplaced {
            generation, rows, ..
        } => {
            assert_eq!(*generation, swap_gen);
            assert_eq!(rows.len(), 6, "the replacement carries the whole graph");
        }
        other => panic!("expected LogReplaced, got {other:?}"),
    }
    drop(events);
    session.close();
}

/// A ref that moved outside the session (a commit in a terminal, a fetch,
/// a switch by another tool) points at commits this graph has never
/// walked, so re-reading the refs has to rebuild — chips alone cannot show
/// them. A re-read that finds every ref where it left it stays silent.
#[tokio::test(flavor = "multi_thread")]
async fn an_external_ref_move_rebuilds_the_graph() {
    let (mut repo, sink, session, _) = settled_and_silent(|s| s.refresh_refs()).await;

    // Now main moves under the session, with the working tree clean on
    // both sides: nothing but the refs can report this.
    repo.commit_file("outside.txt", "x\n", "outside commit");
    session.refresh_refs();
    sink.settled_stream_gen(6).await;
    let events = sink.events.lock().unwrap();
    let replacements = events[..]
        .iter()
        .skip_while(|e| !matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 6))
        .filter(|e| matches!(e, SessionEvent::LogReplaced { .. }))
        .count();
    assert_eq!(
        replacements,
        1,
        "the moved ref rebuilt once: {:?}",
        events[..].iter().collect::<Vec<_>>()
    );
    assert_eq!(
        log_starts(&events[..]),
        1,
        "the rebuild replaced in place; only opening resets and streams"
    );
    drop(events);
    session.close();
}

/// Chips are diffed against the graph that is on screen, so they are only
/// ever sent for that one. A rebuild landing in the middle of a refs read
/// moves every commit down a row (the WIP row goes in at the top), and row
/// numbers taken before it name other commits after it. Nothing takes such
/// a mistake back either: the session believes those chips are on screen,
/// so the next read has nothing to say and the next rebuild nothing to
/// swap.
///
/// The hook makes the interleaving exact rather than hoped for: it holds
/// the read at the sink call that publishes its snapshot while the test
/// rebuilds the graph under it.
#[tokio::test(flavor = "multi_thread")]
async fn chips_read_from_one_graph_do_not_land_on_another() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "middle");
    let head = repo.commit_file("f.txt", "2\n", "head");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(3).await;

    // Something for the read to find, on the last row of the graph it
    // reads it from: a chip that travels as a diff instead of with a walk.
    repo.git(&["tag", "v2", &root]);

    let (arrived, at_the_window) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| {
            matches!(e, SessionEvent::RefsLoaded { snapshot }
                if snapshot.tags.iter().any(|t| t.short == "v2"))
        },
        move || {
            let _ = arrived.send(());
            let _ = held.recv_timeout(Duration::from_secs(20));
        },
    );
    session.refresh_refs();
    at_the_window.await.expect("the read reached the window");

    // Rebuilt from here, with the read held: dirtying the tree puts the
    // WIP row at the top, so every row number that read took moves down
    // one.
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    sink.wait_for("the rebuild that adds the WIP row", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 4))
            .then_some(())
    })
    .await;
    release.send(()).expect("let the read finish");

    sink.settled_stream_gen(4).await;
    // Chips travel on an event of their own: let a late one land rather
    // than reading the graph before it could have arrived.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let events = sink.events.lock().unwrap();
    let rows = crate::support::replay_graph(&events[..]);
    let wearing = |name: &str| -> Vec<&str> {
        rows.values()
            .filter(|seen| seen.labels.iter().any(|l| l.text == name))
            .map(|seen| seen.oid_hex.as_str())
            .collect()
    };
    assert_eq!(
        wearing("v2"),
        vec![root.as_str()],
        "the tag reached the commit it names, and only it: {rows:?}"
    );
    assert_eq!(
        wearing("main"),
        vec![head.as_str()],
        "and the branch stayed where it was: {rows:?}"
    );
    drop(events);
    session.close();
}

/// A pass that was superseded before it could start leaves the graph
/// alone. Which pass is in charge is decided when somebody asks (both
/// entry points cancel the running token before spawning), not by the
/// order the tasks happen to reach the lock — so a reset that arrives
/// late must not clear what is on screen, wiping the record a rebuild
/// compares against and leaving every later chip diff numbered for a
/// graph nobody was ever shown.
///
/// Held under the graph lock, the interleaving is exact: the losing pass
/// cannot reach its reset before the cancel that supersedes it.
#[tokio::test(flavor = "multi_thread")]
async fn a_pass_nobody_asked_for_any_more_leaves_the_graph_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "middle");
    repo.commit_file("f.txt", "2\n", "head");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(3).await;

    // Park in the swap that adds the WIP row: it sends under the graph
    // lock, so everything else is stopped at the door with the graph
    // fully installed behind it.
    let (arrived, at_the_window) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 4),
        move || {
            let _ = arrived.send(());
            let _ = held.recv_timeout(Duration::from_secs(20));
        },
    );
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    at_the_window.await.expect("the rebuild reached the window");
    let settled = sink.count(|_| true);

    // Asked for, then superseded while it waits for the lock.
    session.restart_log();
    tokio::time::sleep(Duration::from_millis(200)).await;
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    // Long enough for both to have run: the superseded stream (which
    // only has to take the lock) and the rebuild behind it (a whole
    // walk, which then finds the graph unchanged and skips its swap).
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let events = sink.events.lock().unwrap();
    let after: Vec<&SessionEvent> = events[settled..]
        .iter()
        .filter(|e| {
            matches!(
                e,
                SessionEvent::LogStarted { .. } | SessionEvent::LogReplaced { .. }
            )
        })
        .collect();
    assert!(
        after.is_empty(),
        "nothing repainted the graph: {after:?}\nall: {:?}",
        events[settled..].iter().collect::<Vec<_>>()
    );

    let rows = crate::support::replay_graph(&events[..]);
    assert_eq!(rows.len(), 4, "the WIP row and three commits: {rows:?}");
    drop(events);
    session.close();
}

/// One tick, one rebuild. A commit made outside the session moves a ref
/// *and* turns the tree clean, and the poll reads both: walking the
/// history once per reader would throw a whole pass away every time
/// someone else commits.
#[tokio::test(flavor = "multi_thread")]
async fn a_poll_rebuilds_the_graph_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    // root + WIP row.
    sink.settled_stream_gen(2).await;

    let replacements = || sink.count(|e| matches!(e, SessionEvent::LogReplaced { .. }));
    let starts = || sink.count(|e| matches!(e, SessionEvent::LogStarted { .. }));
    let (quiet_replacements, quiet_starts) = (replacements(), starts());

    // An idle repository is what the poll spends nearly all its ticks on.
    let quiet_refs = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));
    let quiet_status = sink.count(|e| matches!(e, SessionEvent::StatusLoaded { .. }));
    session.refresh_poll();
    // What has to be over before the commit below is this tick, and the
    // tick says so itself: it publishes both of its reads whatever it
    // finds, so one more of each is it landing (規約 §「もう起きない」を
    // sleep で確かめない). A fixed wait here failed the assertion two
    // paragraphs down the moment the machine was busy enough for the tick
    // to outlast it — the poll's two reads then straddled the commit,
    // reported different worlds, and the graph rebuilt once for each.
    sink.wait_for("the idle poll's two reads", |evs| {
        let refs = evs
            .iter()
            .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
            .count();
        let status = evs
            .iter()
            .filter(|e| matches!(e, SessionEvent::StatusLoaded { .. }))
            .count();
        (refs > quiet_refs && status > quiet_status).then_some(())
    })
    .await;
    assert_eq!(
        (replacements(), starts()),
        (quiet_replacements, quiet_starts),
        "a poll over an unchanged repository stayed silent: {:?}",
        sink.events.lock().unwrap()
    );

    // Both signals move at once: `new.txt` becomes a commit, so the ref
    // advances and the WIP row goes away.
    repo.commit_file("new.txt", "content\n", "outside commit");
    // A poll steps aside while another one is still running, so the tick
    // that sees the commit need not be the first one asked for — the
    // ticker would simply ask again. Asking again cannot add a rebuild of
    // its own: a poll over a repository that has not moved is silent,
    // which is exactly what the paragraph above established.
    let rebuilt = sink.wait_for("the poll's rebuild", |evs| {
        evs.iter()
            .filter(|e| matches!(e, SessionEvent::LogReplaced { .. }))
            .count()
            .gt(&quiet_replacements)
            .then_some(())
    });
    tokio::pin!(rebuilt);
    loop {
        session.refresh_poll();
        tokio::select! {
            () = &mut rebuilt => break,
            () = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }
    // Give the second rebuild this guards against time to show up.
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        replacements(),
        quiet_replacements + 1,
        "the moved ref and the cleaned tree rebuilt once between them: {:?}",
        sink.events.lock().unwrap()
    );
    assert_eq!(
        starts(),
        quiet_starts,
        "the rebuild replaced in place; a poll never resets the graph"
    );
    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_command_log_holds_what_the_user_asked_for() {
    let (repo, _head) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;
    sink.settled_stream_gen(5).await;
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::CommandStarted { .. })),
        0,
        "opening reads a dozen times over; none of it is the user's doing"
    );

    session.stage_all();
    let id = sink
        .wait_for("the staging command", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandStarted { id, display, .. } if display.contains(" add ") => {
                    Some(*id)
                }
                _ => None,
            })
        })
        .await;
    let (end, full) = sink
        .wait_for("its end", |evs| {
            let full = evs.iter().find_map(|e| match e {
                SessionEvent::CommandStarted { id: got, full, .. } if *got == id => {
                    Some(full.clone())
                }
                _ => None,
            })?;
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandFinished { id: got, end, .. } if *got == id => {
                    Some((*end, full.clone()))
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(end, platitude_core::CommandEnd::Exited(0));
    assert!(
        full.contains("LC_ALL=C") && full.contains("--no-optional-locks"),
        "the copyable form carries what is always applied: {full}"
    );

    // The refresh that follows the write is the session's own doing.
    let commands = sink.count(|e| matches!(e, SessionEvent::CommandStarted { .. }));
    assert_eq!(commands, 1, "only the write itself was recorded");

    // Switched on, the reads show up as well.
    session.set_record_background(true);
    session.refresh_status();
    sink.wait_for("a background read", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::CommandStarted { display, .. } if display.contains("status")))
            .then_some(())
    })
    .await;
    session.close();
}

/// Index of the `WriteFinished` for one operation.
fn position_of(events: &[SessionEvent], op: &str) -> Option<usize> {
    events
        .iter()
        .position(|e| matches!(e, SessionEvent::WriteFinished { op: got, .. } if *got == op))
}

fn log_starts(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::LogStarted { .. }))
        .count()
}

/// A `gh-pages`-shaped ref: a parentless commit that only a
/// remote-tracking ref names, sharing no history with anything else.
///
/// The walk offers it either way — `--remotes` names it — but the row it
/// lands on is the whole point. `--topo-order` refuses to intermix
/// independent lines of history, so it emits every commit of the main
/// chain before starting this one and the newest commit in the repository
/// arrives dead last. `--date-order` keeps the same parents-after-children
/// guarantee the graph builder needs and puts it where its timestamp says.
#[tokio::test(flavor = "multi_thread")]
async fn an_independent_history_sits_where_its_date_puts_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "second");
    repo.commit_file("f.txt", "2\n", "third");

    repo.git(&["checkout", "--orphan", "gh-pages"]);
    repo.git(&["rm", "-rf", "."]);
    repo.write_file("index.html", "docs\n");
    repo.git(&["add", "index.html"]);
    repo.git(&["commit", "-m", "docs: api documentation"]);
    let orphan = repo.git(&["rev-parse", "HEAD"]);
    // Only the remote-tracking ref keeps it: no local branch, and nothing
    // reaches it from HEAD.
    repo.git(&["update-ref", "refs/remotes/origin/gh-pages", &orphan]);
    repo.git(&["checkout", "-f", "main"]);
    repo.git(&["branch", "-D", "gh-pages"]);
    repo.git(&["clean", "-fd"]);
    // One more on main *after* the orphan: it is now neither the newest
    // commit nor the oldest, which is what buries it. A tip that is newest
    // of all gets emitted first under either ordering, so a repository
    // shaped that way proves nothing.
    repo.commit_file("f.txt", "3\n", "fourth");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    let total = sink
        .wait_for("LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { total, .. } => Some(*total),
                _ => None,
            })
        })
        .await;
    assert_eq!(total, 5, "four on main plus the orphan");

    // Chips are what make a row unreachable from every branch findable at
    // all, so the row number and the chip are asserted together.
    let seen = sink
        .wait_for("its chip", |evs| {
            let rows = crate::support::replay_graph(evs);
            rows.iter()
                .find(|(_, r)| r.oid_hex == orphan)
                .filter(|(_, r)| !r.labels.is_empty())
                .map(|(row, r)| (*row, r.clone()))
        })
        .await;
    let (row, seen) = seen;
    assert_eq!(
        row, 1,
        "the second-newest commit belongs on the second row, not {row} rows down"
    );
    assert!(
        seen.labels.iter().any(|l| l.text == "origin/gh-pages"),
        "the orphan carries no chip naming it: {:?}",
        seen.labels
    );
    session.close();
}
