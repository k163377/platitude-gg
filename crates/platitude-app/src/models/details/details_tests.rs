//! The changed-file list of a commit: the tree it is shaped into and the
//! walk the arrow keys take over it (規約 §diff のファイル一覧).

use super::{DetailsModel, FileItem};
use crate::encode::Landing;

fn commit(paths: &[&str]) -> DetailsModel {
    let mut model = DetailsModel {
        raw_files: paths
            .iter()
            .map(|path| FileItem {
                change: "M".to_string(),
                path: (*path).to_string(),
                name: (*path).to_string(),
                ..Default::default()
            })
            .collect(),
        ..DetailsModel::default()
    };
    model.rebuild_rows();
    model
}

/// A landing on `row` at `path` — a commit's files sit in no bucket.
fn landed(row: i32, path: &str) -> Option<Landing> {
    Some(Landing {
        row,
        bucket: String::new(),
        path: path.to_string(),
    })
}

#[test]
fn the_arrows_walk_the_changed_files_and_stop_at_the_ends() {
    let mut model = commit(&["a.txt", "z.txt"]);
    model.tree_view = false;
    model.rebuild_rows();

    assert_eq!(
        *model.step_file(String::new(), "a.txt".to_string(), 1),
        landed(1, "z.txt")
    );
    assert_eq!(
        *model.step_file(String::new(), "z.txt".to_string(), -1),
        landed(0, "a.txt")
    );
    // Only the sign is read: one press is one file.
    assert_eq!(
        *model.step_file(String::new(), "a.txt".to_string(), 9),
        *model.step_file(String::new(), "a.txt".to_string(), 1)
    );
    assert!(
        model
            .step_file(String::new(), "a.txt".to_string(), -1)
            .is_none()
    );
    assert!(
        model
            .step_file(String::new(), "z.txt".to_string(), 1)
            .is_none()
    );
    // A path this commit did not touch has nowhere to walk from.
    assert!(
        model
            .step_file(String::new(), "no.txt".to_string(), 1)
            .is_none()
    );
}

/// A folder row has no diff behind it.
#[test]
fn the_arrows_step_over_a_folder_row() {
    let model = commit(&["one/x.txt", "two/y.txt"]);

    // Tree view (the default) puts each folder's row above its files: row
    // 2 is `two`.
    assert_eq!(model.files.len(), 4);
    assert!(model.files[2].folder);
    assert_eq!(
        *model.step_file(String::new(), "one/x.txt".to_string(), 1),
        landed(3, "two/y.txt")
    );
    assert_eq!(
        *model.step_file(String::new(), "two/y.txt".to_string(), -1),
        landed(1, "one/x.txt")
    );
}

/// The walk hands the diff a rename's source, so git reads both ends.
#[test]
fn a_renamed_file_gives_up_its_source_by_path() {
    let mut model = commit(&["new.txt", "plain.txt"]);
    model.raw_files[0].orig_path = "old.txt".to_string();
    model.rebuild_rows();

    assert_eq!(model.orig_of("new.txt".to_string()), "old.txt");
    assert_eq!(model.orig_of("plain.txt".to_string()), "");
    assert_eq!(model.orig_of("absent.txt".to_string()), "");
}
