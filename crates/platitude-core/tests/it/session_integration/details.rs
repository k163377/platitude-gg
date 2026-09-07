//! What the session answers about one commit: its details, its diff, the
//! colours over that diff, and its signature.

use crate::support::TestRepo;
use crate::support::session::opened;
use platitude_core::Oid;
use platitude_core::details::DiffTarget;
use platitude_core::identity::SignatureStatus;
use platitude_core::session::{DiffReadOutcome, SessionEvent};

#[tokio::test(flavor = "multi_thread")]
async fn details_and_diff_round_trip_through_the_session() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\n", "add f");
    repo.write_file("f.txt", "one\ntwo changed\n");
    repo.git(&["commit", "-am", "edit f"]);
    let head = repo.git(&["rev-parse", "HEAD"]);

    let (sink, session) = opened(&repo).await;

    let oid = Oid::from_hex_str(&head).unwrap();
    let task = session.load_details(oid).expect("open session");
    sink.wait_for("DetailsLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::DetailsLoaded { details, .. } => {
                assert_eq!(details.oid, oid);
                assert_eq!(details.message, "edit f");
                assert_eq!(details.files.len(), 1);
                assert_eq!(details.files[0].path, "f.txt");
                Some(())
            }
            _ => None,
        })
    })
    .await;
    assert_eq!(
        crate::support::wait::bounded("details completion", task.outcome()).await,
        platitude_core::session::DetailsOutcome::Sent
    );

    session.load_diff(DiffTarget::Commit {
        oid,
        parent: Some(Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD^"])).unwrap()),
        path: "f.txt".to_string(),
        orig_path: None,
    });
    sink.wait_for("DiffLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::DiffLoaded { patches, .. } => {
                assert_eq!(patches.len(), 1);
                assert!(!patches[0].hunks.is_empty());
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

/// Colours arrive behind the rows they belong to, not with them.
///
/// The order is the whole of it: a diff that waits for its colours is a
/// diff that shows nothing for as long as the colouring takes, which on
/// a file of any size is hundreds of milliseconds
/// (`SessionEvent::DiffColoured`).
#[tokio::test(flavor = "multi_thread")]
async fn a_diff_arrives_before_the_colours_for_it() {
    let mut repo = TestRepo::init();
    // A language the set has rules for, or there would be nothing to say
    // about the lines and no second event at all.
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
        // And the colours have to be about something: an empty answer
        // would pass the ordering above while saying nothing at all.
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

/// A read nobody is waiting for any more hands over nothing.
///
/// Walking down a commit's file list starts a read per row. Colouring is
/// the expensive half — without the epoch check, a colouring per
/// abandoned row is left running behind the reader — and the rows are the
/// cheap half, but they carry the fingerprint the next partial stage is
/// refused against (`RepoSession::diff_epoch`), so an abandoned read that
/// lands last would hand the pane one taken from a file it has left.
#[tokio::test(flavor = "multi_thread")]
async fn a_read_the_reader_has_left_hands_over_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("src/a.rs", "fn a() -> u32 {\n    1\n}\n", "add a");
    repo.commit_file("src/b.rs", "fn b() -> u32 {\n    1\n}\n", "add b");
    repo.write_file("src/a.rs", "fn a() -> u32 {\n    2\n}\n");
    repo.write_file("src/b.rs", "fn b() -> u32 {\n    2\n}\n");

    let (sink, session) = opened(&repo).await;

    // The first click, held where the race is: polled once, so it runs to
    // the point where it waits on its own git and stops there. The second
    // click is then asked for while the first is demonstrably still in
    // flight, rather than while it probably is
    // (core.md §非同期・並行テスト).
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

    // Carried on from where it stopped, it finds itself passed and ends
    // there. That is the completion boundary the counts below are read
    // against: nothing of this read is still on its way.
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

/// The signature question is asked and answered on its own, apart from
/// the details it belongs beside: verifying may run gpg, and the details
/// pane cannot wait for that.
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
