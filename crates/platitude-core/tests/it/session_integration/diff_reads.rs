//! The pane's diff reads against each other: a click stops the git of the
//! read it passes, a re-read never passes a click made while it read, and
//! re-reads asked while one is out share the one after it
//! (`session::diff_reads`).

use std::collections::HashMap;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened};
use crate::support::wait::{bounded, poll_once};
use platitude_core::details::DiffTarget;
use platitude_core::process::CommandEnd;
use platitude_core::session::{DiffReadOutcome, Recording, SessionEvent};

/// Every command started, as the log shows it, with how it ended.
fn commands(sink: &CaptureSink) -> Vec<(String, Option<CommandEnd>)> {
    let events = sink.events.lock().unwrap();
    let ends: HashMap<u64, CommandEnd> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandFinished { id, end, .. } => Some((*id, *end)),
            _ => None,
        })
        .collect();
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { id, display, .. } => {
                Some((display.clone(), ends.get(id).copied()))
            }
            _ => None,
        })
        .collect()
}

fn unstaged(path: &str) -> DiffTarget {
    DiffTarget::Unstaged {
        path: path.to_string(),
    }
}

/// Waits until a diff of `path` has gone out `count` times.
async fn diffs_of(sink: &CaptureSink, path: &str, count: usize) {
    sink.wait_for("the diff", |events| {
        let sent = events
            .iter()
            .filter(|e| {
                matches!(e, SessionEvent::DiffLoaded { target: DiffTarget::Unstaged { path: seen }, .. }
                    if seen == path)
            })
            .count();
        (sent >= count).then_some(())
    })
    .await;
}

/// A walk down a file list leaves nothing of the rows it passed running:
/// the passed read's source is not read once the click is made.
#[tokio::test(flavor = "multi_thread")]
async fn a_click_stops_the_git_of_the_read_it_passes() {
    let mut repo = TestRepo::init();
    repo.commit_file("src/a.rs", "fn a() -> u32 {\n    1\n}\n", "add a");
    repo.commit_file("src/b.rs", "fn b() -> u32 {\n    1\n}\n", "add b");
    repo.write_file("src/a.rs", "fn a() -> u32 {\n    2\n}\n");
    repo.git(&["add", "--", "src/a.rs"]);
    repo.write_file("src/b.rs", "fn b() -> u32 {\n    2\n}\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    session.set_recording(Recording::WithBackground);

    // Staged: its new side is a blob, read by a probe and then a
    // `cat-file` — the second is the one a stopped read must not run.
    let mut first = Box::pin(session.read_diff(DiffTarget::Staged {
        path: "src/a.rs".to_string(),
        orig_path: None,
    }));
    assert!(
        poll_once(&mut first).is_pending(),
        "the first read is waiting inside its own git"
    );
    session.load_diff(unstaged("src/b.rs"));
    assert_eq!(
        bounded("the read the reader left", first).await,
        DiffReadOutcome::Overtaken
    );
    diffs_of(&sink, "src/b.rs", 1).await;

    let ran: Vec<(String, Option<CommandEnd>)> = commands(&sink)
        .into_iter()
        .filter(|(display, _)| display.contains("cat-file") && display.contains("src/a.rs"))
        .collect();
    assert!(
        ran.iter()
            .all(|(_, end)| *end == Some(CommandEnd::Cancelled)),
        "the passed read's source was read: {ran:#?}"
    );
    session.close();
}

/// The re-read a tick or a focus asks for, out when the reader clicks
/// another file, finds its own file moved: it must not take the pane from
/// the click, whose read would then hand over nothing and leave the pane
/// waiting for rows that never come.
#[tokio::test(flavor = "multi_thread")]
async fn a_re_read_does_not_pass_a_click_made_while_it_read() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a1\n", "add a");
    repo.commit_file("b.txt", "b1\n", "add b");
    repo.write_file("a.txt", "a2\n");
    repo.write_file("b.txt", "b2\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    session.load_diff(unstaged("a.txt"));
    diffs_of(&sink, "a.txt", 1).await;

    // Moved since the pane read it, so the re-read has something to send.
    repo.write_file("a.txt", "a3\n");
    let mut tick = Box::pin(session.refresh_diff_tracked(unstaged("a.txt")));
    assert!(
        poll_once(&mut tick).is_pending(),
        "the re-read is waiting inside its own git"
    );
    let click = session.read_diff(unstaged("b.txt"));

    assert_eq!(
        bounded("the re-read", tick).await,
        DiffReadOutcome::Overtaken,
        "the click is the newer ask"
    );
    assert_eq!(
        bounded("the click", click).await,
        DiffReadOutcome::Sent,
        "the file the reader moved to reached the pane"
    );
    session.close();
}

/// A tick and a focus over one file: asks made while a re-read is out
/// share the one after it — not the one out, which may have read the file
/// before their reason — so three asks cost two raw diffs, not three.
#[tokio::test(flavor = "multi_thread")]
async fn asks_while_a_re_read_is_out_share_the_one_after_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "root");
    repo.write_file("f.txt", "2\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    session.load_diff(unstaged("f.txt"));
    diffs_of(&sink, "f.txt", 1).await;

    repo.write_file("f.txt", "3\n");
    session.set_recording(Recording::WithBackground);
    let from = commands(&sink).len();
    let mut lead = Box::pin(session.refresh_diff_tracked(unstaged("f.txt")));
    assert!(
        poll_once(&mut lead).is_pending(),
        "the first re-read is waiting inside its own git"
    );
    let second = session.refresh_diff_tracked(unstaged("f.txt"));
    let third = session.refresh_diff_tracked(unstaged("f.txt"));

    let (lead, second, third) = tokio::join!(
        bounded("the first re-read", lead),
        bounded("the second ask", second),
        bounded("the third ask", third),
    );
    assert_eq!(lead, DiffReadOutcome::Sent, "the file had moved");
    assert_eq!(
        (second, third),
        (DiffReadOutcome::Unchanged, DiffReadOutcome::Unchanged),
        "one re-read answered both, and found the file as the first left it"
    );
    let raw: Vec<String> = commands(&sink)[from..]
        .iter()
        .map(|(display, _)| display.clone())
        .filter(|display| display.contains(" diff ") && display.contains("f.txt"))
        .collect();
    assert_eq!(raw.len(), 2, "{raw:#?}");
    session.close();
}

/// A re-read asked and dropped before it ever ran lets its file go: held,
/// every later ask of that file would wait on a lead that never reads.
#[tokio::test(flavor = "multi_thread")]
async fn a_re_read_dropped_unread_holds_nothing_up() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "root");
    repo.write_file("f.txt", "2\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    session.load_diff(unstaged("f.txt"));
    diffs_of(&sink, "f.txt", 1).await;

    drop(session.refresh_diff_tracked(unstaged("f.txt")));
    assert_eq!(
        bounded(
            "a re-read after the dropped one",
            session.refresh_diff_tracked(unstaged("f.txt"))
        )
        .await,
        DiffReadOutcome::Unchanged
    );
    session.close();
}
