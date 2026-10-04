//! What the rows come out as once a filter, a fold and a tree have had
//! their say.

use super::testkit::*;
use super::*;

/// The match reads the whole name, and the row is still named to git by
/// it (`build_remote_groups`).
#[test]
fn a_filtered_remote_keeps_its_row_and_lays_its_branches_flat_under_it() {
    let mut model = section(
        "remotes",
        Source::Remotes(named(
            vec![
                remote("fork/feature/two"),
                remote("origin/feature/one"),
                remote("origin/main"),
            ],
            &["origin", "fork"],
        )),
    );
    model.filter = "feature".to_string();
    model.arrange();

    assert_eq!(model.shown_rows(), 4);
    assert_eq!(says(&model, 0, Role::Name), "fork");
    assert_eq!(says(&model, 0, Role::Full), "fork");
    assert!(flags(&model, 0, Role::Folder));
    assert_eq!(
        says(&model, 0, Role::Change),
        "",
        "open, whatever the fold's default"
    );
    assert_eq!(says(&model, 1, Role::Name), "feature/two");
    assert_eq!(says(&model, 1, Role::Full), "fork/feature/two");
    assert_eq!(depth_of(&model, 1), 1);
    assert_eq!(says(&model, 2, Role::Name), "origin");
    assert_eq!(says(&model, 3, Role::Name), "feature/one");
    assert_eq!(says(&model, 3, Role::Full), "origin/feature/one");

    // The match reaches across the two halves of the name.
    model.filter = "origin/feat".to_string();
    model.arrange();
    assert_eq!(model.shown_rows(), 2);
    assert_eq!(says(&model, 0, Role::Name), "origin");
    assert_eq!(says(&model, 1, Role::Full), "origin/feature/one");
}

/// `gone` is no configured remote, so the first slash cuts it and it
/// still heads its row.
#[test]
fn the_filtered_remotes_are_cut_by_configured_name_and_then_by_the_first_slash() {
    let mut model = section(
        "remotes",
        Source::Remotes(named(
            vec![
                remote("gone/topic"),
                remote("my/fork/topic"),
                remote("my/topic"),
            ],
            &["my", "my/fork"],
        )),
    );
    model.filter = "topic".to_string();
    model.arrange();

    assert_eq!(model.shown_rows(), 6);
    assert_eq!(says(&model, 0, Role::Name), "gone");
    assert_eq!(says(&model, 1, Role::Full), "gone/topic");
    assert_eq!(says(&model, 2, Role::Name), "my/fork");
    assert_eq!(says(&model, 3, Role::Name), "topic");
    assert_eq!(says(&model, 3, Role::Full), "my/fork/topic");
    assert_eq!(says(&model, 4, Role::Name), "my");
    assert_eq!(says(&model, 5, Role::Full), "my/topic");
}

/// The fold is one answer for both shapes (`folder_expanded`).
#[test]
fn a_filtered_remote_starts_open_and_folds_like_its_row_in_the_tree() {
    let mut model = section(
        "remotes",
        Source::Remotes(named(
            vec![remote("fork/feature/two"), remote("origin/feature/one")],
            &["origin", "fork"],
        )),
    );
    // In the tree the remote's root is folded by default.
    model.arrange();
    assert_eq!(says(&model, 0, Role::Change), FOLDED);
    assert!(!model.folder_expanded("origin", 0));

    // Under a filter it is open by default, so the first toggle folds it.
    model.filter = "feature".to_string();
    model.arrange();
    assert!(model.folder_expanded("origin", 0));
    model.folder_overrides.insert("origin".to_string(), false);
    model.arrange();
    assert_eq!(
        model.shown_rows(),
        3,
        "fork open over its branch, origin folded over its own"
    );
    assert_eq!(says(&model, 2, Role::Name), "origin");
    assert_eq!(says(&model, 2, Role::Change), FOLDED);

    // And the fold carries over into the tree once the filter is gone.
    model.filter.clear();
    model.arrange();
    assert!(!model.folder_expanded("origin", 0));
}

/// The stand-in's indent (`HeadPinRow`) is its row's, and 0 while it has
/// none.
#[test]
fn the_head_row_reports_the_fold_its_stand_in_takes() {
    // `head_name` comes from `RefsMsg::Head`.
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

    // feature / topic-a / main.
    let model = branches("feature/topic-a");
    assert_eq!(model.head_row, 1);
    assert_eq!(depth_of(&model, 1), 1);
    assert_eq!(model.head_depth, 1);

    // No `/`: nested by nothing, nor is its stand-in.
    let mut model = branches("main");
    assert_eq!(model.head_row, 2);
    assert_eq!(depth_of(&model, 2), 0);
    assert_eq!(model.head_depth, 0);

    // Filtered away: no row to follow, so the head of the list.
    model.filter = "feature".to_string();
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_depth, 0);
}

/// Read by `HeadPinRow.seatedUnder`.
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

    // Closed over: the stand-in belongs under the folder on row 0, in its
    // column; nothing is above that folder, so the name is whole.
    model.folder_overrides.insert("feature".to_string(), false);
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_under_row, 0);
    assert_eq!(model.head_depth, 0);
    assert_eq!(model.head_shown, "feature/topic-a");

    // A filter flattens the tree: no folder row to stand behind even
    // while one is folded, and the whole name.
    model.filter = "main".to_string();
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_under_row, -1);
    assert_eq!(model.head_shown, "feature/topic-a");
}

/// Whichever level closes, the stand-in takes that row's seat and column.
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

    // main / team / team/backend / team/backend/api / …; the name begins
    // at the folder that closed.
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

    // A fold not over the branch leaves its row alone.
    let model = folded("team/web");
    assert_eq!(model.head_row, 5);
    assert_eq!(model.head_under_row, -1);
    assert_eq!(model.head_depth, 3);
    assert_eq!(model.head_shown, "team/backend/api/fix-auth");
}

/// Both key shapes: a tree leaf's full name, and a tag's shown one.
#[test]
fn a_row_is_found_by_the_name_it_is_keyed_by() {
    let mut model = section(
        "remotes",
        Source::Remotes(snapshot(
            vec![remote("origin/feature/one"), remote("origin/main")],
            Vec::new(),
        )),
    );
    // A remote root starts closed; a folded-away row is nowhere to scroll.
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
/// Same tags, no model reset.
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

/// A hidden row also takes the folder it was the last of
/// (デザイン規約 §消す操作は先に画面から消す).
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

    // One of the two under `feature`: its sibling keeps the folder.
    model.hidden = vec!["origin/feature/one".to_string()];
    model.arrange();
    assert_eq!(model.shown_rows(), 4);
    assert_eq!(says(&model, 1, Role::Name), "feature");
    assert_eq!(says(&model, 2, Role::Name), "two");
    assert_eq!(says(&model, 3, Role::Name), "main");
    assert_eq!(model.total, 2);

    // Both: their folder goes too.
    model.hidden = vec![
        "origin/feature/one".to_string(),
        "origin/feature/two".to_string(),
    ];
    model.arrange();
    assert_eq!(model.shown_rows(), 2);
    assert_eq!(says(&model, 0, Role::Name), "origin");
    assert_eq!(says(&model, 1, Role::Name), "main");
    assert_eq!(model.total, 1);

    // A refused delete puts them all back, folders included.
    model.hidden = Vec::new();
    model.arrange();
    assert_eq!(model.shown_rows(), 5);
    assert_eq!(model.total, 3);
}

/// A flat section reads the source directly until a row is hidden
/// (`arrange`).
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

/// The page hands over a stash's selector, not its message.
#[test]
fn a_stash_is_hidden_by_the_selector_git_knows_it_by() {
    let entry = |name: &str, message: &str| platitude_core::stash::StashEntry {
        name: name.to_string(),
        oid: oid("a"),
        time: 0,
        message: message.to_string(),
        stands: None,
    };
    let mut model = section(
        "stashes",
        Source::Stashes(vec![
            entry("stash@{0}", "WIP on main"),
            entry("stash@{1}", "WIP on topic"),
        ]),
    );
    // The shown message hides nothing.
    model.hidden = vec!["WIP on main".to_string()];
    model.arrange();
    assert_eq!(model.shown_rows(), 2);

    model.hidden = vec!["stash@{0}".to_string()];
    model.arrange();
    assert_eq!(model.shown_rows(), 1);
    assert_eq!(says(&model, 0, Role::Name), "WIP on topic");
    assert_eq!(model.total, 1);
}

/// The filter and the hidden list are separate cuts; a row can meet both.
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

    assert_eq!(
        model.shown_rows(),
        2,
        "the remote's row and the one branch left"
    );
    assert_eq!(says(&model, 1, Role::Full), "origin/feature/two");
    assert_eq!(model.total, 1);
}
