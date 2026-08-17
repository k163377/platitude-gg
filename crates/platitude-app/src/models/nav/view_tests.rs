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
            ..Default::default()
        })),
    );

    assert_eq!(model.local_name_of("my/fork/feature/one"), "feature/one");
    assert_eq!(model.local_name_of("unknown/feature"), "unknown/feature");
}
/// The arrows walk the rows on screen, one file per press, and stop at
/// each end rather than wrapping (規約 §diff のファイル一覧).
#[test]
fn the_arrows_walk_the_files_and_stop_at_the_ends() {
    let mut model = section("worktree", Source::files(pending()));
    model.tree_view = false;
    model.arrange();

    // Down out of the unstaged run and into the next heading's file: one
    // list, and the band between them is something walking goes past.
    assert_eq!(
        model.step("unstaged", "src/b.txt", 1),
        format!("3{FIELD_SEP}staged{FIELD_SEP}d.txt")
    );
    assert_eq!(
        model.step("untracked", "c.txt", -1),
        format!("0{FIELD_SEP}conflicts{FIELD_SEP}a.txt")
    );
    // Only the sign is read: one press is one file.
    assert_eq!(
        model.step("unstaged", "src/b.txt", 9),
        model.step("unstaged", "src/b.txt", 1)
    );
    // Both ends, and the two rows of the file that is on both sides:
    // asked by bucket, each walks on from its own row.
    assert_eq!(model.step("conflicts", "a.txt", -1), "");
    assert_eq!(model.step("staged", "src/b.txt", 1), "");
    assert_eq!(
        model.step("staged", "d.txt", 1),
        format!("4{FIELD_SEP}staged{FIELD_SEP}src/b.txt")
    );
    // A path no row of this list holds has nowhere to walk from.
    assert_eq!(model.step("unstaged", "nowhere.txt", 1), "");
}
/// A folder is not a file: the walk steps over its row rather than
/// landing on one that has no diff behind it.
#[test]
fn the_arrows_step_over_a_folder_row() {
    let mut model = section("worktree", Source::files(pending()));
    model.tree_view = true;
    model.arrange();

    // The tree puts the folder `src` between the conflicted file and the
    // leaf under it, and a folder has no diff to land on.
    assert!(flags(&model, 1, Role::Folder));
    assert_eq!(
        model.step("conflicts", "a.txt", 1),
        format!("2{FIELD_SEP}unstaged{FIELD_SEP}src/b.txt")
    );
    assert!(flags(&model, 4, Role::Folder));
    assert_eq!(
        model.step("untracked", "c.txt", 1),
        format!("5{FIELD_SEP}staged{FIELD_SEP}src/b.txt")
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
