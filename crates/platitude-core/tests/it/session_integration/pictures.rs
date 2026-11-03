//! A picture's blob side on its way to the pane.

use crate::support::TestRepo;
use crate::support::session::opened;
use platitude_core::details::DiffTarget;
use platitude_core::session::SessionEvent;

/// A picture's blob side has no path a URL could name, so the session
/// hands the pane its bytes, every read anew (`preview::Picture::Bytes`).
#[tokio::test(flavor = "multi_thread")]
async fn a_pictures_blob_reaches_the_pane_as_its_bytes() {
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
    fn bytes_of(
        side: &platitude_core::preview::PreviewSide,
    ) -> (Vec<u8>, Option<platitude_core::preview::ImageFormat>) {
        match &side.picture {
            Some(platitude_core::preview::Picture::Bytes(picture)) => {
                (picture.bytes().to_vec(), picture.format())
            }
            other => panic!(
                "the picture's old side arrived as {other:?} ({} bytes), not as its bytes",
                side.size
            ),
        }
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
    assert_eq!(bytes_of(&first), committed);

    // Read again: the bytes come again, for the pane to hold under a new
    // name.
    session.load_diff(target);
    let second = sink
        .wait_for("the picture's second diff", |events| {
            let sides = old_sides(events);
            (sides.len() > 1).then(|| sides[sides.len() - 1].clone())
        })
        .await;
    assert_eq!(bytes_of(&second), committed);
    session.close_and_wait().await;
}
