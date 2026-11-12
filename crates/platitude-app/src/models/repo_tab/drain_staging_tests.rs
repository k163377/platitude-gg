//! The tab's own half of a stage pressed ahead of git: what the Commit button
//! waits on (`commit_waits`, `staging_unseen`) and the notify that has
//! the page draw the presses again (`staging_seq`). The presses are the hub's
//! (`ops::StageAhead`, tested in `ops::stage_ahead_tests`): a hub is installed
//! on the test's thread with one tab and no session, so nothing reaches git —
//! a write asked at the door is accepted by nobody, and a press is written
//! down as the door would.

use std::collections::HashSet;

use super::*;
use crate::ops::{Reach, Reader, Side};
use platitude_core::OperationKind as K;

const STAGE: u64 = 7;
const NEXT: u64 = 8;
const FETCH: u64 = 9;

/// Status stamps either side of a write's end.
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

const READERS: [Reader; 4] = [
    Reader::Conflicts,
    Reader::Unstaged,
    Reader::Staged,
    Reader::Counts,
];

const LOCKED: &str = "fatal: Unable to create '.git/index.lock': File exists.";

/// A tab on the installed hub's one tab.
fn tab() -> RepoTab {
    RepoTab {
        tab_id: crate::hub::testkit::installed_tab(),
        ..RepoTab::default()
    }
}

/// A stage press on `path` accepted under `id`: the door's half
/// (`ask_session` counts the write out), then the press's own
/// (`pressed_ahead`).
fn press(tab: &mut RepoTab, id: u64, path: &str) {
    tab.write_out(id);
    let reach = Reach::Paths([path.to_string()].into_iter().collect::<HashSet<_>>());
    assert!(
        tab.pressed_ahead(Side::Stage, reach, Some(id)),
        "the press moves what QML reads"
    );
}

/// Every reader draws a status that looked at `looked`, and the tab drains
/// what the hub queued on its feed for that — nothing put there by hand.
fn every_reader_drew(tab: &mut RepoTab, looked: u64) {
    for reader in READERS {
        crate::hub::status_drawn(tab.tab_id, reader, looked);
    }
    let told = crate::hub::testkit::tab_feed_drained(tab.tab_id);
    tab.absorb(told);
}

/// The tab stands in another worktree: the hub's own restand lets go of the
/// old session — opened here, on the runtime nobody drives — and with it its
/// presses (`Hub::restand_tab`), and the tab of what it waited on
/// (`RepoTab::restand`).
fn stand_elsewhere(tab: &mut RepoTab) {
    let id = tab.tab_id;
    crate::hub::testkit::open_installed_tab(id);
    crate::hub::Hub::with(|hub| hub.restand_tab(id, std::path::PathBuf::from("elsewhere")));
    tab.forget_the_worktree();
}

fn start(id: u64, kind: K) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind,
        running: true,
        error: String::new(),
        report: None,
        head_seq: 0,
        reads_from: 0,
    }
}

/// A stage press landed, its fence at `reads_from`.
fn answer_at(id: u64, reads_from: u64) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind: K::Stage,
        running: false,
        error: String::new(),
        report: None,
        head_seq: 0,
        reads_from,
    }
}

fn answer(id: u64, kind: K, error: &str) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind,
        running: false,
        error: error.into(),
        report: None,
        head_seq: 0,
        reads_from: FENCE,
    }
}

/// A slot's press goes through the door — a session opened on the runtime
/// nobody drives, so the write is accepted and never starts — and is
/// written down with the hub under the id the door gave it: the row is
/// reached, the Commit button waits, and the page draws again
/// (`press_told` → `move_paths` → `pressed_ahead`).
#[test]
fn a_press_through_the_door_is_written_down_under_its_id() {
    let mut tab = tab();
    crate::hub::testkit::open_installed_tab(tab.tab_id);
    let seq = tab.staging_seq;
    tab.press_told(|tab| tab.move_paths(Side::Stage, vec!["a.txt".to_string()]));
    assert_eq!(tab.outstanding.len(), 1, "counted out at the door");
    assert!(crate::hub::stage_ahead(tab.tab_id, |ahead| {
        ahead.reaches("unstaged", "a.txt")
    }));
    assert!(tab.staging_unseen && tab.commit_waits);
    assert_ne!(tab.staging_seq, seq, "the page draws the press");
}

/// Commit waits from the press until every reader has drawn the status after
/// it — landed is not enough: until then the rows are the press's word — and
/// the page draws again as it opens (the diff open on a moved file follows
/// then, `RepoPage.followEmptySide`).
#[test]
fn commit_waits_until_the_screen_has_caught_up_with_a_press() {
    let mut tab = tab();
    press(&mut tab, STAGE, "a.txt");
    assert!(tab.staging_unseen);
    assert!(tab.commit_waits, "Commit waits on the press");
    tab.absorb(vec![start(STAGE, K::Stage), answer(STAGE, K::Stage, "")]);
    assert!(
        tab.outstanding.is_empty(),
        "answered, the write is no longer out"
    );
    assert!(tab.commit_waits, "landed, its status not yet drawn");
    let seq = tab.staging_seq;
    every_reader_drew(&mut tab, AFTERWARDS);
    assert!(!tab.staging_unseen);
    assert!(!tab.commit_waits);
    assert_ne!(
        tab.staging_seq, seq,
        "the page draws again: the open diff follows now"
    );
}

/// A refused press goes with its answer: Commit waits no more.
#[test]
fn a_refused_press_lets_commit_go_at_its_answer() {
    let mut tab = tab();
    press(&mut tab, STAGE, "a.txt");
    tab.absorb(vec![
        start(STAGE, K::Stage),
        answer(STAGE, K::Stage, LOCKED),
    ]);
    assert!(!tab.staging_unseen);
    assert!(!tab.commit_waits);
    assert!(tab.write_refused, "the refusal is the group's to tell");
}

/// Two presses: Commit waits until the screen has caught up with both.
#[test]
fn commit_waits_on_every_press_standing() {
    let mut tab = tab();
    press(&mut tab, STAGE, "a.txt");
    press(&mut tab, NEXT, "b.txt");
    tab.absorb(vec![answer(STAGE, K::Stage, "")]);
    every_reader_drew(&mut tab, AFTERWARDS);
    assert!(tab.commit_waits, "the second is still out");
    tab.absorb(vec![answer_at(NEXT, AFTERWARDS + 1)]);
    assert!(tab.commit_waits, "landed, its status not yet drawn");
    every_reader_drew(&mut tab, AFTERWARDS + 1);
    assert!(!tab.commit_waits);
}

/// A write another model asked for (a restore off the discard record) counts
/// out for the Commit button from its acceptance, as the tab's own do, and
/// sets the tab settling.
#[test]
fn a_write_asked_by_another_model_counts_out_from_its_acceptance() {
    let mut tab = tab();
    assert!(tab.counted_out(NEXT), "it was settled before");
    assert!(tab.settling);
    assert!(tab.commit_waits);
    tab.absorb(vec![answer(NEXT, K::Stage, "")]);
    assert!(!tab.commit_waits);
}

/// The Commit button waits on every write out besides — queued ones too,
/// which no count of running writes sees — and on a tab stood elsewhere
/// until its new session answers.
#[test]
fn the_commit_button_waits_on_writes_out_beyond_the_stage_presses() {
    let mut tab = tab();
    tab.absorb(vec![start(FETCH, K::Fetch)]);
    assert!(tab.commit_waits, "a fetch");
    tab.absorb(vec![answer(FETCH, K::Fetch, "")]);
    assert!(!tab.commit_waits);
    assert!(tab.outstanding.is_empty(), "answered, no longer out");

    // Accepted and queued behind another write: not running, still out. A
    // commit let in now would carry its work.
    tab.write_out(NEXT);
    assert_eq!(tab.busy_count, 0, "nothing has started");
    assert!(tab.commit_waits);

    // Its answer will not come: the session went with the stand.
    stand_elsewhere(&mut tab);
    assert!(tab.commit_waits, "standing in another worktree");
    tab.stood();
    assert!(!tab.commit_waits, "nothing of the old session is out");
}

/// The belt under the buttons that take the index whole: a commit, the
/// details pane's amend of HEAD, a stash or an operation's `--continue`
/// asked while a press stands never reaches
/// the door — the run's watch stays armed, where the door (no session here)
/// would turn it down. A way out that takes no index goes on, and once the
/// screen has caught up a commit goes to the door again.
#[test]
fn a_write_that_takes_the_index_is_turned_away_while_a_press_stands() {
    let mut tab = tab();
    press(&mut tab, STAGE, "a.txt");
    let reached = |tab: &mut RepoTab, ask: &dyn Fn(&mut RepoTab)| {
        assert!(tab.write_watch.arm("probe").is_none());
        ask(tab);
        tab.write_watch.stage() == "turned-down"
    };
    assert!(!reached(&mut tab, &|tab| tab.commit_from_fields(
        "s".into(),
        String::new(),
        false,
        false
    )));
    assert!(!reached(&mut tab, &|tab| tab.reword_from_fields(
        "0".repeat(40),
        "s".into(),
        String::new()
    )));
    assert!(!reached(&mut tab, &|tab| tab.stash_push(String::new())));
    assert!(!reached(&mut tab, &|tab| tab.settle_operation("continue".into())));
    assert!(
        reached(&mut tab, &|tab| tab.settle_operation("abort".into())),
        "a way out that takes no index"
    );

    tab.absorb(vec![answer(STAGE, K::Stage, "")]);
    every_reader_drew(&mut tab, AFTERWARDS);
    assert!(reached(&mut tab, &|tab| tab.commit_from_fields(
        "s".into(),
        String::new(),
        false,
        false
    )));
}

/// Stood in another worktree, a press of the old session waits on nothing:
/// its answer and its status went with that session.
#[test]
fn standing_elsewhere_lets_a_press_go() {
    let mut tab = tab();
    press(&mut tab, STAGE, "a.txt");
    stand_elsewhere(&mut tab);
    assert!(
        !crate::hub::stage_ahead(tab.tab_id, |ahead| ahead.unseen()),
        "the hub let go of the old session's press"
    );
    tab.stood();
    assert!(!tab.staging_unseen);
    assert!(!tab.commit_waits);
}
