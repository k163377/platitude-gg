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

/// The commands a rewrite issued, named coarsely enough to read as the
/// route it took rather than as an argument list.
///
/// Matched from the front of the command, not anywhere inside it: the
/// status refresh that follows a stopped rebase reads its progress with
/// `rev-parse --git-path rebase-merge/msgnum`, which a plain `contains`
/// counts as a fourth rebase (measured).
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

/// A squash fired over a dirty tree neither stops nor asks: git refuses to
/// replay while the work is in the tree, so the session goes round the way
/// a person typing the three commands would (デザイン規約
/// §未コミット変更がある状態で履歴を書き換える). What lands is what `stash` →
/// `squash` → `stash pop --index` leaves — **the staged and unstaged
/// halves still told apart**, which is the one thing `--autostash` cannot
/// do: it restores with a plain apply and everything comes back unstaged
/// (measured, 2.55).
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

/// A whole branch moved onto a new base goes round the very same way, so
/// the answer to "does my staging survive a history rewrite" does not
/// depend on which menu row was clicked. Handing this to `--autostash`
/// instead would restore with a plain apply and bring **everything back
/// unstaged** — the split below is exactly what that flag cannot keep
/// (measured, 2.55).
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

/// And when that rebase stops on a conflict, the work waits in the stash
/// exactly as a stopped replay's does — no restore is attempted over a
/// tree git is still holding. This is what the alignment gives up:
/// `--autostash` would have put the work back itself after `--continue`
/// or `--abort` (measured), whereas this entry is the person's to pop.
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

/// Untracked files are not in the way of a replay at all (measured: git takes
/// the plan and leaves them where they are), so no stash is taken for
/// them. The route is the whole assertion — a needless stash would still
/// have ended with the same working tree.
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

/// The replay goes through and the *restore* is what collides. The
/// landing is the one a move already has (規約 §未コミット変更がある状態
/// での移動): markers in the files, and the stash entry kept so the work
/// still exists somewhere other than a marked-up file. Not a failed
/// write — nothing failed.
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

/// The replay stops part-way, and then the restore is not attempted: git
/// will not write into an index that already holds unmerged paths, so a
/// pop there does nothing while reporting the collision it walked into
/// (measured `could not write index` / `needs merge` — 規約 §`stash pop` の
/// 非ゼロを conflict と読んでよいのは). The work waits in the stash,
/// drawn as its own row in the graph, until the operation is over.
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

/// **A commit that lands inside the carry is refused, not replayed over.**
///
/// The plan a screen composes is a fixed list of ids, and the carry spawns
/// the replay twice: refused over the dirty tree, then again once a
/// `detect` and a stash have emptied it — three spawns of room. git writes
/// its own todo from `upstream..HEAD` and the helper replaces it whole, so
/// a commit typed into that stretch is not in the list, and a rebase drops
/// what the todo leaves out **without saying so**. Before the pin moved to
/// sit in front of every spawn, this ran through: `terminal.txt` gone, the
/// history replayed without it, and the write reported as a success.
///
/// The window is entered on purpose rather than waited for: the session is
/// held inside the sink delivery that announces the stash, and the commit
/// is made from the test's own thread while it is parked there
/// ([`crate::support::session::CaptureSink::hook_once`]).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_landing_inside_the_carry_is_refused_rather_than_dropped() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let head = repo.commit_file_id("b.txt", "two\n", "the plan's only step");
    let base = repo.git(&["rev-parse", "HEAD~1"]);
    // What git refuses the first spawn over, which is what sends the write
    // round through the stash in the first place.
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
    // The sink records before it runs the hook, so this is answered by the
    // very delivery that is holding the write.
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

/// What the write of `kind` reported for itself, or `None` where it
/// carried no report — the half of the answer the notice bar is written
/// out of, which the error string alone cannot say.
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

/// **A one-commit edit is pinned to the tip its todo was read against.**
///
/// `drop` / `squash` / reword build their own todo out of `upstream..HEAD`
/// and hand git the whole list ([`platitude_core::sequencer::plan_edit`]),
/// so a commit that lands after the rows are read is not in it — and a
/// rebase drops what the todo leaves out **without saying so**. The plan
/// composed on screen has been pinned since it was built; these three were
/// not, on the reading that the pin cost a `rev-parse HEAD` on the
/// response path of the three edits people click most. It costs nothing:
/// the rows already end at the tip (`EditPlan::tip`).
///
/// The window is entered on purpose: the session is parked inside the sink
/// delivery that announces the tip check, and the commit is made from the
/// test's own thread while it is held there — the todo is read by then and
/// the branch has not been looked at yet.
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
    // The reader is told, rather than left with a log panel of commands
    // that all succeeded: nothing ran that a row could explain
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

/// The same pin, at the other end of the same write: the carry spawns the
/// replay twice, and the stretch between them — a `detect`, a `stash
/// push`, and the pop that may follow — is the wider of the two windows.
///
/// **And the work comes back.** A refusal on the second attempt is one
/// nothing here started, so the stash the carry made is put back before
/// the answer goes out: a reader who is told the branch moved still has
/// what they had staged and unstaged, on the side it was on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_commit_landing_inside_a_one_commit_edits_carry_is_refused() {
    install_todo_editor();
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let doomed = repo.commit_file_id("b.txt", "two\n", "the one to go");
    repo.commit_file("c.txt", "three\n", "after it");
    // What git refuses the first spawn over, which is what sends the write
    // round through the stash in the first place.
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
    // Both edits are in the tree again. **Which side they are on is not
    // this path's to keep**: a replay that came back an `Err` did nothing,
    // so the restore is `stash_round::pop_back_after_failure`'s plain pop
    // and everything lands unstaged — the split survives the way round
    // that lands (`pop_back_split_first`).
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
