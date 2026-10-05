//! What a status read spends on the operation a copy has stopped: file
//! reads under the copy's own git directory, which the opening resolved
//! (`RepoInfo::git_dir`) — no `rev-parse --git-path` per read, and in a
//! linked copy its own directory, not the main copy's.

use platitude_core::session::{Recording, RepoSession, SessionEvent};
use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, opened_at};
use crate::support::wait::bounded;

/// The commands started from event `from` on, as the log shows them.
fn commands_since(sink: &CaptureSink, from: usize) -> Vec<String> {
    sink.events.lock().unwrap()[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .collect()
}

/// What the newest status reading said about the stopped operation:
/// merging, rebasing, the step count, the incoming side's name and the
/// message waiting to be recorded.
#[derive(Debug)]
struct Standing {
    merging: bool,
    rebasing: bool,
    progress: Option<(u32, u32)>,
    theirs: String,
    op_message: String,
}

fn last_standing(sink: &CaptureSink) -> Standing {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::StatusLoaded {
                op_state,
                progress,
                sides,
                op_message,
                ..
            } => Some(Standing {
                merging: op_state.merging,
                rebasing: op_state.rebasing,
                progress: progress.map(|p| (p.current, p.total)),
                theirs: sides.theirs.clone(),
                op_message: op_message.clone(),
            }),
            _ => None,
        })
        .expect("a status was read")
}

/// One status read, every command it spawned counted from the ask — the
/// read itself among them, or a count of what is missing proves nothing.
async fn read_status(sink: &CaptureSink, session: &Arc<RepoSession>) -> Vec<String> {
    session.set_recording(Recording::WithBackground);
    let from = sink.events.lock().unwrap().len();
    session.refresh_status();
    bounded("the status read", session.wait_for_snapshot_reads()).await;
    let ran = commands_since(sink, from);
    assert_eq!(
        ran.iter()
            .filter(|c| c.contains("status --porcelain"))
            .count(),
        1,
        "the status was read and recorded: {ran:#?}"
    );
    ran
}

/// main and side both change the same line of `f.txt`.
fn conflicting_branches() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("f.txt", "side\n", "side change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stopped_merge_is_read_without_asking_git_where_its_files_are() {
    let mut repo = conflicting_branches();
    repo.git_expect_failure(&["merge", "side"]);
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    let ran = read_status(&sink, &session).await;

    let resolved: Vec<&String> = ran.iter().filter(|c| c.contains("--git-path")).collect();
    assert!(
        resolved.is_empty(),
        "the status read resolved paths: {ran:#?}"
    );
    let standing = last_standing(&sink);
    assert!(standing.merging, "{standing:?}");
    assert_eq!(standing.theirs, "side", "{standing:?}");
    assert!(
        standing.op_message.contains("Merge branch 'side'"),
        "{standing:?}"
    );
    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stopped_rebase_is_read_without_asking_git_where_its_files_are() {
    let mut repo = conflicting_branches();
    repo.git_expect_failure(&["rebase", "side"]);
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    let ran = read_status(&sink, &session).await;

    let resolved: Vec<&String> = ran.iter().filter(|c| c.contains("--git-path")).collect();
    assert!(
        resolved.is_empty(),
        "the status read resolved paths: {ran:#?}"
    );
    let standing = last_standing(&sink);
    assert!(standing.rebasing, "{standing:?}");
    assert_eq!(standing.progress, Some((1, 1)), "{standing:?}");
    session.close();
}

/// Each copy stands for itself: the main copy stopped mid-rebase, the
/// linked one mid-merge, and neither tab says the other's.
#[tokio::test(flavor = "multi_thread")]
async fn each_working_copy_reads_its_own_stopped_operation() {
    let mut repo = conflicting_branches();
    let linked = repo.path.with_file_name("linked");
    let linked_arg = linked.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "topic", &linked_arg, "main"]);
    assert!(
        !repo.git_ok(&["-C", &linked_arg, "merge", "side"]),
        "the linked copy's merge stops on the conflict"
    );
    repo.git_expect_failure(&["rebase", "side"]);

    let (main_sink, main) = opened(&repo).await;
    main_sink.opening_settled(&main).await;
    let (linked_sink, copy) = opened_at(&linked).await;
    linked_sink.opening_settled(&copy).await;

    let here = last_standing(&main_sink);
    assert!(here.rebasing && !here.merging, "main copy: {here:?}");
    assert_eq!(here.progress, Some((1, 1)), "main copy: {here:?}");
    assert!(here.op_message.is_empty(), "main copy: {here:?}");

    let there = last_standing(&linked_sink);
    assert!(there.merging && !there.rebasing, "linked copy: {there:?}");
    assert_eq!(there.progress, None, "linked copy: {there:?}");
    assert_eq!(there.theirs, "side", "linked copy: {there:?}");
    assert!(
        there.op_message.contains("Merge branch 'side'"),
        "linked copy: {there:?}"
    );
    main.close();
    copy.close();
}
