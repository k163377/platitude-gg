//! Reads that reuse what a read already landed, rather than asking git again.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, opened_with, write_result};
use platitude_core::details::DiffTarget;
use platitude_core::session::{DiffRefreshOutcome, Recording, SessionEvent};

/// Opening a repository asks for a read, and so does the window becoming
/// active a moment later; on a large repository that pair would be two
/// `for-each-ref` and two `status -uall` for one answer. The second
/// caller books a repeat instead of starting its own — and the point
/// of booking rather than dropping is that the repeat still sees what
/// happened in between.
// `worker_threads = 2` is the test's own premise: the hook below parks a
// worker on a blocking `recv`, and a pool inherited from the host can be
// one thread on a small runner — the parked hook then owns it all.
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

    // Park a read on its way out, which is where a reader stands after it
    // has seen the repository and before it asks whether to go round
    // again. Everything below happens inside that window.
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

    // The tree turns dirty behind the parked read — so what it is holding
    // is already out of date — and somebody asks again. Dropping that ask
    // for being a duplicate is what this is here to catch: the only read
    // that could answer it is the one that already looked.
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

/// The most `git status` processes the session ever had running at once,
/// read off the command log in the order it was written.
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

/// The places that ask for a status read do not know about each other:
/// the periodic tick, a write settling its own working tree, and the
/// window asking again. Two of them reading at once is the whole of a
/// `status --porcelain=v2 -uall` — every tracked and ignored file
/// lstat'd — run twice for one answer, one of the two thrown away after
/// both have already been paid for.
// `worker_threads = 2` is the test's own premise: the hook below parks a
// worker at the spawn it fires on, and the rest of the session has to
// keep running on another (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_ways_in_to_a_status_read_never_run_two_at_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    // The opening's own reads have to be done before recording starts, or
    // the hook below fires on one of them instead.
    sink.opened_graph(&session, 1).await;
    session.set_recording(Recording::WithBackground);

    // Park a read where its process is about to be spawned, which is
    // where a reader stands while it owns the flight. Everything below is
    // asked from inside that window.
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

    // The two ways in that used to read straight past it.
    session.refresh_poll();
    repo.write_file("f.txt", "dirty\n");
    session.stage_paths(vec!["f.txt".to_string()]);
    // git's own answer to the write, which it gives before the reads that
    // settle behind it: the burst is out before the parked read is let go.
    write_result(&sink, "stage").await;
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
/// does not ask a second and third process where HEAD is.
#[tokio::test(flavor = "multi_thread")]
async fn a_refs_read_takes_head_out_of_the_listing_it_already_has() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    // The opening's own reads have to be done before recording starts, or
    // this counts them as the refresh's; `Opened` alone only accepts the
    // path.
    sink.opened_graph(&session, 1).await;
    session.set_recording(Recording::WithBackground);
    // A baseline index, not an erasure: the history stays for the failure
    // message, and the wait below reads only past it.
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

/// Detached HEAD is the case the listing cannot answer — no ref is marked
/// — and it still gets a right answer, by asking.
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
                SessionEvent::RefsLoaded { snapshot } => snapshot.head.clone(),
                _ => None,
            })
        })
        .await;
    assert!(head.detached, "{head:?}");
    assert_eq!(head.branch, None);
    assert_eq!(head.oid.map(|o| o.to_hex()), Some(root));
    session.close();
}

/// A read that finds nothing moved publishes the snapshot it published
/// last — the same one, by pointer — instead of building an equal one.
///
/// Sorting every ref into a snapshot and a label map on every tick just
/// to compare the result equal costs 39ms of a core against
/// `JetBrains/kotlin`, ten seconds apart, for an answer the key already
/// had.
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
                SessionEvent::RefsLoaded { snapshot } => Some(Arc::clone(snapshot)),
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

/// Once a refs read has said where HEAD is, the walk stops asking.
///
/// Spawning `symbolic-ref` and `rev-parse` before every rebuild would put
/// two processes in front of the first chunk, on the path a commit or a
/// fetch takes to reach the screen.
#[tokio::test(flavor = "multi_thread")]
async fn the_walk_reads_head_from_the_refs_read_that_already_landed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 1).await;
    // Background recording starts only now, so the commands below are the
    // refresh's own; the event history stays for the failure message.
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

/// The remotes are read once per refs listing no more: a poll tick that
/// finds nothing moved spawns no `git config` to re-read them. A write
/// puts the question back, because a write is what can add one.
#[tokio::test(flavor = "multi_thread")]
async fn the_remotes_are_read_once_until_something_could_have_changed_them() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 1).await;
    // Background recording starts only now, so `reads` counts the work
    // below; the waits are count-relative, so the history can stay.
    session.set_recording(Recording::WithBackground);

    let reads = |sink: &CaptureSink| {
        commands_of(sink)
            .iter()
            .filter(|c| c.contains("remote\\..*\\.(url|pushurl)"))
            .count()
    };
    // Counted in snapshots delivered, not in elapsed time: a read that is
    // merely slow must not read as a read that did not happen.
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

    // The opening boundary already warmed it. An unchanged listing reuses
    // that answer, even though background command recording only starts now.
    let base = snapshots(&sink);
    session.refresh_refs();
    settle(base + 1).await;
    assert_eq!(reads(&sink), 0, "{:?}", commands_of(&sink));

    // A write can add a remote, so it drops the answer. Its post-write refs
    // snapshot is the causal boundary after the replacement read.
    let before_write = snapshots(&sink);
    session.create_branch("side".into(), None, false);
    write_result(&sink, "branch").await;
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

/// A push mark written into the user's global configuration reaches the
/// refs snapshot by the poll. `git config --global remote.pushDefault` in
/// a terminal moves no ref, lands no write in here, and leaves the
/// repository's own config file — the one the remotes cache stats —
/// untouched, so the status tick's marks read is what has to notice, and
/// send the refs out to publish the new destination
/// (`RepoSession::note_push_default`). Without that, the toolbar keeps
/// naming the old destination while the send (`plan_current_push`, which
/// reads the effective configuration) already obeys the new mark.
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

    // A poll that was refused (busy, or a write in front of it) would
    // prove nothing about what it reads.
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
                SessionEvent::RefsLoaded { snapshot } => snapshot
                    .push_default
                    .as_ref()
                    .is_some_and(|marked| marked.remote == "fork" && !marked.local),
                _ => false,
            })
            .then_some(())
    })
    .await;

    // The move re-read the remotes once, not once per tick: the next poll
    // reads the same marks, finds the held answer equal, and asks git for
    // no listing of its own.
    assert_eq!(listed(&sink), 1, "{:?}", commands_of(&sink));
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
/// reaches the notices by the poll.
///
/// The settings screen writes it against a work tree path rather than
/// through a session (`models::line_endings`, so that it can name a
/// repository nobody is looking at), which moves no ref and lands no write
/// in here — exactly like `git config core.autocrlf` typed in a terminal.
/// So the stat the remotes cache already makes on every tick has to notice
/// it (`RepoSession::forget_what_the_config_decides`). Without that, a
/// repository told to convert its line endings keeps warning about the
/// files git now converts, until the next commit or ref move.
#[tokio::test(flavor = "multi_thread")]
async fn the_line_ending_setting_written_beside_the_session_reaches_the_notices() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\n", "root");
    // Every line's ending flips, which is the one reading the patch bytes
    // settle on their own — and it is silenced by the setting rather than
    // by the bytes, which is what this is about.
    std::fs::write(repo.path.join("f.txt"), "one\r\ntwo\r\n").expect("rewrite with CRLF");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    // **The last status read, not any of them.** Waiting on "some status
    // said nothing" would be answered by the one the session sends before
    // it has settled any marks at all, which is the same shape as the
    // answer this is looking for.
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
    // The reader's own tail (apply, settle, the eol forget that bumps
    // the `Derived` generation) runs after the snapshot events, and a
    // diff racing that tail reads git twice — the settled boundary is
    // what the count needs.
    sink.opening_settled(&session).await;
    session.set_recording(Recording::WithBackground);

    let head = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    let parent = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD^"])).unwrap();
    for path in ["a.txt", "b.txt"] {
        session.load_diff(DiffTarget::Commit {
            oid: head,
            parent: Some(parent),
            path: path.to_string(),
            orig_path: None,
        });
    }
    sink.wait_for("both concurrent diffs", |events| {
        ["a.txt", "b.txt"]
            .iter()
            .all(|path| {
                events.iter().any(|event| {
                    matches!(event, SessionEvent::DiffLoaded { target, .. }
                        if matches!(target, DiffTarget::Commit { path: seen, .. }
                            if seen == path))
                })
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
/// writes it to a file of the run's own and hands the pane that. The file
/// goes when the pane lets go of it, and its directory with the session
/// (`preview::PreviewFiles`).
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

    fn old_files(events: &[SessionEvent]) -> Vec<std::path::PathBuf> {
        events
            .iter()
            .filter_map(|event| match event {
                SessionEvent::DiffLoaded {
                    preview: Some(preview),
                    ..
                } => preview.old.as_ref().and_then(|side| side.file.clone()),
                _ => None,
            })
            .collect()
    }

    let target = DiffTarget::Unstaged {
        path: "logo.png".to_string(),
    };
    session.load_diff(target.clone());
    let first = sink
        .wait_for("the picture's diff", |events| old_files(events).pop())
        .await;
    assert!(
        first.starts_with(platitude_core::preview::run_dir()),
        "written under the run's own directory: {}",
        first.display()
    );
    assert!(first.is_file());
    let dir = first.parent().unwrap().to_path_buf();

    // The pane closed: the file goes, the directory stays for the next
    // read.
    session.release_preview();
    assert!(!first.exists(), "released with the pane");
    assert!(dir.exists());

    // Read again: a new file under a new name, since the URL that names
    // it has to be a new one.
    session.load_diff(target);
    let second = sink
        .wait_for("the picture's second diff", |events| {
            old_files(events).into_iter().find(|file| *file != first)
        })
        .await;
    assert!(second.is_file());
    assert_eq!(second.parent().unwrap(), dir);

    // And the session closing takes the directory itself.
    session.close();
    assert!(!second.exists());
    assert!(!dir.exists(), "the session's directory goes with the close");
}

/// The count of what a remote already has of a plan's range answers
/// through its own event, echoing the range, so the plan that asked can
/// tell the answer from one about a plan since put away.
#[tokio::test(flavor = "multi_thread")]
async fn a_plans_published_count_answers_through_the_session() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");

    let (sink, session) = opened(&repo).await;

    session.check_plan_published("HEAD~1..HEAD".into());
    let published = sink
        .wait_for("PlanPublished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::PlanPublished { range, published } if range == "HEAD~1..HEAD" => {
                    Some(*published)
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(published, 0, "nothing is on a remote");
    session.close();
}

/// Every ask about a delete is answered, the one whose read fell over
/// included — that one as "cannot say", which the row draws the way it
/// draws a merged branch.
///
/// **The two are not told apart by the picture**, so the answer is where
/// they are told apart at all: a menu opened over a name git will not
/// resolve leaves its delete row plain, exactly as a menu over a merged
/// branch does, and a reader looking at the run afterwards has only this
/// event to say which of the two it was watching.
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

/// The fingerprints of every diff of the working-tree side of `path` in
/// what the session has published, in order — what a re-read either adds
/// to or leaves alone.
///
/// Over the events rather than over the sink, because `wait_for` runs its
/// predicate holding that lock: a helper that took it again would wedge
/// the test rather than fail it.
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

/// The tick that re-reads the file the diff pane is holding runs over
/// every open diff, so what it answers most of the time has to be nothing
/// at all: rows republished unasked would swap the list — and the reader's
/// place with it — once every poll.
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

    // The outcome is the boundary: once it has answered, nothing from this
    // read can still be on its way (core.md §非同期・並行テスト).
    assert_eq!(
        crate::support::wait::bounded(
            "the tracked diff refresh",
            session.refresh_diff_tracked(target).outcome()
        )
        .await,
        DiffRefreshOutcome::Unchanged
    );
    assert_eq!(
        diffs_of(&sink, "f.txt"),
        published,
        "the re-read published a diff of a file nobody had touched"
    );
    session.close();
}

/// A conflict resolved in another window, the way a merge tool leaves one:
/// the markers are gone from the file and git has not been told yet.
///
/// **Status cannot see this.** The path is unmerged either way, so every
/// letter and every count the window watches reads the same before and
/// after — and the pane went on drawing the conflict it was opened on.
/// Only the file itself answers.
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
            session.refresh_diff_tracked(target).outcome()
        )
        .await,
        DiffRefreshOutcome::Sent
    );
    diffs_reach(&sink, "f.txt", published.len() + 1).await;
    assert_ne!(
        diffs_of(&sink, "f.txt").last(),
        published.last(),
        "the file the pane is holding was read again as the file it now is"
    );
    session.close();
}
