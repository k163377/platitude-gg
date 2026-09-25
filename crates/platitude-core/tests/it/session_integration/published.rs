//! The published mark the graph rows carry (`session::published`), checked
//! row by row against `git rev-list --not --remotes` — the same question
//! [`platitude_core::publish`] asks everywhere else.

use crate::support::remote::origin_and_clone;
use crate::support::session::{CaptureSink, open_unawaited};
use crate::support::{TestRepo, replay_rows};
use platitude_core::session::SessionEvent;

/// What `git rev-list --count <oid>^! --not --remotes` says: no commits
/// outside the remotes means a remote already has this one.
fn git_says_published(repo: &mut TestRepo, oid: &str) -> bool {
    repo.git(&[
        "rev-list",
        "--count",
        &format!("{oid}^!"),
        "--not",
        "--remotes",
    ]) == "0"
}

async fn rows_of(sink: &CaptureSink, want: usize) -> Vec<platitude_core::session::LogRow> {
    sink.wait_for("the rows", |evs| {
        let rows = replay_rows(evs);
        (rows.len() == want).then(|| rows.into_values().collect())
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_row_is_marked_exactly_when_a_remote_reaches_it() {
    let (_bare, mut work) = origin_and_clone();
    // `origin/main` stands on "pushed"; the two after it are ours alone.
    let pushed = work.commit_file_id("b.txt", "two\n", "pushed");
    work.git(&["push", "origin", "main"]);
    let local = work.commit_file_id("c.txt", "three\n", "local");
    let tip = work.commit_file_id("d.txt", "four\n", "local tip");

    let (sink, _session) = open_unawaited(&work);
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .find_map(|e| matches!(e, SessionEvent::Opened { .. }).then_some(()))
    })
    .await;
    // Root + pushed + local + tip.
    let rows = rows_of(&sink, 4).await;

    for row in &rows {
        assert_eq!(
            row.published,
            git_says_published(&mut work, &row.oid_hex),
            "row {} ({}) disagrees with `rev-list --not --remotes`",
            row.row,
            row.subject,
        );
    }

    // The fixture must mix marked and unmarked rows for the loop to mean
    // anything.
    let marked: Vec<&str> = rows
        .iter()
        .filter(|r| r.published)
        .map(|r| r.subject.as_str())
        .collect();
    assert_eq!(marked, vec!["pushed", "root"], "newest first");
    assert!(
        rows.iter().any(|r| r.oid_hex == local && !r.published)
            && rows.iter().any(|r| r.oid_hex == tip && !r.published),
        "the two commits after the push are ours alone"
    );
    assert!(rows.iter().any(|r| r.oid_hex == pushed && r.published));
}

/// A push from a terminal moves `refs/remotes/...`; the rebuild the refs
/// read triggers must bring the marks with it, or the warnings keep saying
/// what was true before.
#[tokio::test(flavor = "multi_thread")]
async fn a_push_from_outside_publishes_the_rows_it_reached() {
    let (_bare, mut work) = origin_and_clone();
    let mine = work.commit_file_id("b.txt", "two\n", "mine");

    let (sink, session) = open_unawaited(&work);
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .find_map(|e| matches!(e, SessionEvent::Opened { .. }).then_some(()))
    })
    .await;
    let rows = rows_of(&sink, 2).await;
    assert!(
        !rows
            .iter()
            .find(|r| r.oid_hex == mine)
            .expect("mine")
            .published,
        "nobody has it yet"
    );

    // Somebody's terminal, outside the write queue.
    work.git(&["push", "origin", "main"]);
    assert!(git_says_published(&mut work, &mine), "git agrees it is out");
    session.refresh_refs();

    sink.wait_for("the rebuilt rows", |evs| {
        let rows = replay_rows(evs);
        (rows.len() == 2
            && rows
                .values()
                .find(|r| r.oid_hex == mine)
                .is_some_and(|r| r.published))
        .then_some(())
    })
    .await;
}
