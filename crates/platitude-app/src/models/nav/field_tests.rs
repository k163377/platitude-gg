//! Tests of the sidebar row's field answers, in a file of their own
//! (structure.md: a file whose bulk is tests lifts them to a sibling).

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

/// What decides the shape of the tag menu's push row, asked the way the
/// menu asks it: by name **and by the remote the push is going to**. A
/// name two remotes disagree about has one answer per remote, and the
/// destination is the only one that bears on the push.
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

/// A section that holds no readings answers nothing — the branches and
/// the stashes are asked the same question by the same shared row.
#[test]
fn only_the_tags_section_answers_for_a_drift() {
    let mut model = section("branches", Source::Locals(snapshot(Vec::new(), Vec::new())));
    model.arrange();
    assert_eq!(model.remote_tag_drift("v1.0".into(), "origin".into()), "");
    assert_eq!(model.tag_sides("v1.0".into()), "");
    assert_eq!(model.tag_remotes("v1.0".into(), "origin".into()), "");
}

/// The remotes a tag's row opens on, in name order and each with whether
/// it stands where the one this window's tag rows act on has the tag
/// (デザイン規約 §左メニューの所作 の TAGS の段). **The mark is about that
/// reading and not about the copy here**: which of them is the one is not
/// a question the commits answer.
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
                // Both remotes on the one commit the testkit gives a
                // reading, so nobody stands apart…
                ("v-agreed", "fork", "a"),
                ("v-agreed", "origin", "a"),
                // …and here the fork has it somewhere else than the
                // reference does.
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
            .split('\u{1f}')
            .map(|record| record.replace('\u{1e}', ":"))
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
    // **Empty, not a list of one empty record**: a tag nobody out there
    // has opens on nothing at all, which is what the row with no lines
    // under it is drawn from.
    assert_eq!(model.tag_remotes("v-here".into(), "origin".into()), "");
    assert_eq!(model.tag_remotes("v-nobody".into(), "origin".into()), "");
    assert_eq!(model.tag_remotes(String::new(), "origin".into()), "");
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
    // **Empty.** A name no row carries has to be told from one that is
    // only local, or a section still loading would offer the everyday
    // delete on a tag nobody has (`tag_named` answers `None`).
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
    // The unstaged half is what a path asked for by name answers with,
    // as it did when the rows were built.
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
            // git opens the listing with the repository's own copy, so
            // a fixture where none of them is it could not be read off
            // a real one.
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
    // Where a click on the row lands: the commit that checkout is
    // standing on, the same slot a branch row answers a jump out of
    // (デザイン規約 §左メニューの所作).
    assert_eq!(says(&model, 0, Role::OidHex), oid("a").to_hex());
    assert_eq!(says(&model, 1, Role::OidHex), oid("b").to_hex());
    assert_eq!(says(&model, 2, Role::OidHex), "");
}

/// The seat a worktree row opens with, and the words behind it — both
/// out of slots the row shares with the other kinds, so a change to
/// either would draw the wrong mark (`item::LOCKED`).
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
                // git reports both on one entry; the lock is the one
                // somebody chose, so it is the one the seat shows.
                entry("C:\\work\\both", true, "release run", true),
            ],
            current: String::new(),
        },
    );
    model.arrange();
    // The repository's own working copy, which wears the house — and
    // has no words behind it: being the main copy is not a state
    // somebody took, so there is nothing to explain.
    assert_eq!(says(&model, 0, Role::Change), "MAIN");
    assert_eq!(says(&model, 0, Role::OrigPath), "");
    assert_eq!(says(&model, 1, Role::Change), "");
    assert_eq!(says(&model, 1, Role::OrigPath), "");
    assert_eq!(says(&model, 2, Role::Change), "LOCKED");
    assert_eq!(says(&model, 2, Role::OrigPath), "release run");
    // A lock taken without a reason is still a lock: the mark comes
    // out and there is nothing to say beside it.
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

/// One measurement, answered from either end: a branch row carries its
/// own, and the reading it names carries the same pair back — which is
/// what the line naming that branch draws beside it
/// (`NavRowFacts`, デザイン規約 §左メニューの所作). A reading nothing
/// here names has no measurement to report.
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
    // Flattened, so the rows addressed here are the readings and not the
    // folders they fold under.
    theirs.filter = "main".to_string();
    theirs.arrange();
    assert_eq!(says(&theirs, 0, Role::Name), "fork/main");
    assert_eq!(
        numbers(&theirs, 0, Role::Ahead),
        0,
        "a name alone joins nothing"
    );
    assert_eq!(says(&theirs, 1, Role::Name), "origin/main");
    assert_eq!(numbers(&theirs, 1, Role::Ahead), 2);
    assert_eq!(numbers(&theirs, 1, Role::Behind), 1);
    assert_eq!(theirs.tracked_by("origin/main".to_string()), "main");
    assert_eq!(theirs.tracked_by("fork/main".to_string()), "");
    assert_eq!(
        theirs.tracked_by("nobody".to_string()),
        "",
        "a name this section does not carry"
    );
}

/// A branch row wears the state in the same slot the worktree rows
/// use, so the seat draws one mark from one field whichever section
/// the row is in (`item::HELD`).
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

/// The upstream a branch is measured against and cannot reach rides
/// the row in the slot a local branch has no bucket for, as the name
/// rather than a flag: the row draws the badge's state from it and the
/// line it opens says it, so the two cannot disagree
/// (デザイン規約 §左メニューの所作).
#[test]
fn a_branch_carries_the_upstream_that_is_not_here_in_the_shared_slot() {
    let local = |short: &str, gone: &str| platitude_core::session::BranchItem {
        short: short.into(),
        full: format!("refs/heads/{short}").into(),
        oid: oid("a"),
        // The far side deleted it, so no remote-tracking ref is left to
        // set this — which is the whole reason the row cannot say it.
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
    };
    snap.locals.sort_by(|a, b| a.short.cmp(&b.short));
    let mut model = section("branches", Source::Locals(std::sync::Arc::new(snap)));
    model.arrange();
    assert_eq!(
        model.told(Role::Name, "release-1.2", Role::Bucket),
        "origin/release-1.2"
    );
    // A branch whose upstream is here leaves the slot empty — the badge
    // it wears is the ordinary one, drawn off `has_remote`.
    assert_eq!(model.told(Role::Name, "main", Role::Bucket), "");
}

/// What the rows that would run `switch` or `branch --delete` ask
/// before offering: git refuses both for a branch another worktree
/// holds, and answers nothing about the copy this window is in.
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
    // A detached row names no branch, and the empty string finds
    // nothing.
    assert_eq!(model.worktree_holding(String::new()), "");
}
