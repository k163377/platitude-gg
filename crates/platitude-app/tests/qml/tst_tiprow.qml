import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where the shared tooltip stands for a row among rows (デザイン規約 §hover のツールチップ「行の的は、行の横に立つ」): out of the list
// on the side the row names, else its other side, else on the row beside the hand — on the row's own band every time,
// so the rows above and below stay readable.
Item {
    id: root
    width: 1000
    height: 400
    // What `Main.qml` and the popup bases declare (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    PointerWatch { id: hand }

    SharedToolTip {
        id: shared
        host: root
        hand: hand
    }

    // The right panel's shape: room on the left only.
    Item {
        id: rightPane
        x: 700
        y: 40
        width: 300
        height: Theme.rowHeight
        readonly property string tipRowSide: "left"
        HoverHandler { id: onRightPane }
        ToolTip.text: "docs/install.md"
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: onRightPane.hovered
    }
    // The left panel's: asks for the left, has no room there.
    Item {
        id: leftPane
        y: 120
        width: 260
        height: Theme.rowHeight
        readonly property string tipRowSide: "left"
        HoverHandler { id: onLeftPane }
        ToolTip.text: "feature"
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: onLeftPane.hovered
    }
    // A menu's: room both ways, asks for the right.
    Item {
        id: menuRow
        x: 400
        y: 200
        width: 200
        height: Theme.rowHeight
        readonly property string tipRowSide: "right"
        HoverHandler { id: onMenuRow }
        ToolTip.text: "Differs from origin/main"
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: onMenuRow.hovered
    }
    // As wide as the window, so no outside at all.
    Item {
        id: wideRow
        y: 300
        width: root.width
        height: Theme.rowHeight
        readonly property string tipRowSide: "left"
        HoverHandler { id: onWideRow }
        ToolTip.text: "src/main.rs"
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.visible: onWideRow.hovered
    }

    TestCase {
        name: "TipRow"
        when: windowShown

        function initTestCase() {
            shared.dressToolTip()
        }

        function init() {
            // A corner no row reaches, and the beat the box leaves on.
            mouseMove(root, root.width - 1, root.height - 1)
            // `keeping` too: the box falls first and is put back a turn later for the beat.
            tryVerify(() => !shared.sharedTip.visible && !shared.keeping, undefined,
                      "each rest starts with nothing out")
        }

        /// Rests the hand on `row` at `across` (a share of its width) and answers the box's seat in the row's terms.
        /// The box comes out at the previous words' size and takes its own a frame later, so the seat is read once the
        /// size is the words'.
        function restOn(row, across) {
            const tip = shared.sharedTip
            mouseMove(root, row.x + row.width * across, row.y + row.height / 2)
            tryVerify(() => tip.visible && tip.parent === row, undefined, "the rest puts the row's box out")
            tryVerify(() => tip.width === tip.implicitWidth && tip.width > 0, undefined,
                      "the box takes its words' size")
            return { left: tip.x, right: tip.x + tip.width, top: tip.y, bottom: tip.y + tip.height }
        }

        /// The box's band is the row's, centred: the rows either side get no more than the overhang.
        function verifyOnTheRowsBand(row, seat) {
            const overhang = (shared.sharedTip.height - row.height) / 2
            fuzzyCompare(seat.top, -overhang, 1, "centred on the row's band, not above it or below it")
            fuzzyCompare(seat.bottom, row.height + overhang, 1)
        }

        function test_a_a_right_panels_row_puts_it_to_its_left() {
            const seat = restOn(rightPane, 0.5)
            compare(seat.right, 0, "flush against the row's left edge — out of the list, no gap to cross")
            verifyOnTheRowsBand(rightPane, seat)
        }

        function test_b_a_left_panels_row_has_no_left_and_takes_its_right() {
            const seat = restOn(leftPane, 0.5)
            compare(seat.left, leftPane.width, "flush against the row's right edge")
            verifyOnTheRowsBand(leftPane, seat)
        }

        function test_c_a_menu_row_takes_the_side_it_names() {
            const seat = restOn(menuRow, 0.5)
            compare(seat.left, menuRow.width, "on the right although the left has room too")
        }

        function test_d_a_row_as_wide_as_the_window_stands_it_beside_the_hand() {
            const at = 0.6
            const seat = restOn(wideRow, at)
            const handX = wideRow.width * at
            verify(seat.right < handX, "left of the hand, and not under it: " + seat.right + " for " + handX)
            verify(handX - seat.right <= Theme.spaceXs, "a step off the hand, not across the row from it")
            verifyOnTheRowsBand(wideRow, seat)
        }
    }
}
