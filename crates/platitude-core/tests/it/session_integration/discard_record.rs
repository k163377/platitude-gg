//! Writes that take something away leave it on the discard record
//! (破棄記録仕様.md §2) once they have, and a restore brings it back (§4) —
//! each through the session's queue as a write of its own, the log told by
//! [`SessionEvent::DiscardsChanged`].

use crate::support::exec::env;
use crate::support::remote::origin_and_clone;
use crate::support::session::{opened, write_answer};
use crate::support::stage::fp;
use crate::support::{TestRepo, info};
use platitude_core::branch::{CheckoutTarget, ResetMode};
use platitude_core::commit::CommitOptions;
use platitude_core::details::DiffTarget;
use platitude_core::discards::{self, Discard, DiscardKind, Restore, Restored};
use platitude_core::integrate::{Continuation, RebaseOptions};
use platitude_core::patch::HunkSelect;
use platitude_core::remote::PushForce;
use platitude_core::session::SessionEvent;
use platitude_core::stage::DiscardSide;
use platitude_core::{Oid, OperationKind};

fn oid(hex: &str) -> Oid {
    Oid::from_hex_str(hex.trim()).expect("an oid")
}

async fn listed(repo: &TestRepo) -> Vec<Discard> {
    let (executor, cancel) = env();
    discards::read_repo(&executor, &repo.path, None, &cancel)
        .await
        .expect("read the discard log")
}

/// Every `DiscardsChanged` the session sent, in order.
fn told(sink: &crate::support::session::CaptureSink) -> Vec<Vec<Restored>> {
    sink.events
        .lock()
        .expect("the events")
        .iter()
        .filter_map(|event| match event {
            SessionEvent::DiscardsChanged { restored } => Some(restored.clone()),
            _ => None,
        })
        .collect()
}

/// Files discarded are copied, on the record once they went, and come back
/// as they were — the entry then gone from the log.
#[tokio::test(flavor = "multi_thread")]
async fn discarded_files_are_on_the_record_and_come_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    repo.write_file("u.txt", "1\n");
    let before = repo.git(&["status", "--porcelain=v1"]);
    let (sink, session) = opened(&repo).await;

    let id = session
        .discard_chosen(vec![
            ("a.txt".to_string(), DiscardSide::Unstaged),
            ("u.txt".to_string(), DiscardSide::Untracked),
        ])
        .expect("the discard is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "");
    assert_eq!(told(&sink), vec![Vec::<Restored>::new()]);

    let found = listed(&repo).await;
    let entry = found
        .iter()
        .find(|entry| entry.kind == DiscardKind::Discarded)
        .expect("the discard is listed");
    let id = session
        .restore_discard(entry.parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(&sink, id).await, None);

    assert_eq!(repo.git(&["status", "--porcelain=v1"]), before);
    assert_eq!(
        told(&sink),
        vec![Vec::new(), vec![Restored::Whole]],
        "the record, then the restore"
    );
    assert!(listed(&repo).await.is_empty(), "the restore is noted");
    session.close();
}

/// A discard git refuses leaves nothing on the record: the copy made first
/// never goes on it.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_discard_leaves_no_record() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    let (sink, session) = opened(&repo).await;

    let id = session
        .discard_chosen(vec![
            ("a.txt".to_string(), DiscardSide::Unstaged),
            ("missing.txt".to_string(), DiscardSide::Unstaged),
        ])
        .expect("the discard is accepted");
    assert!(
        write_answer(&sink, id).await.is_some(),
        "git refuses the missing path"
    );

    assert!(told(&sink).is_empty());
    assert!(listed(&repo).await.is_empty());
    session.close();
}

/// Twenty lines, the third `A` and the seventeenth `B` where asked.
fn hunks(a: bool, b: bool) -> String {
    (1..=20)
        .map(|n| match n {
            3 if a => "A\n".to_string(),
            17 if b => "B\n".to_string(),
            n => format!("line {n}\n"),
        })
        .collect()
}

/// Throws away hunk `at` of `f.txt`'s unstaged diff as the diff pane does.
async fn discard_hunk(
    repo: &TestRepo,
    sink: &crate::support::session::CaptureSink,
    session: &std::sync::Arc<platitude_core::session::RepoSession>,
    at: usize,
) {
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    let seen = fp(&info(repo).await, &target).await;
    let id = session
        .discard_partial(target, vec![HunkSelect::whole(at)], seen)
        .expect("the discard is accepted");
    assert_eq!(write_answer(sink, id).await, None);
}

/// Brings back the newest thrown-away work.
async fn restore_newest_work(
    repo: &TestRepo,
    sink: &crate::support::session::CaptureSink,
    session: &std::sync::Arc<platitude_core::session::RepoSession>,
    nth: usize,
) {
    let found = listed(repo).await;
    let entry = found
        .iter()
        .filter(|entry| entry.kind == DiscardKind::Discarded)
        .nth(nth)
        .expect("the discard is listed");
    let id = session
        .restore_discard(entry.parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(sink, id).await, None);
}

/// A hunk thrown out of a file whose other hunk stayed comes back beside it,
/// in the working tree.
#[tokio::test(flavor = "multi_thread")]
async fn a_hunk_thrown_away_comes_back_beside_the_one_left() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", &hunks(false, false), "c1");
    repo.write_file("f.txt", &hunks(true, true));
    let (sink, session) = opened(&repo).await;

    discard_hunk(&repo, &sink, &session, 0).await;
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt"),
        hunks(false, true)
    );
    restore_newest_work(&repo, &sink, &session, 0).await;

    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt"),
        hunks(true, true)
    );
    assert_eq!(told(&sink).last(), Some(&vec![Restored::Whole]));
    session.close();
}

/// A hunk thrown away comes back alone after the file's other hunk was
/// thrown away too: what was not asked for stays gone.
#[tokio::test(flavor = "multi_thread")]
async fn a_hunk_thrown_away_comes_back_without_the_one_thrown_away_after() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", &hunks(false, false), "c1");
    repo.write_file("f.txt", &hunks(true, true));
    let (sink, session) = opened(&repo).await;

    discard_hunk(&repo, &sink, &session, 0).await;
    discard_hunk(&repo, &sink, &session, 0).await;
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt"),
        hunks(false, false)
    );
    // Newest first: B's entry, then A's.
    restore_newest_work(&repo, &sink, &session, 1).await;

    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt"),
        hunks(true, false)
    );
    session.close();
}

/// A hunk discard refused for a diff that moved on leaves nothing on the
/// record: the file is as the copy found it.
#[tokio::test(flavor = "multi_thread")]
async fn a_hunk_discard_refused_leaves_no_record() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", &hunks(false, false), "c1");
    repo.write_file("f.txt", &hunks(true, true));
    let (sink, session) = opened(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    let seen = fp(&info(&repo).await, &target).await;
    repo.write_file("f.txt", &hunks(true, false));

    let id = session
        .discard_partial(target, vec![HunkSelect::whole(0)], seen)
        .expect("the discard is accepted");
    assert!(
        write_answer(&sink, id).await.is_some(),
        "the diff moved on under the selection"
    );

    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt"),
        hunks(true, false)
    );
    assert!(told(&sink).is_empty());
    assert!(listed(&repo).await.is_empty());
    session.close();
}

/// A staged rename thrown away takes its old name along, read off the same
/// status the discard reads, and both come back as they were staged.
#[tokio::test(flavor = "multi_thread")]
async fn a_staged_rename_comes_back_with_both_its_names() {
    let mut repo = TestRepo::init();
    repo.commit_file("old.txt", "kept\n", "c1");
    repo.git(&["mv", "old.txt", "new.txt"]);
    let before = repo.git(&["status", "--porcelain=v1"]);
    let (sink, session) = opened(&repo).await;

    let id = session
        .discard_chosen(vec![("new.txt".to_string(), DiscardSide::Staged)])
        .expect("the discard is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "", "both names");
    restore_newest_work(&repo, &sink, &session, 0).await;

    assert_eq!(repo.git(&["status", "--porcelain=v1"]), before);
    assert_eq!(told(&sink).last(), Some(&vec![Restored::Whole]));
    session.close();
}

/// Work thrown away in a linked worktree and brought back from the
/// main one waits its turn there: a commit accepted in that worktree before
/// the restore takes in only what was staged for it, and the work comes
/// back staged behind it (破棄記録仕様.md §4). The commit is held by a hook
/// across the restore's acceptance; the witness is the moment the restore
/// starts, read off the commit's own answers then — the commit's tree alone
/// would let a restore that did not wait pass, since the hook lets go
/// before such a restore gets to the index.
#[tokio::test(flavor = "multi_thread")]
async fn work_brought_back_into_another_worktree_waits_for_its_commit() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "c1");
    repo.commit_file("g.txt", "1\n", "c2");
    let linked = repo.path.with_file_name("linked");
    let at = linked.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "side", &at]);
    std::fs::write(linked.join("f.txt"), "staged\n").expect("change f");
    repo.git(&["-C", &at, "add", "f.txt"]);
    let (side_sink, side) = crate::support::session::opened_at(&linked).await;
    let id = side
        .discard_chosen(vec![("f.txt".to_string(), DiscardSide::Staged)])
        .expect("the discard is accepted");
    assert_eq!(write_answer(&side_sink, id).await, None);

    std::fs::write(linked.join("g.txt"), "2\n").expect("change g");
    repo.git(&["-C", &at, "add", "g.txt"]);
    let release = repo.path.with_file_name("hook-release");
    repo.write_hook("pre-commit", &crate::support::barrier_hook(&release));
    let commit = side
        .commit("g alone".into(), CommitOptions::default())
        .expect("the commit is accepted");
    side_sink
        .wait_for("the commit reached git", |evs| {
            evs.iter()
                .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == commit))
                .then_some(())
        })
        .await;

    let (sink, session) = opened(&repo).await;
    let found = listed(&repo).await;
    let entry = found
        .iter()
        .find(|entry| entry.kind == DiscardKind::Discarded)
        .expect("the linked worktree's discard is listed here");
    let started_after = std::sync::Arc::new(std::sync::Mutex::new(None::<bool>));
    {
        let (seen, commit_sink) = (started_after.clone(), side_sink.clone());
        sink.hook_once(
            |e| {
                matches!(
                    e,
                    SessionEvent::WriteStarted {
                        kind: OperationKind::Restore,
                        ..
                    }
                )
            },
            move || {
                let answered = commit_sink.count(
                    |e| matches!(e, SessionEvent::WriteFinished { id, .. } if *id == commit),
                ) == 1;
                *seen.lock().expect("the witness") = Some(answered);
            },
        );
    }
    let restore = session
        .restore_discard(entry.parts.clone())
        .expect("the restore is accepted");
    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(write_answer(&side_sink, commit).await, None);
    assert_eq!(write_answer(&sink, restore).await, None);
    assert_eq!(
        *started_after.lock().expect("the witness"),
        Some(true),
        "the restore started only once the commit had been answered"
    );
    assert_eq!(
        repo.git(&["-C", &at, "diff", "--name-only", "HEAD~1", "HEAD"]),
        "g.txt",
        "the commit took in g alone"
    );
    assert_eq!(
        repo.git(&["-C", &at, "diff", "--cached", "--name-only"]),
        "f.txt",
        "f came back staged, behind it"
    );
    session.close();
    side.close();
}

/// A discard that fails after it threw some of the work away still puts
/// its copy on the record — the copy is all that is left of what went.
/// Here the unstaged row goes before git refuses the staged one.
#[tokio::test(flavor = "multi_thread")]
async fn a_discard_failing_half_way_is_on_the_record() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    let (sink, session) = opened(&repo).await;

    let id = session
        .discard_chosen(vec![
            ("a.txt".to_string(), DiscardSide::Unstaged),
            ("missing.txt".to_string(), DiscardSide::Staged),
        ])
        .expect("the discard is accepted");
    assert!(
        write_answer(&sink, id).await.is_some(),
        "git refuses the missing path"
    );
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "", "the edit went");

    let found = listed(&repo).await;
    let entry = found
        .iter()
        .find(|entry| entry.kind == DiscardKind::Discarded)
        .expect("the discard is listed");
    let id = session
        .restore_discard(entry.parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "M a.txt");
    session.close();
}

/// `reset --hard` deletes the untracked files in the way of what it writes
/// without a word — a file where the commit has one, a file where it has a
/// folder, a folder where it has a file: they are copied with the rest. A
/// file the index stopped tracking is the copy's tracked side's, not twice.
#[tokio::test(flavor = "multi_thread")]
async fn what_a_hard_reset_writes_over_is_copied() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "theirs\n", "c1");
    repo.commit_file("g.txt", "theirs\n", "c2");
    repo.commit_file("dx/x", "theirs\n", "c3");
    repo.commit_file("df", "theirs\n", "c4");
    let target = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["rm", "--quiet", "f.txt", "dx/x", "df"]);
    repo.git(&["commit", "--quiet", "-m", "c5"]);
    repo.git(&["rm", "--quiet", "--cached", "g.txt"]);
    for path in ["f.txt", "g.txt", "dx", "df/u"] {
        repo.write_file(path, "mine\n");
    }
    repo.write_file("other.txt", "untouched\n");
    let (sink, session) = opened(&repo).await;

    let id = session
        .reset(target, ResetMode::Hard)
        .expect("the reset is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "?? other.txt");

    let found = listed(&repo).await;
    let reset = found
        .iter()
        .find(|entry| entry.kind == DiscardKind::Reset)
        .expect("the reset is listed");
    let copy = reset
        .parts
        .iter()
        .find_map(|part| match &part.restore {
            Restore::Changes { copy, .. } => Some(copy.to_hex()),
            _ => None,
        })
        .expect("the work is a part of it");
    for path in ["f.txt", "dx", "df/u"] {
        assert_eq!(
            repo.git(&["show", &format!("{copy}^3:{path}")]),
            "mine",
            "{path}"
        );
    }
    assert_eq!(repo.git(&["show", &format!("{copy}:g.txt")]), "mine");
    assert!(!repo.git_ok(&["cat-file", "-e", &format!("{copy}^3:g.txt")]));
    session.close();
}

/// A repository of its own inside the working tree stays where it is
/// through a discard (`clean` leaves it), so nothing of it goes on the
/// record.
#[tokio::test(flavor = "multi_thread")]
async fn a_repository_inside_stays_off_the_record() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["init", "--quiet", "nested"]);
    repo.write_file("nested/n.txt", "n\n");
    let (sink, session) = opened(&repo).await;

    let id = session
        .discard_chosen(vec![("nested/".to_string(), DiscardSide::Untracked)])
        .expect("the discard is accepted");
    assert_eq!(write_answer(&sink, id).await, None);

    assert!(repo.path.join("nested/n.txt").exists());
    assert!(listed(&repo).await.is_empty());
    session.close();
}

/// A worktree on no commit yet takes nothing with it: its removal is no
/// entry, and no failure of the record either.
#[tokio::test(flavor = "multi_thread")]
async fn a_worktree_on_no_commit_goes_off_the_record() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let side = repo.path.with_file_name("side");
    repo.git(&[
        "worktree",
        "add",
        "--orphan",
        "-b",
        "fresh",
        &side.to_string_lossy(),
    ]);
    // As the screen has it: the path git lists, which is the one the
    // removal finds the worktree's HEAD by.
    let worktrees = repo.git(&["worktree", "list", "--porcelain"]);
    let side_path = worktrees
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|path| path.ends_with("side"))
        .expect("the worktree is listed")
        .to_string();
    let (sink, session) = opened(&repo).await;

    let id = session
        .remove_worktree(side_path, "side".to_string())
        .expect("the removal is accepted");
    assert_eq!(write_answer(&sink, id).await, None);

    assert!(!side.exists());
    assert!(listed(&repo).await.is_empty());
    let failed = sink
        .events
        .lock()
        .expect("the events")
        .iter()
        .any(|event| matches!(event, SessionEvent::OpFailed { .. }));
    assert!(!failed);
    session.close();
}

/// A write that moves off commits only git's reflogs reach then tells the
/// log — an amend, a reset back, a move off a detached HEAD's own commit —
/// and one that leaves them where a branch holds them says nothing.
#[tokio::test(flavor = "multi_thread")]
async fn writes_that_leave_commits_behind_tell_the_log() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file("a.txt", "2\n", "c2");
    repo.git(&["branch", "kept"]);
    let (sink, session) = opened(&repo).await;
    let amend = || CommitOptions {
        amend: true,
        ..CommitOptions::default()
    };

    let id = session
        .commit("held".to_string(), amend())
        .expect("the amend is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(told(&sink).len(), 0, "`kept` holds what the amend replaced");
    let id = session
        .commit("left".to_string(), amend())
        .expect("the amend is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(told(&sink).len(), 1, "only main had what this one replaced");

    let id = session
        .reset("kept".to_string(), ResetMode::Soft)
        .expect("the reset is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(told(&sink).len(), 2, "the reset left the amended commit");

    repo.git(&["switch", "--quiet", "--detach", "kept"]);
    let id = session
        .checkout(CheckoutTarget::Branch {
            name: "main".to_string(),
        })
        .expect("the move is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(told(&sink).len(), 2, "a branch holds the detached commit");
    repo.git(&["switch", "--quiet", "--detach", "kept"]);
    repo.commit_file("d.txt", "1\n", "d1");
    let id = session
        .checkout(CheckoutTarget::Branch {
            name: "main".to_string(),
        })
        .expect("the move is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(told(&sink).len(), 3, "only the detached HEAD had d1");
    session.close();
}

/// `top` on `base` on `main`, and `main` one commit on — touching `b.txt`
/// as `base` does where `conflict`, so a rebase of `top` onto it stops at
/// `base`'s commit.
fn stacked(repo: &mut TestRepo, conflict: bool) {
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "--quiet", "-c", "base"]);
    repo.commit_file("b.txt", "base\n", "b1");
    repo.git(&["switch", "--quiet", "-c", "top"]);
    repo.commit_file("t.txt", "top\n", "t1");
    repo.git(&["switch", "--quiet", "main"]);
    let file = if conflict { "b.txt" } else { "m.txt" };
    repo.commit_file(file, "main\n", "m1");
    repo.git(&["switch", "--quiet", "top"]);
}

/// The one entry a rebase that moved `top` and `base` together left.
async fn one_rebase_of_both(repo: &TestRepo) {
    let found = listed(repo).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    let names: Vec<&Restore> = found[0].parts.iter().map(|part| &part.restore).collect();
    assert!(
        matches!(
            names[..],
            [Restore::Branch { name: first, .. }, Restore::Branch { name: second, .. }]
                if first == "base-pgg-restored" && second == "top-pgg-restored"
        ),
        "{found:#?}"
    );
}

/// The branches a rebase moves together are one entry also where the
/// user's own setting moved them (`rebase.updateRefs`), not the screen.
#[tokio::test(flavor = "multi_thread")]
async fn branches_the_setting_moves_together_are_one_entry() {
    let mut repo = TestRepo::init();
    stacked(&mut repo, false);
    repo.git(&["config", "rebase.updateRefs", "true"]);
    let (sink, session) = opened(&repo).await;

    let id = session
        .rebase("main".to_string(), RebaseOptions::default())
        .expect("the rebase is accepted");
    assert_eq!(write_answer(&sink, id).await, None);

    one_rebase_of_both(&repo).await;
    session.close();
}

/// A rebase moving branches together that stops on a conflict moves them
/// when it finishes: the continue that finishes it puts them on the record
/// as one entry.
#[tokio::test(flavor = "multi_thread")]
async fn branches_moved_together_after_a_stop_are_one_entry() {
    let mut repo = TestRepo::init();
    stacked(&mut repo, true);
    let (sink, session) = opened(&repo).await;

    let options = RebaseOptions {
        update_refs: true,
        ..RebaseOptions::default()
    };
    let id = session
        .rebase("main".to_string(), options)
        .expect("the rebase is accepted");
    assert_eq!(write_answer(&sink, id).await, None, "a stop is no failure");
    repo.write_file("b.txt", "both\n");
    repo.git(&["add", "b.txt"]);
    let id = session
        .resolve_current(Continuation::Continue)
        .expect("the continue is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert!(!repo.git_ok(&["rev-parse", "--verify", "--quiet", "REBASE_HEAD"]));

    one_rebase_of_both(&repo).await;
    session.close();
}

/// Two branches on one tip that one rebase moved together off it onto one
/// new tip are both the entry's: each comes back under its own name.
#[tokio::test(flavor = "multi_thread")]
async fn two_branches_on_one_tip_one_rebase_moved_both_come_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "--quiet", "-c", "topic"]);
    let tip = repo.commit_file_id("t.txt", "top\n", "t1");
    repo.git(&["branch", "alias"]);
    repo.git(&["switch", "--quiet", "main"]);
    repo.commit_file("m.txt", "main\n", "m1");
    repo.git(&["switch", "--quiet", "topic"]);
    let (sink, session) = opened(&repo).await;

    let options = RebaseOptions {
        update_refs: true,
        ..RebaseOptions::default()
    };
    let id = session
        .rebase("main".to_string(), options)
        .expect("the rebase is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(
        repo.git(&["rev-parse", "alias"]),
        repo.git(&["rev-parse", "topic"]),
        "the rebase moved both"
    );

    let found = listed(&repo).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    let restores: Vec<&Restore> = found[0].parts.iter().map(|part| &part.restore).collect();
    assert!(
        matches!(
            restores[..],
            [Restore::Branch { name: first, tip: a, .. }, Restore::Branch { name: second, tip: b, .. }]
                if first == "alias-pgg-restored" && second == "topic-pgg-restored"
                    && *a == oid(&tip) && *b == oid(&tip)
        ),
        "{found:#?}"
    );
    let id = session
        .restore_discard(found[0].parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(repo.git(&["rev-parse", "alias-pgg-restored"]), tip);
    assert_eq!(repo.git(&["rev-parse", "topic-pgg-restored"]), tip);
    session.close();
}

/// A branch deleted is on the record with its upstream, and comes back
/// under its own name with it.
#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_branch_is_on_the_record_and_comes_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "-c", "spike"]);
    let tip = repo.commit_file_id("s.txt", "1\n", "s1");
    repo.git(&["config", "branch.spike.remote", "origin"]);
    repo.git(&["config", "branch.spike.merge", "refs/heads/spike"]);
    repo.git(&["switch", "main"]);
    let (sink, session) = opened(&repo).await;

    let id = session
        .delete_branch("spike".to_string(), true)
        .expect("the delete is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    let found = listed(&repo).await;
    let entry = found
        .iter()
        .find(|entry| entry.kind == DiscardKind::Deleted)
        .expect("the delete is listed");

    let id = session
        .restore_discard(entry.parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(repo.git(&["rev-parse", "spike"]), tip.trim());
    assert_eq!(
        repo.git(&["config", "branch.spike.merge"]),
        "refs/heads/spike"
    );
    assert!(listed(&repo).await.is_empty());
    session.close();
}

/// A stash dropped is on the record, and comes back to the list under its
/// message.
#[tokio::test(flavor = "multi_thread")]
async fn a_dropped_stash_is_on_the_record_and_comes_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    repo.git(&["stash", "push", "-m", "keep"]);
    let stash = repo.git(&["rev-parse", "stash@{0}"]);
    let (sink, session) = opened(&repo).await;

    let id = session
        .stash_drop("stash@{0}".to_string())
        .expect("the drop is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    let found = listed(&repo).await;
    let entry = found
        .iter()
        .find(|entry| entry.kind == DiscardKind::DroppedStash)
        .expect("the drop is listed");

    let id = session
        .restore_discard(entry.parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(repo.git(&["rev-parse", "stash@{0}"]), stash);
    assert_eq!(
        repo.git(&["stash", "list", "--format=%gs"]),
        "On main: keep"
    );
    session.close();
}

/// What went from a remote: a branch deleted there, one pushed over, and a
/// tag deleted there — each on the record with what the remote held, and
/// each coming back here, under a name clear of the ones here.
#[tokio::test(flavor = "multi_thread")]
async fn what_went_from_a_remote_is_on_the_record_and_comes_back_here() {
    let (_bare, mut work) = origin_and_clone();
    work.git(&["switch", "-c", "gone", "main"]);
    let gone = work.commit_file_id("gone.txt", "1\n", "gone");
    work.git(&["push", "-u", "origin", "gone"]);
    work.git(&["tag", "--annotate", "-m", "one", "v1"]);
    work.git(&["push", "origin", "v1"]);
    let object = work.git(&["rev-parse", "refs/tags/v1"]);
    work.git(&["switch", "-c", "over", "main"]);
    let first = work.commit_file_id("over.txt", "1\n", "first");
    work.git(&["push", "-u", "origin", "over"]);
    work.git(&["reset", "--hard", "HEAD~1"]);
    work.commit_file("over.txt", "2\n", "second");
    let (sink, session) = opened(&work).await;

    let id = session
        .delete_remote_branch("origin".into(), "gone".into(), gone.trim().into())
        .expect("the delete is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    let id = session
        .push_current("origin".into(), PushForce::WithLease { expect: None })
        .expect("the push is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    let id = session
        .delete_remote_tag("origin".into(), "v1".into(), object.clone())
        .expect("the tag's delete is accepted");
    assert_eq!(write_answer(&sink, id).await, None);

    // The reset under the push is git's own line, dated by the fixture's
    // clock long before the session's records: it comes last.
    let all = listed(&work).await;
    let found = &all[..3];
    let kinds: Vec<DiscardKind> = found.iter().map(|entry| entry.kind).collect();
    assert_eq!(
        kinds,
        vec![
            DiscardKind::DeletedRemoteTag,
            DiscardKind::ForcePushed,
            DiscardKind::DeletedRemoteBranch,
        ],
        "newest first: {found:#?}"
    );
    let restores: Vec<&Restore> = found.iter().map(|entry| &entry.parts[0].restore).collect();
    assert_eq!(
        restores,
        vec![
            &Restore::Tag {
                name: "v1-pgg-restored".to_string(),
                object: oid(&object),
            },
            &Restore::Branch {
                name: "over-pgg-restored".to_string(),
                tip: oid(&first),
                upstream: None,
            },
            &Restore::Branch {
                name: "gone-pgg-restored".to_string(),
                tip: oid(&gone),
                upstream: None,
            },
        ]
    );

    let id = session
        .restore_discard(found[1].parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(work.git(&["rev-parse", "over-pgg-restored"]), first.trim());
    session.close();
}

/// A remote's annotated tag goes on the record as the object the remote
/// held, though the lease names the commit the row showed it on: it comes
/// back annotated. Deleted here and there at once, the one tag is one part.
#[tokio::test(flavor = "multi_thread")]
async fn a_remote_annotated_tag_is_recorded_as_the_remote_held_it() {
    let (_bare, mut work) = origin_and_clone();
    work.git(&["tag", "--annotate", "-m", "one", "v1"]);
    work.git(&["tag", "--annotate", "-m", "two", "v2"]);
    work.git(&["push", "origin", "v1", "v2"]);
    let v1 = work.git(&["rev-parse", "refs/tags/v1"]);
    let v2 = work.git(&["rev-parse", "refs/tags/v2"]);
    let shown = work.git(&["rev-parse", "HEAD"]);
    let (sink, session) = opened(&work).await;

    let id = session
        .delete_remote_tag("origin".into(), "v1".into(), shown.clone())
        .expect("the delete is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    let id = session
        .delete_tag_everywhere("v2".into(), "origin".into(), shown.clone())
        .expect("the delete is accepted");
    assert_eq!(write_answer(&sink, id).await, None);

    let found = listed(&work).await;
    let restores: Vec<Vec<&Restore>> = found
        .iter()
        .map(|entry| entry.parts.iter().map(|part| &part.restore).collect())
        .collect();
    assert_eq!(
        restores,
        vec![
            vec![&Restore::Tag {
                name: "v2".to_string(),
                object: oid(&v2),
            }],
            vec![&Restore::Tag {
                name: "v1-pgg-restored".to_string(),
                object: oid(&v1),
            }],
        ],
        "{found:#?}"
    );

    let id = session
        .restore_discard(found[1].parts.clone())
        .expect("the restore is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(work.git(&["rev-parse", "refs/tags/v1-pgg-restored"]), v1);
    session.close();
}

/// A leased push over a remote's annotated tag puts the object it replaced
/// on the record — read by the lease, not the commit the row showed.
#[tokio::test(flavor = "multi_thread")]
async fn a_tag_pushed_over_is_recorded_as_the_remote_held_it() {
    let (_bare, mut work) = origin_and_clone();
    work.git(&["tag", "--annotate", "-m", "one", "v1"]);
    work.git(&["push", "origin", "v1"]);
    let held = work.git(&["rev-parse", "refs/tags/v1"]);
    let shown = work.git(&["rev-parse", "HEAD"]);
    work.commit_file("b.txt", "1\n", "next");
    work.git(&["tag", "--force", "--annotate", "-m", "moved", "v1"]);
    let (sink, session) = opened(&work).await;

    let id = session
        .push_tag("origin".into(), "v1".into(), shown)
        .expect("the push is accepted");
    assert_eq!(write_answer(&sink, id).await, None);

    let found = listed(&work).await;
    let pushed = found
        .iter()
        .find(|entry| entry.kind == DiscardKind::ForcePushedTag)
        .expect("the push over is listed");
    assert_eq!(
        pushed.parts[0].restore,
        Restore::Tag {
            name: "v1-pgg-restored".to_string(),
            object: oid(&held),
        }
    );
    session.close();
}
