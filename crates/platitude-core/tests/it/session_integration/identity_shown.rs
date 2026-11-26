//! The identity as `.mailmap` spells it, read behind the identity and again
//! wherever a status shows HEAD moved — a commit, a pull or a switch can
//! bring another `.mailmap` (`RepoSession::read_identity_shown`).

use crate::support::TestRepo;
use crate::support::session::opened;
use crate::support::wait::bounded;
use platitude_core::session::SessionEvent;

/// The identity's addresses as shown so far, in the order they came.
fn shown_addresses(events: &[SessionEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::AuthorShown { shown } => shown.email.clone(),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn the_identitys_spelling_is_read_again_as_head_brings_another_mailmap() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.wait_for("the identity as configured", |evs| {
        (shown_addresses(evs).first().map(String::as_str) == Some("test@example.com")).then_some(())
    })
    .await;
    // A status before the move, so the one after it shows HEAD moved.
    bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;

    repo.commit_file(
        ".mailmap",
        "Test User <test@new.example.com> <test@example.com>\n",
        "map the address",
    );
    bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
    sink.wait_for("the identity as the new .mailmap spells it", |evs| {
        (shown_addresses(evs).last().map(String::as_str) == Some("test@new.example.com"))
            .then_some(())
    })
    .await;
    session.close_and_wait().await;
}
