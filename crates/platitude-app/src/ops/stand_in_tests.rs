//! What a delete leaves on screen at each of its three moments — the
//! press, git's answer, and the list that draws the reading proving the
//! row gone.
//!
//! Plain Rust: the transitions are the application's own, and
//! this is the whole of what decides them.

use super::*;

/// The id the queue is pretending to have accepted a press under.
/// Any two different numbers would do — what the tests below are
/// about is that the answer is found by this number.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

/// Read stamps either side of the write's end (`Standing::fence`), which
/// is the only thing that tells a listing that saw what the write left
/// from one that was already in flight when it ended.
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

/// Every list has drawn a reading taken at `at` — for the tests
/// that take every list as caught up.
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

// The queue takes nothing once the session is closed, and answers nothing
// it did not take. A row taken away for a write with no id has no answer
// coming to put it back, so it would stay gone until the tab was read
// again — which is the one way the window can lose a row that is really
// there.
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

// The whole of why the wait is kept by id. A fetch running behind the
// press answers in the middle of it, and a delete of a stash entry
// answers under the same word an apply pressed just before it does:
// counted by turn, somebody else's refusal would put this row back under
// the hand that had just taken it away.
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

// git did it, but the list on screen is still the one read before the
// write: the rows themselves are still in it, so the stand-in holds.
#[test]
fn a_landing_leaves_the_row_away_until_the_refs_arrive() {
    let mut gone = branch_landed();
    assert_eq!(gone.rows().branch, "feature/x");
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows(), &Rows::default());
}

// **The reading that answers is the one that looked after the
// write.** A refs pass already in flight when the write ended
// is applied under its own stamp, and taken as proof it would
// put the row straight back under the hand that had just taken
// it away — which counting arrivals cannot prevent, since the
// answer and the listing travel feeds of their own.
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

// The one at the fence is the write's own read: stamped as the fence was,
// it looked after the write ended.
#[test]
fn a_listing_stamped_at_the_fence_is_the_writes_own() {
    let mut gone = branch_landed();
    every_list_drew(&mut gone, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// **A section answers for its own row alone.** The three refs sections
// are handed one snapshot and draw it on three separate turns,
// so the branches section catching up says nothing about what the tags
// section is still showing — folded into one newest-applied, it would
// put a deleted tag back on screen for as long as that section took.
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

// The composite's two rows are two lists, and they catch up in whatever
// order the two drains fall in. Each goes as its own list reaches it,
// free of the other.
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

// …and the other order, which is the one a single newest-applied would
// also have passed: both have to be held for either to mean anything.
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

// A listing already queued when the press landed says nothing about this
// delete either — and until the answer, nothing does.
#[test]
fn a_reading_that_arrives_before_the_answer_answers_for_nothing() {
    let mut gone = branch_asked();
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows().branch, "feature/x");
}

// …but it is kept. The answer and the listing reach this type in
// no fixed order, and whichever comes second completes the pair: a
// listing applied while the answer was still in the other feed is
// the same proof it would have been a moment later.
#[test]
fn a_listing_already_in_hand_settles_the_answer_that_follows_it() {
    let mut gone = branch_asked();
    every_list_drew(&mut gone, AFTERWARDS);
    gone.answered(OURS, false, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// The same, the other way round, for the stash's own listing.
#[test]
fn a_stash_listing_in_hand_settles_the_answer_that_follows_it() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Stash, "stash@{0}")], Some(OURS));
    gone.listing_applied(Row::Stash, AFTERWARDS);
    gone.answered(OURS, false, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// The stash listing is asked for after the graph is rebuilt,
// so a dropped entry read off the refs' arrival would be back
// on screen for the whole of the rebuild.
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

// …and it is measured by its own stamp: the stash pass that was in
// flight when the write ended answers for nothing, as the refs one does.
#[test]
fn a_stash_listing_from_before_the_write_answers_for_nothing() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Stash, "stash@{0}")], Some(OURS));
    gone.answered(OURS, false, FENCE);
    gone.listing_applied(Row::Stash, IN_FLIGHT);
    gone.look_again();
    assert_eq!(gone.rows().stash, "stash@{0}");
}

// `Delete both` is two rows and one write, so a refusal is one thing to
// put back.
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

// The wait is spent with the last row, so an answer under the id that is
// already over moves nothing — a refusal that overtook its own refs read
// cannot take a row away that nothing is asking about any more.
#[test]
fn an_answer_under_a_spent_id_moves_nothing() {
    let mut gone = branch_landed();
    every_list_drew(&mut gone, AFTERWARDS);
    gone.asked(&[(Row::Tag, "v1.0")], Some(SOMEBODY_ELSE));
    gone.answered(OURS, true, FENCE);
    assert_eq!(gone.rows().tag, "v1.0");
}

// The one window where two deletes are on screen at once: the first has
// answered, its listing has not arrived, and the second is pressed. The
// second takes the wait over — one key per list — and a landing
// leaves both rows away.
#[test]
fn a_second_press_takes_the_wait_over_without_putting_the_first_back() {
    let mut gone = branch_landed();
    gone.asked(&[(Row::Tag, "v1.0")], Some(SOMEBODY_ELSE));
    assert_eq!(gone.rows().branch, "feature/x");
    assert_eq!(gone.rows().tag, "v1.0");
    // The first write's answer is over, so the refs read waits on the
    // second one — and on a listing that looked after *it* ended.
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows().branch, "feature/x");
    gone.answered(SOMEBODY_ELSE, false, AFTERWARDS + 1);
    every_list_drew(&mut gone, AFTERWARDS + 2);
    assert_eq!(gone.rows(), &Rows::default());
}

// Nothing is coming: the session that owed the answer
// has been let go, so the rows are put back and the
// wait goes with them.
#[test]
fn letting_the_session_go_puts_every_row_back() {
    let mut gone = branch_asked();
    gone.session_gone();
    assert_eq!(gone.rows(), &Rows::default());
    // …and the write it was waiting on is over with it: the next session
    // numbers its writes from where this one left off, and an answer to
    // one of them is not this press's.
    gone.answered(OURS, true, FENCE);
    assert_eq!(gone.rows(), &Rows::default());
}

// **A tab reopened is a session that counts its reads from the start.**
// The readings the lists had drawn were numbered by the session that is
// over, and one kept across the release sits above everything the new
// session can produce: the next delete would be answered by a listing
// nobody has read, and the row would come back while git was still being
// asked to delete it.
//
// The numbers here are the shape of the bug: a long session
// leaves a high stamp behind, and a fresh one starts at 1.
#[test]
fn a_reading_drawn_under_the_session_before_answers_for_nothing() {
    let mut gone = StandIn::default();
    // A session that ran for a while, and the lists drew plenty.
    every_list_drew(&mut gone, 100);
    gone.session_gone();

    // The tab is opened again: a new session, numbering from the start,
    // and a delete pressed in it.
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

// The other half of that split: a delete ending leaves what
// the lists have drawn alone, because it is theirs. Thrown
// away with each delete, the next one in the same session
// would wait for a listing it had already been given.
#[test]
fn a_delete_ending_leaves_what_the_lists_have_drawn_alone() {
    let mut gone = branch_landed();
    every_list_drew(&mut gone, AFTERWARDS);
    assert_eq!(gone.rows(), &Rows::default());

    // A second delete in the same session, answered under a fence the
    // listing already on screen is above: it is the same listing, and it
    // still speaks for what that write left.
    gone.asked(&[(Row::Tag, "v1.0")], Some(SOMEBODY_ELSE));
    gone.answered(SOMEBODY_ELSE, false, AFTERWARDS);
    assert_eq!(gone.rows(), &Rows::default());
}

// A row with no name stands nothing in — the empty string
// is what a list is told when it is to put everything back,
// so writing one in would read as "put them back".
#[test]
fn an_unnamed_row_stands_nothing_in() {
    let mut gone = StandIn::default();
    gone.asked(&[(Row::Branch, "")], Some(OURS));
    assert_eq!(gone.rows(), &Rows::default());
}

/// Which sections draw a row a delete can stand in for, and which draw
/// none — the working copies and the working tree's buckets have rows,
/// but nothing takes one away ahead of git.
#[test]
fn only_the_four_sections_that_draw_a_stood_in_row_are_named() {
    assert_eq!(Row::drawn_by("branches"), Some(Row::Branch));
    assert_eq!(Row::drawn_by("remotes"), Some(Row::Remote));
    assert_eq!(Row::drawn_by("tags"), Some(Row::Tag));
    assert_eq!(Row::drawn_by("stashes"), Some(Row::Stash));
    assert_eq!(Row::drawn_by("worktrees"), None);
    assert_eq!(Row::drawn_by("worktree"), None);
}
