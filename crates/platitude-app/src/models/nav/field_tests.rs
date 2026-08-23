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
    // has to answer "no drift" rather than the first entry in the run.
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
    // among the unstaged files by name, not after them.
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

    let entry = |path: &str, branch: &str| platitude_core::worktrees::WorktreeEntry {
        path: path.to_string(),
        branch: Some(branch.to_string()),
        head_hex: None,
        bare: false,
        detached: false,
        locked: false,
        lock_reason: String::new(),
        prunable: false,
        prune_reason: String::new(),
    };
    let mut model = section(
        "worktrees",
        Source::Worktrees {
            list: vec![
                entry("C:\\work\\repo", "main"),
                entry("C:\\work\\other", "topic"),
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
}

/// The seat a worktree row opens with, and the words behind it — both
/// out of slots the row shares with the other kinds, so a change to
/// either would draw the wrong mark rather than fail (`item::LOCKED`).
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
        }
    };
    let mut model = section(
        "worktrees",
        Source::Worktrees {
            list: vec![
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
    assert_eq!(says(&model, 0, Role::Change), "");
    assert_eq!(says(&model, 0, Role::OrigPath), "");
    assert_eq!(says(&model, 1, Role::Change), "LOCKED");
    assert_eq!(says(&model, 1, Role::OrigPath), "release run");
    // A lock taken without a reason is still a lock: the mark comes
    // out and there is nothing to say beside it.
    assert_eq!(says(&model, 2, Role::Change), "LOCKED");
    assert_eq!(says(&model, 2, Role::OrigPath), "");
    assert_eq!(says(&model, 3, Role::Change), "PRUNABLE");
    assert_eq!(
        says(&model, 3, Role::OrigPath),
        "gitdir file points to non-existent location"
    );
    assert_eq!(says(&model, 4, Role::Change), "LOCKED");
    assert_eq!(says(&model, 4, Role::OrigPath), "release run");
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
        held_elsewhere: held,
    };
    let mut snap = platitude_core::session::RefsSnapshot {
        locals: vec![local("main", false), local("feature/topic-a", true)],
        remotes: Vec::new(),
        tags: Vec::new(),
        tag_drifts: Vec::new(),
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
    // branch it already has out is a no-op, not a refusal.
    assert_eq!(model.worktree_holding("main".to_string()), "");
    assert_eq!(model.worktree_holding("nobody".to_string()), "");
    // A detached row names no branch, and the empty string must not
    // find it.
    assert_eq!(model.worktree_holding(String::new()), "");
}
