//! Hunks pressed one after another on the same diff, the later ones on the
//! drawing with the earlier taken out (`stage::PressedOn`), join one burst
//! (`session::write`): they run back to back, nothing is read between them,
//! and each lands where it was drawn.

use std::sync::Arc;

use crate::support::session::{opened, write_settled};
use crate::support::wait::bounded;
use crate::support::{TestRepo, barrier_hook};
use platitude_core::OperationId;
use platitude_core::commit::CommitOptions;
use platitude_core::details::{self, DiffTarget};
use platitude_core::parse::diff::parse_patch;
use platitude_core::parts::{Moves, Part};
use platitude_core::patch::HunkSelect;
use platitude_core::session::{RepoSession, SessionEvent};
use platitude_core::stage::PressedOn;

/// Where the first event `wanted` accepts stands.
fn at(events: &[SessionEvent], wanted: impl Fn(&SessionEvent) -> bool) -> usize {
    events
        .iter()
        .position(wanted)
        .expect("the event is in the stream")
}

fn answered(events: &[SessionEvent], id: OperationId) -> usize {
    at(
        events,
        |e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id),
    )
}

fn started(events: &[SessionEvent], id: OperationId) -> usize {
    at(
        events,
        |e| matches!(e, SessionEvent::WriteStarted { id: got, .. } if *got == id),
    )
}

fn status_read(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. } if display.starts_with("git status"))
}

/// A commit its hook holds until `release` is written, so the hunks pressed
/// behind it are in the queue when it answers.
fn held(session: &Arc<RepoSession>) -> OperationId {
    session
        .commit("held by the hook".into(), CommitOptions::default())
        .expect("the commit was accepted")
}

#[tokio::test(flavor = "multi_thread")]
async fn hunks_pressed_one_after_another_join_one_burst() {
    let mut repo = TestRepo::init();
    let base: String = (1..=20).map(|n| format!("line {n}\n")).collect();
    repo.commit_file("f.txt", &base, "root");
    repo.write_file("new.txt", "content\n");
    repo.git(&["add", "new.txt"]);
    let release = repo.path.join(".git").join("hook-release");
    repo.write_hook("pre-commit", &barrier_hook(&release));
    repo.write_file(
        "f.txt",
        &base
            .replace("line 2\n", "line 2 EDITED\n")
            .replace("line 18\n", "line 18 EDITED\n"),
    );
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    bounded("the opening's reads", session.wait_for_snapshot_reads()).await;
    let from = sink.events.lock().unwrap().len();

    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    let (exec, cancel) = crate::support::exec::env();
    let raw = details::file_diff_raw(&exec, &repo.path, &target, &cancel)
        .await
        .expect("diff");
    let seen = details::fingerprint(&raw);
    let read = Arc::new(parse_patch(&raw));

    let commit = held(&session);
    let first = session
        .apply_partial(target.clone(), vec![HunkSelect::whole(0)], seen)
        .expect("the first hunk was accepted");
    // The drawing has the first hunk taken out: the second is its hunk 0.
    let on = PressedOn {
        seen,
        read,
        ahead: vec![Part {
            hunk: 0,
            line: None,
            moves: Moves::Old,
        }],
    };
    let second = session
        .apply_partial_on(target, vec![HunkSelect::whole(0)], on)
        .expect("the second hunk was accepted");
    std::fs::write(&release, b"go").expect("release the hook");

    for id in [commit, first, second] {
        assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    }
    {
        let events = sink.events.lock().unwrap();
        let events = &events[from..];
        assert!(answered(events, commit) < started(events, first));
        assert!(answered(events, first) < started(events, second));
        let between = &events[answered(events, commit)..answered(events, second)];
        assert_eq!(
            between.iter().filter(|e| status_read(e)).count(),
            0,
            "nothing was read between the writes"
        );
        assert!(
            events.iter().all(|e| !matches!(
                e,
                SessionEvent::WriteFinished { id, error: Some(_), .. } if *id == second
            )),
            "the second hunk landed where it was drawn"
        );
    }
    let staged = repo.git(&["show", ":f.txt"]);
    assert!(staged.contains("line 2 EDITED") && staged.contains("line 18 EDITED"));
    session.close_and_wait().await;
}
