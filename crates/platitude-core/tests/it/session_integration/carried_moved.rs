//! A worktree that has committed since its reading was taken: what the
//! graph draws for it in between. The worktree listing rides this tree's
//! reads and each worktree's `status` its own turn, so a row drawn from the
//! older reading would stand on a commit that worktree has left.

use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{opened_with, scenario};
use crate::support::wait::bounded;
use platitude_core::session::{PaceBounds, SessionEvent};

/// A second worktree beside `repo`, dirty enough to draw a row.
fn a_worktree_beside(repo: &mut TestRepo) -> std::path::PathBuf {
    let worktree = repo.path.with_file_name("worktree");
    let at = worktree.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "worktree", &at]);
    std::fs::write(worktree.join("carried.txt"), "u\n").expect("a file in the worktree");
    worktree
}

/// A pass's generation and the commit the worktree's row says it stands on —
/// read off the row itself: the walk hands a worktree out at its commit or a
/// stash on it, whichever it reaches first, so the neighbour below is not
/// always the anchor (`session::rows::CarriedRows`).
fn worktree_row_anchor(event: &SessionEvent) -> Option<(u64, Option<String>)> {
    let (generation, rows) = match event {
        SessionEvent::LogChunk { generation, rows }
        | SessionEvent::LogReplaced {
            generation, rows, ..
        } => (*generation, rows),
        _ => return None,
    };
    Some((
        generation,
        rows.iter()
            .find_map(|r| r.carried.as_ref())
            .map(|c| c.head.to_hex()),
    ))
}

fn drew_a_worktree(events: &[SessionEvent]) -> Option<u64> {
    events
        .iter()
        .filter_map(worktree_row_anchor)
        .find_map(|(generation, anchor)| anchor.is_some().then_some(generation))
}

/// The picture the opening leaves: every read and walk it started over,
/// then the newest pass that drew the worktree's row, and the commit it drew
/// it on. Not the first such pass — an opening walks more than once (the
/// tag pass behind the fast one, the worktrees' pass asking again), and one
/// begun before the worktree moves, landing after the first, rightly draws
/// the row where the worktree still was.
async fn settled_with_row(
    sink: &crate::support::session::CaptureSink,
    session: &std::sync::Arc<platitude_core::session::RepoSession>,
) -> (u64, String) {
    sink.wait_for("a graph drawing the worktree's row", drew_a_worktree)
        .await;
    bounded(
        "the opening's pass over the worktrees",
        session.wait_for_carried_pass(),
    )
    .await;
    bounded("the opening's reads", session.wait_for_snapshot_reads()).await;
    bounded("the opening's walks", session.wait_for_graph_passes()).await;
    let events = sink.events.lock().unwrap();
    events
        .iter()
        .filter_map(worktree_row_anchor)
        .filter_map(|(at, anchor)| Some((at, anchor?)))
        .max_by_key(|(at, _)| *at)
        .expect("the row was drawn above a commit")
}

/// The other order, which a record written only by this tree's listing
/// gets wrong: the worktrees' pass takes its own listing before reading them, so
/// its reading can be the fresher one, and the row is drawn on the commit
/// it names.
#[tokio::test(flavor = "multi_thread")]
async fn a_reading_fresher_than_the_page_s_listing_keeps_its_row() {
    let (mut repo, _head) = scenario();
    let worktree = a_worktree_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    let (with_row, _) = settled_with_row(&sink, &session).await;

    repo.git_in(&worktree, &["add", "carried.txt"]);
    repo.git_in(
        &worktree,
        &["commit", "-m", "feat: the worktree records its own"],
    );
    std::fs::write(worktree.join("still.txt"), "u\n").expect("a second file in the worktree");

    // Only the worktrees' pass: this tree's listing has not run since the
    // commit, so this pass's is the only record of where the worktree stands.
    let pass = session
        .refresh_carried()
        .expect("a pass begins: the worktrees are read and none is out");
    bounded("the pass over the worktrees", pass.outcome()).await;

    let anchor = sink
        .wait_for("a graph drawing the worktree's row again", |events| {
            events
                .iter()
                .filter_map(worktree_row_anchor)
                .filter(|(at, _)| *at > with_row)
                .find_map(|(_, anchor)| anchor)
        })
        .await;
    assert_eq!(
        anchor,
        repo.git_in(&worktree, &["rev-parse", "HEAD"]).trim(),
        "the row was dropped, or drawn somewhere the worktree is not"
    );
    session.close();
}

/// The listing sees the worktree's commit before its reading does: the graph
/// drawn in between leaves the row out, and the reading asked for on the
/// spot puts it back.
#[tokio::test(flavor = "multi_thread")]
async fn a_worktree_that_has_committed_draws_no_row_until_its_reading_catches_up() {
    let (mut repo, _head) = scenario();
    let worktree = a_worktree_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    let (with_row, stood_on) = settled_with_row(&sink, &session).await;

    // Something stays uncommitted, so the worktree still has a row afterwards.
    repo.git_in(&worktree, &["add", "carried.txt"]);
    repo.git_in(
        &worktree,
        &["commit", "-m", "feat: the worktree records its own"],
    );
    std::fs::write(worktree.join("still.txt"), "u\n").expect("a second file in the worktree");

    // This tree's read, under bounds nothing falls due within: the listing
    // moves where the worktree stands, and the refs read sees its branch move,
    // which asks for the walk. The worktree's reading is not retaken by it.
    let hour = Duration::from_secs(3600);
    session.set_pace_bounds(PaceBounds {
        own_floor: hour,
        own_ceiling: hour,
        worktree_floor: hour,
        worktree_ceiling: hour,
    });
    session.set_paced(true);
    session.poll_now();

    // Asked for at once by `RepoSession::carried_current` as that walk
    // draws the worktree behind the listing, and read at the pace.
    sink.wait_for(
        "the worktree's reading asked for when the listing moved",
        |events| {
            events
                .iter()
                .any(|e| matches!(e, SessionEvent::PacedRead { worktree: Some(_) }))
                .then_some(())
        },
    )
    .await;
    let back = sink
        .wait_for("a graph drawing the worktree's row again", |events| {
            events
                .iter()
                .filter_map(worktree_row_anchor)
                .filter(|(at, _)| *at > with_row)
                .find_map(|(_, anchor)| anchor)
        })
        .await;
    assert_ne!(
        back, stood_on,
        "the row came back on the commit the worktree had already left"
    );

    // Over every pass after the move: whether the row leaves at all is the
    // scheduler's (the reading can beat the walk), but none may draw it on
    // the commit the worktree left.
    {
        let events = sink.events.lock().unwrap();
        let drawn_behind = events
            .iter()
            .filter_map(worktree_row_anchor)
            .filter(|(at, _)| *at > with_row)
            .filter_map(|(at, anchor)| Some((at, anchor?)))
            .find(|(_, anchor)| *anchor == stood_on);
        assert!(
            drawn_behind.is_none(),
            "a pass drew the worktree on the commit it had left: {drawn_behind:?}"
        );
    }
    session.close();
}
