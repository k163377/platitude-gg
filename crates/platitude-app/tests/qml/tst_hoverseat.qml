import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The one state machine every hover card, list and the tooltip stand on (`HoverSeat`): what the rest, the beat, the
// holds, a hand-over, a blocker, a held bar and a dismissal do to it, told by events alone — no pointer, so what is
// measured is the machine. The parts' own wiring is each part's test.
Item {
    id: root
    width: 200
    height: 200

    /// What the seat asked of its owner, in order: `open:<key>` / `close:<key>:<why>`.
    property var asked: []

    readonly property HoverSeat seat: HoverSeat {
        id: seat
        restMs: 20
        graceMs: 20
        onOpenAsked: target => root.asked.push("open:" + target.key)
        onCloseAsked: (target, why) => root.asked.push("close:" + target.key + ":" + why)
    }
    HandWatch {
        id: hand
    }
    readonly property HoverSeat handed: HoverSeat {
        id: handed
        hand: hand
        restMs: 20
        graceMs: 20
        onOpenAsked: target => root.asked.push("open:" + target.key)
        onCloseAsked: (target, why) => root.asked.push("close:" + target.key + ":" + why)
    }
    /// A seat whose display opens at once: a hold lifts with the hand's move, a dismissal only with the hand off the
    /// target or a press (`expands`).
    readonly property HoverSeat expanding: HoverSeat {
        id: expanding
        hand: hand
        expands: true
        restMs: 20
        graceMs: 20
        onOpenAsked: target => root.asked.push("open:" + target.key)
        onCloseAsked: (target, why) => root.asked.push("close:" + target.key + ":" + why)
    }
    AppCard {
        id: card
        width: 60
        height: 40
    }
    readonly property HoverSeat carded: HoverSeat {
        id: carded
        card: card
        restMs: 20
        graceMs: 20
        onOpenAsked: target => {
            root.asked.push("open:" + target.key)
            card.open()
        }
        onCloseAsked: (target, why) => root.asked.push("close:" + target.key + ":" + why)
    }
    function a() { return { "key": "a" } }
    function b() { return { "key": "b" } }

    TestCase {
        name: "HoverSeat"
        when: windowShown

        function init() {
            seat.takeDown("init", false)
            handed.takeDown("init", false)
            carded.takeDown("init", false)
            expanding.takeDown("init", false)
            seat.latchedKey = ""
            handed.latchedKey = ""
            carded.latchedKey = ""
            expanding.latchedKey = ""
            seat.holds = ({})
            handed.holds = ({})
            carded.holds = ({})
            expanding.holds = ({})
            seat.blockedBy = ""
            expanding.blockedBy = ""
            seat.exempt = null
            seat.frozen = false
            hand.stillHeld = false
            hand.covered = 0
            hand.hidden = false
            Hand.heldBar = null
            tryCompare(card, "visible", false)
            root.asked = []
        }
        /// Something stands over the hand with the hand inside, or has gone: what a leave the surface did not hear may
        /// be under. A card hung off the rows here (`HandWatch.covered`); a menu with the hand in it is the same to the
        /// seat (`HandWatch.underCover`), and `tst_handwatch.qml` weighs a real one.
        function coverUp(up) {
            hand.covered += up ? 1 : -1
        }

        function test_a_rest_then_shown_then_the_beat_closes() {
            seat.point(root.a())
            compare(seat.state, "waiting")
            compare(root.asked, [], "nothing opens before the rest")
            tryCompare(seat, "state", "shown")
            compare(root.asked, ["open:a"])
            seat.unpoint("a")
            compare(seat.state, "leaving", "the hand off starts the beat, not the close")
            tryCompare(seat, "state", "idle")
            compare(root.asked, ["open:a", "close:a:left"])
        }

        function test_a_leave_before_the_rest_opens_nothing() {
            seat.point(root.a())
            seat.unpoint("a")
            compare(seat.state, "idle")
            // waits(paced): the subject is an opening that must **not** come, so the wait is the rest it counted.
            wait(60)
            compare(root.asked, [])
        }

        function test_what_unfolds_opens_at_once() {
            seat.point(root.a(), true)
            compare(seat.state, "shown")
            compare(root.asked, ["open:a"])
        }

        function test_another_targets_leave_changes_nothing() {
            seat.point(root.a(), true)
            seat.unpoint("b")
            compare(seat.state, "shown", "a stray leave is not a leave of the one out")
            verify(seat.pointed)
        }

        function test_the_hand_back_on_the_one_out_is_held_at_once() {
            seat.point(root.a(), true)
            seat.unpoint("a")
            compare(seat.state, "leaving")
            seat.point(root.a())
            compare(seat.state, "shown", "no rest for what is already out")
            compare(root.asked, ["open:a"], "and not opened twice")
        }

        function test_a_hold_keeps_what_is_out_and_letting_go_starts_the_beat() {
            seat.point(root.a(), true)
            seat.hold("inside", true)
            seat.unpoint("a")
            compare(seat.state, "shown", "held, the hand off is no leave")
            // waits(paced): the subject is a close that must **not** come while held.
            wait(60)
            compare(root.asked, ["open:a"])
            seat.hold("inside", false)
            compare(seat.state, "leaving")
            tryCompare(seat, "state", "idle")
            compare(root.asked, ["open:a", "close:a:left"])
        }

        function test_a_hold_opens_nothing() {
            seat.hold("menu", true)
            compare(seat.state, "idle")
            compare(root.asked, [], "holding is not pointing")
            seat.hold("menu", false)
        }

        function test_another_target_takes_the_one_out_down_and_waits_its_own_rest() {
            seat.point(root.a(), true)
            seat.point(root.b())
            compare(root.asked, ["open:a", "close:a:handover"], "down at once, not after the beat")
            compare(seat.state, "waiting")
            compare(seat.targetKey, "b")
            tryCompare(seat, "state", "shown")
            compare(root.asked, ["open:a", "close:a:handover", "open:b"])
        }

        function test_a_blocker_closes_what_is_out_and_keeps_the_ask() {
            seat.point(root.a(), true)
            seat.blockedBy = "menu"
            compare(root.asked, ["open:a", "close:a:blocked"])
            seat.point(root.b())
            compare(seat.state, "idle", "nothing opens behind the blocker")
            seat.blockedBy = ""
            compare(seat.state, "waiting", "the last ask is made again once it lifts")
            compare(seat.targetKey, "b")
            tryCompare(seat, "state", "shown")
        }

        function test_a_frozen_seat_keeps_its_look_and_answers_when_it_thaws() {
            seat.point(root.a(), true)
            seat.frozen = true
            seat.unpoint("a")
            // waits(paced): the subject is a close that must **not** come while frozen.
            wait(60)
            compare(seat.state, "shown", "nothing closes while frozen")
            seat.point(root.b())
            compare(seat.state, "shown", "and nothing opens")
            seat.frozen = false
            compare(root.asked, ["open:a", "close:a:handover"], "thawed, the ask kept is answered")
            compare(seat.targetKey, "b")
        }

        function test_a_dismissal_is_not_put_back_under_the_same_still_hand() {
            seat.point(root.a(), true)
            seat.takeDown("escape", true)
            compare(root.asked, ["open:a", "close:a:escape"])
            seat.point(root.a())
            compare(seat.state, "idle", "the same target asked again under the same hand stays down")
            seat.unpoint("a")
            seat.point(root.a())
            compare(seat.state, "waiting", "the hand gone and back is a fresh ask")
        }

        /// A seat that rests: any move of the hand is the hand's own, and the rest judges (デザイン規約
        /// §左メニューの所作「手が動いたかの見切り」).
        function test_a_dismissal_lifts_when_the_hand_moves() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            handed.takeDown("escape", true)
            handed.point(root.a())
            compare(handed.state, "idle")
            hand.stir(Qt.point(50, 50))
            compare(handed.state, "idle", "heard again where it was is no move")
            hand.stir(Qt.point(52, 52))
            compare(handed.state, "waiting", "a tremor is a move: the ask kept takes its rest")
        }

        /// A display that opens at once (`expands`) has no rest to judge the hand, so no move of it brings a dismissed
        /// one back — a tremor after the click that closed it would: the hand leaving the target and coming back does,
        /// as a Qt Quick tooltip a press hid returns only when its item's hover changes.
        function test_a_dismissal_of_what_expands_lifts_only_when_the_hand_leaves() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.takeDown("escape", true)
            expanding.point(root.a(), true)
            compare(expanding.state, "idle")
            hand.stir(Qt.point(52, 52))
            compare(expanding.state, "idle", "a tremor does not bring it back")
            hand.stir(Qt.point(50, 90))
            compare(expanding.state, "idle", "nor does any move on the target")
            expanding.unpoint("a")
            expanding.point(root.a(), true)
            compare(expanding.state, "shown", "the hand gone from it and back opens it at once")
        }

        /// A leave the surface did not hear, with nothing over the hand, is the hand gone from the surface — into the
        /// pane beside it, out of the window: back on the target is a fresh ask.
        function test_what_expands_left_with_the_hand_gone_from_the_surface_comes_back_with_it() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.takeDown("escape", true)
            expanding.point(root.a(), true)
            hand.stir(Qt.point(-1, -1))
            expanding.unpoint("a")
            compare(expanding.latchedKey, "", "the hand left the target")
            // Back on it: the target asks before the surface tells the place (`handWatch`).
            expanding.point(root.a(), true)
            hand.stir(Qt.point(50, 50))
            compare(expanding.state, "shown", "and opens at once")
        }

        /// Under a menu or a card with the hand in it, the leave the target reports is the cover over the hand: where
        /// the hand is heard next decides — back on the target, which asks for it before the surface tells the place,
        /// the latch holds and no move on it lifts it; anywhere else, the hand had left.
        function test_what_expands_left_under_a_menu_is_judged_where_the_hand_is_heard_next() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.takeDown("escape", true)
            expanding.point(root.a(), true)
            coverUp(true)
            hand.stir(Qt.point(-1, -1))
            verify(hand.hidden, "the hand is hidden under the cover")
            expanding.unpoint("a")
            compare(expanding.latchedKey, "a", "a leave under the cover lifts nothing")
            coverUp(false)
            expanding.point(root.a(), true)
            hand.stir(Qt.point(50, 50))
            compare(expanding.state, "idle", "heard back on the target as the cover goes: the latch holds")
            hand.stir(Qt.point(52, 52))
            compare(expanding.state, "idle", "and a move on it lifts nothing")

            coverUp(true)
            hand.stir(Qt.point(-1, -1))
            expanding.unpoint("a")
            coverUp(false)
            hand.stir(Qt.point(150, 150))
            compare(expanding.latchedKey, "", "heard where the target does not ask: the hand had left it")
            expanding.point(root.a(), true)
            compare(expanding.state, "shown", "back on it is a fresh ask")
        }

        /// …and out from under the cover to where the surface does not reach — out of the window, onto another pane —
        /// the popup losing it while it stands (`HandWatch.weighCover`): the hand had left, and back on the target is
        /// a fresh ask.
        function test_what_expands_left_under_a_cover_comes_back_once_the_hand_walked_out_of_it() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.takeDown("escape", true)
            expanding.point(root.a(), true)
            coverUp(true)
            hand.stir(Qt.point(-1, -1))
            expanding.unpoint("a")
            compare(expanding.latchedKey, "a", "a leave under the cover lifts nothing")
            hand.weighCover()
            compare(expanding.latchedKey, "a", "nor does the hand still in the cover")
            coverUp(false)
            hand.weighCover()
            verify(!hand.hidden, "out of the cover with nothing hearing it: the hand left")
            compare(expanding.latchedKey, "", "and the latch with it")
            expanding.point(root.a(), true)
            hand.stir(Qt.point(50, 50))
            compare(expanding.state, "shown", "back on it is a fresh ask")
        }

        /// Put down by a press elsewhere, during the beat the hand's leave started: the hand is not on the target, and
        /// the first place heard says so — back on the target is a fresh ask.
        function test_what_expands_put_down_from_elsewhere_comes_back_with_the_hand() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.unpoint("a")
            compare(expanding.state, "leaving")
            expanding.noteClosed()
            compare(expanding.latchedKey, "a", "a dismissal")
            hand.stir(Qt.point(150, 150))
            compare(expanding.latchedKey, "", "the hand heard where the target does not ask")
            expanding.point(root.a(), true)
            compare(expanding.state, "shown")
        }

        /// Put down with the hand in it, off the target — Escape in the rail's peek — and the surface then losing the
        /// hand with nothing over it: the hand left, though the first place it is heard again is the target, which
        /// asks before the surface tells the place.
        function test_what_expands_put_down_and_the_hand_gone_from_the_surface_comes_back_on_the_target() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            hand.stir(Qt.point(80, 80))
            expanding.unpoint("a")
            expanding.hold("inside", true)
            expanding.noteClosed()
            expanding.hold("inside", false)
            compare(expanding.latchedKey, "a", "a dismissal")
            hand.stir(Qt.point(-1, -1))
            compare(expanding.latchedKey, "", "the hand gone from the surface has left the target")
            expanding.point(root.a(), true)
            hand.stir(Qt.point(50, 50))
            compare(expanding.state, "shown", "back on it, it opens at once")
        }

        /// The owner's own dismissal (a click on the rail's cell) keeps the ask of the hand still on the target, as
        /// the card's close does: no move on the target lifts it, though the target asks only as the hand comes on.
        function test_what_expands_the_owner_put_down_under_the_hand_stays_down_for_its_moves() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.takeDown("click", true)
            hand.stir(Qt.point(52, 52))
            compare(expanding.latchedKey, "a", "a move on the target lifts nothing")
            expanding.unpoint("a")
            expanding.point(root.a(), true)
            compare(expanding.state, "shown", "the hand gone from it and back opens it")
        }

        /// A press on it is a new ask, whatever the latch says (`press`).
        function test_a_press_brings_back_what_expands() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.takeDown("click", true)
            expanding.press(root.a())
            compare(expanding.state, "shown")
        }

        /// A blocker rising on the one out — the press that raised a menu on its row — takes it down as a dismissal:
        /// the ask is kept, but answered only once the hand has walked.
        function test_a_blocker_rising_on_the_one_out_latches_it() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            handed.blockedBy = "menu"
            compare(root.asked, ["open:a", "close:a:blocked"])
            handed.blockedBy = ""
            compare(handed.state, "idle", "lifted, the ask kept waits for the walk")
            hand.stir(Qt.point(50, 70))
            compare(handed.state, "waiting", "the walk answers it")
        }

        /// The hand going unheard — under the menu the press raised — and coming back where it was is no walk.
        function test_a_hand_gone_unheard_does_not_lift_the_latch() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            handed.takeDown("press", true)
            hand.stir(Qt.point(-1, -1))
            hand.stir(Qt.point(50, 50))
            handed.point(root.a())
            compare(handed.state, "idle", "back where it was, the latch holds")
            hand.stir(Qt.point(50, 70))
            compare(handed.state, "waiting", "a walk lifts it")
        }

        /// The rows report a leave as the hand goes unheard under the menu the press raised: that hand is still
        /// there, so the leave does not lift the latch — only its walk does.
        function test_a_leave_unheard_under_a_menu_does_not_lift_the_latch() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            handed.takeDown("press", true)
            coverUp(true)
            hand.stir(Qt.point(-1, -1))
            handed.unpoint("a")
            coverUp(false)
            hand.stir(Qt.point(50, 50))
            handed.point(root.a())
            compare(handed.state, "idle", "the leave the menu caused lifted nothing")
            hand.stir(Qt.point(50, 70))
            compare(handed.state, "waiting", "a walk does")
        }

        /// …whichever order Qt tells the two leaves in: it clears the handlers in the order they took the hover, so a
        /// target that took it after the surface hears its leave while the surface still holds the hand.
        function test_a_leave_told_before_the_surface_hears_under_a_menu_lifts_nothing() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            handed.takeDown("press", true)
            coverUp(true)
            handed.unpoint("a")
            compare(handed.latchedKey, "a", "a leave under the menu lifts nothing, heard or not")
            hand.stir(Qt.point(-1, -1))
            coverUp(false)
            hand.stir(Qt.point(50, 50))
            handed.point(root.a())
            compare(handed.state, "idle", "back where it was as the menu goes: the latch holds")
        }

        /// A dismissal made with the hand unheard (in the card it dismissed): the latch is anchored at the first
        /// place heard, and the walk measured from there.
        function test_a_dismissal_under_an_unheard_hand_holds_from_the_first_place_heard() {
            hand.stir(Qt.point(-1, -1))
            handed.point(root.a(), true)
            handed.takeDown("escape", true)
            hand.stir(Qt.point(50, 50))
            handed.point(root.a())
            compare(handed.state, "idle", "the first place heard is where the dismissal was")
            hand.stir(Qt.point(50, 50))
            compare(handed.state, "idle", "heard there again is no move")
            hand.stir(Qt.point(52, 52))
            compare(handed.state, "waiting", "a move from there: the ask kept takes its rest")
        }

        function test_a_target_that_arrived_under_a_still_hand_waits_for_the_hand() {
            hand.stir(Qt.point(50, 50))
            hand.rowsMoved()
            verify(hand.stillHeld)
            handed.point(root.a())
            compare(handed.state, "idle", "nothing opens under a hand that did not move")
            hand.stir(Qt.point(50, 55))
            verify(!hand.stillHeld, "the hand's own move lifts the hold")
            compare(handed.state, "waiting", "and the row it is on is asked for, with its rest")
        }

        /// What expands waits for the hand's move too, and then opens at once (the ask kept was immediate).
        function test_what_expands_under_a_still_hand_waits_for_the_hands_move() {
            hand.stir(Qt.point(50, 50))
            hand.rowsMoved()
            expanding.point(root.a(), true)
            compare(expanding.state, "idle", "nothing opens under a hand that did not move")
            hand.stir(Qt.point(50, 52))
            verify(!hand.stillHeld, "the hand's own move lifts the hold")
            compare(expanding.state, "shown", "and the one it is on opens at once")
        }

        function test_the_one_out_stays_held_while_the_rows_move() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            hand.rowsMoved()
            handed.point(root.a())
            compare(handed.state, "shown", "the hand on the one out holds it, moved rows or not")
        }

        function test_a_target_gone_by_the_rest_opens_nothing() {
            let alive = true
            seat.point({ "key": "a", "alive": () => alive })
            alive = false
            // The rest runs out on its own clock and asks the target then: gone, it opens nothing.
            tryVerify(() => seat.state !== "waiting", undefined, "the rest ran out")
            compare(seat.state, "idle")
            compare(root.asked, [])
        }

        function test_a_late_rest_of_a_target_let_go_opens_nothing() {
            seat.point(root.a())
            seat.unpoint("a")
            seat.point(root.b(), true)
            // waits(paced): the subject is an opening of `a` that must **not** come; the wait is its rest.
            wait(60)
            compare(root.asked, ["open:b"])
        }

        function test_the_card_closing_by_itself_is_a_dismissal() {
            carded.point(root.a(), true)
            tryCompare(card, "opened", true)
            card.close()
            tryCompare(carded, "state", "idle")
            carded.point(root.a())
            compare(carded.state, "idle", "not put back under the same still hand")
        }

        function test_the_seat_closing_the_card_is_no_dismissal() {
            carded.point(root.a(), true)
            tryCompare(card, "opened", true)
            carded.takeDown("list", false)
            tryCompare(card, "visible", false)
            carded.point(root.a())
            compare(carded.state, "waiting", "asked for again, it comes back")
        }

        function test_a_bar_held_outside_takes_the_card_down_and_drops_the_ask() {
            carded.point(root.a(), true)
            tryCompare(card, "opened", true)
            Hand.heldBar = root
            tryCompare(card, "visible", false)
            carded.point(root.b())
            compare(carded.state, "idle", "nothing is asked for while the bar is held")
            Hand.heldBar = null
            compare(carded.state, "idle", "nor afterwards: the rest went with the box")
        }

        function test_the_hand_inside_the_card_holds_it() {
            carded.point(root.a(), true)
            tryCompare(card, "opened", true)
            card.contentPointed = true
            carded.unpoint("a")
            compare(carded.state, "shown")
            card.contentPointed = false
            compare(carded.state, "leaving")
            tryCompare(card, "visible", false)
        }

        /// Frozen is frozen in every state: a rest that runs out meanwhile opens nothing, and opens at the thaw if the
        /// hand is still there.
        function test_a_rest_that_runs_out_while_frozen_opens_at_the_thaw() {
            seat.point(root.a())
            compare(seat.state, "waiting")
            seat.frozen = true
            tryVerify(() => seat.due, undefined, "the rest ran out under the freeze")
            compare(root.asked, [], "nothing opens while frozen")
            seat.frozen = false
            compare(root.asked, ["open:a"], "thawed, the rest that ran out opens it")
            compare(seat.state, "shown")
        }

        function test_a_beat_that_runs_out_while_frozen_closes_at_the_thaw() {
            seat.point(root.a(), true)
            seat.unpoint("a")
            compare(seat.state, "leaving")
            seat.frozen = true
            tryVerify(() => seat.due, undefined, "the beat ran out under the freeze")
            compare(root.asked, ["open:a"], "nothing closes while frozen")
            seat.frozen = false
            compare(root.asked, ["open:a", "close:a:left"], "thawed, the beat that ran out closes it")
            compare(seat.state, "idle")
        }

        function test_a_hand_gone_while_frozen_leaves_nothing_waiting_at_the_thaw() {
            seat.point(root.a())
            seat.frozen = true
            seat.unpoint("a")
            // waits(paced): the rest runs out under the freeze; the subject is what the thaw makes of it.
            wait(60)
            seat.frozen = false
            compare(seat.state, "idle", "a target the hand left while frozen is not waited for")
            compare(root.asked, [])
        }

        function test_the_owner_taking_down_wins_over_the_freeze() {
            seat.point(root.a(), true)
            seat.frozen = true
            seat.takeDown("gone", false)
            compare(root.asked, ["open:a", "close:a:gone"], "a target gone is gone, frozen or not")
            compare(seat.state, "idle")
        }

        /// A held target is not handed over: the hold is about that target (a menu raised on it), and the hand on
        /// another meanwhile is an ask kept for when the hold lifts — answered the moment it lifts, as it would have
        /// been with no hold: a goes then, not after its beat, and b takes its rest.
        function test_a_held_target_is_not_handed_over() {
            seat.point(root.a(), true)
            seat.hold("menu", true)
            seat.unpoint("a")
            seat.point(root.b())
            compare(seat.state, "shown", "held, a stays")
            compare(seat.targetKey, "a")
            compare(root.asked, ["open:a"], "and b waits")
            seat.hold("menu", false)
            compare(root.asked, ["open:a", "close:a:handover"], "the hold lifted, a goes at once")
            compare(seat.state, "waiting", "and b is asked for, with its rest")
            compare(seat.targetKey, "b")
            tryCompare(seat, "state", "shown")
            compare(root.asked, ["open:a", "close:a:handover", "open:b"])
        }

        /// The order Qt tells the card's leave and the next target's enter in decides nothing: the hold lifting first
        /// and the hand on b after it end where b asked first and the hold lifted after it.
        function test_a_hand_over_through_a_hold_is_the_same_either_way() {
            seat.point(root.a(), true)
            seat.hold("inside", true)
            seat.unpoint("a")
            seat.hold("inside", false)
            seat.point(root.b())
            const leaveFirst = root.asked.slice()
            const leaveFirstState = seat.state
            seat.takeDown("init", false)
            root.asked = []

            seat.point(root.a(), true)
            seat.hold("inside", true)
            seat.unpoint("a")
            seat.point(root.b())
            seat.hold("inside", false)
            compare(root.asked, leaveFirst, "the same hand-over")
            compare(seat.state, leaveFirstState)
            compare(seat.targetKey, "b")
        }

        function test_an_ask_kept_behind_a_hold_goes_with_the_hand() {
            seat.point(root.a(), true)
            seat.hold("menu", true)
            seat.unpoint("a")
            seat.point(root.b())
            seat.unpoint("b")
            seat.hold("menu", false)
            tryCompare(seat, "state", "idle")
            compare(root.asked, ["open:a", "close:a:left"], "the hand off b before the hold lifted: nothing opens")
        }

        /// The rows moved under the hand with the one out still asked for (the hand stayed on its row): what is out
        /// goes at once, and the ask is answered again — with its rest — once the hand has moved.
        function test_a_displaced_ask_is_asked_again_once_the_hand_has_moved() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            hand.rowsMoved()
            handed.displace("slid")
            compare(root.asked, ["open:a", "close:a:slid"], "what was out went with the rows")
            compare(handed.state, "idle")
            hand.stir(Qt.point(52, 52))
            compare(handed.state, "waiting", "a tremor is a move: the ask kept takes its rest again")
            tryCompare(handed, "state", "shown")
            compare(root.asked, ["open:a", "close:a:slid", "open:a"])
        }

        /// What expands, displaced, waits for the hand's move before it is asked again — and then opens at once, as
        /// it always does: it never rests.
        function test_a_displaced_ask_of_what_expands_waits_for_the_hands_move() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            hand.rowsMoved()
            expanding.displace("slid")
            compare(expanding.state, "idle")
            hand.stir(Qt.point(52, 52))
            compare(expanding.state, "shown", "the hand's move: the ask kept opens at once")
            compare(root.asked, ["open:a", "close:a:slid", "open:a"])
        }

        /// Displaced under no hold — a hand the rows cannot hear — the ask kept rests again at once.
        function test_a_displaced_ask_under_no_hold_rests_again_at_once() {
            seat.point(root.a(), true)
            seat.displace("slid")
            compare(root.asked, ["open:a", "close:a:slid"])
            compare(seat.state, "waiting", "asked again from the rest")
            tryCompare(seat, "state", "shown")
            compare(root.asked, ["open:a", "close:a:slid", "open:a"])
        }

        /// The ask let go while kept — the hand walked off the row — opens nothing.
        function test_a_displaced_ask_let_go_meanwhile_opens_nothing() {
            hand.stir(Qt.point(50, 50))
            handed.point(root.a(), true)
            hand.rowsMoved()
            handed.displace("slid")
            handed.unpoint("a")
            hand.stir(Qt.point(50, 70))
            compare(handed.state, "idle")
            // waits(paced): the subject is an opening that must **not** come; the wait is the rest.
            wait(60)
            compare(root.asked, ["open:a", "close:a:slid"])
        }

        /// Taken down as gone — the target invalid — the ask is over: the asker asks again with a new target.
        function test_a_target_taken_down_as_gone_is_not_asked_again() {
            seat.point(root.a(), true)
            seat.takeDown("gone", false)
            compare(seat.state, "idle")
            // waits(paced): the subject is an opening that must **not** come; the wait is the rest.
            wait(60)
            compare(root.asked, ["open:a", "close:a:gone"])
        }

        /// The owner retires what a part asked for, picked by what asked (`forget`): the one out goes whatever holds
        /// it — a menu on it, the hand inside — with no latch, and nothing else is touched.
        function test_forgetting_a_target_takes_it_down_through_its_holds() {
            seat.point({ "key": "a", "owner": "row1" }, true)
            seat.hold("menu", true)
            seat.forget(target => target.owner === "row1", "gone")
            compare(root.asked, ["open:a", "close:a:gone"], "held or not, it goes")
            compare(seat.state, "idle")
            compare(seat.latchedKey, "", "no dismissal: the asker asks again freely")
            seat.hold("menu", false)
        }

        /// …and an ask kept for it goes too, while an ask kept for another stays kept — answered once its holder
        /// went, if that was what held it.
        function test_forgetting_drops_its_own_kept_ask_and_answers_anothers() {
            seat.point({ "key": "a", "owner": "row1" }, true)
            seat.hold("menu", true)
            seat.point({ "key": "b", "owner": "row2" })
            compare(seat.pending.target.key, "b", "b is kept behind a's hold")
            seat.forget(target => target.owner === "row1", "gone")
            compare(seat.state, "waiting", "a went, so b is asked for at once")
            compare(seat.targetKey, "b")
            seat.hold("menu", false)

            seat.takeDown("init", false)
            root.asked = []
            seat.blockedBy = "menu"
            seat.point({ "key": "c", "owner": "row3" })
            seat.forget(target => target.owner === "row3", "gone")
            seat.blockedBy = ""
            compare(seat.state, "idle", "the ask kept for c went with it")
            compare(root.asked, [])
        }

        /// A target the blocker does not hold back (`exempt`) comes out under it, and the blocker rising leaves it
        /// out; every other target is held back and kept.
        function test_an_exempt_target_comes_out_under_the_blocker() {
            seat.exempt = target => target.inside === true
            seat.blockedBy = "menu"
            seat.point({ "key": "a" }, true)
            compare(seat.state, "idle", "a is behind the blocker")
            seat.point({ "key": "m", "inside": true }, true)
            compare(seat.state, "shown", "m is not")
            compare(root.asked, ["open:m"])
            seat.blockedBy = ""
            seat.blockedBy = "menu"
            compare(seat.state, "shown", "the blocker rising leaves m out")
            compare(seat.targetKey, "m")
            seat.exempt = null
            seat.blockedBy = ""
        }

        /// What expands, taken down by a blocker with the hand still on it (the menu a press raised), stays down
        /// however the hand moves on it, and opens at once — no rest — when the hand has left it and come back.
        function test_what_expands_taken_down_by_a_blocker_comes_back_with_the_hand() {
            hand.stir(Qt.point(50, 50))
            expanding.point(root.a(), true)
            expanding.blockedBy = "menu"
            compare(root.asked, ["open:a", "close:a:blocked"])
            expanding.blockedBy = ""
            compare(expanding.state, "idle", "latched: the same still hand does not bring it back")
            hand.stir(Qt.point(50, 70))
            compare(expanding.state, "idle", "nor does a move on it")
            expanding.unpoint("a")
            expanding.point(root.a(), true)
            compare(expanding.state, "shown", "the hand gone from it and back opens it with no rest")
            compare(root.asked, ["open:a", "close:a:blocked", "open:a"])
        }
    }
}
