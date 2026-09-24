//! File previews (image files / binary sizes) against real git.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::Oid;
use platitude_core::details::{self, DiffTarget};
use platitude_core::preview::{self, PreviewFiles};

/// A tiny valid PNG (1x1 RGBA). Contains NUL bytes, so git classifies the
/// file as binary; the exact pixels are irrelevant here.
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

/// Somewhere of this test's own for the blob sides to be written to. The
/// temp dir has to outlive the files — dropping them removes their own
/// directory, and the assertions below read what is left.
fn files() -> (tempfile::TempDir, PreviewFiles) {
    let dir = tempfile::tempdir().unwrap();
    let files = PreviewFiles::at(dir.path().join("s"));
    (dir, files)
}

/// What a side's file holds, read back off the disk the pane would read
/// it from.
fn bytes_of(side: &preview::PreviewSide) -> Vec<u8> {
    std::fs::read(side.file.as_ref().expect("a file to read")).unwrap()
}

/// One repository, four targets where one side simply is not there: an
/// untracked file, a staged addition (no HEAD side), a staged deletion
/// (no index side), and a root commit (no parent).
///
/// The staged addition also stands for the unborn-HEAD shape:
/// `HEAD:<path>` not resolving means "no old side" whether the path is
/// missing from HEAD or HEAD does not exist yet — the probe answers and
/// no cat-file runs (`preview::blob_is_there`; what the log then holds is
/// pinned in `answer_reads.rs`).
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
    let (_dir, files) = files();

    // Untracked: new side only, and it is the working-tree file itself —
    // and the all-additions text diff of an untracked binary is flagged
    // binary.
    let target = DiffTarget::Untracked {
        path: "stray.png".to_string(),
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(patches[0].is_binary);
    let p = preview::file_preview(&executor, &repo.path, &target, true, files.read(1), &cancel)
        .await
        .unwrap();
    assert_eq!(p.image_mime, Some("image/png"));
    assert!(p.old.is_none());
    let new = p.new.unwrap();
    assert_eq!(new.size, TINY_PNG.len() as u64);
    assert_eq!(
        new.file.as_deref(),
        Some(repo.path.join("stray.png").as_path())
    );

    // Staged addition: the index blob is written out; there is no HEAD
    // side.
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "art/new.png".to_string(),
            orig_path: None,
        },
        true,
        files.read(2),
        &cancel,
    )
    .await
    .unwrap();
    assert!(p.old.is_none(), "no HEAD side for a newly added file");
    let new = p.new.unwrap();
    assert_eq!(bytes_of(&new), TINY_PNG);
    assert_eq!(new.size, TINY_PNG.len() as u64);
    assert!(
        new.file.as_ref().unwrap().ends_with("2-new.png"),
        "named by the read and the side, with the blob's own extension"
    );

    // Staged deletion: old side only.
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "doomed.png".to_string(),
            orig_path: None,
        },
        true,
        files.read(3),
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(bytes_of(&p.old.unwrap()), TINY_PNG);
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
        files.read(4),
        &cancel,
    )
    .await
    .unwrap();
    assert!(p.old.is_none());
    assert!(p.new.is_some());
}

#[tokio::test]
async fn modified_image_previews_the_index_file_and_the_tree_file() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "add image"]);
    let v2 = tiny_png_v2();
    write_bytes(&repo, "logo.png", &v2);

    let (executor, cancel) = env();
    let (_dir, files) = files();
    let target = DiffTarget::Unstaged {
        path: "logo.png".to_string(),
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, files.read(1), &cancel)
        .await
        .unwrap();
    let old = p.old.unwrap();
    assert_eq!(bytes_of(&old), TINY_PNG, "the index side is written out");
    assert!(old.file.as_ref().unwrap().ends_with("1-old.png"));
    let new = p.new.unwrap();
    assert_eq!(
        new.file.as_deref(),
        Some(repo.path.join("logo.png").as_path()),
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
    let (_dir, files) = files();
    let target = DiffTarget::Commit {
        oid: Oid::from_hex_str(&head).unwrap(),
        parent: Some(Oid::from_hex_str(&parent).unwrap()),
        path: "logo.png".to_string(),
        orig_path: None,
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, files.read(5), &cancel)
        .await
        .unwrap();
    let old = p.old.unwrap();
    let new = p.new.unwrap();
    assert_eq!(bytes_of(&old), TINY_PNG);
    assert_eq!(bytes_of(&new), v2);
    assert_eq!(old.size, TINY_PNG.len() as u64);
    assert_eq!(new.size, v2.len() as u64);
    assert_ne!(old.file, new.file, "two sides, two files");
}

/// The colours are read against the side the commit has, and the probe in
/// front of that read has to say yes to it. Nothing announces a probe that
/// wrongly says no — the rows arrive uncoloured and no error is raised —
/// so the side that is there is pinned as tightly as the side that is not
/// (`preview::source_text`, and `answer_reads.rs` for the missing side).
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

#[tokio::test]
async fn renamed_image_reads_the_old_side_from_orig_path() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "a.png", TINY_PNG);
    repo.git(&["add", "--", "a.png"]);
    repo.git(&["commit", "-m", "add"]);
    repo.git(&["mv", "a.png", "b.png"]);
    repo.git(&["commit", "-m", "rename"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let (executor, cancel) = env();
    let (_dir, files) = files();
    let target = DiffTarget::Commit {
        oid: Oid::from_hex_str(&head).unwrap(),
        parent: Some(Oid::from_hex_str(&parent).unwrap()),
        path: "b.png".to_string(),
        orig_path: Some("a.png".to_string()),
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, files.read(1), &cancel)
        .await
        .unwrap();
    assert_eq!(bytes_of(&p.old.unwrap()), TINY_PNG);
    assert_eq!(bytes_of(&p.new.unwrap()), TINY_PNG);
}

#[tokio::test]
async fn non_image_binary_reports_sizes_without_files() {
    let mut repo = TestRepo::init();
    let old_bytes = [0u8, 1, 2, 3, 4];
    write_bytes(&repo, "blob.bin", &old_bytes);
    repo.git(&["add", "--", "blob.bin"]);
    repo.git(&["commit", "-m", "add blob"]);
    let new_bytes = [0u8, 1, 2, 3, 4, 5, 6, 7];
    write_bytes(&repo, "blob.bin", &new_bytes);

    let (executor, cancel) = env();
    let (dir, files) = files();
    let target = DiffTarget::Unstaged {
        path: "blob.bin".to_string(),
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(patches[0].is_binary);

    let p = preview::file_preview(&executor, &repo.path, &target, true, files.read(1), &cancel)
        .await
        .unwrap();
    assert_eq!(p.image_mime, None);
    let old = p.old.unwrap();
    let new = p.new.unwrap();
    assert_eq!(old.size, old_bytes.len() as u64);
    assert_eq!(new.size, new_bytes.len() as u64);
    assert!(old.file.is_none(), "non-images carry sizes only");
    assert!(new.file.is_none());
    assert!(
        old.unwritten.is_none() && new.unwritten.is_none(),
        "nothing was asked for, so nothing failed"
    );
    assert!(
        !dir.path().join("s").exists(),
        "nothing was written, so nothing was made to write into"
    );
}

/// A picture whose blob cannot be written out still reports its size —
/// and says what stopped it, which is the only place that survives. Told
/// apart from a side no file was ever wanted from
/// (`non_image_binary_reports_sizes_without_files`) by exactly this: the
/// two look the same otherwise, and a pane quietly missing its picture
/// is what a reader is left to diagnose.
#[tokio::test]
async fn a_side_that_could_not_be_written_says_what_stopped_it() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "v1"]);
    write_bytes(&repo, "logo.png", &tiny_png_v2());

    // A file where the session's directory would go: nothing can be made
    // under it, whatever the platform calls that.
    let dir = tempfile::tempdir().unwrap();
    let wall = dir.path().join("not-a-directory");
    std::fs::write(&wall, b"x").unwrap();
    let files = PreviewFiles::at(wall.join("s"));

    let (executor, cancel) = env();
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Unstaged {
            path: "logo.png".to_string(),
        },
        true,
        files.read(1),
        &cancel,
    )
    .await
    .unwrap();
    let old = p.old.unwrap();
    assert_eq!(old.size, TINY_PNG.len() as u64, "the size still arrives");
    assert!(old.file.is_none());
    let why = old.unwritten.expect("what stopped the write");
    assert!(
        why.contains("could not be made") && why.contains(&wall.join("s").display().to_string()),
        "the step and the path it fell over on: {why}"
    );
    // The working-tree side is the file itself, so nothing was written
    // for it and there is nothing to explain.
    let new = p.new.unwrap();
    assert!(new.file.is_some());
    assert!(new.unwritten.is_none());
}

#[tokio::test]
async fn svg_gets_an_image_preview_alongside_its_text_diff() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n";
    repo.write_file("icon.svg", svg);

    let (executor, cancel) = env();
    let (_dir, files) = files();
    let target = DiffTarget::Untracked {
        path: "icon.svg".to_string(),
    };
    // Text diff exists (SVG is text) …
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(!patches[0].is_binary);
    // … and the preview is still offered for rendering.
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &target,
        false,
        files.read(1),
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(p.image_mime, Some("image/svg+xml"));
    assert_eq!(bytes_of(&p.new.unwrap()), svg.as_bytes());
}

/// No cap on either side: a picture past what a data URL could have
/// carried is the working-tree file on one side and a blob streamed out
/// whole on the other, byte for byte, across every chunk the pipe hands
/// over.
#[tokio::test]
async fn a_large_image_is_handed_over_whole() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    // Well past what a data URL would carry, in the tree only: no git
    // object, and the file is the preview.
    let huge_len = 17 * 1024 * 1024 + 1;
    let big = vec![0u8; huge_len];
    write_bytes(&repo, "huge.png", &big);
    // And a blob of some size, so the stream out of `cat-file` runs over
    // many chunks: what lands has to be exactly what went in.
    let staged: Vec<u8> = (0..3 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
    write_bytes(&repo, "staged.png", &staged);
    repo.git(&["add", "--", "staged.png"]);

    let (executor, cancel) = env();
    let (_dir, files) = files();
    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Untracked {
            path: "huge.png".to_string(),
        },
        true,
        files.read(1),
        &cancel,
    )
    .await
    .unwrap();
    let new = p.new.unwrap();
    assert_eq!(new.size, huge_len as u64);
    assert_eq!(
        new.file.as_deref(),
        Some(repo.path.join("huge.png").as_path())
    );

    let p = preview::file_preview(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "staged.png".to_string(),
            orig_path: None,
        },
        true,
        files.read(2),
        &cancel,
    )
    .await
    .unwrap();
    let new = p.new.unwrap();
    assert_eq!(new.size, staged.len() as u64);
    assert_eq!(bytes_of(&new), staged);
}

/// The files of one read go when the next read is published, and the
/// rest when the pane closes — and a read whose target is no picture at
/// all still sweeps, since the pane has moved on all the same.
#[tokio::test]
async fn the_next_read_sweeps_the_files_of_the_one_before_it() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "v1"]);
    write_bytes(&repo, "logo.png", &tiny_png_v2());
    repo.write_file("notes.txt", "a\n");

    let (executor, cancel) = env();
    let (_dir, files) = files();
    let picture = DiffTarget::Unstaged {
        path: "logo.png".to_string(),
    };
    let first = preview::file_preview(
        &executor,
        &repo.path,
        &picture,
        true,
        files.read(1),
        &cancel,
    )
    .await
    .unwrap();
    let first_old = first.old.unwrap().file.unwrap();
    assert!(first_old.exists());

    // The same file read again: the pane is handed a new file under a
    // new name, and the one before it goes.
    let second = preview::file_preview(
        &executor,
        &repo.path,
        &picture,
        true,
        files.read(2),
        &cancel,
    )
    .await
    .unwrap();
    files.sweep_before(2);
    let second_old = second.old.unwrap().file.unwrap();
    assert_ne!(
        first_old, second_old,
        "a re-read is a new file, so a new URL"
    );
    assert!(!first_old.exists(), "swept by the read after it");
    assert!(second_old.exists());

    // A read of something no picture stands in for sweeps just the same.
    let text = DiffTarget::Untracked {
        path: "notes.txt".to_string(),
    };
    let none =
        preview::file_preview(&executor, &repo.path, &text, false, files.read(3), &cancel).await;
    assert!(none.is_none());
    files.sweep_before(3);
    assert!(!second_old.exists());

    // And the pane closing takes whatever the last read left.
    let third = preview::file_preview(
        &executor,
        &repo.path,
        &picture,
        true,
        files.read(4),
        &cancel,
    )
    .await
    .unwrap();
    let third_old = third.old.unwrap().file.unwrap();
    assert!(third_old.exists());
    files.release();
    assert!(!third_old.exists());
    assert!(
        Path::new(&third_old).parent().unwrap().exists(),
        "the directory stays for the next read; the close removes it"
    );
}
