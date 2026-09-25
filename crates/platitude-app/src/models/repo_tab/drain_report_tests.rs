//! What a report riding an answer leaves the properties QML reads
//! saying — the kind the page picks its two lines from, and the names
//! the first of them is written from (`drain::settle_write`).

use super::*;
use platitude_core::OperationKind as K;

/// The page words these four itself (`Words.writeReported`).
#[test]
fn a_refusal_the_far_side_made_arrives_as_something_to_report() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        1,
        K::Push,
        "`git push` exited with code 1: remote: error: Cannot delete a protected branch".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteDelete,
            "origin",
            "main",
            "Cannot delete a protected branch".into(),
        )),
        0,
        0,
    );
    assert!(tab.write_refused, "nothing happened over there");
    assert_eq!(tab.write_report_kind, "delete");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");
    assert_eq!(tab.write_report_reason, "Cannot delete a protected branch");

    // And it goes with its answer: a report left standing would come
    // back up under the next write.
    tab.settle_write(1, K::Push, String::new(), None, 0, 0);
    assert_eq!(tab.write_report_kind, "");
    assert_eq!(tab.write_report_reason, "");
}

#[test]
fn a_refused_send_is_told_apart_from_a_refused_delete() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        1,
        K::Push,
        "! [remote rejected]".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteUpdate,
            "origin",
            "main",
            "Changes must be made through a pull request.".into(),
        )),
        0,
        0,
    );
    assert_eq!(tab.write_report_kind, "update");
}

#[test]
fn a_stale_push_and_a_refused_commit_name_themselves_too() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        1,
        K::Push,
        "! [rejected] (fetch first)".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::Outdated,
            "origin",
            "main",
            "Updates were rejected because the remote contains work that you do not have.".into(),
        )),
        0,
        0,
    );
    assert_eq!(tab.write_report_kind, "outdated");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");

    tab.settle_write(
        1,
        K::Commit,
        "`git commit` exited with code 1".into(),
        Some(platitude_core::WriteReport::local(
            ReportKind::Commit,
            "lint found 1 problem".into(),
        )),
        0,
        0,
    );
    assert_eq!(tab.write_report_kind, "commit");
    assert_eq!(
        (
            tab.write_report_remote.as_str(),
            tab.write_report_name.as_str()
        ),
        ("", ""),
        "nothing over a network and no ref"
    );
    assert_eq!(tab.write_report_reason, "lint found 1 problem");
}

/// The kind is all the page writes both lines from
/// (`Words.rewriteRefusedWhy`). Read off core's producers
/// (`platitude_core::report`), not a second hand-written list — two
/// tables stay green while they drift apart.
#[test]
fn every_withheld_rewrite_reaches_the_page_under_its_own_name() {
    use platitude_core::report;
    // Four refused through a fold, one through a drop; the id only
    // reaches the log's sentence.
    let refused = [
        (K::Squash, report::rewrite_across_merge(), "across-merge"),
        (
            K::Squash,
            report::rewrite_off_branch("9f4a21c0"),
            "off-branch",
        ),
        (
            K::Squash,
            report::fold_first_commit("9f4a21c0"),
            "fold-first",
        ),
        (
            K::Squash,
            report::rewrite_unfetched_base(),
            "unfetched-base",
        ),
        (K::Drop, report::drop_all_commits(), "drop-all"),
    ];
    for (kind, withheld, named) in refused {
        let mut tab = RepoTab::default();
        tab.settle_write(
            1,
            kind,
            withheld.to_string(),
            withheld.report().cloned(),
            0,
            0,
        );
        assert!(tab.write_refused, "{named}: nothing was rewritten");
        assert_eq!(tab.write_report_kind, named);
        assert_eq!(
            (
                tab.write_report_remote.as_str(),
                tab.write_report_name.as_str(),
                tab.write_report_reason.as_str(),
            ),
            ("", "", ""),
            "{named}: nothing over a network, no ref, and nobody else's words to quote"
        );
    }
}

/// One `Words.rewriteRefusedWhy` answers both doors, so a spelling that
/// drifts on either side leaves that door's bar without its second line.
/// A `match` over the refusals, so a fourth added to core stops the build
/// here.
#[test]
fn the_plan_door_names_the_same_refused_history_the_same_way() {
    use platitude_core::rebase_plan::PlanRefusal;

    for refusal in [
        PlanRefusal::AcrossMerge,
        PlanRefusal::OffBranch,
        PlanRefusal::UnfetchedBase,
    ] {
        // The row menu's answer for the same history, from core's
        // producer.
        let withheld = match refusal {
            PlanRefusal::AcrossMerge => platitude_core::report::rewrite_across_merge(),
            PlanRefusal::OffBranch => platitude_core::report::rewrite_off_branch("9f4a21c0"),
            PlanRefusal::UnfetchedBase => platitude_core::report::rewrite_unfetched_base(),
        };
        let mut tab = RepoTab::default();
        tab.settle_write(
            1,
            K::Squash,
            withheld.to_string(),
            withheld.report().cloned(),
            0,
            0,
        );
        assert_eq!(
            crate::models::rebase_plan::refusal_kind(refusal),
            tab.write_report_kind,
            "the two doors have to hand the page one name for one history"
        );
    }
}
