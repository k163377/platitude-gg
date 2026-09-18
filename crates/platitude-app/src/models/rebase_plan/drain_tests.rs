//! What a preview's refusal leaves behind: the name the page picks its
//! two lines from, and the shut plan underneath it (`drain::refused`).

use super::{RebasePlanModel, refusal_kind};
use platitude_core::rebase_plan::PlanRefusal;

/// A model waiting on the answer to one click, which is the only state a
/// refusal is taken in — anything else is the answer to a click the
/// screen has already left behind (`qobject::open`).
fn asking(from: &str) -> RebasePlanModel {
    let mut model = RebasePlanModel::default();
    model.loading = true;
    model.asked_from = from.to_string();
    model
}

/// **The three histories a preview turns a plan down for, each reaching
/// the page under its own name** — and the plan left shut underneath,
/// which is the other half of what a refusal means: nothing was touched
/// and nothing opened.
///
/// A picture cannot make that second half. A window that opened a plan
/// and put it away frames exactly like one that never opened it, so what
/// says the difference is the model, here.
#[test]
fn every_refused_preview_reaches_the_page_under_its_own_name() {
    let named = [
        (PlanRefusal::AcrossMerge, "across-merge"),
        (PlanRefusal::OffBranch, "off-branch"),
        (PlanRefusal::UnfetchedBase, "unfetched-base"),
    ];
    for (refusal, name) in named {
        let mut model = asking("9f4a21c0");
        assert_eq!(model.refused("9f4a21c0", refusal), Some(name));
        assert!(!model.active, "{name}: no plan opened");
        assert!(!model.loading, "{name}: and none is still coming");
        assert!(model.steps.is_empty(), "{name}: with no rows behind it");
    }
}

/// The answer to a click the screen has left behind is let go where it
/// lands — a bar raised over the plan the reader is looking at would be
/// about a commit they right-clicked and moved on from
/// (`super::hub::plan_tests` holds the delivery orders that make one).
#[test]
fn a_refusal_for_another_click_raises_nothing() {
    let mut model = asking("deep");
    assert_eq!(model.refused("shallow", PlanRefusal::AcrossMerge), None);
    assert!(model.loading, "the click that is waiting is still waiting");

    // And the same once nothing is out: a plan already standing is not
    // taken down by an answer nobody is waiting for.
    let mut settled = RebasePlanModel::default();
    settled.active = true;
    assert_eq!(settled.refused("deep", PlanRefusal::OffBranch), None);
    assert!(settled.active);
}

/// The table itself, walked where no model is needed: the names the row
/// menu's door spells the same three by
/// (`repo_tab::drain_report_tests::the_plan_door_names_the_same_refused_history_the_same_way`).
#[test]
fn no_two_refusals_reach_the_page_under_one_name() {
    let all = [
        refusal_kind(PlanRefusal::AcrossMerge),
        refusal_kind(PlanRefusal::OffBranch),
        refusal_kind(PlanRefusal::UnfetchedBase),
    ];
    for (at, name) in all.iter().enumerate() {
        assert!(!name.is_empty(), "every refusal is owed a name");
        assert!(
            !all[..at].contains(name),
            "`{name}` is two refusals' name, and the page has one sentence for it"
        );
    }
}
