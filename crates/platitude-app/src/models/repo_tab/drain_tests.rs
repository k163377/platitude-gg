//! What one answer out of the feed leaves the properties QML reads
//! saying (`drain::settle_write`).

use super::*;
use platitude_core::OperationKind as K;

fn settled(kind: K, error: &str) -> RepoTab {
    let mut tab = RepoTab::default();
    tab.settle_write(1, kind, error.into(), None, 0, 0);
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

#[test]
fn a_tag_answer_says_so_whichever_way_it_went() {
    assert!(settled(K::Tag, "").write_tag_op);
    assert!(settled(K::Tag, "fatal: tag 'v1.0' already exists").write_tag_op);
    assert!(settled(K::DeleteTagEverywhere, "").write_tag_op);
    assert!(!settled(K::Branch, "").write_tag_op);
    // A rename's question sends a push, so its own answer must not
    // re-raise it.
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

// The reader switched branches and pushed again before the first came
// back.
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

/// The refused half is the one the page reads: a failed fetch has already
/// raised its panel, which stays the fetch's to take down
/// (`CommandsOwner`); a refused push is other news in the same panel.
#[test]
fn a_fetch_answer_says_so_whichever_way_it_went() {
    assert!(settled(K::Fetch, "").write_fetched);
    let mut tab = RepoTab::default();
    // Panel already raised, so no `fetch_first_failed` is emitted — a
    // signal needs the proxy no unit test has.
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

#[test]
fn every_answer_rewrites_the_whole_group() {
    let mut tab = RepoTab::default();
    tab.settle_write(1, K::Reword, String::new(), None, 0, 0);
    assert!(tab.write_reworded);
    // A checkout, not a fetch: a failed fetch emits `fetch_first_failed`.
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

/// A run's answer and the fetch the freeze left running, in one drain
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
    tab.settle_write(1, K::Push, String::new(), None, 0, 0);
    let before = tab.write_seq;
    tab.absorb(a_run_and_the_fetch_behind_it());
    let seqs: Vec<i32> = tab.write_answers.iter().map(|a| a.seq).collect();
    assert_eq!(seqs, [before + 1, before + 2]);
}

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

/// A plain `branch --delete` out, its card standing for the answer
/// (`ops_delete::branch_delete`).
fn card_standing() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.arm_branch_delete("feature", Some(OURS));
    tab
}

// The card reads the answer by its own name (`RefBranchMenu`): taken, it
// goes; turned down, its row turns into `-D`. The fetch behind the press
// answers in the same drain and looks exactly like an answer to this one.
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
    // The row is the answer, so nothing is left over for the command log.
    assert!(!tab.write_refused);
}

// `-D`, a create, a rename and `Delete both` leave no card standing.
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

// The name stands (the card reads it as an edge and may not be drawn
// yet); where it answered is the notify's own.
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

// A reported refusal is the notice bar's to say; the card's answer still
// points the page at the words.
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

/// One answer as the bridge carries it.
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

/// The write starting — none of its three boundaries (`session::write`).
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

/// The last boundary. A settle carries only the id; `_kind` keeps the
/// calls reading like the pair above.
fn settle_msg(id: u64, _kind: K) -> TabMsg {
    TabMsg::WriteSettled { id }
}

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

/// The id the queue gave the press under test, and one it gave somebody
/// else.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

/// The editor sent a commit and waits for it
/// (`state::commit_from_fields`).
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
    tab.absorb(vec![TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }]);
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
    // Reported once: the group is the stash's.
    assert!(!tab.write_refused);
    assert!(
        tab.write_stale_diff,
        "the landing beside it is still the group's own"
    );
    assert_eq!(tab.write_report_kind, "");
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

// It stands past the drains in between: the status that finds the tree
// empty is a feed of its own and arrives long after.
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

// A refused stash took no tree away, but its words are still the press's
// to say.
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

// Every stash operation answers as `Stash`: a pop from the details band
// looks identical.
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

// The other order: status and answer are drained apart. Dropped there, the
// emptying is lost — the next status has no change to report.
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
        tab.stash_landing_taken(),
        "while the press's own exit is owed in this very notify"
    );
    assert!(
        !tab.stash_landing_taken(),
        "and owed once — nothing is left for a later status to pay again"
    );
}

/// A run armed its watch and had an ask taken under `id` — by hand, since
/// `ask_session` needs a session.
fn watching(id: u64) -> RepoTab {
    let mut tab = RepoTab::default();
    tab.watch_next_write("press".into());
    tab.write_watch.asked(Some(id));
    tab
}

/// The feed's own messages reach the watch — the wiring `write_watch`'s
/// own tests cannot see — and the timer's and an opening's fetches on the
/// same feed do not open it. Waiting on `writeSeq` instead is fooled by
/// them.
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

/// The run's write carries the lower id and finishes last. Ids come off one
/// counter for every session and answer to equality alone (`OperationId`):
/// nothing ties their order to the order writes finish, so a `>=` boundary
/// would open here on somebody else's answer.
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

/// The mirror: one direction catches `>=`, the other `<=`.
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

/// The tab reports stage and id and refuses nothing: whether arming over a
/// write still out is a breach only the run knows — often that write is
/// one nobody pressed for, like a menu's read on its way open
/// (`AutoActDriver.beginWrite`).
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
        // The group is the fetch's, which landed.
        assert!(tab.write_fetched && !tab.write_refused);
        assert_eq!(tab.write_report_kind, "");
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
