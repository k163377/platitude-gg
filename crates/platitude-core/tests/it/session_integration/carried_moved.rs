//! A copy that has committed since its reading was taken: what the
//! graph draws for it in between.
//!
//! Where every other working copy stands is named by the worktree
//! listing — one short process on the page's own tick — and what each is
//! carrying costs a `status` apiece on a slower tick of its own. So a
//! window learns that a neighbour has committed long before it learns
//! what the neighbour now holds, and the row drawn from the older
//! reading would stand on a commit that copy has left.

use crate::support::TestRepo;
use crate::support::session::{opened_with, scenario};
use crate::support::wait::bounded;
use platitude_core::session::SessionEvent;

/// A second working copy beside `repo`, dirty enough to draw a row.
fn a_copy_beside(repo: &mut TestRepo) -> std::path::PathBuf {
    let copy = repo.path.with_file_name("copy");
    let at = copy.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "copy", &at]);
    std::fs::write(copy.join("carried.txt"), "u\n").expect("a file in the copy");
    copy
}

/// The rows a pass sent, and the commit the copy's row says that copy is
/// standing on.
///
/// **Read off the row's own reading**: a copy is handed out at
/// whichever of its commit and a stash taken on that commit the walk
/// reaches first, so the neighbour below is not always the anchor
/// (`session::rows::CarriedRows`).
fn copy_row_anchor(event: &SessionEvent) -> Option<(u64, Option<String>)> {
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

fn drew_a_copy(events: &[SessionEvent]) -> Option<u64> {
    events
        .iter()
        .filter_map(copy_row_anchor)
        .find_map(|(generation, anchor)| anchor.is_some().then_some(generation))
}

/// The other order, which is the one a record written only by the
/// page's tick gets wrong: the pass over the copies takes a listing
/// of its own before it reads them, so it can learn that a copy has
/// committed before that tick does. Its reading is then the fresher
/// of the two, and the row is drawn from it, on the commit that
/// reading names.
#[tokio::test(flavor = "multi_thread")]
async fn a_reading_fresher_than_the_page_s_listing_keeps_its_row() {
    let (mut repo, _head) = scenario();
    let copy = a_copy_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;

    let with_row = sink
        .wait_for("a graph drawing the copy's row", drew_a_copy)
        .await;
    bounded(
        "the opening's pass over the copies",
        session.wait_for_carried_pass(),
    )
    .await;

    repo.git_in(&copy, &["add", "carried.txt"]);
    repo.git_in(&copy, &["commit", "-m", "feat: the copy records its own"]);
    std::fs::write(copy.join("still.txt"), "u\n").expect("a second file in the copy");

    // The copies' own pass and nothing else — the page's listing has not
    // run since the commit, so the only record of where that copy stands
    // is the one this pass took.
    let pass = session
        .refresh_carried()
        .expect("a pass begins: the copies are read and none is out");
    bounded("the pass over the copies", pass.outcome()).await;

    let anchor = sink
        .wait_for("a graph drawing the copy's row again", |events| {
            events
                .iter()
                .filter_map(copy_row_anchor)
                .filter(|(at, _)| *at > with_row)
                .find_map(|(_, anchor)| anchor)
        })
        .await;
    assert_eq!(
        anchor,
        repo.git_in(&copy, &["rev-parse", "HEAD"]).trim(),
        "the row was dropped, or drawn somewhere the copy is not"
    );
    session.close();
}

/// The copy commits, and the cheap listing says so before the expensive
/// reading does. The graph drawn in between leaves that copy's row
/// out — and the reading the window asks for on the spot puts it
/// back.
#[tokio::test(flavor = "multi_thread")]
async fn a_copy_that_has_committed_draws_no_row_until_its_reading_catches_up() {
    let (mut repo, _head) = scenario();
    let copy = a_copy_beside(&mut repo);
    let (sink, session) = opened_with(&repo, crate::support::exec::isolated()).await;

    let with_row = sink
        .wait_for("a graph drawing the copy's row", drew_a_copy)
        .await;
    bounded(
        "the opening's pass over the copies",
        session.wait_for_carried_pass(),
    )
    .await;
    let stood_on = {
        let events = sink.events.lock().unwrap();
        events
            .iter()
            .filter_map(copy_row_anchor)
            .find_map(|(at, anchor)| (at == with_row).then_some(anchor))
            .flatten()
            .expect("the row was drawn above a commit")
    };

    // The copy commits what it was carrying and keeps something else
    // uncommitted, so it still has a row to draw afterwards.
    repo.git_in(&copy, &["add", "carried.txt"]);
    repo.git_in(&copy, &["commit", "-m", "feat: the copy records its own"]);
    std::fs::write(copy.join("still.txt"), "u\n").expect("a second file in the copy");

    // The page's own tick, which is where both halves of this ride: the
    // listing names where every copy stands (the cheap half, and the one
    // that has moved), and the refs read sees the branch that copy is on
    // move — which is what asks for the walk. The reading behind them
    // has not been taken again yet.
    let owed = bounded(
        "the worktree listing",
        session
            .refresh_worktrees()
            .expect("the session is open, so the listing is read"),
    )
    .await
    .expect("the listing task");
    assert!(owed.is_empty(), "the listing left reads owed: {owed:?}");
    bounded("the page's poll", session.refresh_poll_tracked().outcome()).await;

    // The reading asked for the moment the listing was seen to have
    // moved (`RepoSession::carried_current`).
    bounded(
        "the reading asked for when the listing moved",
        session.wait_for_carried_pass(),
    )
    .await;
    let back = sink
        .wait_for("a graph drawing the copy's row again", |events| {
            events
                .iter()
                .filter_map(copy_row_anchor)
                .filter(|(at, _)| *at > with_row)
                .find_map(|(_, anchor)| anchor)
        })
        .await;
    assert_ne!(
        back, stood_on,
        "the row came back on the commit the copy had already left"
    );

    // **The whole of it, over every pass.** Which of the two a reader
    // sees in between is the scheduler's — the reading can beat the
    // walk, and on a repository this size it usually does, in which
    // case the row never leaves at all. What holds either way is that
    // the row is drawn where the copy is, and that is what every pass
    // after the move is held to.
    {
        let events = sink.events.lock().unwrap();
        let drawn_behind = events
            .iter()
            .filter_map(copy_row_anchor)
            .filter(|(at, _)| *at > with_row)
            .filter_map(|(at, anchor)| Some((at, anchor?)))
            .find(|(_, anchor)| *anchor == stood_on);
        assert!(
            drawn_behind.is_none(),
            "a pass drew the copy on the commit it had left: {drawn_behind:?}"
        );
    }
    session.close();
}
