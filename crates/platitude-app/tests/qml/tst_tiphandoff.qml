import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What the one shared tooltip does as the hand walks between targets (デザイン規約 §hover のツールチップ).
//
// Only a real pointer can answer this: hover cannot be injected into the app (verify-ui スキル), and Qt's hand-over
// moves the instance to the next target without `visible` ever falling, so nothing the app reads back afterwards
// tells a box asked for from one carried over.
Item {
    id: root
    width: 400
    height: 300
    // What `Main.qml` and the popup bases declare (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    PointerWatch { id: hand }

    SharedToolTip {
        id: shared
        host: root
        hand: hand
    }

    // Set apart, so the box one row puts out never covers the other.
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
    // The left panel's shape (`NavRowFacts`): a box that stands beside its target and is asked for from the line
    // above it as well as from the target itself.
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

        function middleOf(row) {
            return row.y + row.height / 2
        }

        // Qt would hand the box to a neighbour for free; every target waits its own rest instead
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

        // 規約「出ているものの的へ戻る手は即通す」: the target's binding falls whenever the pointer steps off it, a hand
        // reaching for the box included, so the fall is not taken at its word.
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
            // Both, settled: the row's binding falls as the box takes the pointer, so for a turn the hand is inside
            // a box that is down.
            tryVerify(() => shared.pointed && tip.visible, undefined,
                      "the hand is inside the box, and the box is the one that was already out")
            compare(tip.parent, rowA, "still the row's")
        }

        // Asked for from the line above, a box up on the target's top is a diagonal walk away: a hand going straight
        // sideways leaves both and it falls (デザイン規約 §hover のツールチップ「手の傍に」).
        function test_d_a_box_beside_its_target_opens_level_with_the_hand() {
            const tip = shared.sharedTip
            const at = middleOf(nameLine)
            mouseMove(root, 150, at)
            tryVerify(() => tip.visible, undefined, "the rest on the line above opens the box")
            compare(tip.parent, aside, "seated against the target under that line")
            const top = aside.y + tip.y
            verify(top <= at && at <= top + tip.height,
                   "the hand's own line is inside the box: " + top + ".." + (top + tip.height) + " for " + at)

            mouseMove(root, aside.x + tip.x + tip.width / 2, at)
            tryVerify(() => shared.pointed && tip.visible, undefined,
                      "a hand that only went sideways is inside the box, and the box is still out")
        }
    }
}
