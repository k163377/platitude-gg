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
/// Where the reader goes when a file leaves its side: the next file under
/// the same heading, else the one before it. Untracked files stand under
/// the unstaged heading; a staged row is another heading's.
#[test]
fn the_file_beside_one_is_the_next_under_its_heading_else_the_one_before() {
    let status = platitude_core::status::WorktreeStatus {
        items: vec![
            tracked('.', 'M', "a.txt"),
            tracked('.', 'M', "b.txt"),
            tracked('M', '.', "s.txt"),
            tracked('.', 'M', "c.txt"),
            platitude_core::status::StatusItem::Untracked {
                path: "d.txt".to_string(),
            },
        ],
        ..Default::default()
    };
    let model = worktree("unstaged", Source::files(status));
    let stays = |_: &str, _: &str| false;

    assert_eq!(model.beside("unstaged", "b.txt", stays), "unstaged:c.txt");
    assert_eq!(
        model.beside("unstaged", "c.txt", stays),
        "untracked:d.txt",
        "an untracked file is under the unstaged heading"
    );
    assert_eq!(
        model.beside("untracked", "d.txt", stays),
        "unstaged:c.txt",
        "the last file looks back"
    );
    assert_eq!(
        model.beside("unstaged", "s.txt", stays),
        "",
        "not on that side"
    );
    assert_eq!(
        model.beside("staged", "s.txt", stays),
        "",
        "alone under its heading"
    );
}
/// A file a stage press still reaches leaves with the status after it, so
/// the reader goes past it, both ways — to where one status after the
/// presses would have left the next file.
#[test]
fn the_file_beside_one_passes_over_files_a_press_still_reaches() {
    let status = platitude_core::status::WorktreeStatus {
        items: vec![
            tracked('.', 'M', "a.txt"),
            tracked('.', 'M', "b.txt"),
            tracked('.', 'M', "c.txt"),
            tracked('.', 'M', "d.txt"),
        ],
        ..Default::default()
    };
    let model = worktree("unstaged", Source::files(status));

    assert_eq!(
        model.beside("unstaged", "b.txt", |_, row| row == "c.txt"),
        "unstaged:d.txt"
    );
    assert_eq!(
        model.beside("unstaged", "c.txt", |_, row| row == "d.txt"),
        "unstaged:b.txt",
        "the files after it all going, it looks back"
    );
    assert_eq!(
        model.beside("unstaged", "b.txt", |_, row| row != "b.txt"),
        "",
        "every other file going"
    );
    assert_eq!(
        model.beside("unstaged", "b.txt", |bucket, row| bucket == "staged"
            && row == "c.txt"),
        "unstaged:c.txt",
        "a row is going only from the bucket named"
    );
}
/// The page's neighbour passes over a row a stage press still reaches — in
/// a list of this window's own tree; another worktree's list, and one whose
/// tab the hub does not hold, walk the rows as they are.
#[test]
fn the_neighbour_passes_over_the_rows_a_press_still_reaches() {
    let tab = crate::hub::testkit::installed_tab();
    let reach = crate::ops::Reach::Paths(std::iter::once("c.txt".to_string()).collect());
    crate::hub::stage_ahead(tab, |ahead| {
        ahead.asked(crate::ops::Side::Stage, reach, Some(7))
    });
    let status = platitude_core::status::WorktreeStatus {
        items: vec![
            tracked('.', 'M', "b.txt"),
            tracked('.', 'M', "c.txt"),
            tracked('.', 'M', "d.txt"),
        ],
        ..Default::default()
    };
    let mut own = worktree("unstaged", Source::files(status.clone()));
    own.tab_id = tab;
    assert_eq!(
        own.beside_past_presses("unstaged", "b.txt"),
        "unstaged:d.txt"
    );

    let mut carried = worktree("whole", Source::files(status.clone()));
    carried.tab_id = tab;
    assert_eq!(
        carried.beside_past_presses("unstaged", "b.txt"),
        "unstaged:c.txt",
        "another worktree's rows are none of the presses'"
    );

    let mut gone = worktree("unstaged", Source::files(status));
    gone.tab_id = tab + 1;
    assert_eq!(
        gone.beside_past_presses("unstaged", "b.txt"),
        "unstaged:c.txt",
        "no tab in the hub, no press"
    );
}
/// A step runs out at its own list's ends; the pane carries the walk on
/// (`FileRowWalk`).
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
