use super::{GitFacts, file_lines};

use crate::encode::{One, Optional};
use crate::models::nav::{BranchUpstream, CardRecord, NewWorktree, RowLead, TagMenu, WorktreeRow};
use platitude_core::offers::{BranchFar, Leads, Reading, TagFar, TreeStanding};

fn tree() -> One<TreeStanding> {
    One::new(TreeStanding {
        branch: "main".into(),
        upstream: "origin/main".into(),
        ..TreeStanding::default()
    })
}

/// The row `topic`, out in the worktree `other` under a lock, with a
/// place for one of its own.
fn held_topic() -> RowLead {
    RowLead {
        kind: "branch".into(),
        full: "topic".into(),
        local: "topic".into(),
        leads: Leads::Holder,
        path: "C:/work/other".into(),
        called: "other".into(),
        card: Some(WorktreeRow {
            name: "other".into(),
            full: "C:/work/other".into(),
            branch: "topic".into(),
            change: crate::models::nav::LOCKED.into(),
            reason: "on a stick".into(),
            head: String::new(),
        }),
        placed: true,
        place: Some(NewWorktree {
            path: "C:/work/repo.worktrees/topic".into(),
            name: "topic".into(),
            ..NewWorktree::default()
        }),
        ..RowLead::default()
    }
}

/// `topic`'s card: tracking `origin/topic`, with a commit of its own.
fn topic_card() -> Optional<CardRecord> {
    Optional::some(CardRecord::Branch(BranchUpstream {
        far: Some(BranchFar {
            reading: Reading::Together,
            tracked: true,
            has_remote: true,
            ahead: 1,
        }),
        counterpart: "origin/topic".into(),
        upstream: "origin/topic".into(),
        ..BranchUpstream::default()
    }))
}

/// One stand answers every row of a ref's menu off the row's lead and
/// its card's record: here a branch another worktree has out, tracking
/// `origin/topic` with a commit of its own.
#[test]
fn a_refs_menu_is_answered_whole_for_one_stand() {
    let lead = held_topic();
    let menu = (*GitFacts.ref_menu(
        "ab".repeat(20),
        true,
        tree(),
        One::new(lead.clone()),
        topic_card(),
        true,
    ))
    .clone();
    assert!(
        menu.switch_to && !menu.switch_asks,
        "the press opens the holder"
    );
    assert!(!menu.delete, "git keeps a branch out elsewhere");
    assert!(menu.delete_remote && menu.set_upstream && menu.unset_upstream);
    assert_eq!(menu.upstream_where, "remote");
    assert_eq!(
        (menu.pull_runs.as_str(), menu.pull_out.as_str()),
        ("aside", "held"),
        "the pull row reads the card's record as the row's own far side"
    );
    assert_eq!(
        (
            menu.making.here,
            menu.making.checkout.as_str(),
            menu.making.out.as_str()
        ),
        (true, "branch", "held")
    );
    assert_eq!(
        (menu.remove_worktree, menu.remove_out.as_str()),
        (true, "locked"),
        "the holder's card, greyed by its lock"
    );
    let press = GitFacts.switch_move(One::new(lead), tree(), true);
    assert_eq!(
        (press.action.as_str(), press.checkout.clone()),
        ("holder", None),
        "the press opens the holder and sends no checkout"
    );
}

/// The row's commit is read as an object name: an empty one and a
/// malformed one are alike a row with none, which makes no branch and
/// no worktree there; a commit makes both.
#[test]
fn a_ref_menu_reads_its_commit_as_an_object_name() {
    let menu_on = |oid: &str| {
        let menu = GitFacts.ref_menu(
            oid.to_string(),
            true,
            tree(),
            One::new(held_topic()),
            topic_card(),
            true,
        );
        (menu.branch_here, menu.making.here)
    };
    assert_eq!(menu_on(&"ab".repeat(20)), (true, true));
    assert_eq!(menu_on(""), (false, false), "no commit");
    assert_eq!(
        menu_on("not-an-object"),
        (false, false),
        "a malformed name is none"
    );
}

/// A tag's row reads its card's record as its far side; a closed tab
/// offers nothing that sends; an operation stopped in this tree greys
/// a new worktree on the branch it holds.
#[test]
fn a_ref_menu_reads_a_tag_card_a_closed_tab_and_a_stopped_tree() {
    let commit = || "ab".repeat(20);
    let tag_pushes = |push_remote: &str| {
        let lead = RowLead {
            kind: "tag".into(),
            full: "v1".into(),
            ..RowLead::default()
        };
        let card = Optional::some(CardRecord::Tag(TagMenu {
            far: TagFar {
                push_remote: push_remote.into(),
                ..TagFar::default()
            },
            ..TagMenu::default()
        }));
        GitFacts
            .ref_menu(commit(), true, tree(), One::new(lead), card, false)
            .push_tag
    };
    assert!(tag_pushes("origin"), "a remote to send the tag to");
    assert!(!tag_pushes(""), "no remote: no push row");

    // Out nowhere, with a place to make a worktree for it.
    let topic = RowLead {
        kind: "branch".into(),
        full: "topic".into(),
        local: "topic".into(),
        placed: true,
        place: held_topic().place,
        ..RowLead::default()
    };
    let menu = |open: bool, lead: RowLead| {
        (*GitFacts.ref_menu(commit(), open, tree(), One::new(lead), topic_card(), true)).clone()
    };
    let open = menu(true, topic.clone());
    assert_eq!((open.integrate, open.making.here), (true, true));
    let closed = menu(false, topic.clone());
    assert!(
        !closed.integrate && !closed.making.here,
        "nothing is sent from a closed tab"
    );
    let stopped = menu(
        true,
        RowLead {
            stopped_by: "rebase".into(),
            ..topic
        },
    );
    assert_eq!(stopped.making.out, "stopped");
}

/// The same row with no worktree card, no place to make one and no
/// card record: nothing to make, take away or read over there.
#[test]
fn a_row_with_nowhere_to_go_is_offered_nothing_it_could_not_do() {
    let nowhere = RowLead {
        card: None,
        placed: false,
        place: None,
        ..held_topic()
    };
    let unplaced = GitFacts.ref_menu(
        "ab".repeat(20),
        true,
        tree(),
        One::new(nowhere),
        Optional::none(),
        true,
    );
    assert_eq!(
        (
            unplaced.upstream_where.as_str(),
            unplaced.pull_runs.as_str()
        ),
        ("", ""),
        "no card record, nothing read over there"
    );
    assert_eq!(
        (
            unplaced.making.here,
            unplaced.making.checkout.as_str(),
            unplaced.remove_worktree
        ),
        (false, "", false),
        "nowhere to make one, and no worktree to take away"
    );
}

/// A remote branch's name is cut by the remote that owns it, the
/// longest winning; with none owning it, at the first slash; and a name
/// no slash divides is all branch, on no remote.
#[test]
fn a_remote_branchs_name_is_cut_by_its_owner_then_the_first_slash() {
    let remotes = || vec!["origin".to_string(), "my/fork".to_string()];
    let cut = |full: &str| {
        let far = GitFacts.remote_branch_of(full.to_string(), remotes());
        (far.remote.clone(), far.branch.clone())
    };
    let pair = |remote: &str, branch: &str| (remote.to_string(), branch.to_string());
    assert_eq!(cut("my/fork/feature/x"), pair("my/fork", "feature/x"));
    assert_eq!(cut("origin/main"), pair("origin", "main"));
    assert_eq!(
        cut("gone/feature/x"),
        pair("gone", "feature/x"),
        "no owner: the first slash"
    );
    assert_eq!(cut("main"), pair("", "main"), "no slash: no remote");
}

#[test]
fn the_lines_named_start_at_line_one_and_run_forward() {
    assert_eq!(file_lines(7, 7), Some(7..=7));
    assert_eq!(file_lines(10, 20), Some(10..=20));
    assert_eq!(
        file_lines(10, 3),
        Some(10..=10),
        "a last before the first is the first"
    );
    assert_eq!(file_lines(3, 0), Some(3..=3), "as is a last of 0");
    assert_eq!(file_lines(0, 3), None, "there is no line 0");
    assert_eq!(file_lines(-1, 3), None);
    assert_eq!(file_lines(3, -1), None);
}
