//! File previews (image bytes / binary sizes) against real git.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::Oid;
use platitude_core::details::{self, DiffTarget};
use platitude_core::preview::{self, IMAGE_BYTE_CAP};

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

#[tokio::test]
async fn untracked_image_previews_the_new_side_only() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    write_bytes(&repo, "logo.png", TINY_PNG);

    let (executor, cancel) = env();
    let target = DiffTarget::Untracked {
        path: "logo.png".to_string(),
    };
    // The all-additions diff of an untracked binary is flagged binary.
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(patches[0].is_binary);

    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert_eq!(p.image_mime, Some("image/png"));
    assert!(p.old.is_none());
    let new = p.new.unwrap();
    assert_eq!(new.size, TINY_PNG.len() as u64);
    assert_eq!(new.bytes.as_deref(), Some(TINY_PNG));
}

/// Also the unborn-HEAD shape: `HEAD:<path>` failing to resolve means "no
/// old side" whether the path is missing from HEAD or HEAD does not exist
/// yet — one cat-file, one answer (`preview::blob_side`).
#[tokio::test]
async fn staged_new_image_reads_the_index_blob() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    std::fs::create_dir_all(repo.path.join("art")).unwrap();
    write_bytes(&repo, "art/logo.png", TINY_PNG);
    repo.git(&["add", "--", "art/logo.png"]);

    let (executor, cancel) = env();
    let target = DiffTarget::Staged {
        path: "art/logo.png".to_string(),
        orig_path: None,
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert!(p.old.is_none(), "no HEAD side for a newly added file");
    assert_eq!(p.new.unwrap().bytes.as_deref(), Some(TINY_PNG));
}

#[tokio::test]
async fn modified_image_previews_both_sides() {
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
    assert_eq!(p.old.unwrap().bytes.as_deref(), Some(TINY_PNG));
    assert_eq!(p.new.unwrap().bytes.as_deref(), Some(v2.as_slice()));
}

#[tokio::test]
async fn staged_deletion_previews_the_old_side_only() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "add image"]);
    repo.git(&["rm", "--", "logo.png"]);

    let (executor, cancel) = env();
    let target = DiffTarget::Staged {
        path: "logo.png".to_string(),
        orig_path: None,
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert_eq!(p.old.unwrap().bytes.as_deref(), Some(TINY_PNG));
    assert!(p.new.is_none(), "deleted from the index");
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
    assert_eq!(p.old.unwrap().bytes.as_deref(), Some(TINY_PNG));
    assert_eq!(p.new.unwrap().bytes.as_deref(), Some(v2.as_slice()));
}

#[tokio::test]
async fn root_commit_image_has_no_old_side() {
    let mut repo = TestRepo::init();
    write_bytes(&repo, "logo.png", TINY_PNG);
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "root"]);
    let head = repo.git(&["rev-parse", "HEAD"]);

    let (executor, cancel) = env();
    let target = DiffTarget::Commit {
        oid: Oid::from_hex_str(&head).unwrap(),
        parent: None,
        path: "logo.png".to_string(),
        orig_path: None,
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert!(p.old.is_none());
    assert!(p.new.is_some());
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
    let target = DiffTarget::Commit {
        oid: Oid::from_hex_str(&head).unwrap(),
        parent: Some(Oid::from_hex_str(&parent).unwrap()),
        path: "b.png".to_string(),
        orig_path: Some("a.png".to_string()),
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert_eq!(p.old.unwrap().bytes.as_deref(), Some(TINY_PNG));
    assert_eq!(p.new.unwrap().bytes.as_deref(), Some(TINY_PNG));
}

#[tokio::test]
async fn non_image_binary_reports_sizes_without_bytes() {
    let mut repo = TestRepo::init();
    let old_bytes = [0u8, 1, 2, 3, 4];
    write_bytes(&repo, "blob.bin", &old_bytes);
    repo.git(&["add", "--", "blob.bin"]);
    repo.git(&["commit", "-m", "add blob"]);
    let new_bytes = [0u8, 1, 2, 3, 4, 5, 6, 7];
    write_bytes(&repo, "blob.bin", &new_bytes);

    let (executor, cancel) = env();
    let target = DiffTarget::Unstaged {
        path: "blob.bin".to_string(),
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(patches[0].is_binary);

    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    assert_eq!(p.image_mime, None);
    let old = p.old.unwrap();
    let new = p.new.unwrap();
    assert_eq!(old.size, old_bytes.len() as u64);
    assert_eq!(new.size, new_bytes.len() as u64);
    assert!(old.bytes.is_none(), "non-images carry sizes only");
    assert!(new.bytes.is_none());
}

#[tokio::test]
async fn plain_text_has_no_preview() {
    let mut repo = TestRepo::init();
    repo.commit_file("notes.txt", "a\n", "base");
    repo.write_file("notes.txt", "b\n");

    let (executor, cancel) = env();
    let target = DiffTarget::Unstaged {
        path: "notes.txt".to_string(),
    };
    let p = preview::file_preview(&executor, &repo.path, &target, false, &cancel).await;
    assert!(p.is_none());
}

#[tokio::test]
async fn svg_gets_an_image_preview_alongside_its_text_diff() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n";
    repo.write_file("icon.svg", svg);

    let (executor, cancel) = env();
    let target = DiffTarget::Untracked {
        path: "icon.svg".to_string(),
    };
    // Text diff exists (SVG is text) …
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert!(!patches[0].is_binary);
    // … and the preview is still offered for rendering.
    let p = preview::file_preview(&executor, &repo.path, &target, false, &cancel)
        .await
        .unwrap();
    assert_eq!(p.image_mime, Some("image/svg+xml"));
    assert_eq!(p.new.unwrap().bytes.as_deref(), Some(svg.as_bytes()));
}

#[tokio::test]
async fn oversized_image_reports_size_only() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    // Sparse-ish: one write, no git object involved (untracked target).
    let big = vec![0u8; (IMAGE_BYTE_CAP + 1) as usize];
    write_bytes(&repo, "huge.png", &big);

    let (executor, cancel) = env();
    let target = DiffTarget::Untracked {
        path: "huge.png".to_string(),
    };
    let p = preview::file_preview(&executor, &repo.path, &target, true, &cancel)
        .await
        .unwrap();
    let new = p.new.unwrap();
    assert_eq!(new.size, IMAGE_BYTE_CAP + 1);
    assert!(new.bytes.is_none(), "over the cap: size only");
}
