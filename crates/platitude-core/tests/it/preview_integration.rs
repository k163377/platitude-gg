//! File previews (image files / binary sizes) against real git.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::Oid;
use platitude_core::details::{self, DiffTarget};
use platitude_core::preview::{self, ImageFormat, Picture};

/// A 1x1 RGBA PNG; its NUL bytes make git classify it as binary.
const TINY_PNG: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D', b'R',
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, b'I', b'D', b'A', b'T', 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, b'I', b'E', b'N', b'D', 0xAE,
    0x42, 0x60, 0x82,
];

/// A second distinct "image" (same header so it stays binary).
fn tiny_png_v2() -> Vec<u8> {
    let mut v = TINY_PNG.to_vec();
    v.extend_from_slice(&[0x00, 0xFF, 0x00, 0xFF]);
    v
}

fn write_bytes(repo: &TestRepo, rel: &str, bytes: &[u8]) {
    std::fs::write(repo.path.join(rel), bytes).unwrap();
}

/// A blob side's bytes, read from `repo` as when the picture is drawn, and
/// the format that goes with them.
async fn drawn(repo: &TestRepo, side: &preview::PreviewSide) -> (Vec<u8>, Option<ImageFormat>) {
    match side.picture.as_ref() {
        Some(Picture::Blob(picture)) => {
            let (executor, cancel) = env();
            let bytes = preview::read_picture(&executor, &repo.path, picture, &cancel)
                .await
                .expect("the picture's object");
            (bytes, picture.format())
        }
        other => panic!("a blob side names its object, not {other:?}"),
    }
}

/// The staged addition also stands for an unborn HEAD: `HEAD:<path>` not
/// resolving means "no old side" either way, and the probe answers without
/// a cat-file (`preview::blob_oid`; the log side is in `answer_reads.rs`).
#[tokio::test]
async fn a_side_that_is_not_there_previews_as_absent() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "root.png", TINY_PNG);
    repo.git(&["add", "--", "root.png"]);
    repo.git(&["commit", "-m", "root"]);
    let root = repo.git(&["rev-parse", "HEAD"]);
    write_bytes(&repo, "doomed.png", TINY_PNG);
    repo.git(&["add", "--", "doomed.png"]);
    repo.git(&["commit", "-m", "one to delete"]);
    repo.git(&["rm", "--", "doomed.png"]);
    std::fs::create_dir_all(repo.path.join("art")).unwrap();
    write_bytes(&repo, "art/new.png", TINY_PNG);
    repo.git(&["add", "--", "art/new.png"]);
    write_bytes(&repo, "stray.png", TINY_PNG);

    let (executor, cancel) = env();

    // Untracked: new side only, the working-tree file itself; its
    // all-additions text diff is still flagged binary.
    let target = DiffTarget::Untracked {
        path: "stray.png".to_string(),
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(patches[0].is_binary);
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert_eq!(p.format, Some(ImageFormat::Png));
    assert!(p.old.is_none());
    let new = p.new.unwrap();
    assert_eq!(new.size, TINY_PNG.len() as u64);
    assert_eq!(
        new.picture,
        Some(Picture::File(repo.path.join("stray.png")))
    );

    // Staged addition: the index blob is named by its object, with the
    // format the path it is at names.
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "art/new.png".to_string(),
            orig_path: None,
        },
        true,
        &cancel,
    )
    .await
    .unwrap();
    assert!(p.old.is_none(), "no HEAD side for a newly added file");
    let new = p.new.unwrap();
    assert_eq!(
        drawn(&repo, &new).await,
        (TINY_PNG.to_vec(), Some(ImageFormat::Png))
    );
    assert_eq!(new.size, TINY_PNG.len() as u64);

    // Staged deletion: old side only.
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "doomed.png".to_string(),
            orig_path: None,
        },
        true,
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(drawn(&repo, &p.old.unwrap()).await.0, TINY_PNG);
    assert!(p.new.is_none(), "deleted from the index");

    // Root commit: no parent, so no old side.
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Commit {
            oid: Oid::from_hex_str(&root).unwrap(),
            parent: None,
            path: "root.png".to_string(),
            orig_path: None,
        },
        true,
        &cancel,
    )
    .await
    .unwrap();
    assert!(p.old.is_none());
    assert!(p.new.is_some());
}

#[tokio::test]
async fn modified_image_previews_the_index_blob_and_the_tree_file() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "add image"]);
    let v2 = tiny_png_v2();
    write_bytes(&repo, "logo.png", &v2);

    let (executor, cancel) = env();
    let target = DiffTarget::Unstaged {
        path: "logo.png".to_string(),
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    let old = p.old.unwrap();
    assert_eq!(
        drawn(&repo, &old).await,
        (TINY_PNG.to_vec(), Some(ImageFormat::Png)),
        "the index side is read out"
    );
    let new = p.new.unwrap();
    assert_eq!(
        new.picture,
        Some(Picture::File(repo.path.join("logo.png"))),
        "the working-tree side is the file itself, copied nowhere"
    );
    assert_eq!(new.size, v2.len() as u64);
}

#[tokio::test]
async fn committed_image_previews_parent_and_commit_blobs() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "v1"]);
    let v2 = tiny_png_v2();
    write_bytes(&repo, "logo.png", &v2);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "v2"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let (executor, cancel) = env();
    let target = DiffTarget::Commit {
        oid: Oid::from_hex_str(&head).unwrap(),
        parent: Some(Oid::from_hex_str(&parent).unwrap()),
        path: "logo.png".to_string(),
        orig_path: None,
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    let old = p.old.unwrap();
    let new = p.new.unwrap();
    assert_eq!(drawn(&repo, &old).await.0, TINY_PNG);
    assert_eq!(drawn(&repo, &new).await.0, v2.as_slice());
    assert_eq!(old.size, TINY_PNG.len() as u64);
    assert_eq!(new.size, v2.len() as u64);
}

/// A probe that wrongly says no raises nothing — the rows just arrive
/// uncoloured — so the present side is pinned as the missing one is in
/// `answer_reads.rs` (`preview::source_text`).
#[tokio::test]
async fn source_text_reads_the_side_the_commit_has() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.rs", "fn main() {}\n", "add");
    repo.commit_file("a.rs", "fn main() {\n    work();\n}\n", "change");
    let head = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let (executor, cancel) = env();
    let source = preview::source_text(
        &executor,
        &repo.path,
        &DiffTarget::Commit {
            oid: Oid::from_hex_str(&head).unwrap(),
            parent: Some(Oid::from_hex_str(&parent).unwrap()),
            path: "a.rs".to_string(),
            orig_path: None,
        },
        &cancel,
    )
    .await;
    assert_eq!(source.as_deref(), Some("fn main() {\n    work();\n}\n"));
}

/// A Rust file twice [`preview::SOURCE_BYTE_CAP`]: where the read stops,
/// git is still writing the rest into a full pipe, so the read's own stop
/// ends the row. Just over the cap, the chunk that passes it is git's last:
/// a reader behind (a loaded machine) reads on to git's exit in the same
/// turn, and the row ends `Exited(0)` — the answer still too big.
fn over_the_cap() -> String {
    let line = "let x = 1;\n";
    line.repeat(2 * usize::try_from(preview::SOURCE_BYTE_CAP).unwrap() / line.len())
}

/// A blob over the cap is not read to its end: the read stops where it
/// passes the cap (the row ends `Cancelled`, not on git's exit), and the
/// other side is not read in its place.
#[tokio::test]
async fn a_blob_over_the_cap_is_not_read_to_its_end() {
    let mut repo = TestRepo::init();
    repo.commit_file("big.rs", "fn main() {}\n", "small");
    repo.commit_file("big.rs", &over_the_cap(), "big");
    let head = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let (executor, log, cancel) = crate::support::exec::logged();
    let source = preview::source_text(
        &executor,
        &repo.path,
        &DiffTarget::Commit {
            oid: Oid::from_hex_str(&head).unwrap(),
            parent: Some(Oid::from_hex_str(&parent).unwrap()),
            path: "big.rs".to_string(),
            orig_path: None,
        },
        &cancel,
    )
    .await;
    assert_eq!(source, None, "no colours are read against it");
    assert_eq!(
        log.ends_of(&["cat-file", "blob"]),
        [platitude_core::process::CommandEnd::Cancelled],
        "one read, stopped at the cap"
    );
    assert_eq!(
        log.ends_of(&["rev-parse", "--verify"]).len(),
        1,
        "the older side is not asked for"
    );
}

/// A working file over the cap is the side the diff shows: the index's
/// copy is a different file, and colouring by it would colour the rows
/// against text they are not.
#[tokio::test]
async fn a_working_file_over_the_cap_reads_no_other_side() {
    let mut repo = TestRepo::init();
    repo.commit_file("big.rs", "fn main() {}\n", "small");
    repo.write_file("big.rs", &over_the_cap());

    let (executor, log, cancel) = crate::support::exec::logged();
    let source = preview::source_text(
        &executor,
        &repo.path,
        &DiffTarget::Unstaged {
            path: "big.rs".to_string(),
        },
        &cancel,
    )
    .await;
    assert_eq!(source, None);
    assert!(
        log.ends_of(&["git"]).is_empty(),
        "nothing was read from git: {:?}",
        log.0.lock().unwrap()
    );
}

/// The old side's bytes come from the path it had, and that path names
/// their format: a rename that changed the extension names the old side's
/// decoder by the old name, and one from a path that names no picture's
/// format leaves the bytes to say.
#[tokio::test]
async fn renamed_image_reads_the_old_side_from_orig_path() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "a.JPG", TINY_PNG);
    write_bytes(&repo, "c.dat", TINY_PNG);
    repo.git(&["add", "--", "a.JPG", "c.dat"]);
    repo.git(&["commit", "-m", "add"]);
    repo.git(&["mv", "a.JPG", "b.png"]);
    repo.git(&["mv", "c.dat", "d.png"]);
    repo.git(&["commit", "-m", "rename"]);
    let head = Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    let parent = Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD^"])).unwrap();

    let (executor, cancel) = env();
    let renamed = |from: &str, to: &str| DiffTarget::Commit {
        oid: head,
        parent: Some(parent),
        path: to.to_string(),
        orig_path: Some(from.to_string()),
    };
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &renamed("a.JPG", "b.png"),
        true,
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(
        drawn(&repo, &p.old.unwrap()).await,
        (TINY_PNG.to_vec(), Some(ImageFormat::Jpeg))
    );
    assert_eq!(
        drawn(&repo, &p.new.unwrap()).await,
        (TINY_PNG.to_vec(), Some(ImageFormat::Png))
    );

    let p = preview::file_preview(
        &executor,
        &repo.path,
        &renamed("c.dat", "d.png"),
        true,
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(
        drawn(&repo, &p.old.unwrap()).await,
        (TINY_PNG.to_vec(), None)
    );
}

#[tokio::test]
async fn non_image_binary_reports_sizes_without_pictures() {
    let mut repo = TestRepo::init();
    let old_bytes = [0u8, 1, 2, 3, 4];
    write_bytes(&repo, "data.bin", &old_bytes);
    repo.git(&["add", "--", "data.bin"]);
    repo.git(&["commit", "-m", "add data"]);
    let new_bytes = [0u8, 1, 2, 3, 4, 5, 6, 7];
    write_bytes(&repo, "data.bin", &new_bytes);

    let (executor, log, cancel) = crate::support::exec::logged();
    let target = DiffTarget::Unstaged {
        path: "data.bin".to_string(),
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(patches[0].is_binary);

    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert_eq!(p.format, None);
    let old = p.old.unwrap();
    let new = p.new.unwrap();
    assert_eq!(old.size, old_bytes.len() as u64);
    assert_eq!(new.size, new_bytes.len() as u64);
    assert!(old.picture.is_none(), "non-images carry sizes only");
    assert!(new.picture.is_none());
    assert!(
        log.ends_of(&["cat-file", "blob"]).is_empty(),
        "a blob that is no picture is sized, never read"
    );
}

#[tokio::test]
async fn svg_gets_an_image_preview_alongside_its_text_diff() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n";
    repo.write_file("icon.svg", svg);
    repo.git(&["add", "--", "icon.svg"]);

    let (executor, cancel) = env();
    let target = DiffTarget::Staged {
        path: "icon.svg".to_string(),
        orig_path: None,
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(!patches[0].is_binary);
    let p = preview::file_preview(&executor, &repo.path, &target, false, &cancel)
        .await
        .unwrap();
    assert_eq!(p.format, Some(ImageFormat::Svg));
    assert_eq!(
        drawn(&repo, &p.new.unwrap()).await,
        (svg.as_bytes().to_vec(), Some(ImageFormat::Svg))
    );
}

/// No cap on either side; a blob arrives whole, byte for byte, across
/// every chunk the pipe hands over.
#[tokio::test]
async fn a_large_image_is_handed_over_whole() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    // Large and in the tree only: no git object, the file is the preview.
    let huge_len = 17 * 1024 * 1024 + 1;
    let big = vec![0u8; huge_len];
    write_bytes(&repo, "huge.png", &big);
    // A blob big enough that `cat-file` streams it over many chunks.
    let staged: Vec<u8> = (0..3 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
    write_bytes(&repo, "staged.png", &staged);
    repo.git(&["add", "--", "staged.png"]);

    let (executor, cancel) = env();
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Untracked {
            path: "huge.png".to_string(),
        },
        true,
        &cancel,
    )
    .await
    .unwrap();
    let new = p.new.unwrap();
    assert_eq!(new.size, huge_len as u64);
    assert_eq!(new.picture, Some(Picture::File(repo.path.join("huge.png"))));

    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "staged.png".to_string(),
            orig_path: None,
        },
        true,
        &cancel,
    )
    .await
    .unwrap();
    let new = p.new.unwrap();
    assert_eq!(new.size, staged.len() as u64);
    assert_eq!(drawn(&repo, &new).await.0, staged.as_slice());
}

/// A blob side is its object: the index moving between the read and the
/// draw leaves the picture the diff was read against.
#[tokio::test]
async fn a_blob_side_draws_the_object_the_diff_was_read_against() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "v1"]);
    write_bytes(&repo, "logo.png", &tiny_png_v2());

    let (executor, cancel) = env();
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Unstaged {
            path: "logo.png".to_string(),
        },
        true,
        &cancel,
    )
    .await
    .unwrap();
    repo.git(&["add", "--", "logo.png"]);
    assert_eq!(drawn(&repo, &p.old.unwrap()).await.0, TINY_PNG);
}

/// A streamed picture tells how its read ended before its chunks stop: a
/// receiver that sees the channel close already knows whether it saw the
/// whole picture (`PictureReader::stream`).
#[tokio::test(flavor = "multi_thread")]
async fn a_streamed_picture_is_told_whole_before_its_chunks_stop() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    let oid = repo.git(&["hash-object", "-w", "logo.png"]);
    let picture = preview::PictureBlob::new(
        Oid::from_hex_str(&oid).expect("an object id"),
        TINY_PNG.len() as u64,
        "logo.png",
    );
    let (executor, cancel) = env();
    let reader = preview::PictureReader::new(
        executor,
        repo.path.clone(),
        tokio::runtime::Handle::current(),
        cancel,
    );
    let (to, mut chunks) = tokio::sync::mpsc::channel(4);
    let open = to.downgrade();
    let (said, heard) = tokio::sync::oneshot::channel();
    reader.stream(picture, to, move |streamed| {
        // A sender still standing is the channel still open.
        said.send((streamed.is_ok(), open.upgrade().is_some())).ok();
    });
    let mut bytes = Vec::new();
    while let Some(chunk) = chunks.recv().await {
        bytes.extend_from_slice(&chunk);
    }
    let (whole, open) = heard.await.expect("the read's end told");
    assert!(whole, "the read ended whole");
    assert!(open, "told while its chunks could still come");
    assert_eq!(bytes, TINY_PNG);
}
