//! Adversarial delivery orders at the core-to-UI boundary.
use super::*;
use platitude_core::Oid;
use platitude_core::details::CommitDetails;
use sink::BridgeSink;

fn details(hex: char, message: &str) -> CommitDetails {
    CommitDetails {
        oid: Oid::from_hex_str(&hex.to_string().repeat(40)).unwrap(),
        parents: Vec::new(),
        author_name: String::new(),
        author_email: String::new(),
        author_time: 0,
        committer_name: String::new(),
        committer_email: String::new(),
        committer_time: 0,
        co_authors: Vec::new(),
        message: message.into(),
        files: Vec::new(),
    }
}

#[test]
fn a_late_old_details_answer_cannot_erase_the_current_answer() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink::new(Arc::clone(&feeds), 1);
    // The user asked A, then B. B finishes first; A arrives before the UI drains.
    sink.event(SessionEvent::DetailsLoaded {
        generation: 2,
        details: details('b', "current B"),
    });
    sink.event(SessionEvent::DetailsLoaded {
        generation: 1,
        details: details('a', "old A"),
    });
    assert!(
        feeds.details.drain().iter().any(|message| {
            matches!(message, DetailsMsg::Loaded { details, .. } if details.message == "current B")
        }),
        "the latest requested answer must survive until the UI can drain it"
    );
}

#[test]
fn returning_to_the_same_oid_still_requires_the_latest_attempt() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink::new(Arc::clone(&feeds), 1);
    for (generation, oid, message) in [(3, 'a', "new A"), (2, 'b', "old B"), (1, 'a', "old A")] {
        sink.event(SessionEvent::DetailsLoaded {
            generation,
            details: details(oid, message),
        });
    }
    let messages = feeds.details.drain();
    assert_eq!(messages.len(), 1);
    assert!(
        matches!(&messages[0], DetailsMsg::Loaded { generation: 3, details } if details.message == "new A")
    );
    sink.event(SessionEvent::DetailsLoaded {
        generation: 1,
        details: details('a', "late duplicate"),
    });
    assert_eq!(
        feeds.details.depth(),
        0,
        "a drained generation must not be requeued"
    );
}

#[test]
fn an_old_failure_cannot_replace_a_new_success_or_raise_an_error() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink::new(Arc::clone(&feeds), 1);
    sink.event(SessionEvent::DetailsLoaded {
        generation: 2,
        details: details('b', "current B"),
    });
    sink.event(SessionEvent::DetailsFailed {
        generation: 1,
        oid: details('a', "").oid,
        error: platitude_core::GitError::Failed {
            command: "show".into(),
            code: 128,
            stderr: "old failure".into(),
        },
    });
    assert_eq!(
        feeds.tab.depth(),
        0,
        "errors belong to the matching details consumer"
    );
    let messages = feeds.details.drain();
    assert!(matches!(
        &messages[0],
        DetailsMsg::Loaded { generation: 2, .. }
    ));
}
