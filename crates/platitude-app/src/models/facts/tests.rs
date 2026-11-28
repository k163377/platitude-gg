use super::{GitFacts, file_lines};

use crate::encode::{MenuAnswer, One, Optional, PushRecord};
use crate::models::nav::{BranchUpstream, CardRecord, NewWorktree, RowLead, TagMenu, WorktreeRow};
use platitude_core::menu::{Nowhere, Placing};
use platitude_core::offers::{BranchFar, Leads, TagFar, Tracks, TreeStanding};
use platitude_core::worktrees::{Keeper, KeptBranch};

/// On `main` at a commit of its own, tracking `origin/main`.
fn tree() -> One<TreeStanding> {
    One::new(TreeStanding {
        branch: "main".into(),
        head_oid: "2222222222222222222222222222222222222222".into(),
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
        placing: Placing::Beside { taken: false },
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
            tracks: Tracks::Together,
            has_remote: true,
            ahead: 1,
        }),
        counterpart: "origin/topic".into(),
        upstream: "origin/topic".into(),
        ..BranchUpstream::default()
    }))
}

/// The menu on one stand of `lead`'s row of the left pane at the commit
/// `ab…`, `card` its card's record and `push` its push.
fn menu_of(lead: RowLead, card: Optional<CardRecord>, push: Optional<PushRecord>) -> MenuAnswer {
    menu_seen("listed", lead, card, push)
}

/// The same, the row right-clicked named by its entrance's word.
fn menu_seen(
    seen: &str,
    lead: RowLead,
    card: Optional<CardRecord>,
    push: Optional<PushRecord>,
) -> MenuAnswer {
    (*GitFacts.menu_on(
        seen.to_string(),
        "ab".repeat(20),
        String::new(),
        true,
        tree(),
        One::new(lead),
        card,
        true,
        push,
    ))
    .clone()
}

/// Why `row` is greyed on `menu` (empty while live); `None` where the
/// menu has no such row.
fn why_of<'a>(menu: &'a MenuAnswer, row: &str) -> Option<&'a str> {
    menu.lines
        .iter()
        .find(|line| line.word == row)
        .map(|line| line.why.as_str())
}

/// The greyed rows of `menu` and why.
fn greyed(menu: &MenuAnswer) -> Vec<(&str, &str)> {
    menu.lines
        .iter()
        .filter(|line| !line.why.is_empty())
        .map(|line| (line.word.as_str(), line.why.as_str()))
        .collect()
}

/// One stand answers every row of a ref's menu off the row's lead and
/// its card's record (`menu::lines`): here a branch another worktree has
/// out under a lock, tracking `origin/topic` with a commit of its own —
/// the rows its holder and the lock grey, and what a press sends.
#[test]
fn a_refs_menu_is_answered_whole_for_one_stand() {
    let lead = held_topic();
    let push = |state: &str| {
        Optional::some(PushRecord {
            state: state.into(),
            ..PushRecord::default()
        })
    };
    let menu = menu_of(lead.clone(), topic_card(), push("ready"));
    assert_eq!(
        menu.words(),
        [
            "branch-here",
            "reach",
            "merge",
            "rebase",
            "pull",
            "push",
            "delete-branch",
            "delete-remote-branch",
            "delete-both",
            "set-upstream",
            "unset-upstream",
            "worktree-here",
            "worktree-add",
            "remove-worktree",
            "tag-here",
        ]
    );
    assert_eq!(
        greyed(&menu),
        [
            ("pull", "held"),
            ("delete-branch", "held"),
            ("delete-both", "held"),
            ("worktree-add", "held"),
            ("remove-worktree", "locked"),
        ]
    );
    assert_eq!(
        (
            menu.switch_asks,
            menu.upstream_where.as_str(),
            menu.pull_runs.as_str(),
            menu.worktree_add.as_str()
        ),
        (false, "remote", "aside", "branch"),
        "the pull row reads the card's record as the row's own far side"
    );
    let clean = menu_of(lead.clone(), topic_card(), push("clean"));
    assert_eq!(why_of(&clean, "push"), Some("clean"), "its push record");
    let unread = menu_of(lead.clone(), topic_card(), Optional::none());
    assert_eq!(
        why_of(&unread, "push"),
        Some("unread"),
        "no record is no answer of where the branch goes"
    );
    let press = GitFacts.switch_move(One::new(lead), tree(), true);
    assert_eq!(
        (press.action.as_str(), press.checkout.clone()),
        ("holder", None),
        "the press opens the holder and sends no checkout"
    );
}

/// What a worktree has out, read off its listing: gone where the listing
/// holds it no more, on no branch where it names none, on another branch
/// where it names another — a rename half read among them
/// (`menu::worktree_left`).
#[test]
fn a_worktrees_listing_reads_into_what_it_has_out() {
    let left =
        |present, out: &str| GitFacts.worktree_left("topic".to_string(), present, out.to_string());
    assert!(left(false, ""), "gone");
    assert!(left(true, ""), "detached");
    assert!(!left(true, "topic"), "still out");
    assert!(left(true, "topic-b"), "a rename half read");
    assert!(left(true, "zz-other"), "switched");
}

/// A graph row's commit the graph says is off the history of HEAD's
/// branch greys its rewrites; where it cannot say, they stay live. In it, a
/// rebase onto it greys down a straight line alone, and a merge either way
/// — the graph's words read as the type's (`menu::History::word`).
#[test]
fn a_graph_row_reads_how_heads_branch_has_it() {
    let why = |history: &str, row: &str| {
        let menu = GitFacts.menu_on(
            "graph".to_string(),
            "ab".repeat(20),
            history.to_string(),
            true,
            tree(),
            One::new(RowLead::default()),
            Optional::none(),
            false,
            Optional::none(),
        );
        why_of(&menu, row).map(str::to_string)
    };
    assert_eq!(why("off", "squash"), Some("off-branch".to_string()));
    assert_eq!(why("straight", "squash"), Some(String::new()));
    assert_eq!(
        why("", "squash"),
        Some(String::new()),
        "the window cannot say"
    );
    assert_eq!(why("straight", "rebase"), Some("in-history".to_string()));
    assert_eq!(
        why("past-merge", "rebase"),
        Some(String::new()),
        "laid again"
    );
    assert_eq!(why("past-merge", "merge"), Some("in-history".to_string()));
    assert_eq!(why("straight", "cherry-pick"), Some(String::new()));
}

/// While the branch HEAD is on has no commit yet, as the tree's reading
/// says it (no commit beside a branch), another branch's row greys its
/// rebase saying so — git has no HEAD to start from — and keeps its
/// merge live: into an unborn branch git takes it to the commit.
#[test]
fn a_branch_row_greys_its_rebase_while_heads_branch_has_no_commit() {
    let menu = |head_oid: &str| {
        let tree = One::new(TreeStanding {
            branch: "topic".into(),
            head_oid: head_oid.into(),
            ..TreeStanding::default()
        });
        let lead = RowLead {
            kind: "branch".into(),
            full: "main".into(),
            local: "main".into(),
            placing: Placing::Beside { taken: false },
            ..RowLead::default()
        };
        (*GitFacts.menu_on(
            "listed".to_string(),
            "ab".repeat(20),
            String::new(),
            true,
            tree,
            One::new(lead),
            Optional::none(),
            false,
            Optional::none(),
        ))
        .clone()
    };
    let unborn = menu("");
    assert_eq!(why_of(&unborn, "rebase"), Some("head-unborn"));
    assert_eq!(why_of(&unborn, "merge"), Some(""));
    let born = menu(&"22".repeat(20));
    assert_eq!(why_of(&born, "rebase"), Some(""));
}

/// The row's commit is read as an object name: an empty one and a
/// malformed one are alike a row with none, whose rows made on a commit
/// grey saying so; a commit makes them live.
#[test]
fn a_ref_menu_reads_its_commit_as_an_object_name() {
    let menu_on = |oid: &str| {
        let menu = GitFacts.menu_on(
            "listed".to_string(),
            oid.to_string(),
            String::new(),
            true,
            tree(),
            One::new(held_topic()),
            topic_card(),
            true,
            Optional::none(),
        );
        (
            why_of(&menu, "branch-here").map(str::to_string),
            why_of(&menu, "worktree-here").map(str::to_string),
        )
    };
    let live = (Some(String::new()), Some(String::new()));
    let none = (Some("no-commits".into()), Some("no-commits".into()));
    assert_eq!(menu_on(&"ab".repeat(20)), live);
    assert_eq!(menu_on(""), none, "no commit");
    assert_eq!(menu_on("not-an-object"), none, "a malformed name is none");
}

/// A graph row naming no ref — or naming one by a kind no menu names
/// itself by — has the rows of its commit, the discard log's entry only
/// what adds; a row of the left pane naming none has no menu, and a word
/// no entrance says answers none.
#[test]
fn a_row_naming_no_ref_has_its_commits_rows() {
    for (kind, full) in [("", ""), ("head", "HEAD")] {
        let lead = || RowLead {
            kind: kind.into(),
            full: full.into(),
            placing: Placing::Beside { taken: false },
            ..RowLead::default()
        };
        let rows = |seen: &str| -> Vec<String> {
            let menu = menu_seen(seen, lead(), Optional::none(), Optional::none());
            menu.words().into_iter().map(String::from).collect()
        };
        assert_eq!(
            rows("graph"),
            [
                "branch-here",
                "cherry-pick",
                "revert",
                "squash",
                "drop",
                "rebase-interactive",
                "merge",
                "rebase",
                "reset-soft",
                "reset-mixed",
                "reset-hard",
                "worktree-here",
                "tag-here"
            ],
            "{kind:?} names no ref"
        );
        assert_eq!(
            rows("provisional"),
            ["branch-here", "cherry-pick", "worktree-here", "tag-here"],
            "{kind:?} on the discard log's entry"
        );
        assert!(rows("listed").is_empty(), "{kind:?} listed");
        assert!(rows("graf").is_empty(), "{kind:?} by no word");
    }
}

/// Another worktree's row has its door and its changes brought here,
/// reading this tree's conflicts off the whole reading and that
/// worktree's off the row's count; a tab still opening greys both rows.
#[test]
fn a_carried_menu_reads_both_sides_conflicts() {
    let menu = |open: bool, here: i32, there: i32| {
        let reading = crate::encode::Reading::new(crate::encode::TreeReading {
            conflict_count: here,
            ..crate::encode::TreeReading::default()
        });
        (*GitFacts.carried_menu(open, reading, there)).clone()
    };
    let clean = menu(true, 0, 0);
    assert_eq!(clean.words(), ["reach", "apply-changes"]);
    assert_eq!(clean.whys(), ["", ""]);
    assert_eq!(
        why_of(&menu(true, 1, 0), "apply-changes"),
        Some("conflicts-here")
    );
    assert_eq!(
        why_of(&menu(true, 0, 2), "apply-changes"),
        Some("conflicts-there")
    );
    let opening = menu(false, 0, 0);
    assert_eq!(opening.words(), ["reach", "apply-changes"]);
    assert_eq!(opening.whys(), ["opening", "opening"]);
}

/// The chosen files' menu has the rows of every section the choice
/// reaches — a word no section has chooses nothing —, greys `stash` off
/// the reading's stash standing, and greys every row before that is
/// read; no file chosen answers no rows.
#[test]
fn a_files_menu_reads_the_sections_chosen_and_the_trees_stash() {
    let menu_in = |open: bool, sections: &[&str], stash: &str| {
        let sections = sections.iter().map(|section| section.to_string()).collect();
        let tree = One::new(crate::encode::TreeReading {
            stash_standing: stash.to_string(),
            ..crate::encode::TreeReading::default()
        });
        (*GitFacts.files_menu(open, sections, tree)).clone()
    };
    let menu = |sections: &[&str], stash: &str| menu_in(true, sections, stash);
    let mixed = menu(&["conflicts", "unstaged"], "conflicts");
    assert_eq!(
        mixed.words(),
        [
            "keep-ours",
            "take-theirs",
            "merge-tool",
            "stash-files",
            "discard"
        ]
    );
    assert_eq!(greyed(&mixed), [("stash-files", "conflicts-here")]);
    assert_eq!(
        menu(&["untracked", "staged"], "ready").words(),
        ["stash-files", "discard"]
    );
    assert_eq!(
        menu(&["conflicts", "elsewhere"], "ready").words(),
        ["keep-ours", "take-theirs", "merge-tool", "stash-files"],
        "a word no section has is no change chosen"
    );
    assert_eq!(
        why_of(&menu(&["unstaged"], "unborn"), "stash-files"),
        Some("no-commits")
    );
    let unread = menu(&["unstaged"], "");
    assert!(
        unread.whys().iter().all(|why| *why == "opening"),
        "{unread:?}"
    );
    let opening = menu_in(false, &["unstaged"], "ready");
    assert_eq!(opening.words(), ["stash-files", "discard"]);
    assert_eq!(
        opening.whys(),
        ["opening", "opening"],
        "a tab still opening"
    );
    assert!(menu(&[], "ready").lines.is_empty());
    assert!(menu(&["elsewhere"], "ready").lines.is_empty());
}

/// The mark greys on the remote the keys already name, and on no other;
/// a tab still opening greys both rows.
#[test]
fn a_remote_menu_greys_the_mark_on_the_marked_remote() {
    let menu = |open: bool, remote: &str, marked: &str| {
        (*GitFacts.remote_menu(open, remote.to_string(), marked.to_string())).clone()
    };
    assert_eq!(
        menu(true, "origin", "").words(),
        ["remote-url", "mark-origin"]
    );
    assert_eq!(
        greyed(&menu(true, "origin", "origin")),
        [("mark-origin", "already-origin")]
    );
    assert!(greyed(&menu(true, "fork", "origin")).is_empty());
    assert!(
        menu(false, "fork", "origin")
            .whys()
            .iter()
            .all(|why| *why == "opening")
    );
}

/// A tag's row reads its card's record as its far side; a closed tab
/// greys every row under one reason; an operation stopped in this tree
/// greys a new worktree on the branch it holds.
#[test]
fn a_ref_menu_reads_a_tag_card_a_closed_tab_and_a_stopped_tree() {
    let tag_push = |push_remote: &str| {
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
        why_of(&menu_of(lead, card, Optional::none()), "push-tag").map(str::to_string)
    };
    assert_eq!(
        tag_push("origin"),
        Some(String::new()),
        "a remote to send to"
    );
    assert_eq!(
        tag_push("").as_deref(),
        Some("no-remote"),
        "no remote: greyed"
    );

    // Out nowhere, with a place to make a worktree for it.
    let topic = RowLead {
        kind: "branch".into(),
        full: "topic".into(),
        local: "topic".into(),
        placing: Placing::Beside { taken: false },
        place: held_topic().place,
        ..RowLead::default()
    };
    let menu = |open: bool, lead: RowLead| {
        (*GitFacts.menu_on(
            "listed".to_string(),
            "ab".repeat(20),
            String::new(),
            open,
            tree(),
            One::new(lead),
            topic_card(),
            true,
            Optional::none(),
        ))
        .clone()
    };
    let open = menu(true, topic.clone());
    assert_eq!(
        (why_of(&open, "merge"), why_of(&open, "worktree-here")),
        (Some(""), Some(""))
    );
    let closed = menu(false, topic.clone());
    assert!(
        !closed.lines.is_empty() && closed.whys().iter().all(|why| *why == "opening"),
        "every row greys while the tab opens: {:?}",
        closed.whys()
    );
    // Kept by a rebase stopped here, as HEAD's reading names it.
    let rebasing = One::new(TreeStanding {
        detached: true,
        kept: Some(KeptBranch {
            branch: "topic".into(),
            by: Keeper::Rebase,
        }),
        ..TreeStanding::default()
    });
    let stopped = (*GitFacts.menu_on(
        "listed".to_string(),
        "ab".repeat(20),
        String::new(),
        true,
        rebasing,
        One::new(topic),
        topic_card(),
        true,
        Optional::none(),
    ))
    .clone();
    assert_eq!(why_of(&stopped, "worktree-add"), Some("rebasing"));
    assert_eq!(why_of(&stopped, "delete-branch"), Some("rebasing"));
}

/// The same row with no worktree card, no place to make one and no card
/// record: nothing to take away or read over there, and the rows that
/// make a worktree greyed for want of a place — saying why there is none,
/// or that where is not read yet.
#[test]
fn a_row_with_nowhere_to_go_greys_what_needs_a_place() {
    for (placing, why) in [
        (Placing::Nowhere(Nowhere::DriveRoot), "drive-root"),
        (Placing::Nowhere(Nowhere::Bare), "bare"),
        (Placing::Unread, "worktrees-unread"),
    ] {
        let nowhere = RowLead {
            card: None,
            leads: Leads::Nowhere,
            placing,
            place: None,
            ..held_topic()
        };
        let menu = menu_of(nowhere, Optional::none(), Optional::none());
        assert_eq!(
            (
                menu.upstream_where.as_str(),
                menu.pull_runs.as_str(),
                why_of(&menu, "pull")
            ),
            ("", "", None),
            "no card record, nothing read over there"
        );
        assert_eq!(
            (
                why_of(&menu, "worktree-here"),
                why_of(&menu, "worktree-add"),
                why_of(&menu, "remove-worktree")
            ),
            (Some(why), Some(why), None),
            "nowhere to make one, and no worktree to take away"
        );
    }
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
