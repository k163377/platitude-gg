//! The page's pace against a real repository and a real second worktree:
//! a read asked for now runs and says so, a worktree the listing has
//! moved past is read at the pace instead of in a pass of its own, and the
//! window coming back reads the pane's worktree. When reads fall due is the
//! rules' (`session::pace`, tested by hand there); here the bounds are an
//! hour, so every read is one the test asked for and none waits on the
//! clock.

use std::sync::Arc;
use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened_with, scenario};
use crate::support::wait::bounded;
use platitude_core::session::{PaceBounds, Recording, RepoSession, SessionEvent, WorktreesPace};

/// Waits out the opening's reading of the worktrees: the worktrees' row drawn —
/// so their pass has landed and the pace lists them — and every read and
/// walk the opening started over.
async fn worktrees_settled(sink: &CaptureSink, session: &Arc<RepoSession>, rows: usize) {
    sink.wait_for("a graph drawing the worktrees' rows", |events| {
        events
            .iter()
            .any(|e| worktree_rows(e) == Some(rows))
            .then_some(())
    })
    .await;
    bounded("the opening's pass", session.wait_for_carried_pass()).await;
    bounded("the opening's reads", session.wait_for_snapshot_reads()).await;
    bounded("the opening's walks", session.wait_for_graph_passes()).await;
}

/// How many worktree rows a picture draws.
fn worktree_rows(event: &SessionEvent) -> Option<usize> {
    match event {
        SessionEvent::LogChunk { rows, .. } | SessionEvent::LogReplaced { rows, .. } => {
            Some(rows.iter().filter(|r| r.carried.is_some()).count())
        }
        _ => None,
    }
}

/// Paces the page under bounds no read falls due within.
fn paced_off_the_clock(session: &Arc<RepoSession>) {
    let hour = Duration::from_secs(3600);
    session.set_pace_bounds(PaceBounds {
        own_floor: hour,
        own_ceiling: hour,
        worktree_floor: hour,
        worktree_ceiling: hour,
    });
    session.set_paced(true);
}

fn a_worktree_beside(repo: &mut TestRepo) -> std::path::PathBuf {
    let worktree = repo.path.with_file_name("worktree");
    let at = worktree.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "worktree", &at]);
    std::fs::write(worktree.join("carried.txt"), "u\n").expect("a file in the worktree");
    worktree
}

fn paced_reads(events: &[SessionEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::PacedRead { worktree } => Some(worktree.clone()),
            _ => None,
        })
        .collect()
}

fn worktree_row_head(event: &SessionEvent) -> Option<String> {
    match event {
        SessionEvent::LogChunk { rows, .. } | SessionEvent::LogReplaced { rows, .. } => rows
            .iter()
            .find_map(|r| r.carried.as_ref())
            .map(|c| c.head.to_hex()),
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_read_asked_for_now_runs_and_says_so() {
    let (repo, _head) = scenario();
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    bounded("the opening's pass", session.wait_for_carried_pass()).await;

    paced_off_the_clock(&session);
    session.poll_now();
    sink.wait_for("this tree's paced read", |events| {
        paced_reads(events).contains(&None).then_some(())
    })
    .await;
    session.close();
}

/// The listing a paced read takes sees the worktree's commit; the worktree's
/// reading, now behind it, is due at once and read at the pace — and the
/// row comes back on the commit the worktree stands on.
#[tokio::test(flavor = "multi_thread")]
async fn a_worktree_behind_the_listing_is_read_at_the_page_s_pace() {
    let (mut repo, _head) = scenario();
    let worktree = a_worktree_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    sink.wait_for("a graph drawing the worktree's row", |events| {
        events.iter().find_map(worktree_row_head)
    })
    .await;
    bounded("the opening's pass", session.wait_for_carried_pass()).await;
    paced_off_the_clock(&session);

    repo.git_in(&worktree, &["add", "carried.txt"]);
    repo.git_in(
        &worktree,
        &["commit", "-m", "feat: the worktree records its own"],
    );
    std::fs::write(worktree.join("still.txt"), "u\n").expect("a second file in the worktree");
    let moved_to = repo
        .git_in(&worktree, &["rev-parse", "HEAD"])
        .trim()
        .to_string();

    session.poll_now();
    // The one worktree there is. Named by its last segment: the temporary
    // directory can reach the test under a short name and git under the
    // long one.
    let name = worktree
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    let read = sink
        .wait_for("the worktree's paced read", |events| {
            paced_reads(events).into_iter().flatten().next()
        })
        .await;
    assert_eq!(
        std::path::Path::new(&read)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned()),
        name
    );
    sink.wait_for("the worktree's row on the commit it moved to", |events| {
        events
            .iter()
            .filter_map(worktree_row_head)
            .any(|head| head == moved_to)
            .then_some(())
    })
    .await;
    session.close();
}

/// A worktree's path as `git worktree list` prints it — the spelling the app
/// stands the pane on (`Carried::path`), which a temp path spelled by
/// hand need not match (a short 8.3 name on Windows).
fn listed_path(repo: &mut TestRepo, name: &str) -> String {
    repo.git(&["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|path| path.ends_with(&format!("/{name}")))
        .expect("the worktree is listed")
        .to_string()
}

/// What the pane was handed, by worktree name.
fn handed(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CarriedStatusLoaded { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// The window coming back reads the worktree the pane stands on at once, and
/// that paced read hands the pane the worktree's list from its own `status` by
/// the time it says it has read: the page asks for no read of the pane
/// beside it (`RepoPage.pollCarried`).
#[tokio::test(flavor = "multi_thread")]
async fn the_window_coming_back_reads_the_pane_s_worktree_and_hands_it_the_list() {
    let (mut repo, _head) = scenario();
    let worktree = a_worktree_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    worktrees_settled(&sink, &session, 1).await;
    session.read_carried_status(listed_path(&mut repo, "worktree"), "worktree".into());
    sink.wait_for("the pane's own read", |events| {
        (!handed(events).is_empty()).then_some(())
    })
    .await;
    let from = sink.events.lock().unwrap().len();
    paced_off_the_clock(&session);

    std::fs::write(worktree.join("more.txt"), "u\n").expect("a second file in the worktree");
    session.poll_now();
    let handed_before_the_end = sink
        .wait_for("the worktree's paced read", |events| {
            let after = &events[from..];
            let end = after
                .iter()
                .position(|e| matches!(e, SessionEvent::PacedRead { worktree: Some(_) }))?;
            Some(handed(&after[..end]))
        })
        .await;
    assert_eq!(handed_before_the_end, ["worktree".to_string()]);
    session.close();
}

/// Worktrees turned back on come back in one pass and one walk: read
/// worktree by worktree at the pace, each row would walk the history again.
#[tokio::test(flavor = "multi_thread")]
async fn worktrees_turned_back_on_come_back_in_one_walk() {
    let (mut repo, _head) = scenario();
    a_worktree_beside(&mut repo);
    let other = repo.path.with_file_name("other");
    let other_at = other.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "other", &other_at]);
    std::fs::write(other.join("other.txt"), "o\n").expect("a file in the other worktree");
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    worktrees_settled(&sink, &session, 2).await;
    paced_off_the_clock(&session);
    session.set_worktrees_pace(WorktreesPace::Off);
    bounded(
        "the walk taking the rows down",
        session.wait_for_graph_passes(),
    )
    .await;

    session.set_recording(Recording::WithBackground);
    let from = sink.events.lock().unwrap().len();
    session.set_worktrees_pace(WorktreesPace::Auto);
    bounded(
        "the pass bringing them back",
        session.wait_for_carried_pass(),
    )
    .await;
    bounded("its walk", session.wait_for_graph_passes()).await;
    let events = sink.events.lock().unwrap();
    let walks = events[from..]
        .iter()
        .filter(|e| matches!(e, SessionEvent::CommandStarted { display, .. } if display.contains("log -z")))
        .count();
    assert_eq!(walks, 1, "one walk for both rows");
    let drawn = events[from..]
        .iter()
        .rev()
        .find_map(worktree_rows)
        .expect("a picture drawn after the worktrees came back");
    assert_eq!(drawn, 2, "both worktrees' rows are back");
    drop(events);
    session.close();
}
