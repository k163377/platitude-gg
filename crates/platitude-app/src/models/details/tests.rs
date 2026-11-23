//! The changed-file list of a commit: the tree it is shaped into and the
//! walk the arrow keys take over it (`FileRowWalk.qml`).

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

/// The menu key puts the file the arrows stand on on screen by its row:
/// the row it is drawn at, folder rows counted.
#[test]
fn a_file_is_found_at_the_row_it_is_drawn_at() {
    let model = commit(&["one/x.txt", "two/y.txt"]);

    assert_eq!(
        model.row_of_file_in(String::new(), "two/y.txt".to_string()),
        3
    );
    assert_eq!(
        model.row_of_file_in(String::new(), "two".to_string()),
        -1,
        "a folder's row is no file's"
    );
    assert_eq!(
        model.row_of_file_in(String::new(), "no.txt".to_string()),
        -1
    );
}

/// A file under a closed folder has no row to put on screen.
#[test]
fn a_file_under_a_closed_folder_is_at_no_row() {
    let mut model = commit(&["one/x.txt", "two/y.txt"]);
    model.folder_overrides.insert("one".to_string(), false);
    model.rebuild_rows();

    assert_eq!(
        model.row_of_file_in(String::new(), "one/x.txt".to_string()),
        -1
    );
    assert_eq!(
        model.row_of_file_in(String::new(), "two/y.txt".to_string()),
        2,
        "the rows below move up past it"
    );
}

/// What the pane keeps of the list is the tree's rows with every folder open, up to two — and the same with a folder
/// shut or the paths view up, so neither moves the list.
#[test]
fn the_list_keeps_the_open_trees_rows_up_to_two() {
    let landed = |paths: &[&str]| {
        let mut model = DetailsModel::default();
        let files: Vec<_> = paths
            .iter()
            .map(|path| platitude_core::parse::name_status::FileChange {
                status: 'M',
                score: None,
                path: (*path).to_string(),
                orig_path: None,
            })
            .collect();
        model.take_files(&files);
        model
    };
    for (paths, rows, why) in [
        (&[][..], 0, "nothing changed, nothing kept"),
        (&["top.txt"][..], 1, "a lone file at the top is one row"),
        (
            &["docs/guide.md"][..],
            2,
            "a lone file under its folder is two rows",
        ),
        (&["a.txt", "b.txt"][..], 2, "two files are two rows"),
        (
            &["a.txt", "b.txt", "c/d.txt"][..],
            2,
            "and no more than two",
        ),
    ] {
        let model = landed(paths);
        assert_eq!(model.floor_rows, rows, "{why}");
        assert_eq!(
            model.files.len().min(2) as i32,
            rows,
            "the open tree's own rows, up to two: {why}"
        );
    }

    let mut model = landed(&["docs/guide.md"]);
    model.folder_overrides.insert("docs".to_string(), false);
    model.rebuild_rows();
    assert_eq!(model.files.len(), 1, "the folder shut stands as one row");
    assert_eq!(
        model.floor_rows, 2,
        "and the floor keeps the two rows it has open"
    );
    model.tree_view = false;
    model.rebuild_rows();
    assert_eq!(model.floor_rows, 2, "the paths view keeps it too");
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
