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
    // And one the walk cannot take away: main reaches this commit with or
    // without the tags, so its chip is the half the option has to answer
    // for on its own.
    repo.git(&["tag", "onmain"]);

    let (sink, session) = open_unawaited(&repo);

    // Tags are walked by default, so the settled opening has two rows —
    // the tag-only commit included. The chips ride the refs snapshot, not
    // the walk: a pass that beats the opening refs read lands its rows
    // bare, and the chips catch up as a `LabelsChanged` diff onto the
    // same generation. So what "is drawn" is the union of the two
    // (`drawn_at`), read off the settled opening, where the refs are in
    // whichever half carried them.
    let base = sink.opened_graph(&session, 2).await;
    let drawn = drawn_at(&sink.events.lock().unwrap(), base.generation);
    assert!(
        drawn.contains(&LabelKind::Tag),
        "the tags are drawn while they are in the graph: {drawn:?}"
    );

    session.set_include_tags(false);
    let off = sink.pass_after("the tag-less graph", base.generation).await;
    assert_eq!(off.total, 1, "the tag-only commit left the walk");
    // The row main still holds keeps its branch chip and loses its tag:
    // taking the tags out of the walk is not the whole of taking them out
    // of the graph, and this is the row where the difference shows.
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

    // Two-phase streaming, deterministically this time. The lookback at
    // the top could only assert the fast pass's order — during an opening,
    // a rebuild may swallow it wordlessly. Here the opening is settled and
    // no other ask is outstanding, so nothing can take the log token from
    // the toggle's own two passes: the fast tag-less pass must land, in
    // full, before the tag-inclusive swap.
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

/// Every chip a batch of rows carries, by kind.
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

/// The walk does not wait for the refs read: a rebuild draws only the
/// chips the session has already read, and a ref it has not is invisible
/// to it however many passes run. The refs read is what carries the
/// chips — as a `LabelsChanged` diff onto the very generation on screen,
/// with no pass in between — and the diff is mirrored into the
/// delivered-rows record so the next rebuild does not swap an identical
/// graph over it (`apply_refs`).
///
/// The opening runs this same exchange with the scheduler picking the
/// order, and the losing order — a pass landing before the opening refs
/// read — turned up as a full-suite flake (a bare 2-row swap read as "no
/// tags drawn"), not as a test. Here the order is held by construction:
/// the ref arrives while nothing is in flight, and each half lands
/// behind its own completion boundary.
#[tokio::test(flavor = "multi_thread")]
async fn chips_catch_up_when_the_refs_read_lands_last() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "base");

    let (sink, session) = open_unawaited(&repo);
    let base = sink.opened_graph(&session, 1).await;

    // A ref moves outside the session: the walk can reach it, the label
    // map has never heard of it.
    repo.git(&["tag", "fresh"]);

    // The rebuild walks the same commit and draws the same chips: this is
    // the half that lands bare under the opening race, said without a
    // scheduler — no repaint carries a ref the session has not read.
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

    // The catch-up also patched the delivered-rows record: the next
    // rebuild finds the picture it would draw already on screen, instead
    // of seeing a phantom difference and swapping an identical graph.
    // The slot boundary first: the refs reader asks for a rebuild of its
    // own right after the diff, and the tracked one below must supersede
    // it, not be superseded by it.
    crate::support::wait::bounded("the refs read slot", session.wait_for_snapshot_reads()).await;
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

    // The opening pass, whichever shape landed it — and the footer read
    // off the very pass the rest of the test anchors on.
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
/// the walk was cut — truncation must follow the walk, not the rows.
#[tokio::test(flavor = "multi_thread")]
async fn truncation_follows_the_walk_not_the_shown_rows() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.commit_file("f.txt", "3\n", "three");
    repo.write_file("f.txt", "wip\n");
    repo.git(&["stash", "push", "-m", "wip stash"]);

    let (sink, session) = open_unawaited(&repo);

    // Full pass first: stash row + three commits, nothing truncated —
    // whichever shape landed it.
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

/// The synthetic WIP row is shown but never walked: a window that holds
/// the whole history must not report truncation just because the WIP row
/// pushes the shown count up to the limit.
///
/// Reached by widening a window that really was cut, rather than by
/// opening the wide one straight away. Both say the same thing about the
/// WIP row, but only the widening changes the graph — and a change is
/// what makes the answer arrive at all. Going straight to the wide window
/// leaves the rows *and the footer* exactly as the opening pass left them
/// (the default window holds this history whole either way), so a rebuild
/// that overtakes the stream (this repository opens dirty, and the status
/// read that notices it asks for one) finds nothing to swap and says
/// nothing, and the test waits for an event that was never sent.
#[tokio::test(flavor = "multi_thread")]
async fn the_wip_row_does_not_trigger_truncation() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.write_file("f.txt", "wip\n"); // dirty → synthetic WIP row

    let (sink, session) = open_unawaited(&repo);

    // Wait until the dirty state is reflected and the stream settles, so
    // the next pass is the reaction to the limit change.
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

/// A window change asks for a stream, but only until somebody else asks
/// for the graph: the stream drops without a word when a background
/// rebuild supersedes it, and that rebuild — reading the options the
/// change just wrote — lands the new window as an atomic replacement
/// instead. The two are the same answer, and which one arrives is a
/// scheduling accident, so waiting for one shape is waiting on a race.
///
/// Held in the swap that adds the WIP row, the interleaving is exact: the
/// stream cannot reach its cancel check until the rebuild behind it has
/// taken its place. Left to the scheduler it is rare — it turned up as a
/// flake on a machine running three other builds, not as a test.
// `worker_threads = 2` is the test's own premise: the hook below parks a
// worker on a blocking `recv`, and a pool inherited from the host can be
// one thread on a small runner — the parked hook then owns it all.
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
    // The sink records before it runs the hook, so the graph the change
    // is measured against is already readable from where it is parked.
    let wip_gen = sink.settled_pass(3).await.generation;

    // The change's stream is stopped at that door; the rebuild asked for
    // behind it takes its place before the stream gets through.
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

/// The same interleaving over a window change that moves nothing but the
/// footer: two commits through a window of two are reported cut (the walk
/// stopped on the limit, which is all truncation can mean), and widening
/// to three leaves every row exactly where it was. The rebuild that
/// overtakes the stream carries the new window, so it is the only thing
/// that can say the history is no longer cut — and a comparison that only
/// looks at rows finds nothing to do, leaving the notice claiming history
/// the user just asked to see.
// `worker_threads = 2`: the parked hook must not own the only worker
// (see the sibling above).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_window_change_only_the_footer_notices_still_lands() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");

    let (sink, session) = open_unawaited(&repo);
    let first_gen = sink.opened_graph(&session, 2).await.generation;

    // A window exactly as wide as the history: every commit is shown, and
    // the walk stopping on the limit is what makes it cut all the same.
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

    // Widen past the end of the history. The stream this asks for is
    // stopped at the door and dropped; the rebuild behind it walks the
    // same two commits, draws the same three rows, and carries the only
    // thing that did change.
    session.set_log_limit(Some(3));
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    let whole = sink.pass_after("the widened window", wip.generation).await;
    assert_eq!(whole.total, 3, "WIP row + both commits, exactly as before");
    assert_eq!(whole.walked, 2, "the walk ran out of history");
    assert!(!whole.truncated, "so nothing is being kept from the user");
    session.close();
}

/// Growing the window is what the graph's tail offers, and it must land
/// **without starting the stream over**: the press is made at the bottom
/// of a graph somebody is reading, and a restart clears the rows and
/// sends them back to the top (`run_direct_pass`). So the wider walk
/// arrives as a replacement, spliced in under what is already drawn.
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

/// The step is measured from the window the graph *opened* with, not
/// from the one it has grown to: a reader who has pressed four times
/// gets the same amount on the fifth press as on the first.
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

    // The door the setting for the initial count comes through
    // (`settings::Defaults::initial_commits`). Under the floor the
    // settings screen offers, because the floor is applied on the way in
    // rather than here — what this pins is that the step follows whatever
    // number arrives.
    session.set_log_limit(Some(400));
    assert_eq!(session.log_window_step(), 100);

    session.grow_log_window();
    assert_eq!(session.log_options().limit, Some(500), "one step wider");
    assert_eq!(session.log_window_step(), 100, "and the step did not grow");

    session.grow_log_window();
    assert_eq!(session.log_options().limit, Some(600));
    assert_eq!(session.log_window_step(), 100);

    // A window widened to the whole history has no next step, and the
    // press that would ask for one is not offered — but the call stays
    // reachable from a graph that finished loading while a hand was on
    // its way down, so it answers by leaving the window where it is.
    session.set_log_limit(None);
    session.grow_log_window();
    assert_eq!(
        session.log_options().limit,
        None,
        "there is nothing past the end of the history to load"
    );

    session.close();
}
