//! Tests of the sidebar row's field answers.

use super::testkit::*;
use super::*;

#[test]
fn a_tag_row_reads_out_of_the_snapshot() {
    let mut model = section(
        "tags",
        Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", true, true), tag("v2.0-theirs", true, false)],
        )),
    );
    model.arrange();

    assert_eq!(model.shown_rows(), 2);
    assert_eq!(says(&model, 0, Role::Name), "v1.0");
    // A tag never went through the tree, so it has no full name and
    // the sidebar keys it by what it shows.
    assert_eq!(says(&model, 0, Role::Full), "");
    assert_eq!(says(&model, 0, Role::OidHex), oid("a").to_hex());
    assert!(flags(&model, 0, Role::HasRemote));
    assert!(!flags(&model, 0, Role::OnlyRemote));
    assert!(flags(&model, 1, Role::OnlyRemote));
    assert!(!flags(&model, 1, Role::Folder));
    assert_eq!(depth_of(&model, 1), 0);
}

/// Asked by name **and by the push's destination**: a name two remotes
/// disagree about has one answer per remote, and only the destination's
/// bears on the tag menu's push row.
#[test]
fn a_drifted_tag_answers_for_the_remote_the_push_would_go_to() {
    let mut model = section(
        "tags",
        Source::Tags(drifted(
            Vec::new(),
            vec![tag("v1.0", true, true), tag("v2.0", true, true)],
            vec![
                drift("v1.0", "mirror", "c"),
                drift("v1.0", "origin", "b"),
                drift("v2.0", "mirror", "d"),
            ],
        )),
    );
    model.arrange();

    assert_eq!(
        model.remote_tag_drift("v1.0".into(), "origin".into()),
        oid("b").to_hex(),
        "the commit the lease has to be pinned to"
    );
    assert_eq!(
        model.remote_tag_drift("v1.0".into(), "mirror".into()),
        oid("c").to_hex(),
        "a second remote is a second answer, not the same one"
    );
    assert_eq!(
        model.remote_tag_drift("v2.0".into(), "origin".into()),
        "",
        "the destination agrees, whatever some other remote says"
    );
    assert_eq!(
        model.remote_tag_drift("v3.0".into(), "origin".into()),
        "",
        "no remote carries the name at all"
    );
    // The row is decided as the menu opens, so a half-formed question
    // has to answer "no drift".
    assert_eq!(model.remote_tag_drift("v1.0".into(), String::new()), "");
    assert_eq!(model.remote_tag_drift(String::new(), "origin".into()), "");
}

/// The branches and stashes are asked the same question by the same
/// shared row.
#[test]
fn only_the_tags_section_answers_for_a_drift() {
    let mut model = section("branches", Source::Locals(snapshot(Vec::new(), Vec::new())));
    model.arrange();
    assert_eq!(model.remote_tag_drift("v1.0".into(), "origin".into()), "");
    assert_eq!(model.tag_sides("v1.0".into()), "");
    assert!(model.tag_remotes("v1.0".into(), "origin".into()).is_empty());
}

/// Each carrier is marked against the remote this window's tag rows act
/// on, not the copy here — which reading is right is not a question the
/// commits answer (デザイン規約 §左メニューの所作 の TAGS の段).
#[test]
fn a_tag_row_opens_on_its_carriers_and_says_which_stand_apart() {
    let mut model = section(
        "tags",
        Source::Tags(carried(
            Vec::new(),
            vec![
                tag("v-agreed", true, true),
                tag("v-apart", true, true),
                tag("v-fork", true, true),
                tag("v-here", false, true),
            ],
            Vec::new(),
            vec![
                // Both remotes on one commit: nobody stands apart…
                ("v-agreed", "fork", "a"),
                ("v-agreed", "origin", "a"),
                // …and here the fork stands apart from the reference.
                ("v-apart", "fork", "b"),
                ("v-apart", "origin", "a"),
                ("v-fork", "fork", "b"),
            ],
        )),
    );
    model.arrange();
    let said = |name: &str, against: &str| {
        model
            .tag_remotes(name.into(), against.into())
            .iter()
            .map(|carrier| format!("{}:{}", carrier.remote, u8::from(carrier.apart)))
            .collect::<Vec<_>>()
            .join(",")
    };

    assert_eq!(said("v-agreed", "origin"), "fork:0,origin:0", "name order");
    assert_eq!(
        said("v-apart", "origin"),
        "fork:1,origin:0",
        "the reading that is not the reference's is the one marked"
    );
    assert_eq!(
        said("v-apart", "fork"),
        "fork:0,origin:1",
        "read against the other one, the mark moves with it"
    );
    assert_eq!(
        said("v-fork", "origin"),
        "fork:0",
        "nobody stands apart from a reading the reference has not got"
    );
    // Empty, not one empty record: the row with no lines under it is
    // drawn from that.
    assert!(
        model
            .tag_remotes("v-here".into(), "origin".into())
            .is_empty()
    );
    assert!(
        model
            .tag_remotes("v-nobody".into(), "origin".into())
            .is_empty()
    );
    assert!(model.tag_remotes(String::new(), "origin".into()).is_empty());
}

/// The same test for one reading by the commit it stands on: the copy
/// here is asked with its own, a graph row's reading with the row's, and
/// both are weighed against the reference alone.
#[test]
fn a_reading_of_a_tag_is_apart_by_its_commit() {
    let mut model = section(
        "tags",
        Source::Tags(carried(
            Vec::new(),
            // Every tag here stands on `a`.
            vec![tag("v-agreed", true, true), tag("v-moved", true, true)],
            Vec::new(),
            vec![
                ("v-agreed", "origin", "a"),
                ("v-agreed", "fork", "b"),
                ("v-moved", "origin", "b"),
            ],
        )),
    );
    model.arrange();
    let apart =
        |name: &str, at: &str| model.tag_apart_at(name.into(), oid(at).to_hex(), "origin".into());

    assert!(
        !apart("v-agreed", "a"),
        "the copy here stands where origin does"
    );
    assert!(apart("v-agreed", "b"), "the fork's reading does not");
    assert!(
        apart("v-moved", "a"),
        "the copy here, moved off origin's commit"
    );
    assert!(!apart("v-moved", "b"), "the reference's own reading");
    // Decided as a row opens, so a half-formed question answers "no".
    assert!(!model.tag_apart_at("v-moved".into(), "not an id".into(), "origin".into()));
    // With no remote to act on, the remotes decide as they would without
    // the reference among them — the same answer the lines are marked by.
    assert!(model.tag_apart_at("v-moved".into(), oid("a").to_hex(), String::new()));
    assert_eq!(
        model
            .tag_remotes("v-moved".into(), String::new())
            .iter()
            .map(|carrier| carrier.apart)
            .collect::<Vec<_>>(),
        [false],
        "origin's own reading, the only one"
    );
}

/// What the menus and notes of a tag are read off, through the section's
/// slots: who decides, where a delete reaches and why it greys, and which
/// remote a chip names on its own. Every tag here stands on `a`.
#[test]
fn a_tags_menu_and_note_come_off_the_readings() {
    let mut model = section(
        "tags",
        Source::Tags(carried(
            Vec::new(),
            vec![tag("v-pair", true, true), tag("v-split", true, true)],
            Vec::new(),
            vec![
                ("v-pair", "fork", "b"),
                ("v-pair", "mirror", "b"),
                ("v-split", "fork", "a"),
                ("v-split", "mirror", "c"),
            ],
        )),
    );
    model.arrange();

    assert_eq!(
        model.tag_weighed_against("v-pair".into(), "origin".into()),
        "fork, mirror",
        "remotes that agree, named as a sentence lists them"
    );
    assert_eq!(
        model.tag_weighed_against("v-split".into(), "origin".into()),
        ""
    );

    let pair = model.tag_menu("v-pair".into(), "origin".into(), String::new());
    assert_eq!(pair.push_remote, "origin");
    assert_eq!(pair.reach, "");
    assert_eq!(pair.held_back, "unnamed");
    assert_eq!(pair.carriers, "fork, mirror");
    let aimed = model.tag_menu("v-pair".into(), "origin".into(), "mirror".into());
    assert_eq!(aimed.push_remote, "mirror");
    assert_eq!(aimed.reach, "mirror");
    assert_eq!(aimed.held_back, "");
    assert_eq!(
        aimed.lease,
        oid("b").to_hex(),
        "mirror has it elsewhere than here"
    );

    assert_eq!(
        model.tag_aim_at("v-split".into(), oid("c").to_hex()),
        "mirror"
    );
    assert_eq!(
        model.tag_aim_at("v-split".into(), oid("a").to_hex()),
        "",
        "that chip is the copy here"
    );
    assert_eq!(model.tag_aim_at("v-pair".into(), oid("b").to_hex()), "");
}

/// Which sides a tag's name stands on — what tells the three delete rows
/// apart, since one row of TAGS carries both (`offers::TagSides`).
#[test]
fn a_tag_row_says_which_sides_its_name_stands_on() {
    let mut model = section(
        "tags",
        Source::Tags(snapshot(
            Vec::new(),
            vec![
                tag("v-here", false, true),
                tag("v-both", true, true),
                tag("v-theirs", true, false),
            ],
        )),
    );
    model.arrange();

    assert_eq!(model.tag_sides("v-here".into()), "here");
    assert_eq!(model.tag_sides("v-both".into()), "both");
    assert_eq!(model.tag_sides("v-theirs".into()), "remote");
    // Empty, not `here`, or a section still loading would offer the
    // everyday delete on a tag nobody has.
    assert_eq!(model.tag_sides("v-nobody".into()), "");
    assert_eq!(model.tag_sides(String::new()), "");
}
#[test]
fn a_file_row_reads_out_of_the_status() {
    let mut model = section("worktree", Source::files(pending()));
    model.tree_view = false;
    model.arrange();

    assert_eq!(model.shown_rows(), 5, "the MM entry is a row on each side");
    let row = |at| {
        (
            says(&model, at, Role::Name),
            says(&model, at, Role::Full),
            says(&model, at, Role::Change),
            says(&model, at, Role::Bucket),
            says(&model, at, Role::Group),
        )
    };
    assert_eq!(
        row(0),
        (
            "a.txt".into(),
            "a.txt".into(),
            "UU".into(),
            "conflicts".into(),
            "conflicts".into()
        )
    );
    // Untracked routes as itself and shows in the unstaged run —
    // among the unstaged files by name.
    assert_eq!(
        row(1),
        (
            "c.txt".into(),
            "c.txt".into(),
            "?".into(),
            "untracked".into(),
            "unstaged".into()
        )
    );
    assert_eq!(
        row(2),
        (
            "src/b.txt".into(),
            "src/b.txt".into(),
            "M".into(),
            "unstaged".into(),
            "unstaged".into()
        )
    );
    assert_eq!(
        row(4),
        (
            "src/b.txt".into(),
            "src/b.txt".into(),
            "M".into(),
            "staged".into(),
            "staged".into()
        )
    );
    assert_eq!(says(&model, 3, Role::OrigPath), "old.txt");
    // A path asked for by name answers with its unstaged half.
    assert_eq!(model.change_of("src/b.txt".to_string()), "M");
    assert_eq!(model.change_of("a.txt".to_string()), "UU");
}
#[test]
fn the_short_sections_read_out_of_what_arrived() {
    let mut model = section(
        "stashes",
        Source::Stashes(vec![platitude_core::stash::StashEntry {
            name: "stash@{0}".to_string(),
            oid: oid("c"),
            time: 0,
            message: "On main: a thing".to_string(),
        }]),
    );
    model.arrange();
    assert_eq!(says(&model, 0, Role::Name), "On main: a thing");
    assert_eq!(says(&model, 0, Role::Full), "stash@{0}");
    assert_eq!(says(&model, 0, Role::OidHex), oid("c").to_hex());

    let entry = |path: &str, branch: Option<&str>, head: Option<&str>| {
        platitude_core::worktrees::WorktreeEntry {
            path: path.to_string(),
            branch: branch.map(str::to_string),
            head_hex: head.map(|digit| oid(digit).to_hex()),
            // The one entry git lists with no commit out is the bare
            // one, and it carries no branch either.
            bare: head.is_none(),
            detached: false,
            locked: false,
            lock_reason: String::new(),
            prunable: false,
            prune_reason: String::new(),
            // A real listing always opens with the repository's own copy.
            main: path.ends_with("repo"),
        }
    };
    let mut model = section(
        "worktrees",
        Source::Worktrees {
            list: vec![
                entry("C:\\work\\repo", Some("main"), Some("a")),
                entry("C:\\work\\other", Some("topic"), Some("b")),
                entry("C:\\work\\bare", None, None),
            ],
            current: "c:/work/other".to_string(),
        },
    );
    model.arrange();
    assert_eq!(says(&model, 0, Role::Name), "repo");
    assert_eq!(says(&model, 0, Role::Full), "C:\\work\\repo");
    assert_eq!(says(&model, 0, Role::Bucket), "main");
    assert!(!flags(&model, 0, Role::IsHead));
    // The one this window is showing is marked, however git spelled it.
    assert!(flags(&model, 1, Role::IsHead));
    assert_eq!(model.head_row, 1);
    // Where a click on the row lands (デザイン規約 §左メニューの所作).
    assert_eq!(says(&model, 0, Role::OidHex), oid("a").to_hex());
    assert_eq!(says(&model, 1, Role::OidHex), oid("b").to_hex());
    assert_eq!(says(&model, 2, Role::OidHex), "");
}

/// The mark and the words both come out of slots shared with the other
/// kinds, so a change to either would draw the wrong mark
/// (`item::LOCKED`).
#[test]
fn a_worktree_row_wears_the_state_of_its_checkout() {
    let entry = |path: &str, locked: bool, reason: &str, prunable: bool| {
        platitude_core::worktrees::WorktreeEntry {
            path: path.to_string(),
            branch: Some("topic".to_string()),
            head_hex: None,
            bare: false,
            detached: false,
            locked,
            lock_reason: reason.to_string(),
            prunable,
            prune_reason: if prunable {
                "gitdir file points to non-existent location".to_string()
            } else {
                String::new()
            },
            main: path.ends_with("home"),
        }
    };
    let mut model = section(
        "worktrees",
        Source::Worktrees {
            list: vec![
                entry("C:\\work\\home", false, "", false),
                entry("C:\\work\\plain", false, "", false),
                entry("C:\\work\\held", true, "release run", false),
                entry("C:\\work\\quiet", true, "", false),
                entry("C:\\work\\gone", false, "", true),
                // git can report both; the lock wins (`item::LOCKED`).
                entry("C:\\work\\both", true, "release run", true),
            ],
            current: String::new(),
        },
    );
    model.arrange();
    // The main copy wears the house and has no words: nobody took it.
    assert_eq!(says(&model, 0, Role::Change), "MAIN");
    assert_eq!(says(&model, 0, Role::OrigPath), "");
    assert_eq!(says(&model, 1, Role::Change), "");
    assert_eq!(says(&model, 1, Role::OrigPath), "");
    assert_eq!(says(&model, 2, Role::Change), "LOCKED");
    assert_eq!(says(&model, 2, Role::OrigPath), "release run");
    // A lock given no reason is still a lock.
    assert_eq!(says(&model, 3, Role::Change), "LOCKED");
    assert_eq!(says(&model, 3, Role::OrigPath), "");
    assert_eq!(says(&model, 4, Role::Change), "PRUNABLE");
    assert_eq!(
        says(&model, 4, Role::OrigPath),
        "gitdir file points to non-existent location"
    );
    assert_eq!(says(&model, 5, Role::Change), "LOCKED");
    assert_eq!(says(&model, 5, Role::OrigPath), "release run");
}

/// The reading a branch names carries the branch's pair back, for the
/// line naming that branch (`NavRowFacts`, デザイン規約 §左メニューの所作).
#[test]
fn a_branch_and_the_reading_it_names_answer_the_same_pair() {
    let mut ours = local("main", true);
    ours.ahead = 2;
    ours.behind = 1;
    let mut mine = section("branches", Source::Locals(locals(vec![ours])));
    mine.arrange();
    assert_eq!(numbers(&mine, 0, Role::Ahead), 2);
    assert_eq!(numbers(&mine, 0, Role::Behind), 1);

    let mut read = remote("origin/main");
    read.tracked_by = "main".into();
    read.ahead = 2;
    read.behind = 1;
    let mut theirs = section(
        "remotes",
        // Name order, the way the join hands the list over: the lookups
        // are a binary search over it (`RefsSnapshot::remote_named`).
        Source::Remotes(snapshot(vec![remote("fork/main"), read], Vec::new())),
    );
    // Filtered, so each reading sits right under its remote's row
    // (`build_remote_groups`) rather than under name folders.
    theirs.filter = "main".to_string();
    theirs.arrange();
    assert_eq!(says(&theirs, 0, Role::Name), "fork");
    assert_eq!(says(&theirs, 1, Role::Full), "fork/main");
    assert_eq!(
        numbers(&theirs, 1, Role::Ahead),
        0,
        "a name alone joins nothing"
    );
    assert_eq!(says(&theirs, 2, Role::Name), "origin");
    assert_eq!(says(&theirs, 3, Role::Full), "origin/main");
    assert_eq!(numbers(&theirs, 3, Role::Ahead), 2);
    assert_eq!(numbers(&theirs, 3, Role::Behind), 1);
    assert_eq!(theirs.tracked_by("origin/main".to_string()), "main");
    assert_eq!(theirs.tracked_by("fork/main".to_string()), "");
    assert_eq!(
        theirs.tracked_by("nobody".to_string()),
        "",
        "a name this section does not carry"
    );
}

/// One field, one mark, whichever section the row is in (`item::HELD`).
#[test]
fn a_branch_another_copy_holds_wears_the_state_in_the_shared_slot() {
    let local = |short: &str, held: bool| platitude_core::session::BranchItem {
        short: short.into(),
        full: format!("refs/heads/{short}").into(),
        oid: oid("a"),
        has_remote: false,
        is_head: false,
        upstream: "".into(),
        upstream_gone: "".into(),
        upstream_oid: None,
        upstream_drifted: false,
        held_elsewhere: held,
        tracked_by: "".into(),
        ahead: 0,
        behind: 0,
    };
    let mut snap = platitude_core::session::RefsSnapshot {
        locals: vec![local("main", false), local("feature/topic-a", true)],
        remotes: Vec::new(),
        tags: Vec::new(),
        tags_by_name: Vec::new(),
        tag_drifts: Vec::new(),
        remote_tags: Arc::default(),
        head: None,
        remote_names: Vec::new(),
        remote_urls: Vec::new(),
        push_default: None,
        checkout_default: None,
    };
    snap.locals.sort_by(|a, b| a.short.cmp(&b.short));
    let mut model = section("branches", Source::Locals(std::sync::Arc::new(snap)));
    model.arrange();
    assert_eq!(model.oid_of_name("main".to_string()), oid("a").to_hex());
    assert_eq!(
        model.told(Role::Name, "feature/topic-a", Role::Change),
        "HELD"
    );
    assert_eq!(model.told(Role::Name, "main", Role::Change), "");
}

/// By name, not flag, so the badge and the line it opens cannot disagree
/// (デザイン規約 §左メニューの所作).
#[test]
fn a_branch_carries_the_upstream_that_is_not_here_in_the_shared_slot() {
    let local = |short: &str, gone: &str| platitude_core::session::BranchItem {
        short: short.into(),
        full: format!("refs/heads/{short}").into(),
        oid: oid("a"),
        // The far side deleted it, so no remote-tracking ref sets this.
        has_remote: false,
        is_head: false,
        upstream: "".into(),
        upstream_gone: gone.into(),
        upstream_oid: None,
        upstream_drifted: false,
        held_elsewhere: false,
        tracked_by: "".into(),
        ahead: 0,
        behind: 0,
    };
    let mut snap = platitude_core::session::RefsSnapshot {
        locals: vec![
            local("main", ""),
            local("release-1.2", "origin/release-1.2"),
        ],
        remotes: Vec::new(),
        tags: Vec::new(),
        tags_by_name: Vec::new(),
        tag_drifts: Vec::new(),
        remote_tags: Arc::default(),
        head: None,
        remote_names: Vec::new(),
        remote_urls: Vec::new(),
        push_default: None,
        checkout_default: None,
    };
    snap.locals.sort_by(|a, b| a.short.cmp(&b.short));
    let mut model = section("branches", Source::Locals(std::sync::Arc::new(snap)));
    model.arrange();
    assert_eq!(
        model.told(Role::Name, "release-1.2", Role::Bucket),
        "origin/release-1.2"
    );
    // An upstream that is here leaves it empty; the ordinary badge comes
    // off `has_remote`.
    assert_eq!(model.told(Role::Name, "main", Role::Bucket), "");
}

/// What the `switch` / `branch --delete` rows ask before offering
/// (`worktree_holding`).
#[test]
fn the_worktree_holding_a_branch_answers_for_every_other_copy() {
    let entry = |path: &str, branch: Option<&str>| platitude_core::worktrees::WorktreeEntry {
        path: path.to_string(),
        branch: branch.map(str::to_string),
        head_hex: None,
        bare: false,
        detached: false,
        locked: false,
        lock_reason: String::new(),
        prunable: false,
        prune_reason: String::new(),
        main: path.ends_with("repo"),
    };
    let mut model = section(
        "worktrees",
        Source::Worktrees {
            list: vec![
                entry("C:\\work\\repo", Some("main")),
                entry("C:\\work\\other", Some("topic")),
                entry("C:\\work\\loose", None),
            ],
            current: "c:/work/repo".to_string(),
        },
    );
    model.arrange();
    assert_eq!(
        model.worktree_holding("topic".to_string()),
        "C:\\work\\other"
    );
    // The copy this window is in refuses nothing — moving onto the
    // branch it already has out is a no-op.
    assert_eq!(model.worktree_holding("main".to_string()), "");
    assert_eq!(model.worktree_holding("nobody".to_string()), "");
    // A detached row names no branch; the empty string finds nothing.
    assert_eq!(model.worktree_holding(String::new()), "");
}
