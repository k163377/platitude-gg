//! The walk the arrow keys take over the file rows, and what a bucket
//! list answers about the files it is not showing.

use super::testkit::*;
use super::*;

fn landed(row: i32, bucket: &str, path: &str) -> Option<Landing> {
    Some(Landing {
        row,
        bucket: bucket.to_string(),
        path: path.to_string(),
    })
}

/// `NavSectionModel::run` cuts what is shown, not what is answered.
#[test]
fn a_bucket_list_shows_one_run_and_answers_for_the_whole_tree() {
    let mut model = worktree("unstaged", Source::files(pending()));
    model.tree_view = false;
    model.arrange();

    assert_eq!(model.shown_rows(), 2);
    assert_eq!(says(&model, 0, Role::Full), "c.txt");
    assert_eq!(says(&model, 1, Role::Full), "src/b.txt");
    // The `MM` file's staged row is not on screen here, yet answered: the
    // page asks whichever list it holds (`RepoPage`).
    assert_eq!(model.row_of_file("staged", "src/b.txt"), -1);
    assert!(model.holds("staged", "src/b.txt"));
    assert_eq!(model.told(Role::Full, "d.txt", Role::Bucket), "staged");
}
/// A step runs out at its own list's ends; the pane carries the walk on
/// (`FileRowWalk`, デザイン規約 §diff のファイル一覧).
#[test]
fn the_arrows_walk_one_bucket_and_stop_at_its_ends() {
    let mut model = worktree("staged", Source::files(pending()));
    model.tree_view = false;
    model.arrange();

    assert_eq!(
        *model.step("staged", "d.txt", 1),
        landed(1, "staged", "src/b.txt")
    );
    // Only the sign is read: one press is one file.
    assert_eq!(
        *model.step("staged", "d.txt", 9),
        *model.step("staged", "d.txt", 1)
    );
    // Both ends of this list.
    assert!(model.step("staged", "d.txt", -1).is_none());
    assert!(model.step("staged", "src/b.txt", 1).is_none());
    // The `MM` file's unstaged row is not in this list: nowhere to walk from.
    assert!(model.step("unstaged", "src/b.txt", 1).is_none());
    assert!(model.step("staged", "nowhere.txt", 1).is_none());
}
/// An empty bucket answers nothing, so the arrows go past it.
#[test]
fn a_walk_crossing_into_a_bucket_lands_at_the_end_it_comes_in_by() {
    let mut model = worktree("staged", Source::files(pending()));
    model.tree_view = false;
    model.arrange();
    assert_eq!(*model.edge(1), landed(0, "staged", "d.txt"));
    assert_eq!(*model.edge(-1), landed(1, "staged", "src/b.txt"));

    let mut empty = worktree("conflicts", Source::files(Default::default()));
    empty.arrange();
    assert!(empty.edge(1).is_none());
    assert!(empty.edge(-1).is_none());
}
/// A folder row has no diff, so the walk and the crossing skip it.
#[test]
fn the_arrows_step_over_a_folder_row() {
    let mut model = worktree("unstaged", Source::files(pending()));
    model.tree_view = true;
    model.arrange();

    // The tree puts the folder `src` above the leaf under it.
    assert_eq!(says(&model, 0, Role::Name), "src");
    assert!(flags(&model, 0, Role::Folder));
    assert_eq!(
        *model.step("untracked", "c.txt", -1),
        landed(1, "unstaged", "src/b.txt")
    );
    assert_eq!(*model.edge(1), landed(1, "unstaged", "src/b.txt"));
}

/// An untracked file keys by its own bucket, not its heading's.
#[test]
fn every_file_row_answers_the_key_a_choice_holds_it_by() {
    let mut model = worktree("unstaged", Source::files(pending()));
    model.tree_view = true;
    model.arrange();

    assert!(flags(&model, 0, Role::Folder));
    assert_eq!(model.file_key(0), "", "a folder is nothing to choose");
    assert_eq!(model.file_key(1), "unstaged:src/b.txt");
    assert_eq!(model.file_key(2), "untracked:c.txt");
    assert_eq!(model.file_key(3), "", "past the end");
}
