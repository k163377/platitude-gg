//! The command log holds what the user asked for, and not the background.

use crate::support::session::{CaptureSink, scenario};
use platitude_core::session::{Recording, RepoSession, SessionEvent};

#[tokio::test(flavor = "multi_thread")]
async fn the_command_log_holds_what_the_user_asked_for() {
    let (repo, _head) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;
    sink.opened_graph(&session, 5).await;
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::CommandStarted { .. })),
        0,
        "opening reads a dozen times over; none of it is the user's doing"
    );

    session.stage_all();
    let id = sink
        .wait_for("the staging command", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandStarted { id, display, .. } if display.contains(" add ") => {
                    Some(*id)
                }
                _ => None,
            })
        })
        .await;
    let (end, full) = sink
        .wait_for("its end", |evs| {
            let full = evs.iter().find_map(|e| match e {
                SessionEvent::CommandStarted { id: got, full, .. } if *got == id => {
                    Some(full.clone())
                }
                _ => None,
            })?;
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandFinished { id: got, end, .. } if *got == id => {
                    Some((*end, full.clone()))
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(end, platitude_core::CommandEnd::Exited(0));
    assert!(
        full.contains("LC_ALL=C") && full.contains("--no-optional-locks"),
        "the copyable form carries what is always applied: {full}"
    );

    // The refresh that follows the write is the session's own doing.
    let commands = sink.count(|e| matches!(e, SessionEvent::CommandStarted { .. }));
    assert_eq!(commands, 1, "only the write itself was recorded");

    // Switched on, the reads show up as well. Exact even though it is set
    // on a live session: the read below is asked for after the store, and
    // the ask is what spawns it.
    session.set_recording(Recording::WithBackground);
    session.refresh_status();
    sink.wait_for("a background read", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::CommandStarted { display, .. } if display.contains("status")))
            .then_some(())
    })
    .await;
    session.close();
}
