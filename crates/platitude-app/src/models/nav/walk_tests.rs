//! The walk the arrow keys take over the file rows, and what a bucket
//! list answers about the files it is not showing. Beside the walk rather
//! than in it, so the ceiling the walk sits under stays about the walk
//! (.claude/rules/structure.md).

use super::testkit::*;
use super::*;
use crate::encode::FIELD_SEP;

/// A bucket list shows its own run and holds the whole status: what is on
/// screen is one heading's files, and what it can be asked about is every
/// file (`NavSectionModel::run`).
#[test]
fn a_bucket_list_shows_one_run_and_answers_for_the_whole_tree() {
    let mut model = worktree("unstaged", Source::files(pending()));
    model.tree_view = false;
    model.arrange();

    assert_eq!(model.shown_rows(), 2);
    assert_eq!(says(&model, 0, Role::Full), "c.txt");
    assert_eq!(says(&model, 1, Role::Full), "src/b.txt");
    // The staged row of the file that is on both sides is not on screen
    // here — and is still what this list answers about, because the page
    // asks whichever list it is holding (`RepoPage`).
    assert_eq!(model.row_of_file("staged", "src/b.txt"), -1);
    assert!(model.holds("staged", "src/b.txt"));
    assert_eq!(model.told(Role::Full, "d.txt", Role::Bucket), "staged");
}
/// The arrows walk the rows on screen, one file per press, and stop at
/// each end rather than wrapping (規約 §diff のファイル一覧). A list is one
/// bucket, so its own ends are where a step runs out — the pane carries
/// the walk into the next list from there (`FileRowWalk`).
#[test]
fn the_arrows_walk_one_bucket_and_stop_at_its_ends() {
    let mut model = worktree("staged", Source::files(pending()));
    model.tree_view = false;
    model.arrange();

    assert_eq!(
        model.step("staged", "d.txt", 1),
        format!("1{FIELD_SEP}staged{FIELD_SEP}src/b.txt")
    );
    // Only the sign is read: one press is one file.
    assert_eq!(
        model.step("staged", "d.txt", 9),
        model.step("staged", "d.txt", 1)
    );
    // Both ends of this list, and the two rows of the file that is on
    // both sides: asked by bucket, each walks on from its own row.
    assert_eq!(model.step("staged", "d.txt", -1), "");
    assert_eq!(model.step("staged", "src/b.txt", 1), "");
    // The unstaged row of the same file is not a row of this list, so it
    // is nowhere to walk from.
    assert_eq!(model.step("unstaged", "src/b.txt", 1), "");
    assert_eq!(model.step("staged", "nowhere.txt", 1), "");
}
/// Where the walk comes in when it crosses out of the bucket above or
/// below: the row at the end of this list it is entering by, and nothing
/// at all from a bucket holding no files (the arrows go past an empty
/// heading rather than stopping in it).
#[test]
fn a_walk_crossing_into_a_bucket_lands_at_the_end_it_comes_in_by() {
    let mut model = worktree("staged", Source::files(pending()));
    model.tree_view = false;
    model.arrange();
    assert_eq!(model.edge(1), format!("0{FIELD_SEP}staged{FIELD_SEP}d.txt"));
    assert_eq!(
        model.edge(-1),
        format!("1{FIELD_SEP}staged{FIELD_SEP}src/b.txt")
    );

    let mut empty = worktree("conflicts", Source::files(Default::default()));
    empty.arrange();
    assert_eq!(empty.edge(1), "");
    assert_eq!(empty.edge(-1), "");
}
/// A folder is not a file: the walk steps over its row rather than
/// landing on one that has no diff behind it, and a list whose only rows
/// are folders is one the crossing walk goes straight past.
#[test]
fn the_arrows_step_over_a_folder_row() {
    let mut model = worktree("unstaged", Source::files(pending()));
    model.tree_view = true;
    model.arrange();

    // The tree puts the folder `src` above the leaf under it.
    assert_eq!(says(&model, 0, Role::Name), "src");
    assert!(flags(&model, 0, Role::Folder));
    assert_eq!(
        model.step("untracked", "c.txt", -1),
        format!("1{FIELD_SEP}unstaged{FIELD_SEP}src/b.txt")
    );
    assert_eq!(
        model.edge(1),
        format!("1{FIELD_SEP}unstaged{FIELD_SEP}src/b.txt")
    );
}
/// The key a choice holds a row by, answered for every row on screen and
/// not only for the ones a view has built a delegate for. A folder row
/// answers nothing — a folder is not a file to choose — and so does a
/// number past the end.
///
/// **A row carries its own bucket.** An untracked file is shown under the
/// unstaged heading, so its key reads `untracked:` while the list's run is
/// `unstaged`; a pane composing the key out of the run it asked would name
/// a row no write can find.
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
