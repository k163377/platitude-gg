//! How a write's boundaries out of the feed — its start, its answer, its
//! settle — reach the run's watch (`write_watch`) and the tab's settling
//! (`unsettle`, `settle_heard`, `settle_drained`), which of them the
//! toolbar's fetch and push rings turn between (`unshown`), and which
//! settled ids the tab keeps for the log's receipts.
//!
//! No test here has the QObject the tab posts itself through, so the call
//! it posts on hearing a settle (`readsDrained`) is made by hand, where Qt
//! would deliver it: behind every drain posted by then.

use super::super::*;
use super::drain_inputs::{OURS, SOMEBODY_ELSE, answered, started};
use platitude_core::OperationKind as K;

/// The settle as the session says it. It carries only the id; `_kind`
/// keeps the calls reading like `answered` and `started`.
fn settle_msg(id: u64, _kind: K) -> TabMsg {
    TabMsg::WriteSettled { id }
}

/// One of the tab's drains as Qt delivers it: whatever its feed holds by
/// then, not what it held when the drain was posted (`hub::Feed`).
fn drain(tab: &mut RepoTab, feed: &Feed<TabMsg>) {
    tab.absorb(feed.drain());
}

/// A run armed its watch and had an ask taken under `id` — by hand, since
/// `ask_session` needs a session.
fn watching(id: u64) -> RepoTab {
    let mut tab = RepoTab::default();
    tab.watch_next_write("press".into());
    tab.write_watch.asked(Some(id));
    tab
}

/// The feed's own messages reach the watch — the wiring `write_watch`'s
/// own tests cannot see — and the timer's and an opening's fetches on the
/// same feed do not open it. Waiting on `writeSeq` instead is fooled by
/// them.
#[test]
fn the_boundaries_reach_the_watch_and_nothing_else_opens_it() {
    let mut tab = watching(OURS);
    tab.absorb(vec![
        started(SOMEBODY_ELSE, K::AutoFetch),
        answered(SOMEBODY_ELSE, K::AutoFetch, ""),
        settle_msg(SOMEBODY_ELSE, K::AutoFetch),
    ]);
    tab.settle_drained(SOMEBODY_ELSE);
    assert_eq!(tab.write_seq, 1, "the answer is counted as an answer");
    assert!(
        !tab.wrote_through(),
        "but it is not the one being waited on"
    );

    tab.absorb(vec![answered(OURS, K::Commit, "")]);
    assert_eq!(
        tab.write_watch_stage(),
        "answered",
        "git has this one; the reads behind it are still out"
    );
    assert!(!tab.wrote_through());

    tab.absorb(vec![settle_msg(OURS, K::Commit)]);
    assert_eq!(
        tab.write_watch_stage(),
        "answered",
        "the session has said they are published; the models have yet to drain them"
    );
    assert!(!tab.wrote_through());

    tab.settle_drained(OURS);
    assert_eq!(tab.write_watch_stage(), "settled");
    assert!(tab.wrote_through());
}

/// The run's write carries the lower id and finishes last. Ids come off one
/// counter for every session and answer to equality alone (`OperationId`):
/// nothing ties their order to the order writes finish, so a `>=` boundary
/// would open here on somebody else's answer.
#[test]
fn a_write_numbered_after_this_one_and_finished_first_does_not_open_it() {
    let mut tab = watching(OURS);
    assert!(!tab.wrote_through(), "nothing has answered yet");

    // Somebody else's write, in front in the queue and higher in the
    // numbering, all the way through.
    tab.absorb(vec![
        started(SOMEBODY_ELSE, K::Branch),
        answered(SOMEBODY_ELSE, K::Branch, ""),
        settle_msg(SOMEBODY_ELSE, K::Branch),
    ]);
    tab.settle_drained(SOMEBODY_ELSE);
    assert!(
        !tab.wrote_through(),
        "an id past this one's is not this one's"
    );
    assert_eq!(
        tab.watched_write_id(),
        OURS as i32,
        "the watch keeps its own"
    );

    tab.absorb(vec![
        answered(OURS, K::Commit, ""),
        settle_msg(OURS, K::Commit),
    ]);
    tab.settle_drained(OURS);
    assert!(tab.wrote_through(), "and this write's own finish opens it");
}

/// The mirror: one direction catches `>=`, the other `<=`.
#[test]
fn nor_does_one_numbered_before_it() {
    let mut tab = watching(SOMEBODY_ELSE);
    tab.absorb(vec![
        started(OURS, K::Commit),
        answered(OURS, K::Commit, ""),
        settle_msg(OURS, K::Commit),
    ]);
    tab.settle_drained(OURS);
    assert!(!tab.wrote_through());

    tab.absorb(vec![
        answered(SOMEBODY_ELSE, K::Branch, ""),
        settle_msg(SOMEBODY_ELSE, K::Branch),
    ]);
    tab.settle_drained(SOMEBODY_ELSE);
    assert!(tab.wrote_through());
}

#[test]
fn a_run_with_no_write_of_its_own_never_passes() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![
        started(OURS, K::Commit),
        answered(OURS, K::Commit, ""),
        settle_msg(OURS, K::Commit),
    ]);
    tab.settle_drained(OURS);
    assert_eq!(tab.write_watch_stage(), "asleep");
    assert_eq!(tab.watched_write_id(), 0);
    assert!(!tab.wrote_through());
}

/// The tab reports stage and id and refuses nothing: whether arming over a
/// write still out is a breach only the run knows — often that write is
/// one nobody pressed for, like a menu's read on its way open
/// (`AutoActDriver.beginWrite`).
#[test]
fn the_tab_says_where_the_watch_stands_and_judges_nothing() {
    let mut tab = watching(OURS);
    assert_eq!(tab.write_watch_stage(), "held");
    assert_eq!(tab.watched_write_id(), OURS as i32);

    tab.watch_next_write("press".into());
    assert_eq!(tab.write_watch_stage(), "armed", "and it arms regardless");
    assert_eq!(tab.watched_write_id(), 0);

    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Fetch, "")]);
    assert_eq!(
        tab.write_watch_stage(),
        "armed",
        "a write nothing is armed for leaves it where it is"
    );
}

/// A write keeps the tab settling from its asking past git's answer, until
/// what it left has been read again and drained into the models — what a
/// menu raised under it waits on before it stands its rows again
/// (`AppMenu.runningWhy`). One queued behind it keeps the tab settling
/// across the gap before it starts. A settle for an id this tab never
/// asked for (an unasked fetch's) settles nothing.
#[test]
fn a_write_settles_after_its_answer_not_at_it() {
    let mut tab = RepoTab::default();
    assert!(tab.unsettle(1), "asked for on a settled tab");
    assert!(!tab.unsettle(2), "and another queued behind it");
    tab.absorb(vec![started(1, K::Commit)]);
    tab.absorb(vec![answered(1, K::Commit, "")]);
    assert_eq!(tab.busy_count, 0, "git has answered");
    assert!(tab.settling, "and its reading is still out");
    tab.absorb(vec![TabMsg::WriteSettled { id: 9 }]);
    tab.settle_drained(9);
    assert!(
        tab.settling,
        "a settle nobody here asked for settles nothing"
    );
    tab.absorb(vec![TabMsg::WriteSettled { id: 1 }]);
    tab.settle_drained(1);
    assert!(
        tab.settling,
        "its own reads are in, and the one queued behind has not started"
    );
    tab.absorb(vec![started(2, K::Stash)]);
    tab.absorb(vec![
        answered(2, K::Stash, ""),
        TabMsg::WriteSettled { id: 2 },
    ]);
    assert!(
        tab.settling,
        "counted once, at its asking, and the models still to drain"
    );
    tab.settle_drained(2);
    assert!(!tab.settling);
}

/// A tab watching its own write `OURS`, asked for and out, and its feed
/// holding all the session says of that write: its start, its answer, and
/// — behind the reads the session made and published to the other models'
/// feeds — its settle. Each push posted one of the tab's drains; the
/// answer's stands in Qt's queue ahead of the models' drains, the settle's
/// behind them.
fn settled_on_the_feed() -> (RepoTab, Arc<Feed<TabMsg>>) {
    let feed = Arc::new(Feed::default());
    let mut tab = RepoTab {
        feed: Some(feed.clone()),
        ..watching(OURS)
    };
    tab.unsettle(OURS);
    feed.push(started(OURS, K::Commit));
    feed.push(answered(OURS, K::Commit, ""));
    feed.push(TabMsg::WriteSettled { id: OURS });
    (tab, feed)
}

/// Neither the run's watch nor `settling` lets go on the settle itself.
fn assert_still_out(tab: &RepoTab, when: &str) {
    assert!(tab.settling, "settling fell {when}");
    assert_eq!(
        tab.write_watch_stage(),
        "answered",
        "the watch opened {when}"
    );
    assert!(!tab.wrote_through(), "the barrier opened {when}");
}

/// The tab's drain posted at the answer runs ahead of the models' drains
/// and takes the settle with it, pushed since. The write is not through on
/// that: the models have not drained what it left. It is through at the
/// call the tab posted itself, which Qt delivers behind them.
#[test]
fn a_settle_taken_by_a_drain_posted_ahead_of_the_models_opens_nothing() {
    let (mut tab, feed) = settled_on_the_feed();
    drain(&mut tab, &feed);
    assert_still_out(&tab, "at a drain posted ahead of the models'");
    assert_eq!(
        tab.settled_writes(),
        vec![OURS as i32],
        "heard, for the log"
    );

    // The models' drains run here, then the drain the settle's push posted.
    drain(&mut tab, &feed);
    tab.settle_drained(OURS);
    assert!(!tab.settling);
    assert!(tab.wrote_through());
}

/// The same with two of the tab's drains standing ahead of the models' —
/// the start's and the answer's. A message the first left on the tab's own
/// feed would be taken by the second, still ahead of them.
#[test]
fn nor_does_a_second_drain_posted_ahead_of_them() {
    let (mut tab, feed) = settled_on_the_feed();
    drain(&mut tab, &feed);
    drain(&mut tab, &feed);
    assert_still_out(&tab, "at the second drain posted ahead of the models'");

    drain(&mut tab, &feed);
    tab.settle_drained(OURS);
    assert!(!tab.settling);
    assert!(tab.wrote_through());
}

/// Each call answers for the write it was posted for, and no other: the
/// call for one write stands in Qt's queue ahead of the drains carrying
/// what a later write left.
#[test]
fn a_call_lets_go_of_its_own_write_alone() {
    let mut tab = watching(SOMEBODY_ELSE);
    tab.unsettle(OURS);
    tab.unsettle(SOMEBODY_ELSE);
    tab.absorb(vec![
        answered(OURS, K::Stage, ""),
        settle_msg(OURS, K::Stage),
        answered(SOMEBODY_ELSE, K::Commit, ""),
        settle_msg(SOMEBODY_ELSE, K::Commit),
    ]);
    tab.settle_drained(OURS);
    assert!(tab.settling, "the later write's reads are still to drain");
    assert!(!tab.wrote_through());
    tab.settle_drained(SOMEBODY_ELSE);
    assert!(!tab.settling);
    assert!(tab.wrote_through());
}

/// Every write the tab sees settle is kept by id, oldest first and
/// bounded, whoever asked for it — the ones the command log's receipts are
/// asked after.
#[test]
fn the_writes_seen_settle_are_kept_for_the_logs_receipts() {
    let mut tab = RepoTab::default();
    assert!(tab.settled_writes().is_empty());
    tab.absorb(vec![
        TabMsg::WriteSettled { id: 4 },
        TabMsg::WriteSettled { id: 2 },
    ]);
    assert_eq!(tab.settled_writes(), vec![4, 2]);
    tab.absorb((10..20).map(|id| TabMsg::WriteSettled { id }).collect());
    assert_eq!(tab.settled_writes(), (12..20).collect::<Vec<i32>>());
}

/// A write this tab did not ask for itself — a restore off the discard
/// record — is counted from its start; one asked for before the models
/// have drained the last keeps the tab settling.
#[test]
fn a_write_asked_elsewhere_settles_from_its_start() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![started(3, K::Restore)]);
    assert!(tab.settling);
    tab.absorb(vec![
        answered(3, K::Restore, ""),
        TabMsg::WriteSettled { id: 3 },
    ]);
    tab.unsettle(4);
    tab.settle_drained(3);
    assert!(tab.settling);
}

/// A fetch nobody pressed as the bridge carries it (`BridgeSink`): off the
/// write state's surface, under the write's own id.
fn unasked(id: u64, kind: K, running: bool) -> TabMsg {
    TabMsg::AutoFetch {
        id,
        kind,
        running,
        error: String::new(),
    }
}

/// The toolbar's two rings: fetch's, push's.
fn rings(tab: &RepoTab) -> (bool, bool) {
    (tab.fetching, tab.pushing)
}

/// A pressed fetch turns the fetch ring, and a push the push ring, from
/// its start — not its asking — past git's answer and the session's
/// settle, until what it left is in the models.
#[test]
fn a_ring_turns_past_gits_answer_until_what_the_write_left_is_in_the_models() {
    for (kind, turning) in [(K::Fetch, (true, false)), (K::Push, (false, true))] {
        let mut tab = RepoTab::default();
        tab.unsettle(OURS);
        assert_eq!(rings(&tab), (false, false), "{kind:?} asked for");
        tab.absorb(vec![started(OURS, kind)]);
        assert_eq!(rings(&tab), turning, "{kind:?} started");
        tab.absorb(vec![answered(OURS, kind, "")]);
        assert_eq!(tab.busy_count, 0, "git has answered the {kind:?}");
        assert_eq!(rings(&tab), turning, "and what it left is not read yet");
        tab.absorb(vec![settle_msg(OURS, kind)]);
        assert_eq!(rings(&tab), turning, "read, and the models yet to drain it");
        tab.settle_drained(OURS);
        assert_eq!(rings(&tab), (false, false), "the {kind:?}'s ring stopped");
    }
}

/// A refused write's ring stops where a landed one's does: the settle
/// comes for both, and the answer stops neither.
#[test]
fn a_refused_push_turns_the_ring_as_long() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![
        started(OURS, K::Push),
        answered(OURS, K::Push, "! [rejected] main -> main (fetch first)"),
    ]);
    assert!(tab.pushing);
    tab.absorb(vec![settle_msg(OURS, K::Push)]);
    assert!(tab.pushing);
    tab.settle_drained(OURS);
    assert!(!tab.pushing);
}

/// The interval's fetch and an opening's turn the fetch ring between the
/// same two boundaries, and count in nothing else: the tab is neither busy
/// nor settling for them.
#[test]
fn a_fetch_nobody_pressed_turns_the_ring_the_same_and_nothing_else() {
    for kind in [K::AutoFetch, K::OpenFetch] {
        let mut tab = RepoTab::default();
        tab.absorb(vec![unasked(SOMEBODY_ELSE, kind, true)]);
        assert_eq!(rings(&tab), (true, false), "{kind:?} started");
        assert_eq!(tab.busy_count, 0);
        assert!(!tab.settling);

        tab.absorb(vec![unasked(SOMEBODY_ELSE, kind, false)]);
        assert_eq!(tab.auto_fetch_seq, 1, "git has answered the {kind:?}");
        assert!(tab.fetching, "and what it brought is not read yet");
        tab.absorb(vec![settle_msg(SOMEBODY_ELSE, kind)]);
        assert!(tab.fetching, "read, and the models yet to drain it");

        tab.settle_drained(SOMEBODY_ELSE);
        assert_eq!(rings(&tab), (false, false));
        assert!(!tab.settling);
    }
}

/// A ring is its own operation's. A commit and a pull turn neither — a
/// pull answers as a write of its own (`RepoTab::fetch_settled`) — and a
/// fetch out beside a push keeps turning once the push's result is in.
#[test]
fn each_ring_follows_its_own_operation() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![started(1, K::Commit), started(2, K::Pull)]);
    assert_eq!(rings(&tab), (false, false));
    tab.absorb(vec![
        answered(1, K::Commit, ""),
        settle_msg(1, K::Commit),
        answered(2, K::Pull, ""),
        settle_msg(2, K::Pull),
    ]);
    assert_eq!(rings(&tab), (false, false));
    tab.settle_drained(1);
    tab.settle_drained(2);

    tab.absorb(vec![started(3, K::Push), unasked(4, K::AutoFetch, true)]);
    assert_eq!(rings(&tab), (true, true));
    tab.absorb(vec![answered(3, K::Push, ""), settle_msg(3, K::Push)]);
    tab.settle_drained(3);
    assert_eq!(rings(&tab), (true, false), "the fetch is still out");
    tab.absorb(vec![
        unasked(4, K::AutoFetch, false),
        settle_msg(4, K::AutoFetch),
    ]);
    tab.settle_drained(4);
    assert_eq!(rings(&tab), (false, false));
}

/// The settles of the worktree being left never come (`forget_the_worktree`).
#[test]
fn standing_elsewhere_stops_the_rings() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![started(1, K::Push), unasked(2, K::OpenFetch, true)]);
    assert_eq!(rings(&tab), (true, true));
    tab.forget_the_worktree();
    assert_eq!(rings(&tab), (false, false));
}
