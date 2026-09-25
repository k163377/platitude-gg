//! What a delete leaves on screen at each of its three moments — the
//! press, git's answer, and the list that draws the reading proving the
//! row gone.

use super::*;

/// A press's id and somebody else's; any two different numbers do.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

/// Read stamps either side of the write's end (`Standing::fence`).
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

/// Every list has drawn a reading taken at `at`.
fn every_list_drew(gone: &mut StandIn, at: u64) {
    for row in [Row::Branch, Row::Remote, Row::Tag, Row::Stash] {
        gone.listing_applied(row, at);
    }
    gone.look_again();
}

/// One branch row taken away for a write the queue accepted.
fn branch_asked() -> StandIn {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Branch, "feature/x")], Some(OURS));
    gone
}

/// …and git took it, with the fence its listings are measured against.
fn branch_landed() -> StandIn {
    let mut gone = branch_asked();
    gone.answered(OURS, false, FENCE);
    gone
}

/// Both halves of `Delete both`, out under one write git has taken.
fn composite_landed() -> StandIn {
    let mut gone = StandIn::default();
    gone.asked(
        &[
            (Row::Branch, "feature/x"),
            (Row::Remote, "origin/feature/x"),
        ],
        Some(OURS),
    );
    gone.answered(OURS, false, FENCE);
    gone
}

#[test]
fn a_press_takes_its_row_off_the_screen_before_git_answers() {
    let gone = branch_asked();
    assert_eq!(gone.rows().branch, "feature/x");
    assert_eq!(gone.rows().remote, "");
}

// No answer is coming to put the row back: it would stay gone until the
// tab was read again.
#[test]
fn a_press_the_queue_would_not_take_leaves_its_row_alone() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Branch, "feature/x")], None);
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn a_press_numbered_zero_is_no_press_at_all() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Branch, "feature/x")], Some(0));
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn a_refusal_puts_back_exactly_what_the_press_took() {
    let mut gone = branch_asked();
    gone.answered(OURS, true, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// A fetch behind the press answers in the middle of it, and a stash drop
// answers under the same word as an apply pressed just before: counted by
// turn, somebody else's refusal would put this row back.
#[test]
fn somebody_elses_answer_leaves_the_row_standing() {
    let mut gone = branch_asked();
    gone.answered(SOMEBODY_ELSE, true, FENCE);
    assert_eq!(gone.rows().branch, "feature/x");
    // …and this write's own answer still lands afterwards, whatever came
    // in between.
    gone.answered(OURS, true, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// The list on screen, read before the write, still holds the row itself.
#[test]
fn a_landing_leaves_the_row_away_until_the_refs_arrive() {
    let mut gone = branch_landed();
    assert_eq!(gone.rows().branch, "feature/x");
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn a_listing_that_looked_before_the_write_ended_answers_for_nothing() {
    let mut gone = branch_landed();
    every_list_drew(&mut gone, IN_FLIGHT);
    assert_eq!(
        gone.rows().branch,
        "feature/x",
        "it was read before the write ended and says nothing about it"
    );
    // The write's own read, behind it, is what the row goes on.
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn a_listing_stamped_at_the_fence_is_the_writes_own() {
    let mut gone = branch_landed();
    every_list_drew(&mut gone, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn a_section_that_has_caught_up_answers_only_for_its_own_row() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Tag, "v1.0")], Some(OURS));
    gone.answered(OURS, false, FENCE);
    gone.listing_applied(Row::Branch, AFTERWARDS);
    gone.look_again();
    assert_eq!(
        gone.rows().tag,
        "v1.0",
        "the branches section has caught up; the tags section has not"
    );
    gone.listing_applied(Row::Tag, AFTERWARDS);
    gone.look_again();
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn the_composite_goes_one_row_at_a_time_as_its_lists_catch_up() {
    let mut gone = composite_landed();
    gone.listing_applied(Row::Remote, AFTERWARDS);
    gone.look_again();
    assert_eq!(gone.rows().remote, "", "the remotes section has caught up");
    assert_eq!(
        gone.rows().branch,
        "feature/x",
        "the branches section has not"
    );
    gone.listing_applied(Row::Branch, AFTERWARDS);
    gone.look_again();
    assert_eq!(gone.rows(), &Rows::default());
}

// A single newest-applied would pass this order too: only the pair binds.
#[test]
fn the_composite_goes_the_other_way_round_just_as_well() {
    let mut gone = composite_landed();
    gone.listing_applied(Row::Branch, AFTERWARDS);
    gone.look_again();
    assert_eq!(gone.rows().branch, "");
    assert_eq!(gone.rows().remote, "origin/feature/x");
    gone.listing_applied(Row::Remote, AFTERWARDS);
    gone.look_again();
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn a_reading_that_arrives_before_the_answer_answers_for_nothing() {
    let mut gone = branch_asked();
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows().branch, "feature/x");
}

// …but it is kept: whichever of the answer and the listing comes second
// completes the pair.
#[test]
fn a_listing_already_in_hand_settles_the_answer_that_follows_it() {
    let mut gone = branch_asked();
    every_list_drew(&mut gone, AFTERWARDS);
    gone.answered(OURS, false, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// The same for the stash's own listing.
#[test]
fn a_stash_listing_in_hand_settles_the_answer_that_follows_it() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Stash, "stash@{0}")], Some(OURS));
    gone.listing_applied(Row::Stash, AFTERWARDS);
    gone.answered(OURS, false, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn the_stash_waits_for_its_own_listing() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Stash, "stash@{0}")], Some(OURS));
    gone.answered(OURS, false, FENCE);
    gone.listing_applied(Row::Branch, AFTERWARDS);
    gone.listing_applied(Row::Remote, AFTERWARDS);
    gone.listing_applied(Row::Tag, AFTERWARDS);
    gone.look_again();
    assert_eq!(gone.rows().stash, "stash@{0}");
    gone.listing_applied(Row::Stash, AFTERWARDS);
    gone.look_again();
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn a_stash_listing_from_before_the_write_answers_for_nothing() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Stash, "stash@{0}")], Some(OURS));
    gone.answered(OURS, false, FENCE);
    gone.listing_applied(Row::Stash, IN_FLIGHT);
    gone.look_again();
    assert_eq!(gone.rows().stash, "stash@{0}");
}

#[test]
fn the_composite_takes_two_rows_and_one_answer_puts_both_back() {
    let mut gone = StandIn::default();
    gone.asked(
        &[
            (Row::Branch, "feature/x"),
            (Row::Remote, "origin/feature/x"),
        ],
        Some(OURS),
    );
    assert_eq!(gone.rows().branch, "feature/x");
    assert_eq!(gone.rows().remote, "origin/feature/x");
    gone.answered(OURS, true, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// A refusal that overtook its own refs read cannot put back a later
// press's row.
#[test]
fn an_answer_under_a_spent_id_moves_nothing() {
    let mut gone = branch_landed();
    every_list_drew(&mut gone, AFTERWARDS);
    gone.asked(&[(Row::Tag, "v1.0")], Some(SOMEBODY_ELSE));
    gone.answered(OURS, true, FENCE);
    assert_eq!(gone.rows().tag, "v1.0");
}

// The one window where two deletes are on screen at once: the first has
// answered, its listing has not arrived, and the second is pressed.
#[test]
fn a_second_press_takes_the_wait_over_without_putting_the_first_back() {
    let mut gone = branch_landed();
    gone.asked(&[(Row::Tag, "v1.0")], Some(SOMEBODY_ELSE));
    assert_eq!(gone.rows().branch, "feature/x");
    assert_eq!(gone.rows().tag, "v1.0");
    // The refs read now waits on the second write's answer and fence.
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows().branch, "feature/x");
    gone.answered(SOMEBODY_ELSE, false, AFTERWARDS + 1);
    every_list_drew(&mut gone, AFTERWARDS + 2);
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn letting_the_session_go_puts_every_row_back() {
    let mut gone = branch_asked();
    gone.session_gone();
    assert_eq!(gone.rows(), &Rows::default());
    // …and the write it waited on is over with it: an answer from the next
    // session is not this press's.
    gone.answered(OURS, true, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// A long session leaves a high stamp behind, and a fresh one counts its
// reads from the start.
#[test]
fn a_reading_drawn_under_the_session_before_answers_for_nothing() {
    let mut gone = StandIn::default();
    every_list_drew(&mut gone, 100);
    gone.session_gone();

    // The tab is opened again, and a delete pressed in it.
    gone.asked(&[(Row::Branch, "feature/x")], Some(OURS));
    gone.answered(OURS, false, 3);
    every_list_drew(&mut gone, 0);
    assert_eq!(
        gone.rows().branch,
        "feature/x",
        "nothing this session read has reached the sidebar yet"
    );

    // …and the new session's own listing is what puts it down.
    every_list_drew(&mut gone, 4);
    assert_eq!(gone.rows(), &Rows::default());
}

// Thrown away with each delete, the next one in the same session would
// wait for a listing it had already been given.
#[test]
fn a_delete_ending_leaves_what_the_lists_have_drawn_alone() {
    let mut gone = branch_landed();
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows(), &Rows::default());

    // A second delete, answered under a fence the listing on screen
    // already meets.
    gone.asked(&[(Row::Tag, "v1.0")], Some(SOMEBODY_ELSE));
    gone.answered(SOMEBODY_ELSE, false, AFTERWARDS);
    assert_eq!(gone.rows(), &Rows::default());
}

#[test]
fn an_unnamed_row_stands_nothing_in() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Branch, "")], Some(OURS));
    assert_eq!(gone.rows(), &Rows::default());
}

/// The working copies and the working tree's buckets have rows, but
/// nothing takes one away ahead of git.
#[test]
fn only_the_four_sections_that_draw_a_stood_in_row_are_named() {
    assert_eq!(Row::drawn_by("branches"), Some(Row::Branch));
    assert_eq!(Row::drawn_by("remotes"), Some(Row::Remote));
    assert_eq!(Row::drawn_by("tags"), Some(Row::Tag));
    assert_eq!(Row::drawn_by("stashes"), Some(Row::Stash));
    assert_eq!(Row::drawn_by("worktrees"), None);
    assert_eq!(Row::drawn_by("worktree"), None);
}
