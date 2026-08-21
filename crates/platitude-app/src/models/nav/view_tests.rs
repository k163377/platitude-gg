//! What the rows come out as once a filter, a fold and a tree have had
//! their say, and the walk the arrow keys take over them. Beside the view
//! rather than in it, so the ceiling the view sits under stays about the
//! view (.claude/rules/structure.md).

use super::testkit::*;
use super::*;
use crate::encode::FIELD_SEP;

#[test]
fn a_filtered_remote_row_shows_its_whole_name() {
    let mut model = section(
        "remotes",
        Source::Remotes(snapshot(
            vec![remote("origin/feature/one"), remote("origin/main")],
            Vec::new(),
        )),
    );
    model.filter = "feature".to_string();
    model.arrange();

    assert_eq!(model.shown_rows(), 1);
    assert_eq!(says(&model, 0, Role::Name), "origin/feature/one");
    assert_eq!(says(&model, 0, Role::Full), "");
}

/// Where to scroll for a name, in the two shapes a row is keyed by:
/// the full one a tree gives a leaf, and the shown one a tag has
/// instead of a full name at all.
#[test]
fn a_row_is_found_by_the_name_it_is_keyed_by() {
    let mut model = section(
        "remotes",
        Source::Remotes(snapshot(
            vec![remote("origin/feature/one"), remote("origin/main")],
            Vec::new(),
        )),
    );
    // A remote root starts closed, and a row folded away is nowhere the
    // view can scroll to.
    model.arrange();
    assert_eq!(model.row_of("origin/main"), -1);

    model.folder_overrides.insert("origin".to_string(), true);
    model.arrange();
    // origin / feature / one / main.
    assert_eq!(model.row_of("origin/feature/one"), 2);
    assert_eq!(model.row_of("origin/main"), 3);
    // The leaf answers to its whole name, never to what it shows.
    assert_eq!(model.row_of("one"), -1);
    assert_eq!(model.row_of("origin/feature/two"), -1);

    // A tag carries no full name: it is keyed by what it shows.
    let mut tags = section(
        "tags",
        Source::Tags(snapshot(Vec::new(), vec![tag("v1.0", false, true)])),
    );
    tags.arrange();
    assert_eq!(tags.row_of("v1.0"), 0);
}

#[test]
fn a_remote_row_derives_its_local_name_from_its_own_snapshot() {
    let model = section(
        "remotes",
        Source::Remotes(Arc::new(platitude_core::session::RefsSnapshot {
            remote_names: vec!["my".into(), "my/fork".into()],
            remote_urls: Vec::new(),
            ..Default::default()
        })),
    );

    assert_eq!(model.local_name_of("my/fork/feature/one"), "feature/one");
    assert_eq!(model.local_name_of("unknown/feature"), "unknown/feature");
}
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
/// A snapshot that carries the same tags is not a reason to rebuild
/// tens of thousands of delegates.
#[test]
fn a_republished_snapshot_moves_nothing() {
    let mut model = section("tags", Source::default());
    assert!(model.take(Source::Tags(snapshot(
        Vec::new(),
        vec![tag("v1.0", true, true)]
    ))));
    assert!(!model.take(Source::Tags(snapshot(
        Vec::new(),
        vec![tag("v1.0", true, true)]
    ))));
    assert!(model.take(Source::Tags(snapshot(
        Vec::new(),
        vec![tag("v1.1", true, true)]
    ))));
}
