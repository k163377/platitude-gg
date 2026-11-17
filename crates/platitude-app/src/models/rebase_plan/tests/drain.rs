//! What a preview's refusal leaves behind: the name the page picks its
//! two lines from, and the shut plan underneath it (`drain::refused`).

use super::{RebasePlanModel, refusal_kind};
use platitude_core::rebase_plan::PlanRefusal;

/// A model waiting on one click's answer — the only state a refusal is
/// taken in (`qobject::open`).
fn asking(from: &str) -> RebasePlanModel {
    RebasePlanModel {
        loading: true,
        asked_from: from.to_string(),
        ..RebasePlanModel::default()
    }
}

/// Each refusal's name, and the plan left shut — which the model must
/// hold: a window that opened a plan and put it away frames exactly like
/// one that never opened it.
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

/// A bar for a stale click would be about a commit the reader moved on
/// from (`crate::hub::plan_tests` holds the delivery orders that make one).
#[test]
fn a_refusal_for_another_click_raises_nothing() {
    let mut model = asking("deep");
    assert_eq!(model.refused("shallow", PlanRefusal::AcrossMerge), None);
    assert!(model.loading, "the click that is waiting is still waiting");

    // Nor is a standing plan taken down by an answer nobody awaits.
    let mut settled = RebasePlanModel {
        active: true,
        ..RebasePlanModel::default()
    };
    assert_eq!(settled.refused("deep", PlanRefusal::OffBranch), None);
    assert!(settled.active);
}

/// The row menu's spelling of the same three is held by
/// `repo_tab::drain_report_tests::the_plan_door_names_the_same_refused_history_the_same_way`.
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
