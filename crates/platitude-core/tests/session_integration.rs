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
    assert_eq!(total, 4, "root + side + main + merge");

    // All rows delivered, topo-consistent, head row exists.
    let rows = sink
        .wait_for("chunk rows", |evs| {
            let mut rows = Vec::new();
            for e in evs {
                if let SessionEvent::LogChunk { rows: r, .. } = e {
                    rows.extend(r.iter().cloned());
                }
            }
            (rows.len() == 4).then_some(rows)
        })
        .await;
    assert_eq!(rows[0].oid_hex, head, "merge commit is the newest row");
    assert_eq!(rows[0].row, 0);
    assert!(rows.iter().all(|r| !r.subject.is_empty()));
    assert!(rows.iter().all(|r| r.author == "Test User"));

    // Labels: head row must end up carrying main (+ v1 tag), either inline
    // or via a LabelsChanged update.
    sink.wait_for("labels on head row", |evs| {
        let mut latest: Vec<String> = Vec::new();
        for e in evs {
            match e {
                SessionEvent::LogChunk { rows, .. } => {
                    for r in rows {
                        if r.row == 0 {
                            latest = r.labels.iter().map(|l| l.text.clone()).collect();
                        }
                    }
                }
                SessionEvent::LabelsChanged { rows } => {
                    for (row, labels) in rows {
                        if *row == 0 {
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
            SessionEvent::StatusLoaded { status, op_state } => {
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

    // The restarted stream re-delivers all rows under the new generation.
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
        (count == 4).then_some(())
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

    // Tags are walked by default → the tag-only commit has a row.
    let first_gen = sink
        .wait_for("tags-on LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished {
                    generation, total, ..
                } if *total == 2 => Some(*generation),
                _ => None,
            })
        })
        .await;

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

    let first_gen = sink
        .wait_for("full LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished {
                    generation,
                    total,
                    truncated,
                    ..
                } if *total == 3 => {
                    assert!(!truncated, "3 commits fit in the default window");
                    Some(*generation)
                }
                _ => None,
            })
        })
        .await;

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
