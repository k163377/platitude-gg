//! What a report riding an answer leaves the properties QML reads
//! saying — the kind the page picks its two lines from, and the names
//! the first of them is written from (`drain::settle_write`).

use super::*;
use platitude_core::OperationKind as K;
use platitude_core::RemoteWhy;

/// The page words both lines itself (`Words.writeReported`,
/// `Words.writeReportedWhy`); the far side's words are the log's.
#[test]
fn a_refusal_the_far_side_made_arrives_as_something_to_report() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        1,
        K::Push,
        "`git push` exited with code 1: remote: error: Cannot delete a protected branch".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteDelete(RemoteWhy::Rule),
            "origin",
            "main",
        )),
        0,
        0,
    );
    assert!(tab.write_refused, "nothing happened over there");
    assert_eq!(tab.write_report_kind, "delete-rule");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");
    assert_eq!(tab.write_report_fact, "");

    // And it goes with its answer: a report left standing would come
    // back up under the next write.
    tab.settle_write(1, K::Push, String::new(), None, 0, 0);
    assert_eq!(tab.write_report_kind, "");
    assert_eq!(tab.write_report_fact, "");
}

/// What was asked and what for, as two words: the heading turns on the
/// first, the line under it on the second. A `match`, so a reason added to
/// core stops the build here.
#[test]
fn a_far_side_refusal_is_named_by_what_was_asked_and_what_for() {
    let why_word = |why: RemoteWhy| match why {
        RemoteWhy::Rule => "rule",
        RemoteWhy::Rewrite => "rewrite",
        RemoteWhy::NoDeletes => "no-deletes",
        RemoteWhy::CheckedOut => "checked-out",
        RemoteWhy::Current => "current",
        RemoteWhy::TooLarge => "too-large",
        RemoteWhy::Shallow => "shallow",
        RemoteWhy::Locked => "locked",
        RemoteWhy::Other => "refused",
    };
    for why in [
        RemoteWhy::Rule,
        RemoteWhy::Rewrite,
        RemoteWhy::NoDeletes,
        RemoteWhy::CheckedOut,
        RemoteWhy::Current,
        RemoteWhy::TooLarge,
        RemoteWhy::Shallow,
        RemoteWhy::Locked,
        RemoteWhy::Other,
    ] {
        for (kind, asked) in [
            (ReportKind::RemoteUpdate(why), "update"),
            (ReportKind::RemoteDelete(why), "delete"),
        ] {
            let mut tab = RepoTab::default();
            tab.settle_write(
                1,
                K::Push,
                "! [remote rejected]".into(),
                Some(platitude_core::WriteReport::on_remote(
                    kind, "origin", "main",
                )),
                0,
                0,
            );
            assert_eq!(tab.write_report_kind, format!("{asked}-{}", why_word(why)));
        }
    }
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
        )),
        0,
        0,
    );
    assert_eq!(tab.write_report_kind, "outdated");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");

    for (kind, named) in [
        (ReportKind::Commit, "commit"),
        (ReportKind::CommitHook, "commit-hook"),
        (ReportKind::CommitUnsigned, "commit-unsigned"),
        (ReportKind::CommitLocked, "commit-locked"),
    ] {
        tab.settle_write(
            1,
            K::Commit,
            "`git commit` exited with code 1".into(),
            Some(platitude_core::WriteReport::local(kind)),
            0,
            0,
        );
        assert_eq!(tab.write_report_kind, named);
        assert_eq!(
            (
                tab.write_report_remote.as_str(),
                tab.write_report_name.as_str(),
                tab.write_report_fact.as_str(),
            ),
            ("", "", ""),
            "{named}: nothing over a network, no ref, no words"
        );
    }
}

/// The reports git's run brings back, from core's producers: the page
/// picks both lines by these names (`Words.writeReported`,
/// `Words.writeReportedWhy`), and a worktree's carries the one fact its
/// line names.
#[test]
fn the_refusals_only_gits_run_tells_reach_the_page_under_their_names() {
    use platitude_core::report;
    let out = |code: i32, stderr: &str| platitude_core::GitOutput {
        code,
        stdout: Vec::new(),
        stderr: stderr.as_bytes().to_vec(),
    };
    let rewrite = |kind| report::rewrite_refused(kind, "git rebase".into(), 1, String::new());
    let failed = platitude_core::GitError::Failed {
        command: "git stash store".into(),
        code: 1,
        stderr: "Unable to create 'C:/t/.git/refs/stash.lock': File exists.".into(),
    };
    let reports = [
        (rewrite(ReportKind::RewritePreRebase), "pre-rebase", ""),
        (rewrite(ReportKind::RewriteUnsigned), "rewrite-unsigned", ""),
        (
            rewrite(ReportKind::RewriteCommitHook),
            "rewrite-commit-hook",
            "",
        ),
        (report::stash_not_renamed(failed), "stash-rename-locked", ""),
        (report::stash_gone("9f4a21c0"), "stash-gone", ""),
        (
            report::worktree_not_removed("wt", "git".into(), &out(128, ""), Some("on a stick")),
            "worktree-locked",
            "on a stick",
        ),
        (
            report::worktree_not_added("wt", "git".into(), &out(128, ""), Some("topic")),
            "worktree-branch-left",
            "topic",
        ),
        (
            report::worktree_hook_failed("wt", "git".into(), &out(2, "")),
            "worktree-hook",
            "",
        ),
    ];
    for (refused, named, fact) in reports {
        let mut tab = RepoTab::default();
        tab.settle_write(
            1,
            K::Stash,
            refused.to_string(),
            refused.report().cloned(),
            0,
            0,
        );
        assert_eq!(tab.write_report_kind, named);
        assert_eq!(tab.write_report_fact, fact, "{named}");
    }
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
                tab.write_report_fact.as_str(),
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

/// A worktree a menu asked for answers to its own press, over the
/// fetch beside it, with its report and the folder it was to be made in:
/// the page stands the tab there on a landing and reports a refusal
/// without the log (`RepoPage.absorbWorktreeAnswer`).
#[test]
fn the_worktree_a_menu_asked_for_is_answered_by_name_with_its_report() {
    let mut tab = RepoTab::default();
    tab.worktree_out.asked(Some(7));
    tab.worktree_answer_path = "C:/work/repo.worktrees/hotfix-patch".into();
    let refused = platitude_core::WriteReport::about(ReportKind::WorktreeNotAdded, "hotfix-patch");
    tab.absorb(vec![TabMsg::WriteState {
        id: 7,
        kind: K::Worktree,
        running: false,
        error: "git worktree add exited with code 255".into(),
        report: Some(refused),
        head_seq: 0,
        reads_from: 0,
    }]);
    let answer = tab
        .write_answer_at(tab.worktree_answer)
        .expect("its own answer");
    assert!(answer.failed);
    assert_eq!(answer.report_kind, "worktree-not-added");
    assert_eq!(answer.report_name, "hotfix-patch");
    assert_eq!(
        tab.worktree_answer_path, "C:/work/repo.worktrees/hotfix-patch",
        "the folder the press was for"
    );
    assert_eq!(
        tab.write_report_kind, "",
        "nobody else reads it off the group"
    );

    tab.absorb(vec![TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }]);
    assert_eq!(tab.worktree_answer, -1, "told once");
}

/// A worktree's folder renamed from a name box answers to its own press,
/// with its report — the bar says it, the box keeps the name — and to
/// nobody else's.
#[test]
fn a_renamed_worktree_is_answered_by_name() {
    let mut tab = RepoTab::default();
    tab.worktree_move_out.asked(Some(9));
    let refused = platitude_core::WriteReport::about(ReportKind::WorktreeNotRenamed, "loose");
    tab.absorb(vec![TabMsg::WriteState {
        id: 9,
        kind: K::Worktree,
        running: false,
        error: "git worktree move exited with code 128".into(),
        report: Some(refused),
        head_seq: 0,
        reads_from: 0,
    }]);
    let answer = tab
        .write_answer_at(tab.worktree_move_answer)
        .expect("its own answer");
    assert!(answer.failed);
    assert_eq!(answer.report_kind, "worktree-not-renamed");
    assert_eq!(answer.report_name, "loose");
    assert_eq!(tab.worktree_answer, -1, "not the made worktree's answer");
    assert_eq!(
        tab.write_report_kind, "",
        "nobody else reads it off the group"
    );
}

/// The press writes down whether this tab stands in the worktree it renames
/// — however the tab and git spell the folder — and that tab stands again on
/// the answer (`RepoPage.absorbWorktreeMove`). No session here: nothing is
/// asked, the note is the press's own.
#[test]
fn a_rename_press_says_whether_the_tab_stands_in_that_worktree() {
    let mut tab = RepoTab {
        repo_path: "C:\\Work\\Loose".into(),
        ..RepoTab::default()
    };
    tab.worktree_move(
        "c:/work/loose".into(),
        "c:/work/renamed".into(),
        "loose".into(),
    );
    assert!(tab.worktree_move_standing, "the tab's own worktree");
    assert_eq!(
        (
            tab.worktree_move_from.as_str(),
            tab.worktree_move_to.as_str()
        ),
        ("c:/work/loose", "c:/work/renamed")
    );
    tab.worktree_move(
        "c:/work/loose-twin".into(),
        "c:/work/twin".into(),
        "loose-twin".into(),
    );
    assert!(!tab.worktree_move_standing, "another worktree");
}
