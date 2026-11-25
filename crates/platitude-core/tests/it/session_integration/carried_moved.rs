//! A worktree that has committed or gone since its reading was taken: what
//! the graph draws for it in between. The worktree listing rides this
//! tree's reads and each worktree's `status` its own turn, so a row drawn
//! from the older reading would stand on a commit that worktree has left,
//! or for a worktree that is not there.

use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{opened_with, other_worktree_rows, scenario};
use crate::support::wait::bounded;
use platitude_core::session::{PaceBounds, Recording, SessionEvent};

/// A second worktree beside `repo`, dirty enough to draw a row.
fn a_worktree_beside(repo: &mut TestRepo) -> std::path::PathBuf {
    let worktree = repo.path.with_file_name("worktree");
    let at = worktree.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "worktree", &at]);
    std::fs::write(worktree.join("carried.txt"), "u\n").expect("a file in the worktree");
    worktree
}

/// A picture's generation and the commit the worktree's row says it stands
/// on — read off the row itself: the walk hands a worktree out at its commit
/// or a stash on it, whichever it reaches first, so the neighbour below is
/// not always the anchor (`session::rows::CarriedRows`).
fn worktree_row_anchor(event: &SessionEvent) -> Option<(u64, Option<String>)> {
    other_worktree_rows(event)
        .map(|(generation, rows)| (generation, rows.first().map(|c| c.head.to_hex())))
}

/// A picture's generation and the folders of the worktrees its rows draw, by
/// their last segment.
fn worktree_folders(event: &SessionEvent) -> Option<(u64, Vec<String>)> {
    other_worktree_rows(event).map(|(generation, rows)| {
        let folders = rows
            .iter()
            .filter_map(|c| c.path.rsplit(['/', '\\']).next().map(str::to_string))
            .collect();
        (generation, folders)
    })
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
    session.close_and_wait().await;
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
    session.set_pace_bounds(PaceBounds::new(hour..=hour, hour..=hour));
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
    session.close_and_wait().await;
}

/// Paces the page with bounds no read falls due under but a worktree new to
/// the listing's, asks for this tree's read as the window coming back does,
/// and waits for it to end — what it laid or walked included.
async fn read_this_tree(
    sink: &crate::support::session::CaptureSink,
    session: &std::sync::Arc<platitude_core::session::RepoSession>,
    from: usize,
) {
    let hour = Duration::from_secs(3600);
    session.set_pace_bounds(PaceBounds::new(hour..=hour, hour..=hour));
    session.set_paced(true);
    session.poll_now();
    sink.wait_for("this tree's paced read", |events| {
        events[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::PacedRead { worktree: None }))
            .then_some(())
    })
    .await;
    bounded("any walk it asked for", session.wait_for_graph_passes()).await;
}

/// The walks begun since `from`.
fn walks_since(events: &[SessionEvent], from: usize) -> usize {
    events[from..]
        .iter()
        .filter(
            |e| matches!(e, SessionEvent::CommandStarted { display, .. } if display.contains("log -z")),
        )
        .count()
}

/// A worktree removed outside: the listing that no longer names it takes its
/// row off the graph at once — not at the next commit, nor at the worktree's
/// next turn, which is paced — and with no walk, the history being as it was.
#[tokio::test(flavor = "multi_thread")]
async fn a_worktree_removed_outside_takes_its_row_off_with_the_listing() {
    let (mut repo, _head) = scenario();
    let worktree = a_worktree_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    let (with_row, _) = settled_with_row(&sink, &session).await;
    assert_eq!(
        sink.newest_picture(),
        Some(with_row),
        "the opening's last picture draws the row"
    );
    session.set_recording(Recording::WithBackground);
    let from = sink.events.lock().unwrap().len();

    repo.git(&["worktree", "remove", "--force", &worktree.to_string_lossy()]);
    // This tree's read alone: no ref moved and this tree is as it was, so the
    // listing is the only word.
    read_this_tree(&sink, &session, from).await;

    {
        let events = sink.events.lock().unwrap();
        let after: Vec<_> = events
            .iter()
            .filter_map(worktree_row_anchor)
            .filter(|(at, _)| *at > with_row)
            .collect();
        assert!(
            !after.is_empty() && after.iter().all(|(_, anchor)| anchor.is_none()),
            "the graph still draws the removed worktree's row: {after:?}"
        );
        assert_eq!(walks_since(&events, from), 0, "the history was walked");
    }
    session.close_and_wait().await;
}

/// A worktree moved outside is a folder gone and a folder new to the listing
/// (its row is keyed by where it is): the row leaves with the old folder and
/// comes back with the new one's first reading, due at once — as a worktree
/// that has committed comes back with its reading. No graph draws it under
/// the folder it left.
#[tokio::test(flavor = "multi_thread")]
async fn a_worktree_moved_outside_comes_back_under_the_new_folder() {
    let (mut repo, _head) = scenario();
    let worktree = a_worktree_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;
    let (with_row, _) = settled_with_row(&sink, &session).await;
    let from = sink.events.lock().unwrap().len();

    let moved = worktree.with_file_name("moved");
    repo.git(&[
        "worktree",
        "move",
        &worktree.to_string_lossy(),
        &moved.to_string_lossy(),
    ]);
    read_this_tree(&sink, &session, from).await;
    // The new folder's reading lays the graph again before the read says it
    // has ended.
    sink.wait_for("the new folder's first reading", |events| {
        events[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::PacedRead { worktree: Some(_) }))
            .then_some(())
    })
    .await;

    {
        let events = sink.events.lock().unwrap();
        let after: Vec<_> = events
            .iter()
            .filter_map(worktree_folders)
            .filter(|(at, _)| *at > with_row)
            .collect();
        assert_eq!(
            after
                .iter()
                .max_by_key(|(at, _)| *at)
                .map(|(_, f)| f.as_slice()),
            Some(["moved".to_string()].as_slice()),
            "the row did not come back under the new folder: {after:?}"
        );
        assert!(
            after
                .iter()
                .all(|(_, folders)| !folders.contains(&"worktree".to_string())),
            "a graph drew the row under the folder it left: {after:?}"
        );
    }
    session.close_and_wait().await;
}
