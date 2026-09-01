//! `avatar`'s tests, split out for length alone (structure.md §分割).

use super::*;

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// A real picture on disk. The store decodes what it is handed now,
/// so a few bytes standing in for one no longer reaches the far side.
fn picture(dir: &Path, name: &str, tint: u8) -> PathBuf {
    let png = crate::picture::png_of(24, 16, |x, _| [tint, x as u8, 40, 255]);
    file_of(dir, name, &png)
}

fn file_of(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write file");
    path
}

#[test]
fn the_key_is_the_address_folded_to_one_spelling() {
    assert_eq!(key("  Ada@Example.COM "), "ada@example.com");
    assert_eq!(key(""), "");
}

#[test]
fn assigning_copies_the_picture_in() {
    let scratch = scratch();
    let source = picture(scratch.path(), "cat.png", 200);
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();

    let file = avatars
        .assign(&store_dir, "Ada@Example.com", "Ada", &source)
        .expect("assign");
    assert!(store_dir.join(&file).is_file(), "the copy is on disk");
    assert_eq!(avatars.file_of("ada@example.com"), Some(file.as_str()));
    // The address folded, the name kept as it was given.
    assert_eq!(avatars.list()[0].email, "ada@example.com");
    assert_eq!(avatars.list()[0].name, "Ada");

    // The picture a person picked can go away without taking the
    // avatar with it.
    std::fs::remove_file(&source).expect("remove source");
    assert!(store_dir.join(&file).is_file());
}

#[test]
fn replacing_a_picture_changes_the_file_name_and_sweeps_the_old_one() {
    let scratch = scratch();
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();
    let first = picture(scratch.path(), "a.png", 10);
    let second = picture(scratch.path(), "b.png", 220);

    let old = avatars
        .assign(&store_dir, "ada@example.com", "Ada", &first)
        .expect("assign");
    let new = avatars
        .assign(&store_dir, "ada@example.com", "Ada", &second)
        .expect("reassign");

    assert_ne!(old, new, "a different picture is a different name");
    assert!(!store_dir.join(&old).exists(), "the old copy is swept");
    assert!(store_dir.join(&new).is_file());
    assert_eq!(avatars.list().len(), 1, "one address, one row");
}

#[test]
fn the_same_picture_for_two_people_is_stored_once() {
    let scratch = scratch();
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();
    let source = picture(scratch.path(), "shared.png", 90);

    let a = avatars
        .assign(&store_dir, "ada@example.com", "Ada", &source)
        .expect("assign");
    let b = avatars
        .assign(&store_dir, "bob@example.com", "Bob", &source)
        .expect("assign");
    assert_eq!(a, b);

    // And taking it off one leaves the other drawable.
    assert!(avatars.remove(&store_dir, "ada@example.com"));
    assert!(store_dir.join(&b).is_file(), "the file is still spoken for");
    assert!(avatars.remove(&store_dir, "bob@example.com"));
    assert!(!store_dir.join(&b).exists(), "now nothing points at it");
}

#[test]
fn something_that_is_not_a_picture_is_refused_before_anything_is_written() {
    let scratch = scratch();
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();
    // Named like a picture, which is exactly why the name is not
    // what decides.
    let source = file_of(scratch.path(), "cat.png", b"hello");
    assert!(matches!(
        avatars.assign(&store_dir, "ada@example.com", "Ada", &source),
        Err(AvatarError::Picture(
            crate::picture::PictureError::Unreadable
        ))
    ));
    assert!(!store_dir.exists(), "nothing was created");
    assert!(avatars.is_empty());
}

#[test]
fn a_file_past_the_ceiling_is_refused_without_being_read() {
    let scratch = scratch();
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();
    let big = vec![0u8; (MAX_BYTES + 1) as usize];
    let source = file_of(scratch.path(), "huge.png", &big);
    assert!(matches!(
        avatars.assign(&store_dir, "ada@example.com", "Ada", &source),
        Err(AvatarError::TooLarge)
    ));
}

#[test]
fn what_is_stored_is_the_small_square_rather_than_what_was_picked() {
    let scratch = scratch();
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();
    // Wide, and far bigger than anything ever drawn.
    let wide = crate::picture::png_of(1200, 800, |x, y| {
        [(x % 251) as u8, (y % 253) as u8, 30, 255]
    });
    let source = file_of(scratch.path(), "wide.png", &wide);

    let file = avatars
        .assign(&store_dir, "ada@example.com", "Ada", &source)
        .expect("assign");
    assert!(file.ends_with(".png"), "always stored as png: {file}");
    let stored = std::fs::read(store_dir.join(&file)).expect("read stored");
    assert!(
        stored.len() < wide.len(),
        "smaller than what was picked: {} vs {}",
        stored.len(),
        wide.len()
    );
    // The header says it plainly: IHDR carries the two sides.
    let side = u32::from_be_bytes([stored[16], stored[17], stored[18], stored[19]]);
    let other = u32::from_be_bytes([stored[20], stored[21], stored[22], stored[23]]);
    assert_eq!((side, other), (crate::picture::SIDE, crate::picture::SIDE));
}

#[test]
fn a_file_name_out_of_the_settings_file_cannot_leave_the_directory() {
    let values = vec![
        toml::Value::Table({
            let mut t = toml::Table::new();
            t.insert("email".into(), toml::Value::String("a@b.c".into()));
            t.insert(
                "file".into(),
                toml::Value::String("../../../etc/passwd".into()),
            );
            t
        }),
        toml::Value::Table({
            let mut t = toml::Table::new();
            t.insert("email".into(), toml::Value::String("d@e.f".into()));
            t.insert("file".into(), toml::Value::String("ab12cd34.png".into()));
            t
        }),
    ];
    let avatars = Avatars::from_values(&values);
    assert_eq!(avatars.list().len(), 1);
    assert_eq!(avatars.list()[0].email, "d@e.f");
}

#[test]
fn an_assignment_whose_image_is_gone_is_forgotten() {
    let scratch = scratch();
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();
    let source = picture(scratch.path(), "a.png", 70);
    let file = avatars
        .assign(&store_dir, "ada@example.com", "Ada", &source)
        .expect("assign");

    assert_eq!(avatars.forget_missing(&store_dir), 0);
    std::fs::remove_file(store_dir.join(&file)).expect("remove copy");
    assert_eq!(avatars.forget_missing(&store_dir), 1);
    assert!(avatars.is_empty());
}

#[test]
fn a_round_trip_through_the_table_keeps_every_assignment() {
    let scratch = scratch();
    let store_dir = scratch.path().join("avatars");
    let mut avatars = Avatars::default();
    let a = picture(scratch.path(), "a.png", 30);
    let b = picture(scratch.path(), "b.jpg", 180);
    avatars
        .assign(&store_dir, "zoe@example.com", "Zoe", &a)
        .expect("assign");
    avatars
        .assign(&store_dir, "ada@example.com", "Ada", &b)
        .expect("assign");

    let back = Avatars::from_values(&avatars.to_values());
    assert_eq!(back, avatars);
    // Sorted by address, so the file a person opens reads in a fixed
    // order rather than in the order they happened to assign.
    assert_eq!(back.list()[0].email, "ada@example.com");
}
