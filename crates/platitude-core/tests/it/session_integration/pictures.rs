//! A picture's blob side on its way to the pane: named by its object at
//! the diff read, read by the session when it is drawn.

use crate::support::TestRepo;
use crate::support::session::opened;
use platitude_core::details::DiffTarget;
use platitude_core::session::SessionEvent;

/// A picture's blob side has no path a URL could name, so the session
/// hands the pane its object, every read anew (`preview::Picture::Blob`),
/// whose bytes the session reads when it is drawn
/// (`RepoSession::picture_reader`) — a latest ask: the pane's next read
/// stops the reader the last one handed out, and the session's close stops
/// every one, whose read would stand in a worktree the session gave up.
#[tokio::test(flavor = "multi_thread")]
async fn a_pictures_blob_reaches_the_pane_as_its_object() {
    let mut repo = TestRepo::init();
    // The extension is all the preview asks; the bytes are anything git
    // calls binary.
    repo.write_file("logo.png", "\u{0}PNG-shaped bytes\n");
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "picture"]);
    repo.write_file("logo.png", "\u{0}PNG-shaped bytes, changed\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;

    /// The old side of every picture read so far, as it arrived.
    fn old_sides(events: &[SessionEvent]) -> Vec<platitude_core::preview::PreviewSide> {
        events
            .iter()
            .filter_map(|event| match event {
                SessionEvent::DiffLoaded {
                    preview: Some(preview),
                    ..
                } if preview.format.is_some() => preview.old.clone(),
                _ => None,
            })
            .collect()
    }

    /// Outside the wait: a panic in the predicate poisons the sink's lock.
    fn object_of(
        side: &platitude_core::preview::PreviewSide,
    ) -> platitude_core::preview::PictureBlob {
        match &side.picture {
            Some(platitude_core::preview::Picture::Blob(picture)) => picture.clone(),
            other => panic!(
                "the picture's old side arrived as {other:?} ({} bytes), not as its object",
                side.size
            ),
        }
    }

    /// What `reader` answers for `picture`; `None` where it dropped the
    /// read unanswered.
    async fn read(
        reader: &platitude_core::preview::PictureReader,
        picture: platitude_core::preview::PictureBlob,
    ) -> Option<Result<Vec<u8>, platitude_core::GitError>> {
        let (said, heard) = tokio::sync::oneshot::channel();
        reader.read(picture, move |read| {
            said.send(read).ok();
        });
        heard.await.ok()
    }

    /// What `reader` answers is a stop: cancelled, or dropped unanswered.
    fn stopped(answer: Option<&Result<Vec<u8>, platitude_core::GitError>>) -> bool {
        answer.is_none_or(|answer| answer.as_ref().is_err_and(|error| error.is_cancelled()))
    }

    /// The side's bytes and format, read by the reader the session hands
    /// out for the diff the pane now shows.
    async fn drawn(
        session: &platitude_core::session::RepoSession,
        side: &platitude_core::preview::PreviewSide,
    ) -> (Vec<u8>, Option<platitude_core::preview::ImageFormat>) {
        let reader = session.picture_reader().expect("the repository is open");
        let picture = object_of(side);
        let format = picture.format();
        let bytes = read(&reader, picture)
            .await
            .expect("an answer")
            .expect("the picture's object");
        (bytes, format)
    }

    let target = DiffTarget::Unstaged {
        path: "logo.png".to_string(),
    };
    let committed = (
        b"\0PNG-shaped bytes\n".to_vec(),
        Some(platitude_core::preview::ImageFormat::Png),
    );
    session.load_diff(target.clone());
    let first = sink
        .wait_for("the picture's diff", |events| old_sides(events).pop())
        .await;
    assert_eq!(drawn(&session, &first).await, committed);
    let passed = session.picture_reader().expect("the repository is open");

    // Read again: the object comes again, for the pane to hold under a new
    // name.
    session.load_diff(target);
    let second = sink
        .wait_for("the picture's second diff", |events| {
            let sides = old_sides(events);
            (sides.len() > 1).then(|| sides[sides.len() - 1].clone())
        })
        .await;
    let behind = read(&passed, object_of(&first)).await;
    assert!(
        stopped(behind.as_ref()),
        "the read the pane passed reads no picture: {behind:?}"
    );
    assert_eq!(drawn(&session, &second).await, committed);

    let last = session.picture_reader().expect("the repository is open");
    session.close_and_wait().await;
    let after = read(&last, object_of(&second)).await;
    assert!(
        stopped(after.as_ref()),
        "a closed session reads no picture: {after:?}"
    );
}
