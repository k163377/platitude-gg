//! Another worktree's uncommitted work applied here, landing as `git stash
//! apply` lands (デザイン規約 §別の worktree を読む): that worktree only read,
//! a refusal a report for the notice bar.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::support::exec::env;
use crate::support::session::{CaptureSink, opened, write_answer};
use crate::support::{TestRepo, barrier_hook};
use platitude_core::OperationId;
use platitude_core::commit::CommitOptions;
use platitude_core::discards;
use platitude_core::process::CommandEnd;
use platitude_core::report::{ReportKind, WriteReport};
use platitude_core::session::{RepoSession, SessionEvent};

/// `main` with one five-line file, and a linked worktree `side` on a branch
/// of its own at the same commit.
fn two_trees() -> (TestRepo, PathBuf) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n2\n3\n4\n5\n", "c1");
    let side = repo.path.with_file_name("side");
    repo.git(&["worktree", "add", "-b", "side", &side.to_string_lossy()]);
    (repo, side)
}

fn write_in(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs");
    }
    std::fs::write(path, content).expect("write file");
}

fn read_in(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).expect("read file")
}

fn status_in(repo: &mut TestRepo, dir: &Path) -> String {
    repo.git_in(dir, &["status", "--porcelain=v1", "-uall"])
}

/// What the write under `id` answered: git's error, and the report the
/// notice bar is written from.
async fn answered(
    sink: &crate::support::session::CaptureSink,
    id: OperationId,
) -> (Option<String>, Option<WriteReport>) {
    sink.wait_for("the apply's answer", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteFinished {
                id: got,
                error,
                report,
                ..
            } if *got == id => Some((error.clone(), report.clone())),
            _ => None,
        })
    })
    .await
}

/// Applies `side`'s work to the main worktree through a session there.
async fn apply_side(repo: &TestRepo, side: &Path) -> (Option<String>, Option<WriteReport>) {
    let (sink, session) = opened(repo).await;
    let id = session
        .apply_carried(side.to_string_lossy().into_owned())
        .expect("the apply is accepted");
    let answer = answered(&sink, id).await;
    session.close();
    answer
}

/// [`apply_side`], with how the `stash apply` row in the log ended and what
/// stands under it.
async fn apply_side_logged(
    repo: &TestRepo,
    side: &Path,
) -> ((Option<String>, Option<WriteReport>), (CommandEnd, String)) {
    let (sink, session) = opened(repo).await;
    let id = session
        .apply_carried(side.to_string_lossy().into_owned())
        .expect("the apply is accepted");
    let answer = answered(&sink, id).await;
    session.close();
    let events = sink.events.lock().unwrap();
    let row = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::CommandStarted { id, display, .. } if display.contains("stash apply") => {
                Some(*id)
            }
            _ => None,
        })
        .expect("the apply ran");
    let ended = events
        .iter()
        .find_map(|e| match e {
            SessionEvent::CommandFinished {
                id, end, message, ..
            } if *id == row => Some((*end, message.clone())),
            _ => None,
        })
        .expect("the apply ended");
    (answer, ended)
}

/// Every side a stash keeps — a staged edit, a staged new file, an
/// unstaged edit over a staged one, a delete, a rename, an untracked file
/// in a folder of its own — read into the same commit `git stash push
/// --include-untracked` makes of them, the worktree, its index and the
/// stash list untouched.
#[tokio::test]
async fn the_work_is_read_as_gits_own_stash_keeps_it() {
    let (mut repo, side) = two_trees();
    repo.git_in(&side, &["switch", "-c", "busy"]);
    write_in(&side, "gone.txt", "g\n");
    write_in(&side, "old.txt", "o\n");
    repo.git_in(&side, &["add", "gone.txt", "old.txt"]);
    repo.git_in(&side, &["commit", "-m", "more"]);
    write_in(&side, "a.txt", "1 staged\n2\n3\n4\n5\n");
    write_in(&side, "n.txt", "new\n");
    repo.git_in(&side, &["add", "a.txt", "n.txt"]);
    write_in(&side, "a.txt", "1 staged\n2\n3\n4\n5 unstaged\n");
    repo.git_in(&side, &["rm", "-q", "gone.txt"]);
    repo.git_in(&side, &["mv", "old.txt", "renamed.txt"]);
    write_in(&side, "loose/u.txt", "untracked\n");
    let before = status_in(&mut repo, &side);

    let (executor, cancel) = env();
    let git_dir = repo.git_in(&side, &["rev-parse", "--absolute-git-dir"]);
    let tracked = ["a.txt", "gone.txt", "n.txt", "old.txt", "renamed.txt"].map(String::from);
    let untracked = ["loose/u.txt".to_string()];
    let ours = discards::stash_of_work(
        &executor,
        &side,
        Path::new(&git_dir),
        &tracked,
        &untracked,
        &cancel,
    )
    .await
    .expect("the work is read")
    .expect("HEAD has a commit");
    assert!(ours.not_copied.is_empty());
    assert_eq!(
        status_in(&mut repo, &side),
        before,
        "the worktree as it was"
    );
    assert_eq!(repo.git_in(&side, &["stash", "list"]), "", "no entry made");

    assert_same_as_gits_stash(&mut repo, &side, &ours);
}

/// `ours` against the stash git itself makes of `side`'s work now: every
/// side of it, the untracked files being the tree git keeps in a third
/// parent.
fn assert_same_as_gits_stash(repo: &mut TestRepo, side: &Path, ours: &discards::WorkStash) {
    repo.git_in(side, &["stash", "push", "-u", "-m", "git's own"]);
    let commit = ours.commit.to_hex();
    for (part, what) in [
        ("^{tree}", "the working tree's side"),
        ("^1", "the commit it stands on"),
        ("^2^{tree}", "the index's side"),
        ("^2^1", "what the index's side stands on"),
    ] {
        assert_eq!(
            repo.git_in(side, &["rev-parse", &format!("{commit}{part}")]),
            repo.git_in(side, &["rev-parse", &format!("stash@{{0}}{part}")]),
            "{what}"
        );
    }
    assert_eq!(
        ours.untracked.map(|tree| tree.to_hex()).unwrap_or_default(),
        repo.git_in(side, &["rev-parse", "stash@{0}^3^{tree}"]),
        "the untracked files"
    );
}

/// A tracked path that is a folder on disk — a file a folder replaced —
/// keeps what the index holds, as git's own stash leaves it, and the files
/// in the folder go with the untracked ones.
#[tokio::test]
async fn a_tracked_path_that_is_a_folder_now_is_read_as_git_reads_it() {
    let (mut repo, side) = two_trees();
    std::fs::remove_file(side.join("a.txt")).expect("take the file away");
    write_in(&side, "a.txt/inside.txt", "a folder now\n");

    let (executor, cancel) = env();
    let git_dir = repo.git_in(&side, &["rev-parse", "--absolute-git-dir"]);
    let ours = discards::stash_of_work(
        &executor,
        &side,
        Path::new(&git_dir),
        &["a.txt".to_string()],
        &["a.txt/inside.txt".to_string()],
        &cancel,
    )
    .await
    .expect("the work is read")
    .expect("HEAD has a commit");

    assert_same_as_gits_stash(&mut repo, &side, &ours);
}

/// Over a HEAD that moved on, as an apply of a stash from there would land:
/// both sides' changes in the file, the new file still added, the
/// untracked one untracked — and the worktree it came from as it was.
#[tokio::test(flavor = "multi_thread")]
async fn the_work_lands_over_a_head_that_moved_on() {
    let (mut repo, side) = two_trees();
    repo.commit_file("a.txt", "1 here\n2\n3\n4\n5\n", "c2");
    write_in(&side, "a.txt", "1\n2\n3\n4\n5 side\n");
    write_in(&side, "n.txt", "new\n");
    repo.git_in(&side, &["add", "n.txt"]);
    write_in(&side, "u.txt", "untracked\n");
    let there = status_in(&mut repo, &side);

    assert_eq!(apply_side(&repo, &side).await, (None, None));

    let here = repo.path.clone();
    // The first line's leading blank goes with `git_in`'s trim.
    assert_eq!(status_in(&mut repo, &here), "M a.txt\nA  n.txt\n?? u.txt");
    assert_eq!(read_in(&repo.path, "a.txt"), "1 here\n2\n3\n4\n5 side\n");
    assert_eq!(status_in(&mut repo, &side), there, "only read there");
    assert_eq!(repo.git(&["stash", "list"]), "", "no entry made");
}

/// Both changed the same line: the conflict is where the apply lands, as
/// an apply's is — not a refusal — and the untracked files come in with it.
#[tokio::test(flavor = "multi_thread")]
async fn a_conflict_is_a_landing() {
    let (mut repo, side) = two_trees();
    repo.commit_file("a.txt", "1\n2\n3 here\n4\n5\n", "c2");
    write_in(&side, "a.txt", "1\n2\n3 side\n4\n5\n");
    write_in(&side, "u.txt", "untracked\n");

    assert_eq!(apply_side(&repo, &side).await, (None, None));

    let here = repo.path.clone();
    assert_eq!(status_in(&mut repo, &here), "UU a.txt\n?? u.txt");
}

/// git turns the apply down for something other than work it would
/// overwrite — here a conflict standing in this worktree — and the report
/// says only that git did, its words being the log's. Nothing is written.
#[tokio::test(flavor = "multi_thread")]
async fn a_refusal_over_nothing_it_would_overwrite_is_reported_as_gits() {
    let (mut repo, side) = two_trees();
    write_in(&side, "b.txt", "side\n");
    repo.git_in(&side, &["add", "b.txt"]);
    write_in(&side, "u.txt", "untracked\n");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("a.txt", "1\n2\n3 other\n4\n5\n", "o");
    repo.git(&["switch", "main"]);
    repo.commit_file("a.txt", "1\n2\n3 main\n4\n5\n", "m");
    repo.git_expect_failure(&["merge", "other"]);

    let ((error, report), (end, said)) = apply_side_logged(&repo, &side).await;

    assert!(error.is_some());
    assert_eq!(report.map(|r| r.kind), Some(ReportKind::CarriedRefused));
    // 2.55 says it could not write the index (its refresh over the unmerged
    // entries fails first); the minimum says nothing at that step.
    if !said.is_empty() {
        assert_eq!(end, CommandEnd::Exited(1), "a refusal is a failed row");
        assert!(
            said.contains("could not write index"),
            "git's words are under it, where the report sends the reader: {said}"
        );
    }
    let here = repo.path.clone();
    assert_eq!(status_in(&mut repo, &here), "UU a.txt");
}

/// Another git holding the index's lock: git turns the apply down, and the
/// report names the lock only where git did — 2.55 says so, the minimum
/// says nothing, which is the plain refusal (the lock file on disk is no
/// word of git's). Nothing is written.
#[tokio::test(flavor = "multi_thread")]
async fn an_index_another_git_holds_is_reported_as_locked_where_git_says_so() {
    let (repo, side) = two_trees();
    write_in(&side, "a.txt", "1\n2\n3\n4\n5 side\n");
    let lock = repo.path.join(".git/index.lock");
    std::fs::write(&lock, "").expect("lock");

    let ((error, report), (end, said)) = apply_side_logged(&repo, &side).await;
    std::fs::remove_file(&lock).expect("unlock");

    assert!(error.is_some());
    let named = said.contains(".lock': File exists");
    assert_eq!(
        report.map(|r| r.kind),
        Some(match named {
            true => ReportKind::CarriedLocked,
            false => ReportKind::CarriedRefused,
        }),
        "{said}"
    );
    if !said.is_empty() {
        assert_eq!(end, CommandEnd::Exited(1), "git's words are a failed row's");
    }
    assert_eq!(read_in(&repo.path, "a.txt"), "1\n2\n3\n4\n5\n");
}

/// `stash.index` (git 2.55) would apply what was staged there as staged
/// here; the apply lands it unstaged, as every apply here does.
#[tokio::test(flavor = "multi_thread")]
async fn what_was_staged_there_lands_unstaged_whatever_stash_index_says() {
    let (mut repo, side) = two_trees();
    write_in(&side, "a.txt", "1 staged\n2\n3\n4\n5\n");
    repo.git_in(&side, &["add", "a.txt"]);
    repo.git(&["config", "stash.index", "true"]);

    assert_eq!(apply_side(&repo, &side).await, (None, None));

    let here = repo.path.clone();
    // The first line's leading blank goes with `git_in`'s trim.
    assert_eq!(status_in(&mut repo, &here), "M a.txt");
    assert_eq!(repo.git(&["diff", "--cached", "--name-only"]), "");
}

/// A file changed here that the work changes too: git turns the apply
/// down and writes nothing — not even the untracked files it brings,
/// which git's own apply puts in after turning the rest down — and what is
/// said is a report, about that worktree, for the screen to word.
#[tokio::test(flavor = "multi_thread")]
async fn a_change_here_in_the_way_is_reported_and_nothing_is_written() {
    let (mut repo, side) = two_trees();
    write_in(&side, "a.txt", "1\n2\n3\n4\n5 side\n");
    write_in(&side, "u.txt", "untracked\n");
    repo.write_file("a.txt", "1 here\n2\n3\n4\n5\n");

    let ((error, report), (end, said)) = apply_side_logged(&repo, &side).await;

    assert!(error.is_some_and(|e| e.contains("would be overwritten")));
    assert_eq!(end, CommandEnd::Exited(1), "a refusal is a failed row");
    assert!(said.contains("would be overwritten"), "{said}");
    let report = report.expect("a report");
    assert_eq!(report.kind, ReportKind::CarriedInTheWay);
    assert_eq!(report.name, "side");
    assert!(report.fact.is_empty(), "the screen writes the reason");
    let here = repo.path.clone();
    assert_eq!(status_in(&mut repo, &here), "M a.txt");
    assert_eq!(read_in(&repo.path, "a.txt"), "1 here\n2\n3\n4\n5\n");
}

/// git applies the tracked changes before it finds a file standing where
/// an untracked one goes, and leaves them applied
/// (`git_leaves_the_tracked_half_applied`) — so that is refused here before
/// git runs, with nothing written.
#[tokio::test(flavor = "multi_thread")]
async fn a_file_where_its_untracked_one_goes_is_refused_before_git() {
    let (mut repo, side) = two_trees();
    write_in(&side, "a.txt", "1\n2\n3\n4\n5 side\n");
    write_in(&side, "u.txt", "theirs\n");
    repo.write_file("u.txt", "mine\n");

    let (error, report) = apply_side(&repo, &side).await;

    assert!(error.is_some_and(|e| e.contains("u.txt")));
    assert_eq!(
        report.map(|r| r.kind),
        Some(ReportKind::CarriedFileInTheWay)
    );
    let here = repo.path.clone();
    assert_eq!(status_in(&mut repo, &here), "?? u.txt");
    assert_eq!(read_in(&repo.path, "a.txt"), "1\n2\n3\n4\n5\n");
    assert_eq!(read_in(&repo.path, "u.txt"), "mine\n");
}

/// A conflict standing there: there is no telling which side of it to
/// bring, so nothing is read or written.
#[tokio::test(flavor = "multi_thread")]
async fn a_conflict_there_is_refused_before_anything_is_read() {
    let (mut repo, side) = two_trees();
    repo.git_in(&side, &["switch", "-c", "other"]);
    write_in(&side, "a.txt", "1\n2\n3 other\n4\n5\n");
    repo.git_in(&side, &["commit", "-am", "o"]);
    repo.git_in(&side, &["switch", "side"]);
    write_in(&side, "a.txt", "1\n2\n3 side\n4\n5\n");
    repo.git_in(&side, &["commit", "-am", "s"]);
    let merged = repo.git_attempt(&side, &["merge", "other"]);
    assert!(!merged.status.success(), "the merge stops on the conflict");

    let (_, report) = apply_side(&repo, &side).await;

    assert_eq!(report.map(|r| r.kind), Some(ReportKind::CarriedUnread));
    let here = repo.path.clone();
    assert_eq!(status_in(&mut repo, &here), "");
}

/// Opens a session on `path` that tells `sink`, waiting for its `Opened` —
/// the `count`th the sink has heard, since two sessions share it.
async fn open_on(sink: &Arc<CaptureSink>, path: &Path, count: usize) -> Arc<RepoSession> {
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        path.to_path_buf(),
        sink.clone(),
        None,
    );
    sink.wait_for("Opened", |evs| {
        (evs.iter()
            .filter(|e| matches!(e, SessionEvent::Opened { .. }))
            .count()
            >= count)
            .then_some(())
    })
    .await;
    session
}

/// A tab open on the worktree the work comes from has a commit running
/// there: the apply waits for it rather than read that tree half-written.
/// Both sessions tell one sink, so the order the two writes started and
/// ended in is the order heard; and what lands here witnesses it too —
/// read under the commit, the staged file it was taking would have come
/// across as work.
#[tokio::test(flavor = "multi_thread")]
async fn a_write_accepted_there_first_is_done_before_the_work_is_read() {
    let (mut repo, side) = two_trees();
    write_in(&side, "a.txt", "1\n2\n3\n4\n5 side\n");
    repo.git_in(&side, &["add", "a.txt"]);
    write_in(&side, "extra.txt", "e\n");
    let release = repo.path.with_file_name("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));
    let sink = CaptureSink::new();
    let there = open_on(&sink, &side, 1).await;
    let commit = there
        .commit("held by the hook".into(), CommitOptions::default())
        .expect("the commit is accepted");
    sink.wait_for("the commit reached git", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == commit))
            .then_some(())
    })
    .await;

    let here = open_on(&sink, &repo.path, 2).await;
    // As the row hands it over: git's spelling, the one the listing prints
    // and the session there was opened on — not the temp folder's, which
    // Windows may hand out in its short form.
    let there_path = repo.git_in(&side, &["rev-parse", "--show-toplevel"]);
    let apply = here
        .apply_carried(there_path)
        .expect("the apply is accepted");
    std::fs::write(&release, b"go").expect("release the hook");

    assert_eq!(write_answer(&sink, commit).await, None);
    assert_eq!(answered(&sink, apply).await, (None, None));
    // Read with the lock let go before asserting: a panic under it poisons
    // the sink for the sessions still telling it.
    let (committed, applying) = {
        let heard = sink.events.lock().unwrap();
        let at = |wanted: &dyn Fn(&SessionEvent) -> bool| heard.iter().position(wanted);
        (
            at(&|e| matches!(e, SessionEvent::WriteFinished { id, .. } if *id == commit)),
            at(&|e| matches!(e, SessionEvent::WriteStarted { id, .. } if *id == apply)),
        )
    };
    assert!(
        committed.is_some() && committed < applying,
        "the apply started once the commit ended: {committed:?} / {applying:?}"
    );
    assert_eq!(
        read_in(&repo.path, "a.txt"),
        "1\n2\n3\n4\n5\n",
        "the commit went first, so nothing of a.txt was left to bring"
    );
    assert_eq!(read_in(&repo.path, "extra.txt"), "e\n");
    there.close();
    here.close();
}

/// Why the untracked files are looked at before git runs: `git stash
/// apply` puts the tracked changes in first, then refuses the untracked
/// file in the way and leaves the rest applied.
#[test]
fn git_leaves_the_tracked_half_applied() {
    let (mut repo, side) = two_trees();
    write_in(&side, "a.txt", "1\n2\n3\n4\n5 side\n");
    write_in(&side, "u.txt", "theirs\n");
    repo.git_in(&side, &["stash", "push", "-u"]);
    let stash = repo.git_in(&side, &["rev-parse", "stash@{0}"]);
    repo.write_file("u.txt", "mine\n");

    let here = repo.path.clone();
    let out = repo.git_attempt(&here, &["stash", "apply", &stash]);

    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("could not restore untracked files"));
    assert_eq!(read_in(&repo.path, "a.txt"), "1\n2\n3\n4\n5 side\n");
}
