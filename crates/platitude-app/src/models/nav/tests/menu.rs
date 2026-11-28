//! Tests of what a ref's menu reads off the sections beside its row
//! (`row_lead`).

use super::super::testkit::*;
use super::super::*;
use platitude_core::menu::Placing;
use platitude_core::offers::Leads;
use platitude_core::worktrees::{Keeper, KeptBranch, WorktreeEntry};

fn entry(path: &str, branch: Option<&str>, kept: Option<(&str, Keeper)>) -> WorktreeEntry {
    WorktreeEntry {
        path: path.to_string(),
        branch: branch.map(str::to_string),
        head_hex: None,
        bare: false,
        detached: branch.is_none(),
        locked: false,
        lock_reason: String::new(),
        prunable: false,
        prune_reason: String::new(),
        main: path.ends_with("repo"),
        kept: kept.map(|(branch, by)| KeptBranch {
            branch: branch.to_string(),
            by,
        }),
    }
}

/// The window stands in the repository's own worktree, stopped part-way
/// through a rebase of `main`; another worktree has `topic` out, and one
/// stands on no branch.
fn worktrees() -> NavSectionModel {
    let mut model = section(
        "worktrees",
        Source::Worktrees {
            list: vec![
                entry("C:/work/repo", None, Some(("main", Keeper::Rebase))),
                entry("C:/work/other", Some("topic"), None),
                entry("C:/work/loose", None, None),
            ],
            current: "c:/work/repo".to_string(),
        },
    );
    model.arrange();
    model
}

/// A branch's row and a remote branch's lead to the worktree holding the
/// local branch they land on, named as its WORKTREES row is — one record,
/// asked once, whichever entrance raised the menu. The WORKTREE card stands
/// on the branch's holder, never on a remote row's: that row only leads
/// there.
#[test]
fn a_branch_row_leads_to_the_worktree_holding_its_local_branch() {
    let model = worktrees();
    let topic = model.row_lead("branch", "topic", "topic", "");
    assert_eq!(
        topic,
        RowLead {
            kind: "branch".into(),
            full: "topic".into(),
            local: "topic".into(),
            leads: Leads::Holder,
            path: "C:/work/other".into(),
            called: model.name_of_worktree("C:/work/other"),
            home: false,
            card: model.worktree_of("C:/work/other"),
            card_here: false,
            placing: Placing::Beside { taken: false },
            place: model.new_worktree("topic"),
        }
    );
    assert!(topic.card.is_some() && topic.place.is_some());
    let remote = model.row_lead("remote", "origin/topic", "topic", "");
    assert_eq!(
        (remote.leads, remote.path.as_str(), remote.full.as_str()),
        (Leads::Holder, "C:/work/other", "origin/topic")
    );
    assert_eq!(remote.card, None, "no WORKTREE card off a remote row");
    assert_eq!(
        model.row_lead("branch", "free", "free", "").leads,
        Leads::Nowhere
    );
    assert_eq!(RowLead::from_map(&topic.to_map()), Some(topic));
}

/// A rebase stopped in this window's own tree holds its branch with no
/// other holder listed: the row leads nowhere — the stop is HEAD's
/// reading's to say (`TreeStanding::kept`), and the remote row's too.
#[test]
fn a_branch_a_rebase_here_holds_leads_nowhere() {
    let model = worktrees();
    let main = model.row_lead("branch", "main", "main", "");
    assert_eq!((main.leads, main.path.as_str()), (Leads::Nowhere, ""));
    assert_eq!(
        model.row_lead("remote", "origin/main", "main", "").leads,
        Leads::Nowhere
    );
}

/// A branch out in this window's own worktree — kept here by a stopped
/// rebase, as with HEAD on it — stands its WORKTREE card on this worktree,
/// as that worktree's own row does: the same branch has the same rows from
/// either entrance. It still leads nowhere.
#[test]
fn a_branch_out_here_stands_its_card_on_this_worktree() {
    let model = worktrees();
    let main = model.row_lead("branch", "main", "main", "");
    assert_eq!(
        (
            main.leads,
            main.path.as_str(),
            main.card.map(|card| card.full),
            main.card_here
        ),
        (Leads::Nowhere, "", Some("C:/work/repo".to_string()), true)
    );
    assert_eq!(model.row_lead("branch", "free", "free", "").card, None);
}

/// A worktree on no branch leads to itself, unless the window stands in it
/// — the path folded as the session spells it. Its WORKTREE card stands on
/// it either way, saying whether the tab stands in it.
#[test]
fn a_worktree_row_leads_to_itself_unless_the_tab_is_in_it() {
    let model = worktrees();
    let loose = model.row_lead("worktree", "C:/work/loose", "", "");
    assert_eq!(
        (
            loose.leads,
            loose.path.as_str(),
            loose.home,
            loose.card.map(|card| card.full),
            loose.card_here
        ),
        (
            Leads::Itself,
            "C:/work/loose",
            false,
            Some("C:/work/loose".to_string()),
            false
        )
    );
    assert_eq!(
        model.row_lead("worktree", "C:\\work\\repo", "", "").leads,
        Leads::Nowhere
    );
    let own = model.row_lead("worktree", "C:/work/repo", "", "");
    assert_eq!(
        (own.card.map(|card| card.full), own.card_here),
        (Some("C:/work/repo".to_string()), true)
    );
}

/// The worktree a menu was opened on is the one its WORKTREE card stands on,
/// over the one holding the row's branch.
#[test]
fn the_worktree_a_menu_was_opened_on_is_its_cards() {
    let model = worktrees();
    let lead = model.row_lead("branch", "topic", "topic", "C:/work/loose");
    assert_eq!(
        (lead.path.as_str(), lead.card.map(|card| card.full)),
        ("C:/work/other", Some("C:/work/loose".to_string())),
        "it still leads to the holder"
    );
}

/// A row that lands on no local branch names none, and a section that
/// lists no worktrees leads nowhere.
#[test]
fn only_a_branchy_row_lands_on_a_local_branch() {
    let model = worktrees();
    let tag = model.row_lead("tag", "v1", "v1", "");
    assert_eq!((tag.leads, tag.local.as_str()), (Leads::Nowhere, ""));
    let branches = section("branches", Source::Locals(locals(Vec::new())));
    let lead = branches.row_lead("branch", "topic", "topic", "");
    assert_eq!(
        (lead.leads, lead.local.as_str(), lead.placing),
        (Leads::Nowhere, "topic", Placing::Unread),
        "no worktree listing read: nothing known of where it goes"
    );
    assert_ne!(
        model.row_lead("branch", "topic", "topic", "").placing,
        Placing::Unread
    );
}

/// `undefined` is a row with no card, which reads nothing over there.
#[test]
fn nothing_handed_is_no_card() {
    use crate::encode::Optional;
    use qtbridge::QVariantConvertible;
    let none = Optional::<CardRecord>::none();
    let back =
        <Optional<CardRecord> as QVariantConvertible>::try_from_qvariant(&none.to_qvariant());
    assert!(back.is_some_and(|back| back.is_none()));
}

/// Where the worktrees' listing leaves a new worktree nowhere to go, the
/// row's lead says why: a bare repository's listing names no worktree of
/// its own, and one whose own stands at its drive's root has nothing
/// beside it (`worktrees_nowhere`); elsewhere it goes beside that one.
#[test]
fn a_rows_lead_says_why_a_new_worktree_has_nowhere_to_go() {
    use platitude_core::menu::Nowhere;
    let placing = |list: Vec<WorktreeEntry>, current: &str| {
        let mut model = section(
            "worktrees",
            Source::Worktrees {
                list,
                current: current.to_string(),
            },
        );
        model.arrange();
        model.row_lead("branch", "topic", "topic", "").placing
    };
    let linked = entry("C:/work/wt", Some("main"), None);
    assert_eq!(
        placing(vec![linked], "c:/work/wt"),
        Placing::Nowhere(Nowhere::Bare)
    );
    let at_root = WorktreeEntry {
        main: true,
        ..entry("C:/", Some("main"), None)
    };
    assert_eq!(
        placing(vec![at_root], "c:/"),
        Placing::Nowhere(Nowhere::DriveRoot)
    );
    assert_eq!(
        placing(
            vec![entry("C:/work/repo", Some("main"), None)],
            "c:/work/repo"
        ),
        Placing::Beside { taken: false }
    );
}
