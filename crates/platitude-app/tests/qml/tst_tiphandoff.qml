import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What the one shared tooltip does when the hand walks from one target to the next, and what it does when the hand
// comes back to the one it is already out on (デザイン規約 §hover のツールチップ).
//
// **Only a real pointer can answer this.** Hover cannot be injected into the app (verify-ui スキル), and the thing
// under test is Qt's own hand-over: the attached property moves the instance to the next target inside the same
// delivery that moved the pointer — parent, seat and words all become that target's while `visible` never falls, so
// nothing the app reads back afterwards can tell a box that was asked for from one that was carried over.
Item {
    id: root
    width: 400
    height: 300

    PointerWatch { id: hand }

    SharedToolTip {
        id: shared
        host: root
        hand: hand
    }

    // Apart rather than touching, so the box one row puts out never covers the other: what is being asked here is
    // where the pointer went, and a box in the way would answer for it.
    Item {
        id: rowA
        y: 120
        width: 300
        height: 40
        HoverHandler { id: onA }
        ToolTip.text: "row A"
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: onA.hovered
    }
    Item {
        id: rowB
        y: 240
        width: 300
        height: 40
        HoverHandler { id: onB }
        ToolTip.text: "row B"
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: onB.hovered
    }

    TestCase {
        name: "TipHandoff"
        when: windowShown

        function initTestCase() {
            shared.dressToolTip()
        }

        function init() {
            // A corner neither row reaches, and the beat the box leaves on.
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !shared.sharedTip.visible, undefined, "each walk starts with nothing out")
        }

        /// The middle of a row, in the root's own coordinates.
        function middleOf(row) {
            return row.y + row.height / 2
        }

        // The rest before a box is the question "was that a hand going past, or one that meant it?", and every target
        // is asked it — the hand-over to a neighbour is the one case Qt would answer for free
        // (規約 §hover のツールチップ「隣の的への即時の移し替えは不採用」).
        function test_a_the_next_row_is_asked_the_question_too() {
            const tip = shared.sharedTip
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => tip.visible, undefined, "the row the hand rested on puts a box out")
            compare(tip.parent, rowA, "on that row")
            mouseMove(root, 150, middleOf(rowB))
            tryVerify(() => !tip.visible, undefined, "the box does not travel to a row that has only been reached")
            compare(tip.delay, Metrics.tipDelayMs,
                    "and what it goes back through is that row's own rest, not the holding open above")
            tryVerify(() => tip.visible, undefined, "which puts it out once the rest is over")
            compare(tip.parent, rowB, "on the row the hand is on now")
        }

        // And the hand that never left is not asked twice (規約「出ているものの的へ戻る手は待たせない」). The target's
        // binding falls the instant the pointer steps off it, which is also the instant a hand reaching for the box
        // has left the row, so the fall is not taken at its word.
        function test_b_the_row_the_box_is_out_on_is_not() {
            const tip = shared.sharedTip
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => tip.visible)
            mouseMove(root, root.width - 1, root.height - 1)
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => tip.visible, undefined, "the box is back before the beat that would have taken it down")
            verify(shared.keeping, "held open rather than asked for again")
            compare(tip.parent, rowA)
        }

        // The walk the beat exists for: into the box itself, which takes the pointer off the row that raised it.
        function test_c_a_hand_that_steps_into_the_box_keeps_it() {
            const tip = shared.sharedTip
            mouseMove(root, 150, middleOf(rowA))
            tryVerify(() => tip.visible)
            mouseMove(root, 150, rowA.y + tip.y + tip.height / 2)
            // **Both, settled.** The row's own binding falls the moment the box takes the pointer, so there is a turn
            // in between where the hand is inside a box that is down; what is being asked is where that lands.
            tryVerify(() => shared.pointed && tip.visible, undefined,
                      "the hand is inside the box, and the box is the one that was already out")
            compare(tip.parent, rowA, "still the row's")
        }
    }
}
