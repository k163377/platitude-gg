//! Git LFS against real git: which paths git's attributes give
//! `filter=lfs`, what `git lfs version` answers on this machine, and the
//! count a session's status carries for the band's `NO LFS`.
//!
//! Whether LFS runs is mocked in the session tests (`lfs::mock_runs`): a
//! desk has it and the container does not, and the count has to be seen
//! both ways on each. mry keeps a free function's mock per thread and
//! wants `#[mry::lock(lfs::runs)]` to set one, so a session that is to see
//! it runs on the test's own thread (`#[tokio::test]`, not `multi_thread`:
//! a worker thread calls the real one).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use platitude_core::GitError;
use platitude_core::lfs;
use platitude_core::session::{Recording, RefreshOutcome, RepoSession, SessionEvent};

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::session::{CaptureSink, opened};

const LFS_LINE: &str = "filter=lfs diff=lfs merge=lfs -text";

async fn filtered(repo: &TestRepo, paths: &[String]) -> Vec<String> {
    let (executor, cancel) = env();
    lfs::filtered(&executor, &repo.path, paths, &cancel)
        .await
        .expect("read the filter attribute")
}

async fn required(repo: &TestRepo) -> bool {
    let (executor, cancel) = env();
    lfs::required(&executor, &repo.path, &cancel)
        .await
        .expect("read filter.lfs.required")
}

// ------------------------------------------------------------- attributes

/// Every place git reads attributes from counts, and only the `lfs`
/// filter does. The paths need not exist: attributes are about names.
#[tokio::test]
async fn every_attribute_source_that_gives_the_lfs_filter_counts() {
    let mut repo = TestRepo::init();
    repo.write_file(
        ".gitattributes",
        &format!("*.psd {LFS_LINE}\n*.txt filter=other\n"),
    );
    repo.write_file("art/.gitattributes", "*.bin filter=lfs\n");
    std::fs::write(repo.path.join(".git/info/attributes"), "*.mp4 filter=lfs\n")
        .expect("write info/attributes");
    repo.commit_file("README.md", "readme\n", "root");

    let paths = [
        "cover.psd",
        "art/cover.psd",
        "art/sheet.bin",
        "sheet.bin",
        "intro.mp4",
        "notes.txt",
        "README.md",
    ]
    .map(String::from);
    // `sheet.bin` is outside the directory whose file names it.
    assert_eq!(
        filtered(&repo, &paths).await,
        ["cover.psd", "art/cover.psd", "art/sheet.bin", "intro.mp4"].map(String::from)
    );
}

/// A path handed in twice is found once — beside itself, and again past
/// one command line, where the second naming goes in a later batch.
#[tokio::test]
async fn a_path_named_twice_is_found_once() {
    let mut repo = TestRepo::init();
    repo.write_file(".gitattributes", &format!("*.psd {LFS_LINE}\n"));
    repo.commit_file("README.md", "readme\n", "root");
    let deep = "layers/".repeat(30);
    let mut paths = ["a.psd", "b.txt", "a.psd"].map(String::from).to_vec();
    paths.extend((0..200).map(|i| format!("{deep}layer-{i}.txt")));
    paths.push("a.psd".to_string());
    assert!(paths.iter().map(|p| p.len() + 1).sum::<usize>() > 32_767);

    assert_eq!(filtered(&repo, &paths).await, ["a.psd".to_string()]);
}

/// `filter.lfs.required` read as git reads a boolean: unset is not
/// required, and so is a false spelled any way git accepts.
#[tokio::test]
async fn whether_the_lfs_filter_is_required_is_the_config_read_as_a_boolean() {
    let mut repo = TestRepo::init();
    assert!(!required(&repo).await, "unset");
    repo.git(&["config", "filter.lfs.required", "yes"]);
    assert!(required(&repo).await, "a true spelled `yes`");
    repo.git(&["config", "filter.lfs.required", "off"]);
    assert!(!required(&repo).await, "a false spelled `off`");
}

/// Paths long enough that one command line cannot hold them all — several
/// times Windows' 32767 characters — are all counted: each batch stays
/// under the line, where one past it would not start at all.
#[tokio::test]
async fn paths_past_one_command_line_are_all_counted() {
    let mut repo = TestRepo::init();
    repo.write_file(".gitattributes", &format!("*.psd {LFS_LINE}\n"));
    repo.commit_file("README.md", "readme\n", "root");
    let deep = "layers/".repeat(30);
    let paths: Vec<String> = (0..600).map(|i| format!("{deep}layer-{i}.psd")).collect();
    assert!(paths.iter().map(|p| p.len() + 1).sum::<usize>() > 3 * 32_767);

    assert_eq!(filtered(&repo, &paths).await.len(), 600);
}

/// What `git lfs version` says on this machine, read by hand beside the
/// app's reading: a desk with LFS and a container without it take the two
/// ways through the exit code. No mock is set on this thread, so the real
/// one answers.
#[tokio::test]
async fn whether_lfs_runs_is_what_git_lfs_version_says() {
    let repo = TestRepo::init();
    let witness = std::process::Command::new("git")
        .args(["lfs", "version"])
        .current_dir(&repo.path)
        .output()
        .expect("run git lfs version")
        .status
        .success();

    let (executor, cancel) = env();
    let runs = lfs::runs(&executor, &repo.path, &cancel)
        .await
        .expect("an answer either way");
    assert_eq!(runs, witness);
}

// ---------------------------------------------------------------- session

/// The count the last status carried.
fn last_count(events: &[SessionEvent]) -> Option<usize> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::StatusLoaded { lfs_needed, .. } => Some(*lfs_needed),
        _ => None,
    })
}

async fn counted(sink: &CaptureSink, what: &str, count: usize) {
    sink.wait_for(what, |evs| (last_count(evs) == Some(count)).then_some(()))
        .await;
}

/// One status tick, awaited to its end.
async fn poll(session: &Arc<RepoSession>) {
    let polled =
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await;
    assert!(
        matches!(polled, RefreshOutcome::Changed | RefreshOutcome::Unchanged),
        "the poll ran: {polled:?}"
    );
}

/// Writes a file the tree did not have and ticks until a status names it —
/// a status only a tick that saw the tree move can send.
async fn new_file_read(
    repo: &TestRepo,
    sink: &CaptureSink,
    session: &Arc<RepoSession>,
    name: &str,
) {
    repo.write_file(name, "later\n");
    poll(session).await;
    sink.wait_for(&format!("the status with {name}"), |evs| {
        evs.iter()
            .rev()
            .find_map(|e| match e {
                SessionEvent::StatusLoaded { status, .. } => Some(status),
                _ => None,
            })
            .filter(|status| status.items.iter().any(|item| item.path() == name))
            .map(|_| ())
    })
    .await;
}

/// How many times the pending diffs were read, off the command log with
/// the background kept (`Recording::WithBackground`): only a tick that
/// counts again reads them (the line-ending marks share its beat).
fn counts_taken(sink: &CaptureSink) -> usize {
    sink.count(|e| {
        matches!(e, SessionEvent::CommandStarted { display, .. }
                 if display.contains("diff --cached --no-ext-diff"))
    })
}

/// A repository tracking `*.psd` through LFS, with pending changes of
/// every kind: two the working tree still has to stage (`kept.psd`
/// changed, `new.psd` new), and four it does not — a removal
/// (`gone.psd`), a change already staged (`staged.psd`), a rename
/// (`moved.psd`) and a text file LFS has nothing to do with.
fn lfs_repo() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file(
        ".gitattributes",
        &format!("*.psd {LFS_LINE}\n"),
        "track psd",
    );
    for name in ["kept.psd", "gone.psd", "staged.psd", "moved.psd"] {
        repo.commit_file(name, "layers\n", name);
    }
    repo.commit_file("notes.txt", "notes\n", "notes");
    repo.write_file("kept.psd", "more layers\n");
    repo.write_file("new.psd", "layers\n");
    repo.write_file("staged.psd", "more layers\n");
    repo.git(&["add", "--", "staged.psd"]);
    repo.git(&["mv", "moved.psd", "renamed.psd"]);
    repo.write_file("notes.txt", "more notes\n");
    std::fs::remove_file(repo.path.join("gone.psd")).expect("remove gone.psd");
    repo
}

/// Where git cannot run LFS, the files the filter has still to clean are
/// counted — not the removal, the staged change or the rename, which are
/// past it.
#[tokio::test]
#[mry::lock(lfs::runs)]
async fn the_status_counts_what_needs_lfs_where_git_cannot_run_it() {
    lfs::mock_runs().returns_with(|| Ok(false));
    let repo = lfs_repo();
    let (sink, session) = opened(&repo).await;

    counted(&sink, "the two files counted", 2).await;
    session.close();
}

/// The paths a discard could not copy, as the last status carried them,
/// sorted.
fn last_not_copied(events: &[SessionEvent]) -> Option<Vec<String>> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::StatusLoaded { not_copied, .. } => {
            let mut paths = not_copied.as_ref().clone();
            paths.sort();
            Some(paths)
        }
        _ => None,
    })
}

/// Where the filter is required — what `git lfs install` writes — the
/// files it would have to clean are the ones a discard cannot copy: `git
/// add` refuses them (`discards_integration::recording`).
///
/// The filter here runs (`cat`) and only `lfs::runs` says it does not: a
/// required filter that really fails stops `git status` itself wherever git
/// has to clean a file to read the tree (a rename, a racy entry —
/// `fatal: <path>: clean filter 'lfs' failed`), and this repository has
/// both.
#[tokio::test]
#[mry::lock(lfs::runs)]
async fn a_required_filter_names_the_files_a_discard_cannot_copy() {
    lfs::mock_runs().returns_with(|| Ok(false));
    let mut repo = lfs_repo();
    repo.git(&["config", "filter.lfs.clean", "cat"]);
    repo.git(&["config", "filter.lfs.required", "true"]);
    let (sink, session) = opened(&repo).await;

    counted(&sink, "the two files counted", 2).await;
    assert_eq!(
        last_not_copied(&sink.events.lock().unwrap()),
        Some(vec!["kept.psd".to_string(), "new.psd".to_string()])
    );
    session.close();
}

/// Where the filter is not required, the same files are counted for the
/// badge and none is named: git takes a file its filter cannot clean whole,
/// so a discard copies it.
#[tokio::test]
#[mry::lock(lfs::runs)]
async fn a_filter_that_is_not_required_names_nothing_a_discard_cannot_copy() {
    lfs::mock_runs().returns_with(|| Ok(false));
    let repo = lfs_repo();
    let (sink, session) = opened(&repo).await;

    counted(&sink, "the two files counted", 2).await;
    assert_eq!(
        last_not_copied(&sink.events.lock().unwrap()),
        Some(Vec::new())
    );
    session.close();
}

/// Where git runs LFS nothing is counted, and LFS is asked once however
/// often the tree moves after.
#[tokio::test]
#[mry::lock(lfs::runs)]
async fn where_git_runs_lfs_it_is_asked_once_and_nothing_is_counted() {
    lfs::mock_runs().returns_with(|| Ok(true));
    let repo = lfs_repo();
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    for name in ["later.psd", "later.txt"] {
        new_file_read(&repo, &sink, &session, name).await;
    }

    lfs::mock_runs().assert_called(1);
    assert_eq!(
        sink.count(
            |e| matches!(e, SessionEvent::StatusLoaded { lfs_needed, .. } if *lfs_needed > 0)
        ),
        0,
        "a status counted files that LFS was there for"
    );
    session.close();
}

/// LFS installed while the count stands clears it on the next tick,
/// though nothing in the tree moved: that tick counts nothing again, it
/// only asks after LFS.
#[tokio::test]
#[mry::lock(lfs::runs)]
async fn lfs_arriving_clears_the_count_on_the_next_tick() {
    let installed = Arc::new(AtomicBool::new(false));
    let answer = Arc::clone(&installed);
    lfs::mock_runs().returns_with(move || Ok(answer.load(Ordering::SeqCst)));
    let repo = lfs_repo();
    let (sink, session) = opened(&repo).await;
    session.set_recording(Recording::WithBackground);
    counted(&sink, "the two files counted", 2).await;
    sink.opening_settled(&session).await;
    // A tick that counts — a new file moves the status, which only this
    // tick can report — so the witness is seen to see one; it also takes
    // whatever the opening left marked stale off the tick under test.
    new_file_read(&repo, &sink, &session, "later.txt").await;
    assert_eq!(last_count(&sink.events.lock().unwrap()), Some(2));
    let before = counts_taken(&sink);
    assert!(before > 0, "the witness saw no counting tick at all");

    installed.store(true, Ordering::SeqCst);
    poll(&session).await;
    counted(&sink, "the count cleared", 0).await;
    assert_eq!(counts_taken(&sink), before, "the tick counted again");
    session.close();
}

/// A count that could not be had is tried again on the next tick, with
/// nothing in the tree moving: waiting for the tree would leave the files
/// uncounted until then. The asking fails for as long as the opening
/// lasts, however many times it counts.
#[tokio::test]
#[mry::lock(lfs::runs)]
async fn a_count_that_could_not_be_had_is_tried_on_the_next_tick() {
    let failing = Arc::new(AtomicBool::new(true));
    let asked = Arc::new(AtomicUsize::new(0));
    let (fails, seen) = (Arc::clone(&failing), Arc::clone(&asked));
    lfs::mock_runs().returns_with(move || {
        seen.fetch_add(1, Ordering::SeqCst);
        if fails.load(Ordering::SeqCst) {
            Err(GitError::Failed {
                command: "git lfs version".to_string(),
                code: 128,
                stderr: "fatal: not now".to_string(),
            })
        } else {
            Ok(false)
        }
    });
    let repo = lfs_repo();
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    assert!(asked.load(Ordering::SeqCst) >= 1, "LFS was asked");
    assert_eq!(
        last_count(&sink.events.lock().unwrap()),
        Some(0),
        "nothing could be counted"
    );
    // A tick still failing, which takes whatever the opening left marked
    // stale: the tick under test is then one that only the retry counts on.
    poll(&session).await;

    failing.store(false, Ordering::SeqCst);
    poll(&session).await;
    counted(&sink, "the two files counted after all", 2).await;
    session.close();
}
