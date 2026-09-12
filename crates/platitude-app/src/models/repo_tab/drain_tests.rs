//! What one answer out of the feed leaves the properties QML reads
//! saying (`drain::settle_write` / `settle_signature`).

use super::*;

/// Two commits out of the `co-authors` demo repository, named here
/// because the wedge below was measured on them: row 3 is the one the
/// reader is on, row 0 the selection the page made on its way there.
const ON_SCREEN: &str = "f7892e60a51c14c36a159b33169be38823edc07f";
const LEFT_BEHIND: &str = "1db47e8896c214d7ef5fa1aa2c34d010ab9f7481";

/// Both rows asked about before either is answered — what two
/// selections in quick succession leave in flight.
fn asked_about_both() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.look_up_signature(LEFT_BEHIND.into());
    tab.look_up_signature(ON_SCREEN.into());
    tab
}

// Two reads race, and the one about the selection already left behind
// can win — a coin flip decided by how long two `ssh-keygen` runs take,
// which under concurrent runs land close enough together for either
// order (observed). The answer the pane is waiting for had already
// landed, so letting the late one overwrite it puts the pane back to
// "nothing has answered" — and nothing asks again, so it stays there.
#[test]
fn a_signature_answer_about_a_selection_left_behind_is_dropped() {
    let mut tab = asked_about_both();
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    tab.settle_signature(
        LEFT_BEHIND.into(),
        "signed".into(),
        "U".into(),
        "stranger@example.com".into(),
    );
    assert_eq!(tab.signature_oid, ON_SCREEN);
    assert_eq!(tab.signature_kind, "verified");
    assert_eq!(tab.signature_code, "G");
    assert_eq!(tab.signature_signer, "demo@example.com");
}

#[test]
fn the_answer_the_pane_is_waiting_for_lands_however_late() {
    let mut tab = asked_about_both();
    tab.settle_signature(
        LEFT_BEHIND.into(),
        "signed".into(),
        "U".into(),
        "stranger@example.com".into(),
    );
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    assert_eq!(tab.signature_oid, ON_SCREEN);
    assert_eq!(tab.signature_code, "G");
}

// Coming back to a row asks again (デザイン規約 §署名の表示 「同じ行へ
// 戻ってくれば投げ直す」), and the answer to that second asking is
// about the commit on screen — so it is the one that is kept.
#[test]
fn coming_back_to_a_row_takes_the_answer_to_the_asking_that_brought_it() {
    let mut tab = asked_about_both();
    tab.look_up_signature(LEFT_BEHIND.into());
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    tab.settle_signature(
        LEFT_BEHIND.into(),
        "signed".into(),
        "U".into(),
        "stranger@example.com".into(),
    );
    assert_eq!(tab.signature_oid, LEFT_BEHIND);
    assert_eq!(tab.signature_code, "U");
}

// A read that answers a session nobody is asking any more — the shape
// `Feed::clear_queued` exists for, arriving one drain too late.
#[test]
fn an_answer_nobody_asked_for_is_dropped() {
    let mut tab = RepoTab::default();
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    assert_eq!(tab.signature_oid, "");
}

fn settled(op: &str, error: &str) -> RepoTab {
    let mut tab = RepoTab::default();
    tab.settle_write(op.into(), error.into(), None, 0);
    tab
}

#[test]
fn a_landed_stage_leaves_the_shown_diff_stale() {
    let tab = settled("stage", "");
    assert!(!tab.write_refused);
    assert!(tab.write_stale_diff);
}

#[test]
fn a_refused_stage_is_the_drifted_rows_own_answer() {
    let tab = settled("stage", "error: patch does not apply");
    assert!(tab.write_refused);
    assert!(
        tab.write_stale_diff,
        "the refusal's answer is the fresh file"
    );
}

#[test]
fn a_refused_commit_asks_for_no_reread() {
    let tab = settled("commit", "nothing to commit, working tree clean");
    assert!(tab.write_refused);
    assert!(!tab.write_stale_diff);
    assert!(
        !tab.write_committed,
        "a rejected commit keeps the editor's text"
    );
}

#[test]
fn a_landed_commit_lands_and_moves_the_sides() {
    let tab = settled("commit", "");
    assert!(tab.write_committed);
    assert!(tab.write_stale_diff);
    assert!(
        !tab.write_at_tip,
        "the landing is the editor's, not a tip jump"
    );
}

#[test]
fn a_merge_that_landed_answers_at_the_tip() {
    assert!(settled("merge", "").write_at_tip);
    assert!(settled("cherry-pick", "").write_at_tip);
    assert!(!settled("merge", "fatal: refusing to merge").write_at_tip);
}

#[test]
fn a_merge_that_stopped_does_not_claim_the_tip() {
    let mut tab = RepoTab::default();
    // The stop arrives before the answer that ends the write
    // (`TabMsg::WriteStopped`), so the flag is already standing.
    tab.last_write_stopped = true;
    tab.settle_write("merge".into(), String::new(), None, 0);
    assert!(!tab.write_at_tip, "nothing landed at the tip to go to");
}

#[test]
fn a_branch_answer_says_so_whichever_way_it_went() {
    assert!(settled("branch", "").write_branch_op);
    assert!(settled("branch", "error: not fully merged").write_branch_op);
    assert!(!settled("tag", "").write_branch_op);
}

#[test]
fn a_push_answer_says_so_whichever_way_it_went() {
    assert!(settled("push", "").write_pushed);
    assert!(settled("push", "! [rejected]").write_pushed);
}

/// The refused half is the one the page reads: a fetch that could not
/// reach the far side has said so in the tab's own terms already, and
/// the panel it raised stays the fetch's to take down again
/// (`CommandsOwner`). A push answering the same way is somebody else's
/// news in the same panel.
#[test]
fn a_fetch_answer_says_so_whichever_way_it_went() {
    assert!(settled("fetch", "").write_fetched);
    let mut tab = RepoTab::default();
    // The run has raised its panel already, so this failure has no
    // `fetch_first_failed` to emit — there is no proxy to emit it
    // through here, and the refused half is what the page reads.
    tab.fetch_log_raised = true;
    tab.settle_write(
        "fetch".into(),
        "fatal: Could not read from remote".into(),
        None,
        0,
    );
    assert!(tab.write_fetched);
    assert!(!settled("push", "! [rejected]").write_fetched);
}

#[test]
fn a_landed_checkout_or_reset_moved_head() {
    assert!(settled("checkout", "").write_moved_head);
    assert!(settled("reset", "").write_moved_head);
    assert!(!settled("checkout", "fatal: invalid reference").write_moved_head);
}

#[test]
fn a_landed_reword_carries_the_saved_message() {
    assert!(settled("reword", "").write_reworded);
    assert!(!settled("reword", "fatal: bad revision").write_reworded);
}

/// The far side keeping a branch is a report, not a failure of this
/// window's — the page reads these four and says so in its own words
/// (`Words.writeReported`).
#[test]
fn a_refusal_the_far_side_made_arrives_as_something_to_report() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        "push".into(),
        "`git push` exited with code 1: remote: error: Cannot delete a protected branch".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteDelete,
            "origin",
            "main",
            "Cannot delete a protected branch".into(),
        )),
        0,
    );
    assert!(tab.write_refused, "nothing happened over there");
    assert_eq!(tab.write_report_kind, "delete");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");
    assert_eq!(tab.write_report_reason, "Cannot delete a protected branch");

    // And it goes with its answer: a report left standing would come
    // back up under the next write.
    tab.settle_write("push".into(), String::new(), None, 0);
    assert_eq!(tab.write_report_kind, "");
    assert_eq!(tab.write_report_reason, "");
}

/// A push that was sending rather than removing says so, since that
/// is the whole of what the sentence turns on.
#[test]
fn a_refused_send_is_told_apart_from_a_refused_delete() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        "push".into(),
        "! [remote rejected]".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteUpdate,
            "origin",
            "main",
            "Changes must be made through a pull request.".into(),
        )),
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
        "push".into(),
        "! [rejected] (fetch first)".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::Outdated,
            "origin",
            "main",
            "Updates were rejected because the remote contains work that you do not have.".into(),
        )),
        0,
    );
    assert_eq!(tab.write_report_kind, "outdated");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");

    tab.settle_write(
        "commit".into(),
        "`git commit` exited with code 1".into(),
        Some(platitude_core::WriteReport::local(
            ReportKind::Commit,
            "lint found 1 problem".into(),
        )),
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

#[test]
fn every_answer_rewrites_the_whole_group() {
    let mut tab = RepoTab::default();
    tab.settle_write("stash".into(), String::new(), None, 0);
    assert!(tab.write_stashed);
    // Not a fetch: a failed fetch raises `fetch_first_failed`, and a
    // signal needs the proxy no unit test has.
    tab.settle_write(
        "checkout".into(),
        "fatal: invalid reference".into(),
        None,
        0,
    );
    assert!(!tab.write_stashed, "nothing armed survives the next answer");
    assert!(tab.write_refused);
    assert_eq!(tab.write_seq, 2);
}

/// One drain carrying a whole run and the fetch behind it — what the
/// freeze leaves running comes back while the run's landing is still
/// being waited out, and the two answers meet in the same batch
/// (`take_feed` drains the queue whole and notifies once).
fn a_run_and_the_fetch_behind_it() -> Vec<TabMsg> {
    vec![
        TabMsg::WriteStopped,
        TabMsg::WriteState {
            op: "rebase".into(),
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
        },
        TabMsg::WriteState {
            op: "fetch".into(),
            running: true,
            error: String::new(),
            report: None,
            head_seq: 0,
        },
        TabMsg::WriteState {
            op: "fetch".into(),
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
        },
    ]
}

// The group properties describe the answer that came last, and a batch
// leaves only that one to be read: a reader waiting for the run's own
// answer by name never sees it, and the stop it left standing is taken
// back down by the fetch *starting*. Both are read out of the list
// instead, which keeps every answer the notify carried.
#[test]
fn every_answer_in_one_drain_is_published_not_just_the_last() {
    let mut tab = RepoTab::default();
    tab.absorb(a_run_and_the_fetch_behind_it());
    assert!(
        !tab.last_write_stopped,
        "the fetch starting took the stop back down"
    );
    let ops: Vec<&str> = tab.write_answers.iter().map(|a| a.op.as_str()).collect();
    assert_eq!(
        ops,
        ["rebase", "fetch"],
        "both answers, in the order they came"
    );
}

#[test]
fn the_runs_answer_keeps_the_stop_that_was_its_own() {
    let mut tab = RepoTab::default();
    tab.absorb(a_run_and_the_fetch_behind_it());
    let run = &tab.write_answers[0];
    assert!(run.stopped, "git stopped part-way through this one");
    assert!(!run.failed);
    assert!(!run.at_tip, "a stop leaves no commit at the tip to go to");
    assert!(
        !tab.write_answers[1].stopped,
        "the stop was not the fetch's"
    );
}

/// Which answer a run is waiting for is told by the counter it read
/// before pressing, so one already counted cannot arm it.
#[test]
fn each_answer_carries_the_seq_it_was_counted_at() {
    let mut tab = RepoTab::default();
    tab.settle_write("push".into(), String::new(), None, 0);
    let before = tab.write_seq;
    tab.absorb(a_run_and_the_fetch_behind_it());
    let seqs: Vec<i32> = tab.write_answers.iter().map(|a| a.seq).collect();
    assert_eq!(seqs, [before + 1, before + 2]);
}

// The tip landing is waited for by meaning rather than by name, and the
// answer carrying it is not the one the group is left describing.
#[test]
fn a_landing_at_the_tip_is_found_in_the_list_a_fetch_answered_over() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![
        TabMsg::WriteState {
            op: "merge".into(),
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
        },
        TabMsg::WriteState {
            op: "fetch".into(),
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
        },
    ]);
    assert!(!tab.write_at_tip, "the group describes the fetch");
    let landed: Vec<&str> = tab
        .write_answers
        .iter()
        .filter(|a| a.at_tip)
        .map(|a| a.op.as_str())
        .collect();
    assert_eq!(landed, ["merge"]);
}

// A drain that brought no write answer says so, rather than leaving the
// last one's list standing for a second notify to read over again.
#[test]
fn a_drain_with_no_write_answer_publishes_none() {
    let mut tab = RepoTab::default();
    tab.absorb(a_run_and_the_fetch_behind_it());
    assert_eq!(tab.write_answers.len(), 2);
    tab.absorb(vec![TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }]);
    assert!(tab.write_answers.is_empty());
}

// The plain branch delete leaves its menu standing for git's answer, and
// the card reads that answer by its own name off the tab (`RefBranchMenu`):
// taken, the card goes; turned down, its row turns into `-D`.
#[test]
fn a_plain_delete_git_took_names_the_branch_for_the_card_to_go_on() {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", false);
    tab.settle_write("branch".into(), String::new(), None, 0);
    assert_eq!(tab.branch_delete_landed, "feature");
    assert_eq!(tab.branch_delete_refused, "");
}

#[test]
fn a_plain_delete_git_refused_names_the_branch_for_the_row_to_turn_on() {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", false);
    tab.settle_write(
        "branch".into(),
        "error: the branch 'feature' is not fully merged".into(),
        None,
        0,
    );
    assert_eq!(tab.branch_delete_refused, "feature");
    assert_eq!(tab.branch_delete_landed, "");
}

// The forced form is already the answer to a refusal, and nothing stays
// up for it.
#[test]
fn the_forced_delete_stands_for_nothing() {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", true);
    tab.settle_write("branch".into(), String::new(), None, 0);
    assert_eq!(tab.branch_delete_landed, "");
    assert_eq!(tab.branch_delete_refused, "");
}

// A branch answer nobody stayed up for — a create, a rename, `Delete both`
// — names no card, whichever way it went.
#[test]
fn a_branch_answer_nobody_stayed_up_for_names_no_card() {
    let mut tab = RepoTab::default();
    tab.settle_write("branch".into(), String::new(), None, 0);
    assert_eq!(tab.branch_delete_landed, "");
    tab.settle_write("branch".into(), "error: not fully merged".into(), None, 0);
    assert_eq!(tab.branch_delete_refused, "");
}

// Not part of the write group: the answer stands through whatever answers
// after it — a fetch in the same drain would otherwise take it down before
// the card had seen it — and goes as the next plain delete is asked. The
// seq is how a reader tells the answer in hand from one standing over.
#[test]
fn the_delete_answer_stands_until_the_next_plain_delete_is_asked() {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", false);
    tab.settle_write("branch".into(), String::new(), None, 0);
    assert_eq!(tab.branch_delete_seq, tab.write_seq);
    tab.settle_write("fetch".into(), String::new(), None, 0);
    tab.settle_write("branch".into(), String::new(), None, 0);
    assert_eq!(tab.branch_delete_landed, "feature", "the answer stands");
    assert_ne!(
        tab.branch_delete_seq, tab.write_seq,
        "and says it is not the answer in hand"
    );
    tab.arm_branch_delete("other", false);
    assert_eq!(tab.branch_delete_landed, "");
    assert_eq!(tab.branch_delete_seq, 0);
}

// A refusal that comes with a report is the report's to say: the page's
// notice bar carries it, and no row turns.
#[test]
fn a_refusal_with_a_report_turns_no_row() {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", false);
    tab.settle_write(
        "branch".into(),
        "remote: refused".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteDelete,
            "origin",
            "feature",
            "refused".into(),
        )),
        0,
    );
    assert_eq!(tab.branch_delete_refused, "");
    assert_eq!(tab.branch_delete_landed, "");
}
