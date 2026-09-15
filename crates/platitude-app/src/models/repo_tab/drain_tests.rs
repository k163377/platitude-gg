//! What one answer out of the feed leaves the properties QML reads
//! saying (`drain::settle_write` / `settle_signature`).

use super::*;
use platitude_core::OperationKind as K;

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

fn settled(kind: K, error: &str) -> RepoTab {
    let mut tab = RepoTab::default();
    tab.settle_write(1, kind, error.into(), None, 0, 0);
    tab
}

/// Whether the answer this tab took last left a commit at the tip. Read
/// off the answer because that is where it is kept — a landing is waited
/// for by answer, never off a property the next one rewrites
/// (`WriteAnswer::at_tip`).
fn at_tip(tab: &RepoTab) -> bool {
    tab.write_answers.last().is_some_and(|a| a.at_tip)
}

#[test]
fn a_landed_stage_leaves_the_shown_diff_stale() {
    let tab = settled(K::Stage, "");
    assert!(!tab.write_refused);
    assert!(tab.write_stale_diff);
}

#[test]
fn a_refused_stage_is_the_drifted_rows_own_answer() {
    let tab = settled(K::Stage, "error: patch does not apply");
    assert!(tab.write_refused);
    assert!(
        tab.write_stale_diff,
        "the refusal's answer is the fresh file"
    );
}

// A commit nobody here pressed for — the page that sent it is gone, or
// the queue took nothing — has no editor waiting to be emptied, so its
// answer falls to the group like any other nobody named.
#[test]
fn a_commit_nobody_waits_for_falls_to_the_group() {
    let tab = settled(K::Commit, "nothing to commit, working tree clean");
    assert!(tab.write_refused);
    assert!(!tab.write_stale_diff, "a refusal asks for no re-read");
    assert_eq!(tab.commit_answer, -1);

    let tab = settled(K::Commit, "");
    assert!(tab.write_stale_diff);
    assert!(!at_tip(&tab), "the landing is the editor's, not a tip jump");
}

#[test]
fn a_merge_that_landed_answers_at_the_tip() {
    assert!(at_tip(&settled(K::Merge, "")));
    assert!(at_tip(&settled(K::CherryPick, "")));
    assert!(!at_tip(&settled(K::Merge, "fatal: refusing to merge")));
}

#[test]
fn a_merge_that_stopped_does_not_claim_the_tip() {
    let mut tab = RepoTab::default();
    // The stop arrives before the answer that ends the write
    // (`TabMsg::WriteStopped`), so the flag is already standing.
    tab.last_write_stopped = true;
    tab.settle_write(1, K::Merge, String::new(), None, 0, 0);
    assert!(!at_tip(&tab), "nothing landed at the tip to go to");
}

#[test]
fn a_branch_answer_says_so_whichever_way_it_went() {
    assert!(settled(K::Branch, "").write_branch_op);
    assert!(settled(K::Branch, "error: not fully merged").write_branch_op);
    assert!(!settled(K::Tag, "").write_branch_op);
}

// ---- the toolbar's push ----------------------------------------------

/// The button pressed: the push went to the queue under `OURS`, for
/// `topic`.
fn push_pressed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.push_out.asked(Some(OURS), "topic".into());
    tab
}

// A push that git turned down and the fetch that landed behind it come
// back in one drain. The answer is handed to the press by id, with the
// branch it was sent for and git's own words for that one — and the
// fetch is what is left over for the group, so the page's leftover
// branch reads a fetch that landed, not a push that was refused.
#[test]
fn a_push_answer_is_handed_to_the_press_that_sent_it_with_its_branch_and_words() {
    let mut tab = push_pressed();
    tab.absorb(vec![
        answered(OURS, K::Push, "! [rejected] topic -> topic (fetch first)"),
        answered(SOMEBODY_ELSE, K::Fetch, ""),
    ]);
    assert_eq!(tab.push_answer, 0, "the press's own answer, by id");
    assert_eq!(tab.push_answer_branch, "topic");
    assert!(tab.write_answers[0].failed);
    assert_eq!(
        tab.write_answers[0].error, "! [rejected] topic -> topic (fetch first)",
        "the words are the answer's own"
    );
    assert!(!tab.write_refused, "the group is the fetch's, which landed");
    assert!(tab.write_fetched);
}

// A push the far side turned down with a reason of its own — this end
// was behind it — is handed to the press with that reason on the answer,
// where the page reads it into the bar (`RepoPage.absorbPushAnswer`).
// The group stays put: the report is the answer's own, and a copy there
// would have the page say it twice.
#[test]
fn a_push_refused_with_a_report_hands_the_report_to_the_press() {
    let mut tab = push_pressed();
    tab.absorb(vec![reported(
        OURS,
        K::Push,
        "! [rejected] topic -> topic (fetch first)",
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::Outdated,
            "origin",
            "topic",
            "fetch first".into(),
        )),
    )]);
    assert_eq!(tab.push_answer, 0);
    let answer = &tab.write_answers[0];
    assert!(answer.failed);
    assert_eq!(answer.report_kind, "outdated");
    assert_eq!(answer.report_remote, "origin");
    assert_eq!(answer.report_name, "topic");
    assert_eq!(answer.report_reason, "fetch first");
    assert!(
        !tab.write_refused,
        "the report is the answer's, not the group's"
    );
    assert_eq!(tab.write_report_kind, "");
}

// Two presses in a row — the reader switched branches and pushed again
// before the first came back — answer in one drain. Only the press still
// waited for is handed its answer; the other is nobody's and folds into
// the group, and neither is handled twice or lost.
#[test]
fn two_pushes_in_a_row_are_answered_by_id_and_neither_twice() {
    let mut tab = push_pressed();
    tab.push_out.asked(Some(SOMEBODY_ELSE), "other".into());
    tab.absorb(vec![
        answered(OURS, K::Push, ""),
        answered(SOMEBODY_ELSE, K::Push, "! [rejected]"),
    ]);
    assert_eq!(
        tab.push_answer, 1,
        "the second press's answer, not the first's"
    );
    assert_eq!(tab.push_answer_branch, "other");
    assert!(tab.write_answers[1].failed);
    assert!(
        !tab.write_refused,
        "the first press's answer is the leftover, and it landed"
    );
}

// A push nobody here pressed for — one a page that has since gone sent —
// has no press waiting: its answer falls to the group like any other
// nobody named. **Both owners are empty here**, which is what makes this
// the leftover case rather than the ref row's own (below).
#[test]
fn a_push_nobody_here_pressed_for_falls_to_the_group() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Push, "! [rejected]")]);
    assert_eq!(tab.push_answer, -1);
    assert!(tab.write_refused);
    assert_eq!(tab.last_write_error, "! [rejected]");
}

// The answer's place is the notify's own: the drain after the one that
// carried it says the press is not answered here, so the page cannot
// act on the same refusal twice.
#[test]
fn the_push_answer_is_this_notifys_own() {
    let mut tab = push_pressed();
    tab.absorb(vec![answered(OURS, K::Push, "! [rejected]")]);
    assert_eq!(tab.push_answer, 0);
    tab.absorb(vec![TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }]);
    assert_eq!(tab.push_answer, -1);
    assert_eq!(
        tab.push_answer_branch, "topic",
        "the branch outlives the answer, for the refusal remembered against it"
    );
}

/// The refused half is the one the page reads: a fetch that could not
/// reach the far side has said so in the tab's own terms already, and
/// the panel it raised stays the fetch's to take down again
/// (`CommandsOwner`). A push answering the same way is somebody else's
/// news in the same panel.
#[test]
fn a_fetch_answer_says_so_whichever_way_it_went() {
    assert!(settled(K::Fetch, "").write_fetched);
    let mut tab = RepoTab::default();
    // The run has raised its panel already, so this failure has no
    // `fetch_first_failed` to emit — there is no proxy to emit it
    // through here, and the refused half is what the page reads.
    tab.fetch_log_raised = true;
    tab.settle_write(
        1,
        K::Fetch,
        "fatal: Could not read from remote".into(),
        None,
        0,
        0,
    );
    assert!(tab.write_fetched);
    assert!(!settled(K::Push, "! [rejected]").write_fetched);
}

#[test]
fn a_landed_checkout_or_reset_moved_head() {
    assert!(settled(K::Checkout, "").write_moved_head);
    assert!(settled(K::Reset, "").write_moved_head);
    assert!(!settled(K::Checkout, "fatal: invalid reference").write_moved_head);
}

#[test]
fn a_landed_reword_carries_the_saved_message() {
    assert!(settled(K::Reword, "").write_reworded);
    assert!(!settled(K::Reword, "fatal: bad revision").write_reworded);
}

/// The far side keeping a branch is a report, not a failure of this
/// window's — the page reads these four and says so in its own words
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

/// A push that was sending rather than removing says so, since that
/// is the whole of what the sentence turns on.
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

#[test]
fn every_answer_rewrites_the_whole_group() {
    let mut tab = RepoTab::default();
    tab.settle_write(1, K::Reword, String::new(), None, 0, 0);
    assert!(tab.write_reworded);
    // Not a fetch: a failed fetch raises `fetch_first_failed`, and a
    // signal needs the proxy no unit test has.
    tab.settle_write(
        1,
        K::Checkout,
        "fatal: invalid reference".into(),
        None,
        0,
        0,
    );
    assert!(
        !tab.write_reworded,
        "nothing armed survives the next answer"
    );
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
            id: 1,
            kind: K::Rebase,
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
            reads_from: 0,
        },
        TabMsg::WriteState {
            id: 1,
            kind: K::Fetch,
            running: true,
            error: String::new(),
            report: None,
            head_seq: 0,
            reads_from: 0,
        },
        TabMsg::WriteState {
            id: 1,
            kind: K::Fetch,
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
            reads_from: 0,
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
    let ops: Vec<K> = tab.write_answers.iter().map(|a| a.kind).collect();
    assert_eq!(
        ops,
        [K::Rebase, K::Fetch],
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
    tab.settle_write(1, K::Push, String::new(), None, 0, 0);
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
            id: 1,
            kind: K::Merge,
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
            reads_from: 0,
        },
        TabMsg::WriteState {
            id: 1,
            kind: K::Fetch,
            running: false,
            error: String::new(),
            report: None,
            head_seq: 0,
            reads_from: 0,
        },
    ]);
    assert!(!at_tip(&tab), "the answer that came last is the fetch's");
    let landed: Vec<K> = tab
        .write_answers
        .iter()
        .filter(|a| a.at_tip)
        .map(|a| a.kind)
        .collect();
    assert_eq!(landed, [K::Merge]);
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

/// A tab whose plain `branch --delete` is out and whose card is standing
/// for its answer (`ops_delete::branch_delete` writes the name and the
/// id down together at the press).
fn card_standing() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", Some(OURS));
    tab
}

// The plain branch delete leaves its menu standing for git's answer, and
// the card reads that answer by its own name off the tab
// (`RefBranchMenu`): taken, the card goes; turned down, its row turns
// into `-D`. The fetch running behind the press answers in the same
// drain and looks exactly like an answer to this one.
#[test]
fn a_plain_delete_git_took_names_the_branch_over_the_fetch_behind_it() {
    let mut tab = card_standing();
    tab.absorb(vec![
        answered(OURS, K::Branch, ""),
        answered(SOMEBODY_ELSE, K::Fetch, ""),
    ]);
    assert_eq!(tab.branch_delete_landed, "feature");
    assert_eq!(tab.branch_delete_refused, "");
    assert!(tab.branch_delete_answer >= 0, "and this notify carried it");
    assert!(
        tab.write_fetched,
        "while the group is the fetch's, being the last answer nobody named"
    );
}

#[test]
fn a_plain_delete_git_refused_names_the_branch_for_the_row_to_turn_on() {
    let mut tab = card_standing();
    tab.absorb(vec![answered(
        OURS,
        K::Branch,
        "error: the branch 'feature' is not fully merged",
    )]);
    assert_eq!(tab.branch_delete_refused, "feature");
    assert_eq!(tab.branch_delete_landed, "");
    // The row is the answer, so nothing is left over for the page to
    // raise the command log on.
    assert!(!tab.write_refused);
}

// The forced form is already the answer to a refusal, and nothing stays
// up for it; nor does a create, a rename or `Delete both`.
#[test]
fn a_branch_answer_nobody_stayed_up_for_names_no_card() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![answered(OURS, K::Branch, "error: not fully merged")]);
    assert_eq!(tab.branch_delete_refused, "");
    assert!(
        tab.write_refused,
        "and it falls to the group like any other nobody named"
    );
}

// The name stands through whatever answers after it — the card reads it
// as an edge and may not have been drawn yet — while where it answered
// is the notify's own, which is how the page tells the answer in hand
// from the one standing above it.
#[test]
fn the_delete_answer_stands_but_says_which_notify_carried_it() {
    let mut tab = card_standing();
    tab.absorb(vec![answered(OURS, K::Branch, "")]);
    assert!(tab.branch_delete_answer >= 0);
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Fetch, "")]);
    assert_eq!(tab.branch_delete_landed, "feature", "the answer stands");
    assert_eq!(
        tab.branch_delete_answer, -1,
        "and says this notify carried none of it"
    );
    tab.arm_branch_delete("other", Some(SOMEBODY_ELSE));
    assert_eq!(tab.branch_delete_landed, "");
}

// A refusal that comes with a report is the report's to say: the page's
// notice bar carries it, and no row turns. It is still this card's
// answer, so the page is told where to find the words.
#[test]
fn a_refusal_with_a_report_turns_no_row() {
    let mut tab = card_standing();
    tab.absorb(vec![reported(
        OURS,
        K::Branch,
        "remote: refused",
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteDelete,
            "origin",
            "feature",
            "refused".into(),
        )),
    )]);
    assert_eq!(tab.branch_delete_refused, "");
    assert_eq!(tab.branch_delete_landed, "");
    let answer = tab
        .write_answer_at(tab.branch_delete_answer)
        .expect("the card's own answer");
    assert_eq!(answer.report_kind, "delete");
    assert_eq!(answer.report_reason, "refused");
}

/// One answer as the bridge carries it, under the id its press was
/// given, with something to report or without.
fn reported(id: u64, kind: K, error: &str, report: Option<platitude_core::WriteReport>) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind,
        running: false,
        error: error.into(),
        report,
        head_seq: 0,
        reads_from: 0,
    }
}

fn answered(id: u64, kind: K, error: &str) -> TabMsg {
    reported(id, kind, error, None)
}

/// The other two of the write's three boundaries (`session::write`).
fn started(id: u64, kind: K) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind,
        running: true,
        error: String::new(),
        report: None,
        head_seq: 0,
        reads_from: 0,
    }
}

/// The last boundary. The kind is taken and dropped so the calls below
/// read like the pair above them — what a settle says is the id, and
/// whose write it was is the caller's own business.
fn settle_msg(id: u64, _kind: K) -> TabMsg {
    TabMsg::WriteSettled { id }
}

fn stash_answered(id: u64, error: &str) -> TabMsg {
    answered(id, K::Stash, error)
}

// Two stash answers in one drain — an apply that landed and the pop
// pressed behind it, which git refused — look alike to the group and to
// the op name. The id each press was given finds its own answer, and
// reads nothing into which came first or how many there were.
#[test]
fn an_answer_is_found_by_the_id_its_press_was_given_whatever_came_in_between() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![
        stash_answered(7, ""),
        stash_answered(
            8,
            "error: Your local changes to the following files would be overwritten by merge",
        ),
    ]);
    assert_eq!(tab.write_answer_index_of(8), Some(1));
    assert!(
        tab.write_answer_at(1).is_some_and(|a| a.failed),
        "the pop's own answer says it was refused"
    );
    assert_eq!(tab.write_answer_index_of(7), Some(0));
    assert!(
        tab.write_answer_at(0).is_some_and(|a| !a.failed),
        "the apply's own answer says it landed"
    );
    assert_eq!(
        tab.write_answer_index_of(9),
        None,
        "a press that did not answer in this notify is not found"
    );
    assert!(
        tab.write_refused,
        "the group describes the last answer, which was the refusal"
    );
}

// The same two answers the other way round: the id is what finds the
// answer, not its place in the list.
#[test]
fn the_id_finds_the_answer_wherever_it_stands_in_the_list() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![stash_answered(8, "refused"), stash_answered(7, "")]);
    assert_eq!(tab.write_answer_index_of(8), Some(0));
    assert_eq!(tab.write_answer_index_of(7), Some(1));
    assert!(tab.write_answer_at(0).is_some_and(|a| a.failed));
}

// The list is this notify's and no other: a notify raised from
// somewhere else carries no answers, and the one before it must not be
// read a second time.
#[test]
fn a_notify_that_carried_no_answer_says_so() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![stash_answered(7, "")]);
    assert_eq!(tab.write_answer_index_of(7), Some(0));
    tab.absorb(vec![TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }]);
    assert_eq!(
        tab.write_answer_index_of(7),
        None,
        "the answer this notify did not carry is not found in it"
    );
}

// ---- the editor's commit, found by the id its press was given --------

/// The id the queue took the editor's commit under, and one it gave
/// somebody else. What these tests are about is that the editor's answer
/// is found by the number rather than by its turn.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

/// A tab whose editor has sent a commit and is waiting for it
/// (`state::commit_from_fields` writes the id down at the press).
fn editor_pressed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.commit_out.asked(Some(OURS));
    tab
}

/// What this notify carried for that editor, if anything.
fn for_the_editor(tab: &RepoTab) -> Option<&WriteAnswer> {
    tab.write_answer_at(tab.commit_answer)
}

// The fetch left running comes back while the commit's answer is still
// being waited out, and the two meet in one drain (`take_feed` empties
// the queue whole and notifies once). Written into one group, the
// fetch's answer — landed, nothing to report — is the one the page would
// read, and the editor would sit there full of a message that is already
// a commit.
#[test]
fn the_editors_commit_answers_over_the_fetch_behind_it() {
    let mut tab = editor_pressed();
    tab.absorb(vec![
        answered(OURS, K::Commit, ""),
        answered(SOMEBODY_ELSE, K::Fetch, ""),
    ]);
    let answer = for_the_editor(&tab).expect("the editor's own answer");
    assert_eq!(answer.kind, K::Commit);
    assert!(!answer.failed, "git wrote the commit");
    assert!(
        tab.write_fetched,
        "and the fetch is still the group's — it is nobody's by name"
    );
}

// The other way round, to say that nothing reads the order: the fetch
// answers first and the commit behind it.
#[test]
fn the_order_the_two_came_back_in_is_not_read_into() {
    let mut tab = editor_pressed();
    tab.absorb(vec![
        answered(SOMEBODY_ELSE, K::Fetch, ""),
        answered(OURS, K::Commit, ""),
    ]);
    let answer = for_the_editor(&tab).expect("the editor's own answer");
    assert_eq!(answer.kind, K::Commit);
    assert!(!answer.failed);
    assert!(tab.write_fetched);
}

// Once. The answer belongs to the notify that carried it, so the drain
// after it tells the editor nothing — a second telling would empty boxes
// somebody has since typed into.
#[test]
fn the_editor_is_told_once() {
    let mut tab = editor_pressed();
    tab.absorb(vec![answered(OURS, K::Commit, "")]);
    assert!(for_the_editor(&tab).is_some());
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Fetch, "")]);
    assert_eq!(tab.commit_answer, -1, "this drain carried none of it");
    tab.absorb(vec![TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }]);
    assert_eq!(tab.commit_answer, -1);
}

// A hook turned the commit down while a stash pressed behind it landed.
// The refusal is the one with something to say, and it is the one a
// single group loses: read there, the page would find a landing with
// nothing to report and the words would never reach the screen.
#[test]
fn a_refused_commit_keeps_its_words_past_the_landing_beside_it() {
    let mut tab = editor_pressed();
    tab.absorb(vec![
        reported(
            OURS,
            K::Commit,
            "`git commit` exited with code 1",
            Some(platitude_core::WriteReport::local(
                ReportKind::Commit,
                "lint found 1 problem".into(),
            )),
        ),
        answered(SOMEBODY_ELSE, K::Stash, ""),
    ]);
    let answer = for_the_editor(&tab).expect("the editor's own answer");
    assert!(answer.failed, "a rejected commit keeps the editor's text");
    assert_eq!(answer.report_kind, "commit");
    assert_eq!(answer.report_reason, "lint found 1 problem");
    // …and it is reported once: the group is the stash's, so nothing the
    // page reads there can report the same refusal a second time.
    assert!(!tab.write_refused);
    assert!(
        tab.write_stale_diff,
        "the landing beside it is still the group's own"
    );
    assert_eq!(tab.write_report_kind, "");
}

// The group is this notify's, like the answers beside it. A refusal the
// page has already reported would otherwise still be standing in it when
// the next drain notifies, and be reported over again on an answer that
// went somewhere else entirely.
#[test]
fn a_drain_whose_answer_had_an_owner_leaves_the_group_saying_nothing() {
    let mut tab = editor_pressed();
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Push, "! [rejected]")]);
    assert!(tab.write_refused);
    tab.absorb(vec![answered(OURS, K::Commit, "")]);
    assert!(
        !tab.write_refused,
        "the push's refusal was the last drain's news"
    );
    assert!(for_the_editor(&tab).is_some());
}

// ---- the stash that emptied the tree ---------------------------------

/// The report of HEAD a write's answer names as the first whose counts
/// can speak for what it left, and the tree those counts describe.
const FENCE: u64 = 41;
const EMPTIED: bool = true;

/// One answer as the bridge carries it, naming that report.
fn fenced(id: u64, kind: K, error: &str) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind,
        running: false,
        error: error.into(),
        report: None,
        head_seq: FENCE,
        reads_from: 0,
    }
}

/// A tab whose band press put the working tree away and is waiting for
/// the reading that finds the tree gone (`ops_remote::stash_push` writes
/// the id down at the press).
fn tree_stashed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.stash_out.asked(Some(OURS));
    tab
}

// The fetch running behind the press answers in the same drain, and it
// is the one a property every answer rewrites is left describing: read
// there, the tree emptying behind it is nobody's doing, the reader is
// left standing over a pane that describes nothing, and the viewport
// never follows the entry they just made.
#[test]
fn the_stash_that_emptied_the_tree_is_ours_over_the_fetch_behind_it() {
    let mut tab = tree_stashed();
    tab.absorb(vec![
        fenced(OURS, K::Stash, ""),
        answered(SOMEBODY_ELSE, K::Fetch, ""),
    ]);
    assert!(tab.stash_out.ours());
    let answer = tab
        .write_answer_at(tab.stash_answer)
        .expect("the press's own answer");
    assert_eq!(answer.kind, K::Stash);
    assert!(
        !answer.failed,
        "and the file it left stale is read off that answer, \
         not off a group the fetch behind it rewrites"
    );
    assert!(
        tab.write_fetched,
        "while the group is the fetch's, being the last answer nobody named"
    );
}

// …and it stands past the drains in between, because the status whose
// counts find the tree empty is a feed of its own and arrives long after.
#[test]
fn the_landing_stands_until_the_tree_is_read() {
    let mut tab = tree_stashed();
    tab.absorb(vec![fenced(OURS, K::Stash, "")]);
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Fetch, "")]);
    tab.absorb(vec![TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }]);
    assert!(tab.stash_out.ours(), "nothing in between answered for it");
    assert_eq!(
        tab.stash_answer, -1,
        "while where it answered went with its own notify"
    );
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(tab.stash_landing_taken());
    assert!(
        !tab.stash_landing_taken(),
        "one press empties one tree, and the reading settles it"
    );
}

// A stash git would not make took no tree away, so the tree emptying
// next emptied for some other reason — but the words it was refused with
// are still the press's own to say.
#[test]
fn a_refused_stash_claims_no_tree_and_keeps_its_words() {
    let mut tab = tree_stashed();
    tab.absorb(vec![fenced(
        OURS,
        K::Stash,
        "error: Your local changes would be overwritten",
    )]);
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(!tab.stash_landing_taken());
    assert!(
        tab.write_answer_at(tab.stash_answer)
            .is_some_and(|a| a.failed),
        "the press's own answer says git would not do it"
    );
    assert!(
        !tab.write_refused,
        "so nothing is left over to say it twice"
    );
}

// Every stash operation answers under the same word, so the answer alone
// cannot say it was the press that emptied the tree: a pop pressed from
// the details band lands in the same drain and looks identical.
#[test]
fn somebody_elses_stash_answer_claims_no_tree() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![fenced(SOMEBODY_ELSE, K::Stash, "")]);
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(!tab.stash_landing_taken());
    assert_eq!(tab.stash_answer, -1);
    assert!(
        tab.write_stale_diff,
        "and it falls to the group like any other nobody named"
    );
}

// **The other order.** The status the press published can be applied
// before the page has read the answer that says whose it was: the two
// are drained apart. Dropped there, the tree emptying is gone — the next
// status has no change to report, and the reader is left standing over a
// pane describing a tree that is not there.
#[test]
fn a_tree_read_empty_before_the_answer_still_lands_the_press() {
    let mut tab = tree_stashed();
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(!tab.stash_landing_taken(), "nothing to settle on yet");
    tab.absorb(vec![fenced(OURS, K::Stash, "")]);
    assert!(
        tab.stash_landing_taken(),
        "the answer settles on the tree already read"
    );
}

// …and the same drain brings somebody else's refusal, which is what the
// group is left describing. **Everything the landing needs is in this
// notify** — the tree was read before it and the answer is in it — so
// the press is owed its exit here, and nothing is coming to ask again:
// the status that would have is the one that already went by. Sequenced
// off the group, the refusal's branch ends the whole of the page's
// answer before the exit is reached, and the reader sits on a pane
// describing a tree that is gone until something else moves.
#[test]
fn a_landing_owed_in_the_drain_that_refuses_somebody_elses_write() {
    let mut tab = tree_stashed();
    tab.tree_was_read(FENCE, EMPTIED);
    // A push rather than a fetch: a failed fetch raises
    // `fetch_first_failed`, and a signal needs the proxy no unit test
    // has. Which write it was changes nothing — the group's branch ends
    // the page's answer on any refusal it is left describing.
    tab.absorb(vec![
        fenced(OURS, K::Stash, ""),
        answered(SOMEBODY_ELSE, K::Push, "! [rejected]"),
    ]);
    assert!(
        tab.write_refused,
        "the group is the push's, and its branch returns early"
    );
    assert!(
        tab.stash_landing_taken(),
        "while the press's own exit is owed in this very notify"
    );
    assert!(
        !tab.stash_landing_taken(),
        "and owed once — nothing is left for a later status to pay again"
    );
}

/// A tab whose run has armed its watch and had an ask taken under `id` —
/// what `ask_session` does inside the call that returns that id, which a
/// tab with no session behind it cannot be driven through.
fn watching(id: u64) -> RepoTab {
    let mut tab = RepoTab::default();
    tab.watch_next_write("press".into());
    tab.write_watch.asked(Some(id));
    tab
}

/// **The two boundaries reach the watch off the feed's own messages** —
/// the wiring `write_watch`'s own tests cannot see. Nothing else opens it:
/// the fetch a timer fires and the one an opening makes come down the same
/// feed, and the answer to each carries its own id.
///
/// Before the watch the barrier armed on a count and waited for it to
/// move. Neither half of that was about the press: those fetches move
/// `writeSeq` on their own, and between the press and the moment git has
/// the write there is a stretch where nothing is running — so one of them
/// landing inside it satisfied both halves, and the picture was taken
/// before the write ran (P3-確認事項).
#[test]
fn the_boundaries_reach_the_watch_and_nothing_else_opens_it() {
    let mut tab = watching(OURS);
    tab.absorb(vec![
        started(SOMEBODY_ELSE, K::AutoFetch),
        answered(SOMEBODY_ELSE, K::AutoFetch, ""),
        settle_msg(SOMEBODY_ELSE, K::AutoFetch),
    ]);
    assert_eq!(tab.write_seq, 1, "the answer is counted as an answer");
    assert!(
        !tab.wrote_through(),
        "but it is not the one being waited on"
    );

    tab.absorb(vec![answered(OURS, K::Commit, "")]);
    assert_eq!(
        tab.write_watch_stage(),
        "answered",
        "git has this one; the reads behind it are still out"
    );
    assert!(!tab.wrote_through());

    tab.absorb(vec![settle_msg(OURS, K::Commit)]);
    assert_eq!(tab.write_watch_stage(), "settled");
    assert!(tab.wrote_through());
}

/// **An ask that was numbered first and queued second**: the run's own
/// write carries the *lower* id and finishes *last*, and the barrier opens
/// on it and on nothing before it.
///
/// This is the arrangement the identity comparison exists for.
/// `OperationId::next()` and the queue's own lock are not the same moment
/// — a request takes its number and then queues — so a caller can be
/// numbered and then lose its turn. A boundary read as `>=` would have
/// opened here on somebody else's answer, and the run would have gone on
/// to photograph a page its own write had not reached.
#[test]
fn a_write_numbered_after_this_one_and_finished_first_does_not_open_it() {
    let mut tab = watching(OURS);
    assert!(!tab.wrote_through(), "nothing has answered yet");

    // Somebody else's write, in front in the queue and higher in the
    // numbering, all the way through.
    tab.absorb(vec![
        started(SOMEBODY_ELSE, K::Branch),
        answered(SOMEBODY_ELSE, K::Branch, ""),
        settle_msg(SOMEBODY_ELSE, K::Branch),
    ]);
    assert!(
        !tab.wrote_through(),
        "an id past this one's is not this one's"
    );
    assert_eq!(
        tab.watched_write_id(),
        OURS as i32,
        "the watch keeps its own"
    );

    tab.absorb(vec![
        answered(OURS, K::Commit, ""),
        settle_msg(OURS, K::Commit),
    ]);
    assert!(tab.wrote_through(), "and this write's own finish opens it");
}

/// The same the other way round — the run's own ask numbered *after* the
/// one that finishes first. Neither direction is the test on its own: one
/// catches `>=`, the other catches `<=`.
#[test]
fn nor_does_one_numbered_before_it() {
    let mut tab = watching(SOMEBODY_ELSE);
    tab.absorb(vec![
        started(OURS, K::Commit),
        answered(OURS, K::Commit, ""),
        settle_msg(OURS, K::Commit),
    ]);
    assert!(!tab.wrote_through());

    tab.absorb(vec![
        answered(SOMEBODY_ELSE, K::Branch, ""),
        settle_msg(SOMEBODY_ELSE, K::Branch),
    ]);
    assert!(tab.wrote_through());
}

/// **A run that has asked for nothing passes nothing.** The watch is
/// asleep, and no answer to anybody's write makes it say otherwise.
#[test]
fn a_run_with_no_write_of_its_own_never_passes() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![
        started(OURS, K::Commit),
        answered(OURS, K::Commit, ""),
        settle_msg(OURS, K::Commit),
    ]);
    assert_eq!(tab.write_watch_stage(), "asleep");
    assert_eq!(tab.watched_write_id(), 0);
    assert!(!tab.wrote_through());
}

/// **What the run reads to hold its own contract**: the stage and the id
/// are the whole of it, and the tab refuses nothing.
///
/// Arming over a write still out is a breach when the run had pressed for
/// that write and not otherwise — what the watch is carrying is as often
/// as not one nobody pressed for (the read a menu makes on its way open),
/// and only the run knows which. So the tab answers where it stands and
/// leaves the judgement upstairs (`AutoActDriver.beginWrite`).
#[test]
fn the_tab_says_where_the_watch_stands_and_judges_nothing() {
    let mut tab = watching(OURS);
    assert_eq!(tab.write_watch_stage(), "held");
    assert_eq!(tab.watched_write_id(), OURS as i32);

    tab.watch_next_write("press".into());
    assert_eq!(tab.write_watch_stage(), "armed", "and it arms regardless");
    assert_eq!(tab.watched_write_id(), 0);

    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Fetch, "")]);
    assert_eq!(
        tab.write_watch_stage(),
        "armed",
        "a write nothing is armed for leaves it where it is"
    );
}

// ---- the pushes a ref row sends ---------------------------------------

/// The row pressed: a `push --delete` went to the queue under `OURS`, for
/// the remote row `origin/topic`.
fn ref_push_pressed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.ref_push_asked("origin/topic", Some(OURS));
    tab
}

/// The far side's refusal reaches the press that asked for it, whichever
/// order the drain carried the two answers in.
///
/// **All of these answer under the word `push`** — the two `push
/// --delete`s, the rename git has no command for, the two pairs that
/// reach over there after doing something here — and so does the fetch
/// running behind them. One drain empties the whole queue and notifies
/// once, and the group the page reads when any answer will do describes
/// whichever ownerless answer finished last: a fetch that landed after
/// the refusal took the report away with it, and the reader was told
/// nothing at all (P3-確認事項, observed).
///
/// Both orders, because only one of them is the bug: a test that ran the
/// other alone would pass against the arrangement that lost it.
#[test]
fn a_ref_rows_push_keeps_its_refusal_when_a_fetch_answers_beside_it() {
    for fetch_first in [false, true] {
        let mut tab = ref_push_pressed();
        let refusal = reported(
            OURS,
            K::Push,
            "remote: Cannot delete a protected branch",
            Some(platitude_core::WriteReport::on_remote(
                ReportKind::RemoteDelete,
                "origin",
                "topic",
                "Cannot delete a protected branch".into(),
            )),
        );
        let fetch = answered(SOMEBODY_ELSE, K::Fetch, "");
        tab.absorb(if fetch_first {
            vec![fetch, refusal]
        } else {
            vec![refusal, fetch]
        });

        let at = tab.ref_push_answer;
        assert!(
            at >= 0,
            "the press's own answer, by id (fetch first: {fetch_first})"
        );
        let answer = tab.write_answer_at(at).expect("the answer it points at");
        assert!(answer.failed, "and it is the refusal, not the fetch");
        assert_eq!(answer.report_kind, "delete");
        assert_eq!(answer.report_reason, "Cannot delete a protected branch");
        assert_eq!(
            tab.ref_push_target, "origin/topic",
            "named with the row it was sent for"
        );
        // Which is exactly why it cannot be read off the group: that one
        // is the fetch's, and it landed.
        assert!(tab.write_fetched && !tab.write_refused);
        assert_eq!(tab.write_report_kind, "");
    }
}

/// And the next press starts from nothing. An answer left standing would
/// be read a second time under a press this drain brought no answer for
/// at all — the same thing every other owner puts down at the top of a
/// drain (`Press::new_notify`).
#[test]
fn a_ref_rows_push_does_not_hand_the_next_press_the_last_answer() {
    let mut tab = ref_push_pressed();
    tab.absorb(vec![answered(OURS, K::Push, "refused")]);
    assert!(tab.ref_push_answer >= 0);

    tab.ref_push_asked("origin/other", Some(SOMEBODY_ELSE));
    tab.absorb(vec![answered(OURS, K::Push, "refused")]);
    assert_eq!(
        tab.ref_push_answer, -1,
        "the answer standing is the first press's, and that press is over"
    );
    assert_eq!(
        tab.ref_push_target, "origin/other",
        "and the row waited for is the one just pressed"
    );
}
