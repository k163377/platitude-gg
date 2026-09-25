//! The log window: what the walk covers, what of it is shown, and what
//! moving the window lands.

use crate::support::TestRepo;
use crate::support::session::{open_unawaited, pass_of};
use platitude_core::session::{LabelKind, RefreshOutcome, SessionEvent};

#[tokio::test(flavor = "multi_thread")]
async fn tag_only_commits_follow_the_include_tags_option() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "base");
    // A commit reachable only through a tag (detached, then back to main).
    repo.git(&["checkout", "--detach", "HEAD"]);
    repo.commit_file("g.txt", "t\n", "tag only work");
    repo.git(&["tag", "islet"]);
    repo.git(&["checkout", "main"]);
    // And a tag on main: the walk keeps its commit either way, so its chip
    // is what the option alone answers for.
    repo.git(&["tag", "onmain"]);

    let (sink, session) = open_unawaited(&repo);

    // Tags are walked by default: two rows. A pass that beats the refs read
    // lands bare and the chips catch up as a `LabelsChanged` diff onto the
    // same generation, so what is drawn is the union (`drawn_at`).
    let base = sink.opened_graph(&session, 2).await;
    let drawn = drawn_at(&sink.events.lock().unwrap(), base.generation);
    assert!(
        drawn.contains(&LabelKind::Tag),
        "the tags are drawn while they are in the graph: {drawn:?}"
    );

    session.set_include_tags(false);
    let off = sink.pass_after("the tag-less graph", base.generation).await;
    assert_eq!(off.total, 1, "the tag-only commit left the walk");
    // The row main holds keeps its branch chip and loses its tag: out of the
    // walk is not out of the graph.
    {
        let left = drawn_at(&sink.events.lock().unwrap(), off.generation);
        assert!(
            left.contains(&LabelKind::LocalBranch),
            "the branch is still drawn: {left:?}"
        );
        assert!(
            !left.contains(&LabelKind::Tag),
            "no tag is drawn with the tags out of the graph: {left:?}"
        );
    }

    // Two-phase streaming after a settled baseline: nothing can take the log
    // token from the toggle's two passes, so the fast tag-less pass must
    // land whole before the tag-inclusive swap.
    let base = sink.opened_graph(&session, 1).await;
    session.set_include_tags(true);
    let swap = sink
        .wait_for("the tag-inclusive swap", move |evs| {
            evs.iter()
                .filter_map(pass_of)
                .find(|p| p.generation > base.generation && p.total == 2)
        })
        .await;
    let fast = {
        let events = sink.events.lock().unwrap();
        events.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                generation, total, ..
            } if *generation > base.generation && *generation < swap.generation => Some(*total),
            _ => None,
        })
    };
    assert_eq!(
        fast,
        Some(1),
        "the fast tag-less pass paints first, and paints whole"
    );

    session.close();
}

fn chips_of(rows: &[platitude_core::session::LogRow]) -> Vec<LabelKind> {
    rows.iter()
        .flat_map(|row| row.labels.iter().map(|label| label.kind))
        .collect()
}

/// Every chip drawn onto the pass `generation`: the rows as they landed,
/// plus the `LabelsChanged` diffs that caught up with them afterwards.
fn drawn_at(events: &[SessionEvent], generation: u64) -> Vec<LabelKind> {
    events
        .iter()
        .flat_map(|e| match e {
            SessionEvent::LogChunk {
                generation: g,
                rows,
            }
            | SessionEvent::LogReplaced {
                generation: g,
                rows,
                ..
            } if *g == generation => chips_of(rows),
            SessionEvent::LabelsChanged {
                generation: g,
                rows,
            } if *g == generation => rows
                .iter()
                .flat_map(|(_, labels)| labels.iter().map(|label| label.kind))
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// A rebuild draws only chips the session has already read. The refs read
/// carries a new ref's chips as a `LabelsChanged` diff onto the generation
/// on screen, mirrored into the delivered-rows record so the next rebuild
/// has nothing to swap (`apply_refs`).
///
/// The order is held by construction: the ref arrives while nothing is in
/// flight, and each half lands behind its own completion boundary.
#[tokio::test(flavor = "multi_thread")]
async fn chips_catch_up_when_the_refs_read_lands_last() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "base");

    let (sink, session) = open_unawaited(&repo);
    let base = sink.opened_graph(&session, 1).await;

    // A ref moves outside the session: the walk can reach it, the label
    // map has never heard of it.
    repo.git(&["tag", "fresh"]);

    // The rebuild walks the same commit and draws the same chips: no
    // repaint carries a ref the session has not read.
    let outcome = crate::support::wait::bounded(
        "the rebuild before the refs read",
        session.refresh_log_tracked().outcome(),
    )
    .await;
    assert_eq!(
        outcome,
        RefreshOutcome::Unchanged,
        "a rebuild cannot draw refs the session has not read"
    );

    session.refresh_refs();
    let (caught_gen, chips) = sink
        .wait_for("the chips catch up", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LabelsChanged { generation, rows } => {
                    let kinds: Vec<LabelKind> = rows
                        .iter()
                        .flat_map(|(_, labels)| labels.iter().map(|label| label.kind))
                        .collect();
                    kinds
                        .contains(&LabelKind::Tag)
                        .then_some((*generation, kinds))
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(
        caught_gen, base.generation,
        "the diff names the graph on screen"
    );
    assert!(
        chips.contains(&LabelKind::LocalBranch),
        "the diff carries the row's whole chip set: {chips:?}"
    );

    // The refs reader asks for a rebuild of its own from inside its pass, so
    // the flight boundary covers it; the tracked one below supersedes it.
    crate::support::wait::bounded("the refs read flight", session.wait_for_snapshot_reads()).await;
    let outcome = crate::support::wait::bounded(
        "the rebuild after the catch-up",
        session.refresh_log_tracked().outcome(),
    )
    .await;
    assert_eq!(
        outcome,
        RefreshOutcome::Unchanged,
        "the chips were mirrored into the delivered rows"
    );

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn log_limit_truncates_the_window() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.commit_file("f.txt", "3\n", "three");

    let (sink, session) = open_unawaited(&repo);

    let full = sink.opened_graph(&session, 3).await;
    assert!(!full.truncated, "3 commits fit in the default window");
    let first_gen = full.generation;

    session.set_log_limit(Some(2));
    let limited = sink.pass_after("the limited window", first_gen).await;
    assert_eq!(limited.total, 2);
    assert_eq!(limited.walked, 2, "the footer's number is the limit itself");
    assert!(limited.truncated);

    session.close();
}

/// A stash's synthetic index parent is walked but sifted out of the
/// shown rows, so the shown count sits below the window limit even when
/// the walk was cut — truncation must follow the walk.
#[tokio::test(flavor = "multi_thread")]
async fn truncation_follows_the_walk_not_the_shown_rows() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.commit_file("f.txt", "3\n", "three");
    repo.write_file("f.txt", "wip\n");
    repo.git(&["stash", "push", "-m", "wip stash"]);

    let (sink, session) = open_unawaited(&repo);

    // Stash row + three commits, nothing truncated.
    let full = sink.opened_graph(&session, 4).await;
    assert!(!full.truncated);
    let first_gen = full.generation;

    // The walk emits 4 rows (stash, its index parent, "three", "two") and
    // is cut before "one"; the sifted index parent leaves 3 shown rows.
    session.set_log_limit(Some(4));
    let limited = sink.pass_after("the limited window", first_gen).await;
    assert_eq!(limited.total, 3, "stash + three + two, index parent sifted");
    assert_eq!(limited.walked, 4, "the walk count stays on the limit");
    assert!(limited.truncated, "the walk was cut before the root commit");

    session.close();
}

/// The synthetic WIP row is shown but never walked: a window that holds the
/// whole history says so, whatever the WIP row adds to the shown count.
///
/// Reached by widening a window that really was cut: going straight to the
/// wide window changes nothing, so an overtaking rebuild says nothing and
/// the wait never ends.
#[tokio::test(flavor = "multi_thread")]
async fn the_wip_row_does_not_trigger_truncation() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.write_file("f.txt", "wip\n"); // dirty → synthetic WIP row

    let (sink, session) = open_unawaited(&repo);

    // Settled with the WIP row, so the next pass reacts to the limit change.
    let first_gen = sink.opened_graph(&session, 3).await.generation;

    // One commit through a window of one: cut, and the WIP row rides on
    // top of it regardless.
    session.set_log_limit(Some(1));
    let cut = sink.pass_after("the cut window", first_gen).await;
    assert_eq!(cut.total, 2, "WIP row + the one commit walked");
    assert_eq!(cut.walked, 1, "the walk stopped on the limit");
    assert!(cut.truncated, "older history exists and is not shown");

    // Two commits walk through a window of three; the WIP row makes three
    // shown rows, which is not a truncated window.
    session.set_log_limit(Some(3));
    let whole = sink.pass_after("the widened window", cut.generation).await;
    assert_eq!(whole.total, 3, "WIP row + two commits");
    assert_eq!(whole.walked, 2, "the WIP row is shown but never walked");
    assert!(!whole.truncated, "the whole history fits the window");

    session.close();
}

/// A window change's stream drops silently when a background rebuild
/// supersedes it, and that rebuild — reading the options the change just
/// wrote — lands the new window as a replacement instead. Which one
/// arrives is a scheduling accident, so waiting for one shape is waiting
/// on a race.
///
/// Held in the swap that adds the WIP row, the stream cannot reach its
/// cancel check until the rebuild behind it has taken its place.
// `worker_threads = 2`: the hook below parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_window_change_a_rebuild_overtakes_still_lands_the_new_window() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");

    let (sink, session) = open_unawaited(&repo);
    sink.opened_graph(&session, 2).await;

    // Park in the swap that adds the WIP row: it sends under the graph
    // lock, so every pass asked for from here waits at that door.
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3),
        move || {
            held.recv().expect("the test releases the graph swap");
        },
    );
    repo.write_file("f.txt", "wip\n");
    session.refresh_status();
    sink.wait_for("the rebuild reached the window", |events| {
        events
            .iter()
            .any(|event| matches!(event, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3))
            .then_some(())
    })
    .await;
    // The sink records before the hook runs, so the parked graph is readable.
    let wip_gen = sink.settled_pass(3).await.generation;

    session.set_log_limit(Some(1));
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    let cut = sink.pass_after("the new window", wip_gen).await;
    assert_eq!(cut.total, 2, "WIP row + the one commit walked");
    assert_eq!(cut.walked, 1, "the walk stopped on the limit");
    assert!(cut.truncated, "older history exists and is not shown");
    assert_eq!(
        sink.count(
            |e| matches!(e, SessionEvent::LogFinished { generation, .. } if *generation > wip_gen)
        ),
        0,
        "the superseded stream stayed silent, so the replacement is the \
         only thing that could have carried the window: {:?}",
        sink.events.lock().unwrap()
    );
    session.close();
}

/// The same interleaving over a change only the footer notices: two commits
/// through a window of two are cut, and widening to three moves no row. The
/// overtaking rebuild is the only thing that can say so — a comparison over
/// rows alone leaves the footer claiming hidden history.
// `worker_threads = 2`: the hook below parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_window_change_only_the_footer_notices_still_lands() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");

    let (sink, session) = open_unawaited(&repo);
    let first_gen = sink.opened_graph(&session, 2).await.generation;

    // A window exactly as wide as the history is still cut: the walk
    // stopped on the limit.
    session.set_log_limit(Some(2));
    let cut = sink.pass_after("the window on the limit", first_gen).await;
    assert_eq!(cut.total, 2, "both commits fit");
    assert_eq!(cut.walked, 2, "the walk stopped on the limit");
    assert!(cut.truncated, "which is all the footer knows");

    // Park in the swap that adds the WIP row (see the test above): from
    // here every pass waits at the graph lock.
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3),
        move || {
            held.recv().expect("the test releases the graph swap");
        },
    );
    repo.write_file("f.txt", "wip\n");
    session.refresh_status();
    sink.wait_for("the rebuild reached the window", |events| {
        events
            .iter()
            .any(|event| matches!(event, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3))
            .then_some(())
    })
    .await;
    let wip = sink.settled_pass(3).await;
    assert!(wip.truncated, "the window is still sitting on the limit");

    // Widen past the history: the stream is dropped at the door, and the
    // rebuild behind it carries the only change, the footer.
    session.set_log_limit(Some(3));
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    let whole = sink.pass_after("the widened window", wip.generation).await;
    assert_eq!(whole.total, 3, "WIP row + both commits, exactly as before");
    assert_eq!(whole.walked, 2, "the walk ran out of history");
    assert!(!whole.truncated, "so nothing is being kept from the user");
    session.close();
}

/// Growing the window lands as a replacement: the press is made at the
/// bottom of a graph somebody is reading, and a restart would clear the
/// rows and send the reader back to the top (`run_direct_pass`).
#[tokio::test(flavor = "multi_thread")]
async fn growing_the_window_walks_further_without_starting_over() {
    let mut repo = TestRepo::init();
    for n in 1..=6 {
        repo.commit_file("f.txt", &format!("{n}\n"), &format!("commit {n}"));
    }

    let (sink, session) = open_unawaited(&repo);

    let full = sink.opened_graph(&session, 6).await;
    assert!(!full.truncated, "6 commits fit in the default window");

    // A window of four, whose step is therefore one commit.
    session.set_log_limit(Some(4));
    let cut = sink.pass_after("the limited window", full.generation).await;
    assert_eq!(cut.walked, 4);
    assert!(cut.truncated, "two commits are past the cut");

    let streams = sink.count(|e| matches!(e, SessionEvent::LogStarted { .. }));
    session.grow_log_window();
    let grown = sink.pass_after("the grown window", cut.generation).await;
    assert_eq!(grown.walked, 5, "the step is a quarter of the window");
    assert_eq!(grown.total, 5);
    assert!(grown.truncated, "one commit is still past the cut");
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::LogStarted { .. })),
        streams,
        "the wider walk was spliced in, not streamed over a cleared graph"
    );

    session.close();
}

/// The step is measured from the window the graph *opened* with: a
/// reader who has pressed four times gets the same amount on the
/// fifth press as on the first.
#[tokio::test(flavor = "multi_thread")]
async fn the_step_stays_a_quarter_of_the_window_the_graph_opened_with() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");

    let (sink, session) = open_unawaited(&repo);
    sink.opened_graph(&session, 1).await;

    assert_eq!(
        session.log_window_step(),
        platitude_core::session::DEFAULT_LOG_LIMIT / 4,
        "the default window's own quarter"
    );

    // The door the initial-count setting comes through
    // (`settings::Defaults::initial_commits`). 400 is under the settings
    // screen's floor, which is applied on the way in: the step follows
    // whatever arrives.
    session.set_log_limit(Some(400));
    assert_eq!(session.log_window_step(), 100);

    session.grow_log_window();
    assert_eq!(session.log_options().limit, Some(500), "one step wider");
    assert_eq!(session.log_window_step(), 100, "and the step did not grow");

    session.grow_log_window();
    assert_eq!(session.log_options().limit, Some(600));
    assert_eq!(session.log_window_step(), 100);

    // A whole-history window has no next step. The press is not offered,
    // but can still arrive from a graph that finished loading mid-reach, so
    // it leaves the window where it is.
    session.set_log_limit(None);
    session.grow_log_window();
    assert_eq!(
        session.log_options().limit,
        None,
        "there is nothing past the end of the history to load"
    );

    session.close();
}
