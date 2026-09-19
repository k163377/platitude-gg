//! What the rows come out as once a filter, a fold and a tree have had
//! their say. Beside the view, so the ceiling the view sits under stays
//! about the view (.claude/rules/structure.md).

use super::testkit::*;
use super::*;

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

/// What the sticky stand-in steps itself in by (`HeadPinRow`): the folds
/// of the row it stands for, and none at all while there is no such row.
#[test]
fn the_head_row_reports_the_fold_its_stand_in_takes() {
    // The row is the one the record names
    // (`RefsMsg::Head`).
    let branches = |head: &str| {
        let mut model = section(
            "branches",
            Source::Locals(locals(vec![
                local("feature/topic-a", false),
                local("main", false),
            ])),
        );
        model.head_name = head.to_string();
        model.arrange();
        model
    };

    // feature / topic-a / main: a branch under a folder is one step in,
    // and the stand-in for it takes the same step.
    let model = branches("feature/topic-a");
    assert_eq!(model.head_row, 1);
    assert_eq!(depth_of(&model, 1), 1);
    assert_eq!(model.head_depth, 1);

    // A branch with no `/` is nested by nothing, and neither is its
    // stand-in — the step a stand-in always took put its name in a column
    // no row was in.
    let mut model = branches("main");
    assert_eq!(model.head_row, 2);
    assert_eq!(depth_of(&model, 2), 0);
    assert_eq!(model.head_depth, 0);

    // Filtered away: the stand-in has no row to follow and takes a seat of
    // its own at the head of the list, where the rows themselves begin.
    model.filter = "feature".to_string();
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_depth, 0);
}

/// Which row the sticky stand-in sits under once a fold has closed over
/// the branch it stands for (`HeadPinRow.seatedUnder`).
#[test]
fn a_folded_head_names_the_row_it_stands_behind() {
    let mut model = section(
        "branches",
        Source::Locals(locals(vec![
            local("feature/topic-a", false),
            local("main", false),
        ])),
    );
    model.head_name = "feature/topic-a".to_string();
    model.arrange();

    // Open, so the branch has a row of its own and is behind nothing.
    assert_eq!(model.head_row, 1);
    assert_eq!(model.head_under_row, -1);

    // Closed over: the row is gone, and what took it is the folder on
    // row 0 — where the stand-in belongs, rather than at the head of a
    // list the branch is not at the head of.
    model.folder_overrides.insert("feature".to_string(), false);
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_under_row, 0);
    // And it begins its name in the folder's own column, not the one its
    // row had — nothing stands above that folder here, so the name it
    // says is the whole one.
    assert_eq!(model.head_depth, 0);
    assert_eq!(model.head_shown, "feature/topic-a");

    // A filter flattens the tree away, so there is no folder row left to
    // stand behind even while one is folded — and nothing on screen says
    // any part of the name, so the stand-in says all of it.
    model.filter = "main".to_string();
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_under_row, -1);
    assert_eq!(model.head_shown, "feature/topic-a");
}

/// The same, with folders to spare: whichever level closes is the one
/// the stand-in sits under, and its column is that row's.
#[test]
fn the_stand_in_follows_whichever_fold_closed_over_it() {
    let folded = |key: &str| {
        let mut model = section(
            "branches",
            Source::Locals(locals(vec![
                local("main", false),
                local("team/backend/api/add-cache", false),
                local("team/backend/api/fix-auth", false),
                local("team/web/landing", false),
            ])),
        );
        model.head_name = "team/backend/api/fix-auth".to_string();
        model.folder_overrides.insert(key.to_string(), false);
        model.arrange();
        model
    };

    // main / team / team/backend / team/backend/api / …, so the folder
    // that closed is at a different row and a different depth each time
    // — and the name begins at that folder, the ones above it being rows
    // the reader can still see.
    let model = folded("team");
    assert_eq!(model.head_under_row, 1);
    assert_eq!(model.head_depth, 0);
    assert_eq!(model.head_shown, "team/backend/api/fix-auth");

    let model = folded("team/backend");
    assert_eq!(model.head_under_row, 2);
    assert_eq!(model.head_depth, 1);
    assert_eq!(model.head_shown, "backend/api/fix-auth");

    let model = folded("team/backend/api");
    assert_eq!(model.head_under_row, 3);
    assert_eq!(model.head_depth, 2);
    assert_eq!(model.head_shown, "api/fix-auth");

    // A fold that does not lie over the branch leaves its row alone, and
    // the stand-in that is not standing says the whole name.
    let model = folded("team/web");
    assert_eq!(model.head_row, 5);
    assert_eq!(model.head_under_row, -1);
    assert_eq!(model.head_depth, 3);
    assert_eq!(model.head_shown, "team/backend/api/fix-auth");
}

/// Where to scroll for a name, in the two shapes a row is keyed by:
/// the full one a tree gives a leaf, and the shown one a tag carries
/// on its own.
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
    // The leaf answers to its whole name.
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

/// A row the page is showing as already deleted leaves the list, the
/// count, and — where it was the last one under it — the folder it stood
/// in (デザイン規約 §消す操作は先に画面から消す).
#[test]
fn a_row_shown_as_gone_leaves_the_list_and_the_count() {
    let mut model = section(
        "remotes",
        Source::Remotes(snapshot(
            vec![
                remote("origin/feature/one"),
                remote("origin/feature/two"),
                remote("origin/main"),
            ],
            Vec::new(),
        )),
    );
    model.folder_overrides.insert("origin".to_string(), true);
    model.arrange();
    // origin / feature / one / two / main.
    assert_eq!(model.shown_rows(), 5);
    assert_eq!(model.total, 3);

    // One of the two under `feature`: the folder stays, opened by the
    // sibling that is still there.
    model.hidden = vec!["origin/feature/one".to_string()];
    model.arrange();
    assert_eq!(model.shown_rows(), 4);
    assert_eq!(says(&model, 1, Role::Name), "feature");
    assert_eq!(says(&model, 2, Role::Name), "two");
    assert_eq!(says(&model, 3, Role::Name), "main");
    // The band counts what the repository has, and this row is being
    // shown as no longer one of them.
    assert_eq!(model.total, 2);

    // Both of them: the folder they were the whole of goes with them.
    model.hidden = vec![
        "origin/feature/one".to_string(),
        "origin/feature/two".to_string(),
    ];
    model.arrange();
    assert_eq!(model.shown_rows(), 2);
    assert_eq!(says(&model, 0, Role::Name), "origin");
    assert_eq!(says(&model, 1, Role::Name), "main");
    assert_eq!(model.total, 1);

    // Taken back — a refused delete puts every one of them back where it
    // was, folders included.
    model.hidden = Vec::new();
    model.arrange();
    assert_eq!(model.shown_rows(), 5);
    assert_eq!(model.total, 3);
}

/// The sections that arrange nothing at all still have to leave a row
/// out: with no tree and no filter their rows are the source's own, and
/// the one being shown as gone is not among them (`arrange`).
#[test]
fn a_flat_section_leaves_out_the_row_shown_as_gone() {
    let mut model = section(
        "tags",
        Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", false, true), tag("v1.1", false, true)],
        )),
    );
    model.arrange();
    assert!(model.arranged.is_none(), "nothing to arrange yet");
    assert_eq!(model.total, 2);

    model.hidden = vec!["v1.0".to_string()];
    model.arrange();
    assert_eq!(model.shown_rows(), 1);
    assert_eq!(says(&model, 0, Role::Name), "v1.1");
    assert_eq!(model.total, 1);
}

/// A stash is named to git by its selector, and the selector is what
/// the page hands over.
#[test]
fn a_stash_is_hidden_by_the_selector_git_knows_it_by() {
    let entry = |name: &str, message: &str| platitude_core::stash::StashEntry {
        name: name.to_string(),
        oid: oid("a"),
        time: 0,
        message: message.to_string(),
    };
    let mut model = section(
        "stashes",
        Source::Stashes(vec![
            entry("stash@{0}", "WIP on main"),
            entry("stash@{1}", "WIP on topic"),
        ]),
    );
    // What the row shows is not what git is asked about, so it hides
    // nothing.
    model.hidden = vec!["WIP on main".to_string()];
    model.arrange();
    assert_eq!(model.shown_rows(), 2);

    model.hidden = vec!["stash@{0}".to_string()];
    model.arrange();
    assert_eq!(model.shown_rows(), 1);
    assert_eq!(says(&model, 0, Role::Name), "WIP on topic");
    assert_eq!(model.total, 1);
}

/// Filtering and hiding answer different questions and a row can be
/// caught by both: the filter says which rows match, the hidden list says
/// which the repository is being shown as no longer having.
#[test]
fn a_filtered_list_leaves_out_the_row_shown_as_gone_as_well() {
    let mut model = section(
        "remotes",
        Source::Remotes(snapshot(
            vec![remote("origin/feature/one"), remote("origin/feature/two")],
            Vec::new(),
        )),
    );
    model.filter = "feature".to_string();
    model.hidden = vec!["origin/feature/one".to_string()];
    model.arrange();

    assert_eq!(model.shown_rows(), 1);
    assert_eq!(says(&model, 0, Role::Name), "origin/feature/two");
    assert_eq!(model.total, 1);
}
