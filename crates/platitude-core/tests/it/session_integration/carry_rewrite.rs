//! Rewrites driven through the session — squash, reword, rebase — and the
//! dirty work they carry across.

use crate::support::TestRepo;
use crate::support::session::{
    CaptureSink, install_todo_editor, opened, write_result, write_stopped,
};
use platitude_core::OperationKind;
use platitude_core::sequencer::RebaseStep;
use platitude_core::session::SessionEvent;

#[tokio::test(flavor = "multi_thread")]
async fn squash_and_reword_run_through_the_write_queue() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "keep me");
    let fold = repo.commit_file_id("c.txt", "three\n", "fold me in");

    let (sink, session) = opened(&repo).await;
    session.squash_into_parent(fold);
    assert_eq!(write_result(&sink, OperationKind::Squash).await, None);
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");

    // Rewording HEAD takes the amend path: no replay, same parent.
    let parent = repo.git(&["rev-parse", "HEAD~1"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    session.reword(head, "reworded head\n".into());
    assert_eq!(write_result(&sink, OperationKind::Reword).await, None);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "reworded head");
    assert_eq!(repo.git(&["rev-parse", "HEAD~1"]), parent);
    session.close();
}

/// The route a rewrite took, as the commands it issued.
///
/// Matched from the front: a plain `contains` counts the stopped rebase's
/// `rev-parse --git-path rebase-merge/msgnum` status read as a fourth rebase.
fn rewrite_route(sink: &CaptureSink) -> Vec<&'static str> {
    let mut out = Vec::new();
    for event in sink.events.lock().unwrap().iter() {
        let SessionEvent::CommandStarted { display, .. } = event else {
            continue;
        };
        // Longest first: "stash pop --index" also starts with "stash
        // pop", and every interactive rebase with "rebase".
        for step in [
            "rebase --interactive",
            "stash push",
            "stash pop --index",
            "stash pop",
            "rebase",
        ] {
            if display.starts_with(&format!("git {step}")) {
                out.push(step);
                break;
            }
        }
    }
    out
}

fn stopped_part_way(repo: &TestRepo) -> bool {
    repo.path.join(".git").join("rebase-merge").exists()
}

/// A squash over a dirty tree goes round through `stash` → `squash` →
/// `stash pop --index` (デザイン規約 §未コミット変更がある状態で履歴を書き換える),
/// so the staged and unstaged halves stay apart — `--autostash` restores
/// with a plain apply and brings everything back unstaged.
#[tokio::test(flavor = "multi_thread")]
async fn a_squash_over_a_dirty_tree_carries_the_work_across() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "keep me");
    let fold = repo.commit_file_id("c.txt", "three\n", "fold me in");
    repo.write_file("a.txt", "staged edit\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("b.txt", "unstaged edit\n");
    repo.write_file("u.txt", "untracked\n");

    let (sink, session) = opened(&repo).await;
    session.squash_into_parent(fold);
    assert_eq!(write_result(&sink, OperationKind::Squash).await, None);

    assert_eq!(
        rewrite_route(&sink),
        vec![
            // Refused, without touching anything.
            "rebase --interactive",
            "stash push",
            "rebase --interactive",
            "stash pop --index",
        ]
    );
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(
        repo.git(&["diff", "--name-only", "--cached"]),
        "a.txt",
        "the staged half is still staged"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only"]),
        "b.txt",
        "and the unstaged half still is not"
    );
    assert_eq!(
        repo.git(&["ls-files", "--others", "--exclude-standard"]),
        "u.txt"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
    session.close();
}

/// What a `rebase <current> onto it` fires with: the flags the two menu
/// rows pass — no autostash knob among them.
fn rebase_onto() -> platitude_core::integrate::RebaseOptions {
    platitude_core::integrate::RebaseOptions {
        update_refs: true,
        ..Default::default()
    }
}

/// A rebase onto a new base goes round the same way (not `--autostash`, as
/// with the squash above), so whether staging survives a rewrite does not
/// depend on which menu row was clicked.
#[tokio::test(flavor = "multi_thread")]
async fn a_rebase_onto_over_a_dirty_tree_carries_the_work_across() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["switch", "-c", "topic"]);
    repo.commit_file("t.txt", "topic\n", "topic one");
    repo.git(&["switch", "main"]);
    repo.commit_file("m.txt", "main\n", "main moved");
    repo.git(&["switch", "topic"]);
    repo.write_file("a.txt", "staged edit\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("t.txt", "unstaged edit\n");
    repo.write_file("u.txt", "untracked\n");

    let (sink, session) = opened(&repo).await;
    session.rebase("main".into(), rebase_onto());
    assert_eq!(write_result(&sink, OperationKind::Rebase).await, None);

    assert_eq!(
        rewrite_route(&sink),
        vec![
            // Refused, without touching anything.
            "rebase",
            "stash push",
            "rebase",
            "stash pop --index",
        ]
    );
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "3");
    assert_eq!(
        repo.git(&["diff", "--name-only", "--cached"]),
        "a.txt",
        "the staged half is still staged"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only"]),
        "t.txt",
        "and the unstaged half still is not"
    );
    assert_eq!(
        repo.git(&["ls-files", "--others", "--exclude-standard"]),
        "u.txt"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
    session.close();
}

/// When that rebase stops, the work waits in the stash as a stopped
/// replay's does — the entry is the person's to pop, where `--autostash`
/// would have restored it after `--continue` / `--abort`.
#[tokio::test(flavor = "multi_thread")]
async fn a_rebase_onto_that_stops_leaves_the_work_in_the_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.commit_file("keep.txt", "keep\n", "second");
    repo.git(&["switch", "-c", "topic"]);
    repo.commit_file("f.txt", "topic's line\n", "topic one");
    repo.git(&["switch", "main"]);
    repo.commit_file("f.txt", "main's line\n", "main moved");
    repo.git(&["switch", "topic"]);
    repo.write_file("keep.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.rebase("main".into(), rebase_onto());
    assert_eq!(
        write_result(&sink, OperationKind::Rebase).await,
        None,
        "a stop is not a failed write (by design)"
    );
    assert!(
        write_stopped(&sink, OperationKind::Rebase),
        "and the landing is said out loud, because the answer cannot say it"
    );

    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase", "stash push", "rebase"],
        "no restore is attempted over a tree git is still holding"
    );
    assert!(stopped_part_way(&repo), "the operation is waiting");
    assert_eq!(repo.git(&["stash", "list"]).lines().count(), 1);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("keep.txt")).expect("read"),
        "keep\n",
        "the uncommitted edit is in the entry, not in the tree"
    );
    session.close();
}

/// Untracked files are not in a replay's way, so no stash is taken for
/// them. The route is the whole assertion: a needless stash would end with
/// the same tree.
#[tokio::test(flavor = "multi_thread")]
async fn untracked_files_alone_are_replayed_straight_over() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let gone = repo.commit_file_id("b.txt", "two\n", "drop me");
    repo.commit_file("c.txt", "three\n", "keep me");
    repo.write_file("u.txt", "untracked\n");

    let (sink, session) = opened(&repo).await;
    session.drop_commit(gone);
    assert_eq!(write_result(&sink, OperationKind::Drop).await, None);

    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase --interactive"],
        "one command, no stash"
    );
    assert!(!repo.path.join("b.txt").exists(), "the commit went");
    assert_eq!(
        repo.git(&["ls-files", "--others", "--exclude-standard"]),
        "u.txt"
    );
    session.close();
}

/// The replay goes through and the restore collides: markers in the files
/// and the entry kept, as a move lands
/// (デザイン規約 §未コミット変更がある状態での移動). Nothing failed.
#[tokio::test(flavor = "multi_thread")]
async fn a_restore_that_collides_lands_in_the_files_and_keeps_the_entry() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("file.txt", "a\nb\nc\n", "root");
    let gone = repo.commit_file_id("file.txt", "a\nmiddle\nc\n", "drop me");
    repo.commit_file("other.txt", "side\n", "keep me");
    // Touches the same line the dropped commit did, and nothing the
    // replay itself has to apply.
    repo.write_file("file.txt", "a\nlocal\nc\n");

    let (sink, session) = opened(&repo).await;
    session.drop_commit(gone);
    assert_eq!(
        write_result(&sink, OperationKind::Drop).await,
        None,
        "nothing failed"
    );

    assert_eq!(
        rewrite_route(&sink),
        vec![
            "rebase --interactive",
            "stash push",
            "rebase --interactive",
            "stash pop --index",
        ]
    );
    assert!(!stopped_part_way(&repo), "the replay itself finished");
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"]),
        "file.txt",
        "the collision is in the file, waiting to be settled"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "and the work is still in the stash as well"
    );
    session.close();
}

/// The replay stops part-way and the restore is held back: a pop onto an
/// index with unmerged paths does nothing yet reports a collision
/// (rules-refs/core.md「`stash pop` の非ゼロを conflict と読めるのは」).
/// The work waits in the stash until the operation is over.
#[tokio::test(flavor = "multi_thread")]
async fn a_replay_that_stops_part_way_leaves_the_work_in_the_stash() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "base\n", "root");
    repo.commit_file("file.txt", "one\n", "one");
    let gone = repo.commit_file_id("file.txt", "one\ntwo\n", "drop me");
    repo.commit_file("file.txt", "one\ntwo\nthree\n", "needs the one before");
    repo.write_file("base.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.drop_commit(gone);
    assert_eq!(
        write_result(&sink, OperationKind::Drop).await,
        None,
        "a stop is not a failed write (by design)"
    );
    assert!(
        write_stopped(&sink, OperationKind::Drop),
        "and the landing is said out loud, because the answer cannot say it"
    );

    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase --interactive", "stash push", "rebase --interactive"],
        "no restore is attempted over a tree git is still holding"
    );
    assert!(stopped_part_way(&repo), "the operation is waiting");
    assert_eq!(repo.git(&["stash", "list"]).lines().count(), 1);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("base.txt")).expect("read"),
        "base\n",
        "the uncommitted edit is in the entry, not in the tree"
    );
    session.close();
}

/// Whether the carry has begun — the one delivery that lands between the
/// two spawns of a replay that was refused over a dirty tree.
fn reached_the_stash(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. }
        if display.starts_with("git stash push"))
}

/// The composed plan is a fixed list of ids the helper writes over git's
/// todo whole, and the carry spawns the replay twice (refused over the
/// dirty tree, then after a `detect` and a stash) — so a commit typed in
/// between is not in the list, and a rebase drops what the todo leaves out
/// without saying so. The pin sits in front of every spawn.
///
/// The session is held inside the sink delivery that announces the stash
/// while the test commits from its own thread
/// ([`crate::support::session::CaptureSink::hook_once`]).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_landing_inside_the_carry_is_refused_rather_than_dropped() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let head = repo.commit_file_id("b.txt", "two\n", "the plan's only step");
    let base = repo.git(&["rev-parse", "HEAD~1"]);
    // Dirty, so the first spawn is refused and the write goes round the stash.
    repo.write_file("a.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(reached_the_stash, move || {
        held.recv().expect("the test releases the carry");
    });
    session.rebase_interactive(
        base,
        vec![RebaseStep::pick(head.clone(), "the plan's only step")],
        platitude_core::integrate::RebaseOptions::default(),
        // The tip the plan was composed against: sound when it was pressed.
        head.clone(),
    );
    // The sink records before running the hook, so the delivery holding
    // the write answers this.
    sink.wait_for("the carry reaching its stash", |events| {
        events.iter().any(reached_the_stash).then_some(())
    })
    .await;
    repo.commit_file("terminal.txt", "typed\n", "landed while the carry ran");
    let injected = repo.git(&["rev-parse", "HEAD"]);
    release.send(()).expect("the carry is released");

    let refusal = write_result(&sink, OperationKind::Rebase)
        .await
        .expect("the replay is refused");
    assert!(
        refusal.contains("tip moved") && refusal.contains("nothing was rewritten"),
        "the refusal says the plan's premise went: {refusal}"
    );
    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase --interactive", "stash push", "stash pop"],
        "the second spawn never happened, and the work came back"
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        injected,
        "the commit that landed inside the window is still the tip"
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "landed while the carry ran",
        "and it is the one the terminal made, not a replay of the plan"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
        "uncommitted\n",
        "with the uncommitted edit back in the tree"
    );
    session.close();
}

/// The tip check in front of the spawn, as the one-commit edits reach it.
fn reached_the_tip_check(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. }
        if display.starts_with("git rev-parse --verify HEAD"))
}

/// What the write of `kind` reported, or `None` — the half of the answer
/// the notice bar is written from.
async fn write_report(
    sink: &CaptureSink,
    kind: OperationKind,
) -> Option<platitude_core::report::WriteReport> {
    sink.wait_for(kind.label(), |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteFinished {
                kind: got, report, ..
            } if *got == kind => Some(report.clone()),
            _ => None,
        })
    })
    .await
}

/// A one-commit edit is pinned to the tip its todo was read against.
///
/// `drop` / `squash` / reword hand git a whole todo built from
/// `upstream..HEAD` ([`platitude_core::sequencer::plan_edit`]), so a commit
/// landing after the rows are read is dropped the same way. The pin costs
/// no process (`EditPlan::tip`).
///
/// The session is parked inside the delivery that announces the tip check
/// while the test commits: the todo is read by then, the branch not yet
/// looked at.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_landing_after_a_one_commit_edits_todo_is_refused() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let doomed = repo.commit_file_id("b.txt", "two\n", "the one to go");
    repo.commit_file("c.txt", "three\n", "after it");

    let (sink, session) = opened(&repo).await;
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(reached_the_tip_check, move || {
        held.recv().expect("the test releases the edit");
    });
    session.drop_commit(doomed.clone());
    sink.wait_for("the edit reaching its tip check", |events| {
        events.iter().any(reached_the_tip_check).then_some(())
    })
    .await;
    repo.commit_file("terminal.txt", "typed\n", "landed after the todo was read");
    let injected = repo.git(&["rev-parse", "HEAD"]);
    release.send(()).expect("the edit is released");

    let refusal = write_result(&sink, OperationKind::Drop)
        .await
        .expect("the replay is refused");
    assert!(
        refusal.contains("tip moved") && refusal.contains("nothing was rewritten"),
        "the refusal says the plan's premise went: {refusal}"
    );
    // The reader is told, since nothing ran that a log row could explain
    // (デザイン規約 §答えの要らない報せ).
    let report = write_report(&sink, OperationKind::Drop)
        .await
        .expect("a report the notice bar is written from");
    assert_eq!(
        report.kind,
        platitude_core::report::ReportKind::RewriteTipMoved
    );
    assert!(
        report.reason.is_empty(),
        "nobody outside wrote a sentence for this one: {report:?}"
    );
    assert!(
        rewrite_route(&sink).is_empty(),
        "the tree was clean, so the refusal comes before any spawn: {:?}",
        rewrite_route(&sink)
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        injected,
        "the commit that landed inside the window is still the tip"
    );
    assert_eq!(
        repo.git(&["log", "--format=%s"])
            .lines()
            .collect::<Vec<_>>(),
        vec![
            "landed after the todo was read",
            "after it",
            "the one to go",
            "root"
        ],
        "and nothing was rewritten — the commit the edit was about is still there"
    );
    session.close();
}

/// The same pin across the carry's two spawns, the wider window. And the
/// work comes back: a refusal on the second attempt puts the carry's stash
/// back before the answer goes out.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_landing_inside_a_one_commit_edits_carry_is_refused() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let doomed = repo.commit_file_id("b.txt", "two\n", "the one to go");
    repo.commit_file("c.txt", "three\n", "after it");
    // Dirty, so the first spawn is refused and the write goes round the stash.
    repo.write_file("a.txt", "staged edit\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("c.txt", "unstaged edit\n");

    let (sink, session) = opened(&repo).await;
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(reached_the_stash, move || {
        held.recv().expect("the test releases the carry");
    });
    session.drop_commit(doomed.clone());
    sink.wait_for("the carry reaching its stash", |events| {
        events.iter().any(reached_the_stash).then_some(())
    })
    .await;
    repo.commit_file("terminal.txt", "typed\n", "landed while the carry ran");
    let injected = repo.git(&["rev-parse", "HEAD"]);
    release.send(()).expect("the carry is released");

    let refusal = write_result(&sink, OperationKind::Drop)
        .await
        .expect("the replay is refused");
    assert!(
        refusal.contains("tip moved") && refusal.contains("nothing was rewritten"),
        "the refusal says the plan's premise went: {refusal}"
    );
    assert_eq!(
        rewrite_route(&sink),
        vec!["rebase --interactive", "stash push", "stash pop"],
        "the second spawn never happened, and the work came back"
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        injected,
        "the commit that landed inside the window is still the tip"
    );
    assert!(
        repo.git(&["log", "--format=%s"]).contains("the one to go"),
        "and nothing was rewritten"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
    // Back in the tree, but unstaged: after an `Err` the restore is
    // `stash_round::pop_back_after_failure`'s plain pop (the split survives
    // only the way round that lands, `pop_back_split_first`).
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
        "staged edit\n",
        "the edit that was staged is back in the tree"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("c.txt")).expect("read"),
        "unstaged edit\n",
        "and so is the one that was not"
    );
    session.close();
}
