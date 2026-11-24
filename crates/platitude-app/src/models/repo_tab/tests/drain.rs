//! What a write's answer out of the feed leaves the properties QML reads
//! saying (`drain::settle_write`): the group of the last answer nobody
//! waits for by name, the notify's list of answers, and the answer handed
//! back to the press that sent it.

use super::super::*;
use super::drain_inputs::{OURS, SOMEBODY_ELSE, answered, no_answer, reported, settle, started};
use platitude_core::OperationKind as K;

// ---- the group: what the last answer nobody named says ---------------

fn settled(kind: K, error: &str) -> RepoTab {
    let mut tab = RepoTab::default();
    settle(&mut tab, kind, error, None);
    tab
}

/// Whether the answer this tab took last left a commit at the tip — kept
/// per answer, not in the group (`WriteAnswer::at_tip`).
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

// No editor waits: the page that sent it is gone, or the queue took nothing.
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
    // A pull lands the same way: the reader is put on what it brought in.
    assert!(at_tip(&settled(K::Pull, "")));
    assert!(!at_tip(&settled(K::Merge, "fatal: refusing to merge")));
}

#[test]
fn a_merge_that_stopped_does_not_claim_the_tip() {
    // The stop arrives before the answer that ends the write
    // (`TabMsg::WriteStopped`), so the flag is already standing.
    let mut tab = RepoTab {
        last_write_stopped: true,
        ..RepoTab::default()
    };
    settle(&mut tab, K::Merge, "", None);
    assert!(!at_tip(&tab), "nothing landed at the tip to go to");
}

#[test]
fn a_branch_answer_says_so_whichever_way_it_went() {
    assert!(settled(K::Branch, "").write_branch_op);
    assert!(settled(K::Branch, "error: not fully merged").write_branch_op);
    assert!(!settled(K::Tag, "").write_branch_op);
}

#[test]
fn a_tag_answer_says_so_whichever_way_it_went() {
    assert!(settled(K::Tag, "").write_tag_op);
    assert!(settled(K::Tag, "fatal: tag 'v1.0' already exists").write_tag_op);
    assert!(settled(K::DeleteTagEverywhere, "").write_tag_op);
    assert!(!settled(K::Branch, "").write_tag_op);
    // A rename's question sends a push, so its own answer must not re-raise it.
    assert!(!settled(K::Push, "").write_tag_op);
}

// ---- the toolbar's push ----------------------------------------------

fn push_pressed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.push_out.asked(Some(OURS), "topic".into());
    tab
}

// A refused push and the fetch that landed behind it, in one drain: the
// fetch is what is left over for the group.
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

// The report rides on the answer, where the page reads it into the bar
// (`RepoPage.absorbPushAnswer`); a copy in the group would say it twice.
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
        )),
    )]);
    assert_eq!(tab.push_answer, 0);
    let answer = &tab.write_answers[0];
    assert!(answer.failed);
    assert_eq!(answer.report.as_ref().expect("a report").kind, "outdated");
    assert_eq!(answer.report.as_ref().expect("a report").remote, "origin");
    assert_eq!(answer.report.as_ref().expect("a report").name, "topic");
    assert_eq!(answer.report.as_ref().expect("a report").fact, "");
    assert!(
        !tab.write_refused,
        "the report is the answer's, not the group's"
    );
    assert_eq!(tab.write_report, None);
}

// The reader switched branches and pushed again before the first came back.
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

// Sent by a page that has since gone. Both owners (toolbar and ref row)
// are empty, which is what makes this the leftover case.
#[test]
fn a_push_nobody_here_pressed_for_falls_to_the_group() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Push, "! [rejected]")]);
    assert_eq!(tab.push_answer, -1);
    assert!(tab.write_refused);
    assert_eq!(tab.last_write_error, "! [rejected]");
}

// The next drain says -1, so the page cannot act on the same refusal twice.
#[test]
fn the_push_answer_is_this_notifys_own() {
    let mut tab = push_pressed();
    tab.absorb(vec![answered(OURS, K::Push, "! [rejected]")]);
    assert_eq!(tab.push_answer, 0);
    tab.absorb(vec![no_answer()]);
    assert_eq!(tab.push_answer, -1);
    assert_eq!(
        tab.push_answer_branch, "topic",
        "the branch outlives the answer, for the refusal remembered against it"
    );
}

// ---- the group: a fetch, a move of HEAD, a reword --------------------

/// The refused half is the one the page reads: a failed fetch has already
/// raised its panel, which stays the fetch's to take down
/// (`CommandsOwner`); a refused push is other news in the same panel.
#[test]
fn a_fetch_answer_says_so_whichever_way_it_went() {
    assert!(settled(K::Fetch, "").write_fetched);
    // Panel already raised, so no `fetch_first_failed` is emitted — a
    // signal needs the proxy no unit test has.
    let mut tab = RepoTab {
        fetch_log_raised: true,
        ..RepoTab::default()
    };
    settle(
        &mut tab,
        K::Fetch,
        "fatal: Could not read from remote",
        None,
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

#[test]
fn every_answer_rewrites_the_whole_group() {
    let mut tab = RepoTab::default();
    settle(&mut tab, K::Reword, "", None);
    assert!(tab.write_reworded);
    // A checkout, not a fetch: a failed fetch emits `fetch_first_failed`.
    settle(&mut tab, K::Checkout, "fatal: invalid reference", None);
    assert!(
        !tab.write_reworded,
        "nothing armed survives the next answer"
    );
    assert!(tab.write_refused);
    assert_eq!(tab.write_seq, 2);
}

// ---- the notify's list of answers ------------------------------------

/// A run's answer and the fetch the freeze left running, in one drain
/// (`drain` takes the queue whole and notifies once).
fn a_run_and_the_fetch_behind_it() -> Vec<TabMsg> {
    vec![
        TabMsg::WriteStopped,
        answered(1, K::Rebase, ""),
        started(1, K::Fetch),
        answered(1, K::Fetch, ""),
    ]
}

// The group describes only the last answer: the run's own is lost there,
// and its stop is taken down by the fetch *starting*. The list keeps both.
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

/// A run tells its answer by the counter it read before pressing, so one
/// already counted cannot arm it.
#[test]
fn each_answer_carries_the_seq_it_was_counted_at() {
    let mut tab = RepoTab::default();
    settle(&mut tab, K::Push, "", None);
    let before = tab.write_seq;
    tab.absorb(a_run_and_the_fetch_behind_it());
    let seqs: Vec<i32> = tab.write_answers.iter().map(|a| a.seq).collect();
    assert_eq!(seqs, [before + 1, before + 2]);
}

#[test]
fn a_landing_at_the_tip_is_found_in_the_list_a_fetch_answered_over() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![answered(1, K::Merge, ""), answered(1, K::Fetch, "")]);
    assert!(!at_tip(&tab), "the answer that came last is the fetch's");
    let landed: Vec<K> = tab
        .write_answers
        .iter()
        .filter(|a| a.at_tip)
        .map(|a| a.kind)
        .collect();
    assert_eq!(landed, [K::Merge]);
}

#[test]
fn a_drain_with_no_write_answer_publishes_none() {
    let mut tab = RepoTab::default();
    tab.absorb(a_run_and_the_fetch_behind_it());
    assert_eq!(tab.write_answers.len(), 2);
    tab.absorb(vec![no_answer()]);
    assert!(tab.write_answers.is_empty());
}

// ---- the plain branch delete a menu stays up for ---------------------

/// A plain `branch --delete` out, its menu standing for the answer
/// (`ops_delete::branch_delete`).
fn menu_standing() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", Some(OURS));
    tab
}

// The card reads the answer by its own name (`RefBranchMenu`): taken, the
// menu goes; turned down, the delete row turns into `-D`. The fetch behind
// the press answers in the same drain and looks exactly like an answer to
// this one.
#[test]
fn a_plain_delete_git_took_names_the_branch_over_the_fetch_behind_it() {
    let mut tab = menu_standing();
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
    let mut tab = menu_standing();
    tab.absorb(vec![answered(
        OURS,
        K::Branch,
        "error: the branch 'feature' is not fully merged",
    )]);
    assert_eq!(tab.branch_delete_refused, "feature");
    assert_eq!(tab.branch_delete_landed, "");
    // The row is the answer, so nothing is left over for the command log.
    assert!(!tab.write_refused);
}

// `-D`, a create, a rename and `Delete both` leave no menu standing.
#[test]
fn a_branch_answer_nobody_stayed_up_for_names_nothing() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![answered(OURS, K::Branch, "error: not fully merged")]);
    assert_eq!(tab.branch_delete_refused, "");
    assert!(
        tab.write_refused,
        "and it falls to the group like any other nobody named"
    );
}

// The name stands: the card reads it as an edge and may not be drawn yet.
#[test]
fn the_delete_answer_stands_but_says_which_notify_carried_it() {
    let mut tab = menu_standing();
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

// A reported refusal is the notice bar's to say; the plain delete's own
// answer still points the page at the words.
#[test]
fn a_refusal_with_a_report_turns_no_row() {
    let mut tab = menu_standing();
    tab.absorb(vec![reported(
        OURS,
        K::Branch,
        "remote: refused",
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteDelete(platitude_core::RemoteWhy::Other),
            "origin",
            "feature",
        )),
    )]);
    assert_eq!(tab.branch_delete_refused, "");
    assert_eq!(tab.branch_delete_landed, "");
    let answer = tab
        .write_answer_at(tab.branch_delete_answer)
        .expect("the plain delete's own answer");
    assert_eq!(
        answer.report.as_ref().expect("a report").kind,
        "delete-refused"
    );
}

// ---- the list, searched by a press's id ------------------------------

fn stash_answered(id: u64, error: &str) -> TabMsg {
    answered(id, K::Stash, error)
}

// An apply that landed and a pop git refused look alike to the group and
// to the op name; only the id tells them apart.
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

#[test]
fn the_id_finds_the_answer_wherever_it_stands_in_the_list() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![stash_answered(8, "refused"), stash_answered(7, "")]);
    assert_eq!(tab.write_answer_index_of(8), Some(0));
    assert_eq!(tab.write_answer_index_of(7), Some(1));
    assert!(tab.write_answer_at(0).is_some_and(|a| a.failed));
}

#[test]
fn a_notify_that_carried_no_answer_says_so() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![stash_answered(7, "")]);
    assert_eq!(tab.write_answer_index_of(7), Some(0));
    tab.absorb(vec![no_answer()]);
    assert_eq!(
        tab.write_answer_index_of(7),
        None,
        "the answer this notify did not carry is not found in it"
    );
}

// ---- the editor's commit, found by the id its press was given --------

/// The editor sent a commit and waits for it — its answer, and the reading
/// that finds the tree gone (`state::commit_from_fields`).
fn editor_pressed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.commit_out.asked(Some(OURS));
    tab
}

/// What this notify carried for that editor, if anything.
fn for_the_editor(tab: &RepoTab) -> Option<&WriteAnswer> {
    tab.write_answer_at(tab.commit_answer)
}

// Read off the group, the fetch's answer would be the one the page saw,
// and the editor would sit full of a message that is already a commit.
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

// A second telling would empty boxes somebody has since typed into.
#[test]
fn the_editor_is_told_once() {
    let mut tab = editor_pressed();
    tab.absorb(vec![answered(OURS, K::Commit, "")]);
    assert!(for_the_editor(&tab).is_some());
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Fetch, "")]);
    assert_eq!(tab.commit_answer, -1, "this drain carried none of it");
    tab.absorb(vec![no_answer()]);
    assert_eq!(tab.commit_answer, -1);
}

// A hook refused the commit while a stash behind it landed: read off the
// group, the page would find the landing and the words would never show.
#[test]
fn a_refused_commit_keeps_its_words_past_the_landing_beside_it() {
    let mut tab = editor_pressed();
    tab.absorb(vec![
        reported(
            OURS,
            K::Commit,
            "`git commit` exited with code 1",
            Some(platitude_core::WriteReport::local(ReportKind::CommitHook)),
        ),
        answered(SOMEBODY_ELSE, K::Stash, ""),
    ]);
    let answer = for_the_editor(&tab).expect("the editor's own answer");
    assert!(answer.failed, "a rejected commit keeps the editor's text");
    assert_eq!(
        answer.report.as_ref().expect("a report").kind,
        "commit-hook"
    );
    // Reported once: the group is the stash's.
    assert!(!tab.write_refused);
    assert!(
        tab.write_stale_diff,
        "the landing beside it is still the group's own"
    );
    assert_eq!(tab.write_report, None);
}

// The group is emptied per notify, or a refusal already reported would be
// reported again on the next drain's answer.
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
/// A report from before the write, whose counts are the tree as it was.
const IN_FLIGHT: u64 = 40;
const EMPTIED: bool = true;
const STILL_DIRTY: bool = false;

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

/// The band's stash press, waiting for the reading that finds the tree
/// gone (`ops_remote::stash_push`).
fn tree_stashed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.stash_out.asked(Some(OURS));
    tab
}

// Read off the group (the fetch's), the tree emptying is nobody's doing
// and the viewport never follows the entry just made.
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

// The status that finds the tree empty is a feed of its own and arrives
// long after.
#[test]
fn the_landing_stands_until_the_tree_is_read() {
    let mut tab = tree_stashed();
    tab.absorb(vec![fenced(OURS, K::Stash, "")]);
    tab.absorb(vec![answered(SOMEBODY_ELSE, K::Fetch, "")]);
    tab.absorb(vec![no_answer()]);
    assert!(tab.stash_out.ours(), "nothing in between answered for it");
    assert_eq!(
        tab.stash_answer, -1,
        "while where it answered went with its own notify"
    );
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(tab.stash_landing_taken().is_some());
    assert!(
        tab.stash_landing_taken().is_none(),
        "one press empties one tree, and the reading settles it"
    );
}

#[test]
fn a_refused_stash_claims_no_tree_and_keeps_its_words() {
    let mut tab = tree_stashed();
    tab.absorb(vec![fenced(
        OURS,
        K::Stash,
        "error: Your local changes would be overwritten",
    )]);
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(tab.stash_landing_taken().is_none());
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

// Every stash operation answers as `Stash`: a pop from the details band
// looks identical.
#[test]
fn somebody_elses_stash_answer_claims_no_tree() {
    let mut tab = RepoTab::default();
    tab.absorb(vec![fenced(SOMEBODY_ELSE, K::Stash, "")]);
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(tab.stash_landing_taken().is_none());
    assert_eq!(tab.stash_answer, -1);
    assert!(
        tab.write_stale_diff,
        "and it falls to the group like any other nobody named"
    );
}

// The other order: status and answer are drained apart. Dropped there, the
// emptying is lost — the next status has no change to report.
#[test]
fn a_tree_read_empty_before_the_answer_still_lands_the_press() {
    let mut tab = tree_stashed();
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(
        tab.stash_landing_taken().is_none(),
        "nothing to settle on yet"
    );
    tab.absorb(vec![fenced(OURS, K::Stash, "")]);
    assert!(
        tab.stash_landing_taken().is_some(),
        "the answer settles on the tree already read"
    );
}

// …with somebody else's refusal in the same drain. The tree was read
// before it, so this notify is the last chance to pay the exit; sequenced
// off the group, the refusal's branch returns before the exit is reached.
#[test]
fn a_landing_owed_in_the_drain_that_refuses_somebody_elses_write() {
    let mut tab = tree_stashed();
    tab.tree_was_read(FENCE, EMPTIED);
    // A push, not a fetch (`fetch_first_failed`); any refusal would do.
    tab.absorb(vec![
        fenced(OURS, K::Stash, ""),
        answered(SOMEBODY_ELSE, K::Push, "! [rejected]"),
    ]);
    assert!(
        tab.write_refused,
        "the group is the push's, and its branch returns early"
    );
    assert!(
        tab.stash_landing_taken().is_some(),
        "while the press's own exit is owed in this very notify"
    );
    assert!(
        tab.stash_landing_taken().is_none(),
        "and owed once — nothing is left for a later status to pay again"
    );
}

// ---- the commit that emptied the tree --------------------------------

// The landing that follows reads HEAD at or after the fence, never the
// report in hand — an amend of nothing leaves the counts as they were, and
// the HEAD in hand is the one it replaced.
#[test]
fn the_commit_that_emptied_the_tree_lands_on_its_fence_whichever_half_comes_first() {
    let mut tab = editor_pressed();
    tab.absorb(vec![fenced(OURS, K::Commit, "")]);
    tab.tree_was_read(IN_FLIGHT, EMPTIED);
    assert!(
        tab.commit_landing_taken().is_none(),
        "a status from before the write is the tree as it was"
    );
    tab.tree_was_read(FENCE, EMPTIED);
    assert_eq!(tab.commit_landing_taken(), Some(FENCE));
    assert!(tab.commit_landing_taken().is_none(), "paid once");

    let mut tab = editor_pressed();
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(
        tab.commit_landing_taken().is_none(),
        "nothing to settle on yet"
    );
    tab.absorb(vec![fenced(OURS, K::Commit, "")]);
    assert_eq!(tab.commit_landing_taken(), Some(FENCE));
}

// The editor still hears the refusal.
#[test]
fn a_commit_that_left_files_or_was_refused_claims_no_tree() {
    let mut tab = editor_pressed();
    tab.absorb(vec![fenced(OURS, K::Commit, "")]);
    tab.tree_was_read(FENCE, STILL_DIRTY);
    assert!(tab.commit_landing_taken().is_none());
    let mut tab = editor_pressed();
    tab.absorb(vec![fenced(OURS, K::Commit, "nothing to commit")]);
    tab.tree_was_read(FENCE, EMPTIED);
    assert!(tab.commit_landing_taken().is_none());
    assert!(
        tab.write_answer_at(tab.commit_answer)
            .is_some_and(|a| a.failed)
    );
}

// ---- the pushes a ref row sends --------------------------------------

fn ref_push_pressed() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.ref_push_asked("origin/topic", Some(OURS));
    tab
}

/// Every ref row push answers as `Push`, and the group describes whichever
/// ownerless answer finished last — a fetch landing after the refusal
/// would take the report with it. Both orders, because only one of them
/// fails against the group.
#[test]
fn a_ref_rows_push_keeps_its_refusal_when_a_fetch_answers_beside_it() {
    for fetch_first in [false, true] {
        let mut tab = ref_push_pressed();
        let refusal = reported(
            OURS,
            K::Push,
            "remote: Cannot delete a protected branch",
            Some(platitude_core::WriteReport::on_remote(
                ReportKind::RemoteDelete(platitude_core::RemoteWhy::Rule),
                "origin",
                "topic",
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
        assert_eq!(
            answer.report.as_ref().expect("a report").kind,
            "delete-rule"
        );
        assert_eq!(
            tab.ref_push_target, "origin/topic",
            "named with the row it was sent for"
        );
        // The group is the fetch's, which landed.
        assert!(tab.write_fetched && !tab.write_refused);
        assert_eq!(tab.write_report, None);
    }
}

/// An answer left standing would be read again under the next press, which
/// this drain brought no answer for (`Press::new_notify`).
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
