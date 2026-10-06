//! Reads that reuse what a read already landed.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, opened_with, write_result};
use platitude_core::OperationKind;
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::session::{DiffReadOutcome, Recording, RefreshOutcome, SessionEvent};
use platitude_core::stage;

/// Asks coalesce, but one made while a read runs books a repeat, and the
/// repeat sees what changed in between.
// `worker_threads = 2`: the hook below parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_made_while_a_read_runs_gets_a_read_of_its_own() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.wait_for("the opening status read", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { .. }))
            .then_some(())
    })
    .await;

    // Park a read between seeing the repository and asking whether to go
    // round again; everything below happens inside that window.
    let (release, held) = std::sync::mpsc::channel::<()>();
    let clean_reads = sink
        .count(|e| matches!(e, SessionEvent::StatusLoaded { status, .. } if !status.is_dirty()));
    sink.hook_once(
        |e| matches!(e, SessionEvent::StatusLoaded { status, .. } if !status.is_dirty()),
        move || {
            held.recv().expect("the test releases the status read");
        },
    );
    session.refresh_status();
    sink.wait_for("the read reached the window", |events| {
        (events
            .iter()
            .filter(
                |e| matches!(e, SessionEvent::StatusLoaded { status, .. } if !status.is_dirty()),
            )
            .count()
            > clean_reads)
            .then_some(())
    })
    .await;

    // The tree turns dirty behind the parked read and somebody asks again.
    // Dropping that ask as a duplicate is the defect: the only read that
    // could answer it has already looked.
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    release.send(()).expect("let the read finish");

    sink.wait_for("a status read that sees the dirty tree", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { status, .. } if status.is_dirty()))
            .then_some(())
    })
    .await;
    session.close();
}

fn is_status_spawn(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. } if display.starts_with("git status"))
}

/// The most `git status` processes running at once, read off the command log.
fn status_reads_at_once(sink: &CaptureSink) -> usize {
    let mut running = std::collections::HashSet::new();
    let mut most = 0;
    for event in sink.events.lock().unwrap().iter() {
        match event {
            SessionEvent::CommandStarted { id, .. } if is_status_spawn(event) => {
                running.insert(*id);
                most = most.max(running.len());
            }
            SessionEvent::CommandFinished { id, .. } => {
                running.remove(id);
            }
            _ => {}
        }
    }
    most
}

/// The ways in to a status read — the tick, a write settling its tree, the
/// window asking again — do not know about each other; two reading at once
/// pay a whole `status -uall` twice for one answer.
// `worker_threads = 2`: the hook below parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_ways_in_to_a_status_read_never_run_two_at_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    // The opening's reads first, or the hook below fires on one of them.
    sink.opened_graph(&session, 1).await;
    session.set_recording(Recording::WithBackground);

    // Park a read at its spawn, while it owns the flight; everything below
    // is asked from inside that window.
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(is_status_spawn, move || {
        held.recv().expect("the test releases the parked read");
    });
    let spawned = sink.count(is_status_spawn);
    session.refresh_status();
    sink.wait_for("the parked read reached its process", move |evs| {
        (evs.iter().filter(|e| is_status_spawn(e)).count() > spawned).then_some(())
    })
    .await;

    // The two ways in that could read straight past it.
    session.refresh_poll();
    repo.write_file("f.txt", "dirty\n");
    session.stage_paths(vec!["f.txt".to_string()]);
    // git's answer comes before the reads that settle behind the write, so
    // the burst is out before the parked read is let go.
    write_result(&sink, OperationKind::Stage).await;
    release.send(()).expect("let the parked read finish");

    sink.wait_for("a status read that sees the staged file", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { status, .. } if status.is_dirty()))
            .then_some(())
    })
    .await;
    crate::support::wait::bounded(
        "the readers left the flight",
        session.wait_for_snapshot_reads(),
    )
    .await;
    assert_eq!(
        status_reads_at_once(&sink),
        1,
        "two status reads overlapped: {:?}",
        commands_of(&sink)
    );
    session.close();
}

/// The same pair under a replay, the one write the poll is let through
/// (`RepoSession::refresh_poll`). `stage_paths` cannot make this pair: a
/// write that does not replay turns the tick away before it reads.
///
/// The tick is asked on git's answer to the write, which arrives before
/// the write's refreshes and inside the flags the tick reads
/// (`session::write::serve`) — so it always meets the replay.
// `worker_threads = 2`: the hook below parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_poll_let_through_by_a_replay_does_not_read_beside_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["switch", "-c", "topic"]);
    repo.commit_file("t.txt", "topic\n", "topic one");
    repo.git(&["switch", "main"]);
    repo.commit_file("m.txt", "main\n", "main moved");
    repo.git(&["switch", "topic"]);

    let (sink, session) = opened(&repo).await;
    // The opening's reads first, as above.
    sink.opened_graph(&session, 3).await;
    session.set_recording(Recording::WithBackground);

    // Park a read at its spawn: everything asked below queues behind its
    // flight, the replay's own settling included.
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(is_status_spawn, move || {
        held.recv().expect("the test releases the parked read");
    });
    let spawned = sink.count(is_status_spawn);
    session.refresh_status();
    sink.wait_for("the parked read reached its process", move |evs| {
        (evs.iter().filter(|e| is_status_spawn(e)).count() > spawned).then_some(())
    })
    .await;

    session.rebase("main".into(), Default::default());
    // The reads that settle behind the replay are stuck on the park, so the
    // write still holds the repository at the line below.
    assert_eq!(write_result(&sink, OperationKind::Rebase).await, None);

    let poll = session.refresh_poll_tracked();
    release.send(()).expect("let the parked read finish");
    assert_ne!(
        crate::support::wait::bounded("the tracked poll", poll.outcome()).await,
        RefreshOutcome::WriteBusy,
        "the tick was turned away, so nothing of it ever met the replay"
    );
    crate::support::wait::bounded(
        "the readers left the flight",
        session.wait_for_snapshot_reads(),
    )
    .await;
    assert_eq!(
        status_reads_at_once(&sink),
        1,
        "the tick read status beside the replay's own: {:?}",
        commands_of(&sink)
    );
    session.close();
}

fn commands_of(sink: &CaptureSink) -> Vec<String> {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .collect()
}

/// The refs listing already marks the branch HEAD is on, so a refs read
/// takes HEAD out of it.
#[tokio::test(flavor = "multi_thread")]
async fn a_refs_read_takes_head_out_of_the_listing_it_already_has() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    // The opening's reads first (`Opened` is too early), or this counts
    // them as the refresh's.
    sink.opened_graph(&session, 1).await;
    session.set_recording(Recording::WithBackground);
    // The wait reads past this index; the history stays for the failure message.
    let from = sink.events.lock().unwrap().len();

    session.refresh_refs();
    sink.wait_for("the refs answer", move |evs| {
        evs[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
            .then_some(())
    })
    .await;
    let seen = commands_of(&sink);
    assert!(
        !seen.iter().any(|c| c.contains("symbolic-ref")),
        "HEAD came out of the listing: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c.contains("rev-parse --verify")),
        "and so did the commit it is on: {seen:?}"
    );
    session.close();
}

/// Detached HEAD marks no ref in the listing, so it is asked for.
#[tokio::test(flavor = "multi_thread")]
async fn a_detached_head_is_still_read_correctly() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("f.txt", "0\n", "root");
    repo.commit_file("g.txt", "1\n", "second");
    repo.git(&["checkout", "--detach", &root]);

    let (sink, session) = opened(&repo).await;
    let head = sink
        .wait_for("the refs snapshot", |evs| {
            evs.iter().rev().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot, .. } => snapshot.head.clone(),
                _ => None,
            })
        })
        .await;
    assert!(head.detached, "{head:?}");
    assert_eq!(head.branch, None);
    assert_eq!(head.oid.map(|o| o.to_hex()), Some(root));
    session.close();
}

/// A read that finds nothing moved republishes the last snapshot, by
/// pointer: re-sorting every ref on every tick only to compare equal costs
/// in proportion to the refs (ci/baseline/code-costs-windows-x64.md).
#[tokio::test(flavor = "multi_thread")]
async fn an_unmoved_repository_republishes_the_snapshot_it_already_built() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.git(&["branch", "side"]);
    let (sink, session) = opened(&repo).await;

    let latest = |sink: &CaptureSink| {
        sink.events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot, .. } => Some(Arc::clone(snapshot)),
                _ => None,
            })
            .expect("a snapshot")
    };
    let settle = async |want: usize| {
        sink.wait_for("a refs snapshot", move |evs| {
            (evs.iter()
                .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
                .count()
                >= want)
                .then_some(())
        })
        .await
    };

    settle(1).await;
    let first = latest(&sink);
    let before = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));

    session.refresh_refs();
    settle(before + 1).await;
    let again = latest(&sink);
    assert!(
        Arc::ptr_eq(&first, &again),
        "the same snapshot, not an equal one"
    );

    // A ref really moving still rebuilds, and the sidebar is told.
    repo.git(&["branch", "-D", "side"]);
    let before = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));
    session.refresh_refs();
    settle(before + 1).await;
    let after = latest(&sink);
    assert!(
        !Arc::ptr_eq(&first, &after),
        "a moved ref is a new snapshot"
    );
    assert_eq!(
        after.locals.len(),
        1,
        "and it is the repository as it stands: {:?}",
        after.locals
    );
    session.close();
}

/// Once a refs read has said where HEAD is, the walk stops asking:
/// `symbolic-ref` and `rev-parse` would be two processes in front of the
/// first chunk of every rebuild.
#[tokio::test(flavor = "multi_thread")]
async fn the_walk_reads_head_from_the_refs_read_that_already_landed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 1).await;
    // Recording starts after the opening, so the commands below are the refresh's.
    session.set_recording(Recording::WithBackground);

    // An external commit moves the refs, which is what rebuilds the graph.
    repo.commit_file("g.txt", "1\n", "second");
    session.refresh_refs();
    sink.settled_pass(2).await;

    let seen = commands_of(&sink);
    assert!(
        seen.iter().any(|c| c.contains("log -z")),
        "the walk did run: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c.contains("symbolic-ref")),
        "and did not ask where HEAD is: {seen:?}"
    );
    session.close();
}

/// The remotes are not read once per refs listing: a poll tick that
/// finds nothing moved spawns no `git config` to re-read them. A write
/// puts the question back, because a write is what can add one.
#[tokio::test(flavor = "multi_thread")]
async fn the_remotes_are_read_once_until_something_could_have_changed_them() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 1).await;
    // Recording starts after the opening, so `reads` counts the work below.
    session.set_recording(Recording::WithBackground);

    let reads = |sink: &CaptureSink| {
        commands_of(sink)
            .iter()
            .filter(|c| c.contains("remote\\..*\\.(url|pushurl)"))
            .count()
    };
    // Counted in snapshots delivered, not time: a slow read still counts.
    let snapshots =
        |sink: &CaptureSink| sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. })) as u32;
    let settle = async |want: u32| {
        sink.wait_for("a refs snapshot", move |evs| {
            (evs.iter()
                .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
                .count() as u32
                >= want)
                .then_some(())
        })
        .await
    };

    // The opening already warmed it; an unchanged listing reuses that answer.
    let base = snapshots(&sink);
    session.refresh_refs();
    settle(base + 1).await;
    assert_eq!(reads(&sink), 0, "{:?}", commands_of(&sink));

    // A write drops the answer; its post-write snapshot is the boundary
    // after the replacement read.
    let before_write = snapshots(&sink);
    session.create_branch("side".into(), None, false);
    write_result(&sink, OperationKind::Branch).await;
    settle(before_write + 1).await;
    assert_eq!(reads(&sink), 1, "{:?}", commands_of(&sink));

    // Now nothing moves, and the listings that follow ask git nothing.
    let from = snapshots(&sink);
    for n in 1..=3 {
        session.refresh_refs();
        settle(from + n).await;
    }
    assert_eq!(reads(&sink), 1, "still the one: {:?}", commands_of(&sink));
    session.close();
}

/// A push mark written into the global configuration reaches the refs
/// snapshot by the poll: it moves no ref and leaves the repository's own
/// config (the one the remotes cache stats) untouched, so the status tick's
/// marks read has to notice (`RepoSession::note_push_default`). Without it
/// the toolbar names the old destination while the send obeys the new one.
#[tokio::test(flavor = "multi_thread")]
async fn a_global_mark_moved_in_a_terminal_reaches_the_snapshot() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let exec = crate::support::exec::isolated_global(repo.global_config());
    let (sink, session) = opened_with(&repo, exec).await;
    sink.opened_graph(&session, 1).await;
    session.set_recording(Recording::WithBackground);
    let listed = |sink: &CaptureSink| {
        commands_of(sink)
            .iter()
            .filter(|c| c.contains("remote\\..*\\.(url|pushurl)"))
            .count()
    };

    repo.git(&["config", "--global", "remote.pushDefault", "fork"]);

    // A refused poll (busy, or a write in front) would prove nothing.
    let polled =
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await;
    assert!(
        matches!(
            polled,
            platitude_core::session::RefreshOutcome::Changed
                | platitude_core::session::RefreshOutcome::Unchanged
        ),
        "the poll ran: {polled:?}"
    );
    sink.wait_for("the global mark in the snapshot", |evs| {
        evs.iter()
            .any(|e| match e {
                SessionEvent::RefsLoaded { snapshot, .. } => snapshot
                    .push_default
                    .as_ref()
                    .is_some_and(|marked| marked.remote == "fork" && !marked.local),
                _ => false,
            })
            .then_some(())
    })
    .await;

    // The move re-read the remotes once; the next poll finds the held answer
    // equal and asks for no listing. Exactly one: the opening's boundary
    // leaves no remotes read in flight, so the move's forget is the only
    // invalidation and the listing behind it the only miss. A two is one of
    // those broken, and the commands say which — two listings either side of
    // a `config` read are a read an invalidation met part-way, which reads
    // again and caches nothing (`Derived::get_or_try_init`); two apart are a
    // second forget, the config stat taking the repository's own file for
    // moved (`forget_what_the_config_decides`). Neither is answered by
    // allowing two.
    assert_eq!(
        listed(&sink),
        1,
        "the global mark's move asked for one listing of the remotes: {:?}",
        commands_of(&sink)
    );
    let polled =
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await;
    assert!(
        matches!(
            polled,
            platitude_core::session::RefreshOutcome::Changed
                | platitude_core::session::RefreshOutcome::Unchanged
        ),
        "the second poll ran: {polled:?}"
    );
    assert_eq!(
        listed(&sink),
        1,
        "a settled mark is not re-read: {:?}",
        commands_of(&sink)
    );
    session.close();
}

/// The line-ending setting written into this repository's own file
/// reaches the notices by the poll. The settings screen writes it outside
/// the session (`models::line_endings`), like `git config core.autocrlf` in
/// a terminal, so the per-tick config stat has to notice it
/// (`RepoSession::forget_what_the_config_decides`); without that the
/// repository keeps warning about files git now converts.
#[tokio::test(flavor = "multi_thread")]
async fn the_line_ending_setting_written_beside_the_session_reaches_the_notices() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\n", "root");
    // Every line's ending flips: warned about from the patch bytes alone,
    // and silenced by the setting.
    std::fs::write(repo.path.join("f.txt"), "one\r\ntwo\r\n").expect("rewrite with CRLF");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    // The last status read: the one sent before any marks were settled
    // also says nothing.
    let marked = |events: &[SessionEvent]| {
        events.iter().rev().find_map(|e| match e {
            SessionEvent::StatusLoaded { eol_marks, .. } => Some(eol_marks.len()),
            _ => None,
        })
    };
    sink.wait_for("the flip warned about", |evs| {
        (marked(evs) == Some(1)).then_some(())
    })
    .await;

    repo.git(&["config", "--local", "core.autocrlf", "true"]);
    let polled =
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await;
    assert!(
        matches!(
            polled,
            platitude_core::session::RefreshOutcome::Changed
                | platitude_core::session::RefreshOutcome::Unchanged
        ),
        "the poll ran: {polled:?}"
    );

    sink.wait_for("the notice withdrawn", |evs| {
        (marked(evs) == Some(0)).then_some(())
    })
    .await;
    session.close();
}

/// Whether git normalises line endings is repository configuration, so
/// concurrent diff reads share one in-flight settings query.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_diffs_share_the_line_ending_setting_read() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let (sink, session) = opened(&repo).await;
    // The reader's tail (the eol forget bumps the `Derived` generation)
    // runs after the snapshot events, and a diff racing it reads git twice.
    sink.opening_settled(&session).await;
    session.set_recording(Recording::WithBackground);

    let head = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    let parent = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD^"])).unwrap();
    let file = |path: &str| DiffTarget::Commit {
        oid: head,
        parent: Some(parent),
        path: path.to_string(),
        orig_path: None,
    };

    // The first read is held where it asks for the setting, so the second
    // meets it in flight. Both read: the row the reader left is dropped only
    // after its ask (`RepoSession::diff_epoch`).
    let mut first = Box::pin(session.read_diff(file("a.txt")));
    assert!(
        crate::support::wait::poll_once(&mut first).is_pending(),
        "the first read is waiting on the reads it shares"
    );
    session.load_diff(file("b.txt"));
    assert_eq!(
        crate::support::wait::bounded("the row the reader left", first).await,
        DiffReadOutcome::Overtaken
    );
    sink.wait_for("the diff the reader stayed on", |events| {
        events
            .iter()
            .any(|event| {
                matches!(event, SessionEvent::DiffLoaded { target, .. }
                    if matches!(target, DiffTarget::Commit { path: seen, .. }
                        if seen == "b.txt"))
            })
            .then_some(())
    })
    .await;

    let reads = commands_of(&sink)
        .iter()
        .filter(|c| c.contains("autocrlf"))
        .count();
    assert_eq!(reads, 1, "{:?}", commands_of(&sink));
    session.close();
}

/// A picture's blob side has no path a URL could name, so the session
/// writes it to a file of the run's own (`preview::PreviewFiles`).
#[tokio::test(flavor = "multi_thread")]
async fn a_pictures_file_goes_with_the_pane_and_its_directory_with_the_session() {
    let mut repo = TestRepo::init();
    // The extension is all the preview asks; the bytes are anything git
    // calls binary.
    repo.write_file("logo.png", "\u{0}PNG-shaped bytes\n");
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "picture"]);
    repo.write_file("logo.png", "\u{0}PNG-shaped bytes, changed\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    /// What a picture read said about its old side. A side without a file
    /// is final, not pending (`preview::blob_side`): waiting for the file
    /// would spend the silence budget and report only that nothing was said.
    enum OldSide {
        File(std::path::PathBuf),
        /// The size, and what stopped the write — the reason is what the
        /// failure has to say (`preview::PreviewSide::unwritten`).
        SizeOnly(u64, Option<String>),
    }

    fn old_sides(events: &[SessionEvent]) -> Vec<OldSide> {
        events
            .iter()
            .filter_map(|event| match event {
                // Only a picture asks for a file; a binary that is no
                // picture lands by size on purpose.
                SessionEvent::DiffLoaded {
                    preview: Some(preview),
                    ..
                } if preview.image_mime.is_some() => preview.old.as_ref(),
                _ => None,
            })
            .map(|side| match &side.file {
                Some(path) => OldSide::File(path.clone()),
                None => OldSide::SizeOnly(side.size, side.unwritten.clone()),
            })
            .collect()
    }

    /// Outside the wait: a panic in the predicate poisons the sink's lock.
    fn file_of(side: OldSide) -> std::path::PathBuf {
        match side {
            OldSide::File(path) => path,
            OldSide::SizeOnly(size, why) => panic!(
                "the picture's old side arrived by size alone ({size} bytes), \
                 with no file for the pane to show it from: {}",
                why.unwrap_or_else(|| "nothing said why".to_string())
            ),
        }
    }

    let target = DiffTarget::Unstaged {
        path: "logo.png".to_string(),
    };
    session.load_diff(target.clone());
    let first = file_of(
        sink.wait_for("the picture's diff", |events| old_sides(events).pop())
            .await,
    );
    assert!(
        first.starts_with(platitude_core::preview::run_dir()),
        "written under the run's own directory: {}",
        first.display()
    );
    assert!(first.is_file());
    let dir = first.parent().unwrap().to_path_buf();

    // The pane closed: the file goes, the directory stays.
    session.release_preview();
    assert!(!first.exists(), "released with the pane");
    assert!(dir.exists());

    // Read again: a new name, since the URL has to change.
    session.load_diff(target);
    let second = file_of(
        sink.wait_for("the picture's second diff", |events| {
            old_sides(events)
                .into_iter()
                .find(|side| !matches!(side, OldSide::File(file) if *file == first))
        })
        .await,
    );
    assert!(second.is_file());
    assert_eq!(second.parent().unwrap(), dir);

    session.close();
    assert!(!second.exists());
    assert!(!dir.exists(), "the session's directory goes with the close");
}

/// Every ask about a delete is answered, a failed read included — as
/// "cannot say". The row draws that like a merged branch, so this event is
/// the only place the two are told apart.
#[tokio::test(flavor = "multi_thread")]
async fn an_ask_about_a_delete_is_answered_even_where_the_read_fails() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["branch", "topic"]);

    let (sink, session) = opened(&repo).await;

    session.check_branch_delete("topic".into());
    assert_eq!(
        branch_delete_answer(&sink, "topic").await,
        Some(true),
        "a branch its reference point reaches deletes quietly"
    );

    // git resolves no such name and leaves with 128, which is neither of
    // the two answers `merge-base --is-ancestor` gives.
    session.check_branch_delete("gone".into());
    assert_eq!(
        branch_delete_answer(&sink, "gone").await,
        None,
        "the failed read answered instead of going quiet"
    );
    session.close();
}

/// The answer to one `check_branch_delete`, waited for by the name it
/// echoes.
async fn branch_delete_answer(sink: &CaptureSink, branch: &str) -> Option<bool> {
    sink.wait_for("BranchDeleteChecked", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::BranchDeleteChecked {
                branch: named,
                merged,
            } if named == branch => Some(*merged),
            _ => None,
        })
    })
    .await
}

/// The fingerprints of every published unstaged diff of `path`, in order.
/// Over the events, not the sink: `wait_for` runs its predicate holding the
/// lock, and taking it again would wedge the test.
fn diffs_in(events: &[SessionEvent], path: &str) -> Vec<u64> {
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

fn diffs_of(sink: &CaptureSink, path: &str) -> Vec<u64> {
    diffs_in(&sink.events.lock().unwrap(), path)
}

async fn diffs_reach(sink: &CaptureSink, path: &str, count: usize) {
    sink.wait_for("the diff", |events| {
        (diffs_in(events, path).len() >= count).then_some(())
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
    let published = diffs_of(&sink, "f.txt");

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
        diffs_of(&sink, "f.txt"),
        published,
        "the re-read published a diff of a file nobody had touched"
    );
    session.close();
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
    let published = diffs_of(&sink, "f.txt");

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
        diffs_of(&sink, "f.txt").last(),
        published.last(),
        "the file the pane is holding was read again as the file it now is"
    );
    session.close();
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
    let held = diffs_of(&sink, "notes.txt")[0];

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
    let published = diffs_of(&sink, "notes.txt");
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
        diffs_of(&sink, "notes.txt"),
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
    session.close();
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
    let published = diffs_of(&sink, "f.txt");
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
        diffs_of(&sink, "f.txt").len(),
        3,
        "the re-read published nothing for the reader to be handed"
    );
    session.close();
}

/// Worktree reads take different times, and the pane shows the worktree
/// stepped onto last.
#[tokio::test(flavor = "multi_thread")]
async fn the_worktree_asked_about_last_is_the_one_the_pane_is_handed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    let first = repo.path.parent().expect("a parent").join("first-worktree");
    let second = repo
        .path
        .parent()
        .expect("a parent")
        .join("second-worktree");
    repo.git(&["worktree", "add", "--detach", &first.to_string_lossy()]);
    repo.git(&["worktree", "add", "--detach", &second.to_string_lossy()]);
    std::fs::write(first.join("f.txt"), "first\n").expect("write the first worktree's file");
    std::fs::write(second.join("f.txt"), "second\n").expect("write the second worktree's file");

    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    let at = |p: &std::path::Path| p.to_string_lossy().to_string();
    session.read_carried_status(at(&first), "first-worktree".to_string());
    session.read_carried_status(at(&second), "second-worktree".to_string());

    let carried = sink
        .wait_for("the worktree the pane is showing", |events| {
            events
                .iter()
                .filter_map(|e| match e {
                    SessionEvent::CarriedStatusLoaded { name, .. } => Some(name.clone()),
                    _ => None,
                })
                .next_back()
        })
        .await;
    assert_eq!(
        carried, "second-worktree",
        "the read the reader stepped off was left on screen"
    );
    // Passed before it starts: the second ask takes the slot on the caller's
    // thread, so the worktree stepped off spends no `status` (`session::latest`).
    let names: Vec<String> = sink
        .events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CarriedStatusLoaded { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        vec!["second-worktree".to_string()],
        "the worktree the reader stepped off answered as well"
    );
    session.close();
}
