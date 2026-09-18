//! What a report riding an answer leaves the properties QML reads
//! saying — the kind the page picks its two lines from, and the names
//! the first of them is written from (`drain::settle_write`).

use super::*;
use platitude_core::OperationKind as K;

/// The far side keeping a branch is a report — the page reads these
/// four and says so in its own words
/// (`Words.writeReported`).
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

/// A push that was sending says so, since that is the whole of what
/// the sentence turns on.
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

/// The other two reports reach the same four properties: nobody over
/// there said anything about either, and the page still has to tell them
/// apart to pick its sentence.
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

/// **The five shapes a history is turned down for, each reaching the page
/// under its own name.** git was never asked and nobody outside said
/// anything, so the kind is the whole of what the screen has to write
/// both of its lines from (`Words.rewriteRefusedWhy`): a refusal
/// arriving as its neighbour's tells a reader whose history is fine to
/// switch branches, or to deepen a clone that is whole.
///
/// **Read off the producers themselves** (`platitude_core::report`), not
/// a list of kinds written out again on this side: the claim is that
/// what core withholds a rewrite with is what the page names it, and two
/// hand-written tables stay green while they drift apart.
#[test]
fn every_withheld_rewrite_reaches_the_page_under_its_own_name() {
    use platitude_core::report;
    // Four of them are asked for through a fold and the fifth through a
    // drop; the id is the row the press was on, which only the sentence
    // the log keeps ever spells.
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

/// **The three of them the plan door turns down too, named the same way
/// there.** One page table answers both doors with the sentence under
/// the heading (`Words.rewriteRefusedWhy`), so a spelling that drifts on
/// either side leaves that door's bar with a heading and nothing under
/// it — and two hand-written tables stay green while they part.
///
/// Written as a `match` over the preview's refusals, so a fourth added
/// to core stops the build here rather than reaching the page under a
/// name nobody wrote a sentence for.
#[test]
fn the_plan_door_names_the_same_refused_history_the_same_way() {
    use platitude_core::rebase_plan::PlanRefusal;

    for refusal in [
        PlanRefusal::AcrossMerge,
        PlanRefusal::OffBranch,
        PlanRefusal::UnfetchedBase,
    ] {
        // This door's own answer for the same history, taken from the
        // producer core withholds with.
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
