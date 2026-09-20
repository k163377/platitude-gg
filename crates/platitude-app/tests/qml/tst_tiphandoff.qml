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

    // Set apart, so the box one row puts out never covers the other: what is being asked here is where the
    // pointer went, and a box in the way would answer for it.
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
    // The left panel's own shape: a box that stands **beside** its target instead of over it, and is asked for from
    // the line above that target as well as from the target itself (`NavRowFacts` — the open row's name is one road
    // to it and the lines it opened are the other, and those lines are what the box is seated against).
    Item {
        id: nameLine
        y: 20
        width: 300
        height: 24
        HoverHandler { id: onName }
    }
    Item {
        id: aside
        y: 44
        width: 300
        height: 42
        readonly property bool tipBeside: true
        HoverHandler { id: onAside }
        ToolTip.text: "beside"
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: onName.hovered || onAside.hovered
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
        // (規約 §hover のツールチップ「隣の的へは箱を下ろしてから移る」).
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

        // And the hand that never left is not asked twice (規約「出ているものの的へ戻る手は即通す」). The target's
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

        // A box that stands **beside** its target opens level with the hand, not with the target. What asks for one
        // of these is not always the item it is seated against — the panel's open row asks from its own name, a
        // line above the lines the box sits beside — and a box up on the target's top is a diagonal walk away: the
        // reader goes straight sideways, leaves both, and it falls (デザイン規約 §hover のツールチップ「手の傍に」).
        function test_d_a_box_beside_its_target_opens_level_with_the_hand() {
            const tip = shared.sharedTip
            const at = middleOf(nameLine)
            mouseMove(root, 150, at)
            tryVerify(() => tip.visible, undefined, "the rest on the line above opens the box")
            compare(tip.parent, aside, "seated against the target under that line")
            const top = aside.y + tip.y
            verify(top <= at && at <= top + tip.height,
                   "the hand's own line is inside the box: " + top + ".." + (top + tip.height) + " for " + at)

            // Which is the whole of what the seat is for: the walk into it is one sideways move.
            mouseMove(root, aside.x + tip.x + tip.width / 2, at)
            tryVerify(() => shared.pointed && tip.visible, undefined,
                      "a hand that only went sideways is inside the box, and the box is still out")
        }
    }
}
