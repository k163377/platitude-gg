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
// can win — a coin flip decided by how long two `ssh-keygen` runs
// take (2026-08-25 実測: 143µs apart under six concurrent runs). The
// answer the pane is waiting for had already landed, so letting the
// late one overwrite it puts the pane back to "nothing has answered"
// — and nothing asks again, so it stays there.
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
    tab.settle_write(op.into(), error.into(), None);
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
    tab.settle_write("merge".into(), String::new(), None);
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
    );
    assert!(tab.write_refused, "nothing happened over there");
    assert_eq!(tab.write_report_kind, "delete");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");
    assert_eq!(tab.write_report_reason, "Cannot delete a protected branch");

    // And it goes with its answer: a report left standing would come
    // back up under the next write.
    tab.settle_write("push".into(), String::new(), None);
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
    tab.settle_write("stash".into(), String::new(), None);
    assert!(tab.write_stashed);
    // Not a fetch: a failed fetch raises `fetch_first_failed`, and a
    // signal needs the proxy no unit test has.
    tab.settle_write("checkout".into(), "fatal: invalid reference".into(), None);
    assert!(!tab.write_stashed, "nothing armed survives the next answer");
    assert!(tab.write_refused);
    assert_eq!(tab.write_seq, 2);
}
