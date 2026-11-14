use super::drain_inputs::{OURS, SOMEBODY_ELSE, answered, started};
use super::*;
use platitude_core::OperationKind as K;

/// The last boundary. A settle carries only the id; `_kind` keeps the
/// calls reading like `answered` and `started`.
fn settle_msg(id: u64, _kind: K) -> TabMsg {
    TabMsg::WriteSettled { id }
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
    assert!(!tab.wrote_through());

    tab.absorb(vec![
        answered(SOMEBODY_ELSE, K::Branch, ""),
        settle_msg(SOMEBODY_ELSE, K::Branch),
    ]);
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
    let feed = Arc::new(Feed::default());
    let mut tab = RepoTab {
        feed: Some(feed.clone()),
        ..RepoTab::default()
    };
    assert!(tab.unsettle(1), "asked for on a settled tab");
    assert!(!tab.unsettle(2), "and another queued behind it");
    tab.absorb(vec![started(1, K::Commit)]);
    tab.absorb(vec![answered(1, K::Commit, "")]);
    assert_eq!(tab.busy_count, 0, "git has answered");
    assert!(tab.settling, "and its reading is still out");
    tab.absorb(vec![TabMsg::WriteSettled { id: 9 }]);
    tab.absorb(vec![TabMsg::WriteSettled { id: 1 }]);
    assert!(
        feed.drain().is_empty(),
        "nothing is drained for while the one queued behind has not started"
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
    let posted = feed.drain();
    assert!(matches!(posted.as_slice(), [TabMsg::ReadsDrained]));
    tab.absorb(posted);
    assert!(!tab.settling);
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
    tab.absorb(vec![TabMsg::ReadsDrained]);
    assert!(tab.settling);
}
