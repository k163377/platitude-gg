//! End-to-end RepoSession tests on real temp repositories.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use platitude_core::details::DiffTarget;
use platitude_core::session::{RepoSession, SessionEvent, SessionSink};
use platitude_core::{GitExecutor, Oid};
use support::TestRepo;

struct CaptureSink {
    events: Mutex<Vec<SessionEvent>>,
}

impl CaptureSink {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
        })
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
            evs.iter()
                .filter(|e| {
                    matches!(
                        e,
                        SessionEvent::LogStarted { .. }
                            | SessionEvent::LogChunk { .. }
                            | SessionEvent::LogFinished { .. }
                            | SessionEvent::LogReplaced { .. }
                            | SessionEvent::LogFailed { .. }
                    )
                })
                .count()
        }
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let (newest, seen) = {
                let evs = self.events.lock().unwrap();
                let newest = evs
                    .iter()
                    .filter_map(|e| match e {
                        SessionEvent::LogFinished {
                            generation,
                            total: t,
                            ..
                        } if *t == total => Some(*generation),
                        SessionEvent::LogReplaced {
                            generation, rows, ..
                        } if rows.len() as u32 == total => Some(*generation),
                        _ => None,
                    })
                    .max();
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
            assert!(
                Instant::now() < deadline,
                "stream never settled at {total} rows; events so far: {:?}",
                self.events.lock().unwrap()
            );
        }
    }

    /// Polls until `pred` over the event list returns `Some`.
    async fn wait_for<T>(&self, what: &str, pred: impl Fn(&[SessionEvent]) -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            {
                let events = self.events.lock().unwrap();
                if let Some(v) = pred(&events) {
                    return v;
                }
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}; events so far: {:?}",
                self.events.lock().unwrap()
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl SessionSink for CaptureSink {
    fn event(&self, event: SessionEvent) {
        self.events.lock().unwrap().push(event);
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
        let mut latest: Vec<String> = Vec::new();
        for e in evs {
            match e {
                SessionEvent::LogChunk { rows, .. } => {
                    for r in rows {
                        if r.row == 1 {
                            latest = r.labels.iter().map(|l| l.text.clone()).collect();
                        }
                    }
                }
                SessionEvent::LabelsChanged { rows } => {
                    for (row, labels) in rows {
                        if *row == 1 {
                            latest = labels.iter().map(|l| l.text.clone()).collect();
                        }
                    }
                }
                _ => {}
            }
        }
        (latest.contains(&"main".to_string()) && latest.contains(&"v1".to_string())).then_some(())
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
    sink.wait_for("tags-off LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                generation, total, ..
            } if *generation > first_gen && *total == 1 => Some(()),
            _ => None,
        })
    })
    .await;

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
    sink.wait_for("limited LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                generation,
                total,
                truncated,
                ..
            } if *generation > first_gen => {
                assert_eq!(*total, 2);
                assert!(truncated);
                Some(())
            }
            _ => None,
        })
    })
    .await;

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
    sink.wait_for("limited LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                generation,
                total,
                truncated,
                ..
            } if *generation > first_gen => {
                assert_eq!(*total, 3, "stash + three + two, index parent sifted");
                assert!(truncated, "the walk was cut before the root commit");
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

/// The synthetic WIP row is shown but never walked: a window that holds
/// the whole history must not report truncation just because the WIP row
/// pushes the shown count up to the limit.
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
    // the next generation is the reaction to the limit change.
    let first_gen = sink.settled_stream_gen(3).await;

    // Two commits walk through a window of three; the WIP row makes three
    // shown rows, which is not a truncated window.
    session.set_log_limit(Some(3));
    sink.wait_for("limited LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                generation,
                total,
                truncated,
                ..
            } if *generation > first_gen => {
                assert_eq!(*total, 3, "WIP row + two commits");
                assert!(!truncated, "the whole history fits the window");
                Some(())
            }
            _ => None,
        })
    })
    .await;

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

/// "Leave my changes on this branch": the stash and the switch are one
/// job, so the changes stay behind instead of coming along.
#[tokio::test(flavor = "multi_thread")]
async fn switching_can_stash_the_working_tree_first() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.git(&["branch", "other"]);
    repo.write_file("f.txt", "uncommitted\n");
    repo.write_file("untracked.txt", "also mine\n");

    let (sink, session) = opened(&repo).await;
    session.checkout_stashing(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, "checkout").await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "0\n",
        "the change stayed behind in the stash"
    );
    assert!(!repo.path.join("untracked.txt").exists(), "untracked too");
    assert_eq!(repo.git(&["stash", "list"]).lines().count(), 1);
    session.close();
}

/// A stash that cannot run must not let the switch happen anyway.
#[tokio::test(flavor = "multi_thread")]
async fn a_switch_whose_stash_fails_does_not_move_head() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.git(&["branch", "other"]);
    // Nothing to stash: `git stash push` on a clean tree exits non-zero
    // only with --staged/paths, so make the failure the switch's own.
    repo.write_file("f.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.checkout_stashing(platitude_core::branch::CheckoutTarget::Branch {
        name: "no-such-branch".into(),
    });
    assert!(
        write_result(&sink, "checkout").await.is_some(),
        "git rejected the branch name"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    session.close();
}

/// Puts the todo-editor helper where the session looks for it — beside the
/// running executable, which for a test is the test binary's own directory.
/// Packaging carries the same obligation for the application.
fn install_todo_editor() {
    let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"));
    let name = format!(
        "{}{}",
        platitude_core::sequencer::HELPER_NAME,
        std::env::consts::EXE_SUFFIX
    );
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(name)));
    if let Some(beside) = beside
        && beside != built
    {
        // A previous run may have left one behind and Windows locks a
        // running executable; either way the copy that is there will do.
        let _ = std::fs::copy(&built, &beside);
    }
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

/// The auto-fetch timer runs the fetch it promises, and turns off again.
#[tokio::test(flavor = "multi_thread")]
async fn auto_fetch_runs_on_its_interval_and_stops() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);

    let (sink, session) = opened(&clone).await;
    session.set_auto_fetch(Some(Duration::from_millis(120)));
    let error = sink
        .wait_for("an automatic fetch", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::WriteFinished { op, error }
                    if *op == platitude_core::session::AUTO_FETCH_OP =>
                {
                    Some(error.clone())
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(error, None, "the file:// remote fetched cleanly");
    assert_eq!(
        clone.git(&["rev-parse", "origin/main"]),
        origin.git(&["rev-parse", "main"]),
    );

    session.set_auto_fetch(None);
    let started = || {
        sink.count(|e| {
            matches!(e, SessionEvent::WriteStarted { op }
                     if *op == platitude_core::session::AUTO_FETCH_OP)
        })
    };
    // A fetch already queued when the timer stopped still runs; let the
    // queue drain before taking the baseline.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let after_stop = started();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        started(),
        after_stop,
        "no further fetch started once it was turned off"
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
    sink.wait_for("clean again", |evs| {
        evs.iter().rev().find_map(|e| match e {
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

/// A background rebuild that finds nothing changed must stay silent — no
/// reset, no chunk, no repaint. This is what keeps a quiet auto-fetch
/// interval (or any other background refresh) from flickering the graph.
#[tokio::test(flavor = "multi_thread")]
async fn background_refresh_swaps_only_on_change() {
    let (mut repo, _) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(5).await;

    let stream_count = || {
        sink.count(|e| {
            matches!(
                e,
                SessionEvent::LogStarted { .. }
                    | SessionEvent::LogChunk { .. }
                    | SessionEvent::LogFinished { .. }
                    | SessionEvent::LogReplaced { .. }
                    | SessionEvent::LogFailed { .. }
            )
        })
    };
    let baseline = stream_count();

    // Nothing changed since the last delivery, so the rebuild must not
    // emit a single stream event. "Nothing happens" can only be observed
    // by giving the pass ample time to run.
    session.refresh_log();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        stream_count(),
        baseline,
        "an unchanged rebuild stayed silent: {:?}",
        sink.events.lock().unwrap()
    );

    // History moved outside the session: the same call now delivers one
    // atomic replacement — a single LogReplaced carrying every row, so
    // the consumer never holds an empty model in between.
    repo.commit_file("h.txt", "x\n", "outside commit");
    session.refresh_log();
    let swap_gen = sink.settled_stream_gen(6).await;
    let events = sink.events.lock().unwrap();
    let after: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SessionEvent::LogStarted { .. }
                    | SessionEvent::LogChunk { .. }
                    | SessionEvent::LogFinished { .. }
                    | SessionEvent::LogReplaced { .. }
                    | SessionEvent::LogFailed { .. }
            )
        })
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
