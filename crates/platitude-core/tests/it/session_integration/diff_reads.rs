//! The pane's diff reads against each other: a click stops the git of the
//! read it passes, a re-read never passes a click made while it read, and
//! re-reads asked while one is out share the one after it
//! (`session::diff_reads`).

use std::collections::HashMap;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened};
use crate::support::wait::{bounded, poll_once};
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::process::CommandEnd;
use platitude_core::session::{DiffReadOutcome, Recording, SessionEvent};
use platitude_core::stage;

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
    session.close_and_wait().await;
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
    session.close_and_wait().await;
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
    session.close_and_wait().await;
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
    session.close_and_wait().await;
}

/// A diff git cannot produce — a textconv that always fails — still
/// answers the pane (`SessionEvent::DiffUnread`, a latest read's failure:
/// `session::asks`), beside git's words on the error surface, so the pane
/// stops waiting for rows that are not coming.
#[tokio::test(flavor = "multi_thread")]
async fn a_diff_git_cannot_read_still_answers_the_pane() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "root");
    repo.write_file("f.txt", "2\n");
    repo.write_file(".git/info/attributes", "f.txt diff=boom\n");
    repo.git(&["config", "diff.boom.textconv", "false"]);
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    session.load_diff(unstaged("f.txt"));
    let target = sink
        .wait_for("the unread answer", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::DiffUnread { target } => Some(target.clone()),
                _ => None,
            })
        })
        .await;
    assert_eq!(target, unstaged("f.txt"));
    let (failed, loaded) = {
        let events = sink.events.lock().unwrap();
        (
            events
                .iter()
                .any(|e| matches!(e, SessionEvent::OpFailed { op: "diff", .. })),
            events
                .iter()
                .any(|e| matches!(e, SessionEvent::DiffLoaded { .. })),
        )
    };
    assert!(failed, "git's words went to the error surface");
    assert!(!loaded, "and no rows came");
    session.close_and_wait().await;
}

/// The fingerprints of every published unstaged diff of `path`, in order.
/// Over the events, not the sink: `wait_for` runs its predicate holding the
/// lock, and taking it again would wedge the test.
fn fingerprints_in(events: &[SessionEvent], path: &str) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::DiffLoaded {
                target: DiffTarget::Unstaged { path: seen },
                fingerprint,
                ..
            } if seen == path => Some(*fingerprint),
            _ => None,
        })
        .collect()
}

fn fingerprints_of(sink: &CaptureSink, path: &str) -> Vec<u64> {
    fingerprints_in(&sink.events.lock().unwrap(), path)
}

async fn diffs_reach(sink: &CaptureSink, path: &str, count: usize) {
    sink.wait_for("the diff", |events| {
        (fingerprints_in(events, path).len() >= count).then_some(())
    })
    .await;
}

/// The diff tick runs on every poll, so for an untouched file it has to say
/// nothing: republished rows would swap the list, and the reader's place.
#[tokio::test(flavor = "multi_thread")]
async fn a_re_read_of_a_file_nobody_touched_says_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    repo.write_file("f.txt", "two\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    let target = DiffTarget::Unstaged {
        path: "f.txt".to_string(),
    };
    session.load_diff(target.clone());
    diffs_reach(&sink, "f.txt", 1).await;
    let published = fingerprints_of(&sink, "f.txt");

    // Once the outcome has answered, nothing from this read is on its way.
    assert_eq!(
        crate::support::wait::bounded(
            "the tracked diff refresh",
            session.refresh_diff_tracked(target)
        )
        .await,
        DiffReadOutcome::Unchanged
    );
    assert_eq!(
        fingerprints_of(&sink, "f.txt"),
        published,
        "the re-read published a diff of a file nobody had touched"
    );
    session.close_and_wait().await;
}

/// A conflict resolved in another window, as a merge tool leaves one: the
/// markers are gone and git has not been told. Status reads the same before
/// and after (the path is unmerged either way), so only the file answers.
#[tokio::test(flavor = "multi_thread")]
async fn a_file_typed_over_outside_the_window_is_re_read() {
    let mut repo = crate::support::integrate::conflicting_branches();
    repo.git_expect_failure(&["merge", "side"]);
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    let target = DiffTarget::Unstaged {
        path: "f.txt".to_string(),
    };
    session.load_diff(target.clone());
    diffs_reach(&sink, "f.txt", 1).await;
    let published = fingerprints_of(&sink, "f.txt");

    let before = repo.git(&["status", "--porcelain", "--", "f.txt"]);
    repo.write_file("f.txt", "settled\n");
    let after = repo.git(&["status", "--porcelain", "--", "f.txt"]);
    assert_eq!(
        before, after,
        "the whole of this case is that status says the same thing"
    );

    assert_eq!(
        crate::support::wait::bounded(
            "the tracked diff refresh",
            session.refresh_diff_tracked(target)
        )
        .await,
        DiffReadOutcome::Sent
    );
    diffs_reach(&sink, "f.txt", published.len() + 1).await;
    assert_ne!(
        fingerprints_of(&sink, "f.txt").last(),
        published.last(),
        "the file the pane is holding was read again as the file it now is"
    );
    session.close_and_wait().await;
}

/// Two reads of the same file, the older landing last: the pane keeps the
/// newer one, and can still stage against it.
///
/// One partial stage asks for the file twice (the write's answer, the
/// status behind it), and the reads need not finish in order. Publishing
/// the older would leave the pane holding a fingerprint of bytes no longer
/// there, and the next partial stage is refused against it
/// (`stage::refusal::verify_fingerprint`).
#[tokio::test(flavor = "multi_thread")]
async fn the_older_of_two_reads_of_one_file_publishes_nothing() {
    let mut repo = TestRepo::init();
    let base: String = (1..=20).map(|n| format!("line {n}\n")).collect();
    repo.commit_file("notes.txt", &base, "root");
    // Two well-separated edits, so the stage below can take one of them
    // and leave the other unstaged.
    repo.write_file(
        "notes.txt",
        &base
            .replace("line 2\n", "line 2 EDITED\n")
            .replace("line 18\n", "line 18 EDITED\n"),
    );
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    let target = DiffTarget::Unstaged {
        path: "notes.txt".to_string(),
    };
    session.load_diff(target.clone());
    diffs_reach(&sink, "notes.txt", 1).await;
    let held = fingerprints_of(&sink, "notes.txt")[0];

    // The older read, polled to where it waits inside its git and left there.
    let mut older = Box::pin(session.read_diff(target.clone()));
    assert!(
        crate::support::wait::poll_once(&mut older).is_pending(),
        "the older read is waiting inside its own git"
    );

    // The stage takes the second hunk, so the file is no longer the one the
    // older read holds; then the second ask, as the status behind it raises.
    let (exec, cancel) = crate::support::exec::env();
    let repo_info = crate::support::info(&repo).await;
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(1)],
        held,
        &cancel,
    )
    .await
    .expect("the stage the two reads are about");
    session.load_diff(target.clone());
    diffs_reach(&sink, "notes.txt", 2).await;
    let published = fingerprints_of(&sink, "notes.txt");
    assert_ne!(
        published[0], published[1],
        "the stage moved the file between the two reads"
    );

    // Only now does the older one carry on: it finds itself passed and ends.
    assert_eq!(
        crate::support::wait::bounded("the older read", older).await,
        DiffReadOutcome::Overtaken
    );
    assert_eq!(
        fingerprints_of(&sink, "notes.txt"),
        published,
        "the older read published rows of its own"
    );

    // The pane holds the file's own fingerprint, so the next stage is not refused.
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        published[1],
        &cancel,
    )
    .await
    .expect("the stage after the pair is not refused as stale");
    session.close_and_wait().await;
}

/// The pane reads other worktrees' files under the same paths, so a
/// read's file is keyed by the worktree as well as the path.
#[tokio::test(flavor = "multi_thread")]
async fn a_worktrees_file_is_not_the_same_file_as_ours_of_that_name() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    repo.write_file("f.txt", "ours\n");
    let other = repo.path.parent().expect("a parent").join("other-worktree");
    repo.git(&["worktree", "add", "--detach", &other.to_string_lossy()]);
    std::fs::write(other.join("f.txt"), "theirs\n").expect("write the worktree's file");

    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".to_string(),
    };

    // Ours, then the worktree's: a reader stepping onto another worktree's row.
    session.load_diff(target.clone());
    diffs_reach(&sink, "f.txt", 1).await;
    session.load_carried_diff(other.to_string_lossy().to_string(), target.clone());
    diffs_reach(&sink, "f.txt", 2).await;
    let published = fingerprints_of(&sink, "f.txt");
    assert_ne!(
        published[0], published[1],
        "the two worktrees hold different bytes under that name"
    );

    // Ours is typed over into the worktree's bytes. Keyed by the path alone, the
    // re-read below would match the worktree's record and call our file
    // unmoved, leaving the worktree's rows on the pane.
    repo.write_file("f.txt", "theirs\n");
    assert_eq!(
        crate::support::wait::bounded(
            "the tracked diff refresh",
            session.refresh_diff_tracked(target)
        )
        .await,
        DiffReadOutcome::Sent,
        "the re-read of our file was answered against the worktree's fingerprint"
    );
    assert_eq!(
        fingerprints_of(&sink, "f.txt").len(),
        3,
        "the re-read published nothing for the reader to be handed"
    );
    session.close_and_wait().await;
}
