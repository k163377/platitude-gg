//! What the session answers about one commit beside its details
//! (`details_order`): the diff, the colours over that diff, and its
//! signature.

use crate::support::TestRepo;
use crate::support::session::opened;
use platitude_core::Oid;
use platitude_core::details::DiffTarget;
use platitude_core::identity::SignatureStatus;
use platitude_core::session::{DiffReadOutcome, SessionEvent};

/// Colours arrive behind their rows: a diff that waited for its colours
/// would show nothing while the colouring runs (`SessionEvent::DiffColoured`).
#[tokio::test(flavor = "multi_thread")]
async fn a_diff_arrives_before_the_colours_for_it() {
    let mut repo = TestRepo::init();
    // A language the set has rules for, or there is no second event.
    repo.commit_file("src/f.rs", "fn one() -> u32 {\n    1\n}\n", "add f");
    repo.write_file("src/f.rs", "fn one() -> u32 {\n    2\n}\n");

    let (sink, session) = opened(&repo).await;

    session.load_diff(DiffTarget::Unstaged {
        path: "src/f.rs".to_string(),
    });
    sink.wait_for("DiffColoured", |evs| {
        let rows = evs
            .iter()
            .position(|e| matches!(e, SessionEvent::DiffLoaded { .. }))?;
        let painted = evs
            .iter()
            .position(|e| matches!(e, SessionEvent::DiffColoured { .. }))?;
        assert!(
            rows < painted,
            "the rows have to be out before the colours for them"
        );
        // An empty answer would pass the ordering while saying nothing.
        match &evs[painted] {
            SessionEvent::DiffColoured { colors, .. } => assert!(
                !colors.is_empty(),
                "a language the set knows should colour something"
            ),
            _ => unreachable!(),
        }
        Some(())
    })
    .await;

    session.close();
}

/// A read nobody is waiting for any more hands over nothing — not even its
/// rows, which carry the fingerprint the next partial stage is refused
/// against (`RepoSession::diff_epoch`): an abandoned read landing last
/// would hand the pane one from a file it has left.
#[tokio::test(flavor = "multi_thread")]
async fn a_read_the_reader_has_left_hands_over_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("src/a.rs", "fn a() -> u32 {\n    1\n}\n", "add a");
    repo.commit_file("src/b.rs", "fn b() -> u32 {\n    1\n}\n", "add b");
    repo.write_file("src/a.rs", "fn a() -> u32 {\n    2\n}\n");
    repo.write_file("src/b.rs", "fn b() -> u32 {\n    2\n}\n");

    let (sink, session) = opened(&repo).await;

    // The first click, polled once so it stops waiting on its own git; the
    // second is asked for while the first is provably in flight.
    let mut first = Box::pin(session.read_diff(DiffTarget::Unstaged {
        path: "src/a.rs".to_string(),
    }));
    assert!(
        crate::support::wait::poll_once(&mut first).is_pending(),
        "the first read is waiting inside its own git"
    );
    session.load_diff(DiffTarget::Unstaged {
        path: "src/b.rs".to_string(),
    });

    // Resumed, it finds itself passed and ends: the completion boundary
    // the counts below are read against.
    assert_eq!(
        crate::support::wait::bounded("the read the reader left", first).await,
        DiffReadOutcome::Overtaken
    );
    sink.wait_for("DiffColoured for b", |evs| {
        evs.iter()
            .any(|e| match e {
                SessionEvent::DiffColoured { target, .. } => {
                    matches!(target, DiffTarget::Unstaged { path } if path == "src/b.rs")
                }
                _ => false,
            })
            .then_some(())
    })
    .await;

    let about_a = sink.count(|e| {
        let target = match e {
            SessionEvent::DiffLoaded { target, .. } | SessionEvent::DiffColoured { target, .. } => {
                target
            }
            _ => return false,
        };
        matches!(target, DiffTarget::Unstaged { path } if path == "src/a.rs")
    });
    assert_eq!(
        about_a, 0,
        "the abandoned read should have published neither rows nor colours"
    );

    session.close();
}

/// Asked and answered apart from the details: verifying may run gpg, which
/// the details pane cannot wait for.
#[tokio::test(flavor = "multi_thread")]
async fn a_signature_answer_names_the_commit_it_is_about() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "add f");
    let head = repo.git(&["rev-parse", "HEAD"]);

    let (sink, session) = opened(&repo).await;

    let oid = Oid::from_hex_str(&head).unwrap();
    session.check_signature(oid);
    sink.wait_for("SignatureChecked", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::SignatureChecked {
                oid: asked,
                signature,
            } => {
                assert_eq!(asked, &head, "the answer says which commit it is about");
                assert_eq!(signature.status, SignatureStatus::Absent);
                assert!(!signature.status.is_signed());
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}
