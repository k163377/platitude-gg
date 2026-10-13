import QtQuick
import QtTest

// The graph row's press area while a name box stands on its chip column (`GraphRowDelegate.rowMouse`): the box can
// end where the chip ended or reach past the column, so the area cannot simply start at the box's right edge — the
// room left of a box standing against the chip's end would take neither a press nor the hand. A `containmentMask`
// leaving out the box's span hands the presses and the hover on either side to the row, and the ones on the box to the
// box drawn under the area. This is Qt's side of it, on an area of its own; the product's area is asked by
// `PGG_AUTO_ACT=graph-rename-fit` (`hits=`).
Item {
    id: root
    width: 300
    height: 40

    property int boxPresses: 0
    property int rowPresses: 0

    // Under the area, as the name box is under the delegate's.
    MouseArea {
        id: box
        x: 100
        width: 60
        height: root.height
        onPressed: root.boxPresses++
    }
    MouseArea {
        id: row
        anchors.fill: parent
        hoverEnabled: true
        containmentMask: QtObject {
            function contains(point: point): bool {
                if (point.x < 0 || point.y < 0 || point.x > row.width || point.y > row.height)
                    return false
                return point.x < box.x || point.x >= box.x + box.width
            }
        }
        onPressed: root.rowPresses++
    }

    TestCase {
        name: "MaskedRowBox"
        when: windowShown

        function init() {
            root.boxPresses = 0
            root.rowPresses = 0
        }

        function test_either_side_of_the_box_is_the_row() {
            mouseClick(root, 50, 20)
            mouseClick(root, 250, 20)
            compare(root.rowPresses, 2, "the room left of the box and the lanes right of it are the row's")
            compare(root.boxPresses, 0, "and neither reaches the box")
        }

        function test_the_box_takes_its_own_presses() {
            mouseClick(root, 130, 20)
            compare(root.boxPresses, 1, "the press on the box goes through the area to the box")
            compare(root.rowPresses, 0, "and the row does not take it too")
        }

        // From the lanes in, so the row is first handed the hand by a hover: after the clicks of the cases before,
        // its `containsMouse` can still stand from them.
        function test_the_hand_leaves_the_row_over_the_box_and_comes_back_left_of_it() {
            mouseMove(root, 250, 20)
            tryVerify(() => row.containsMouse, undefined, "right of the box the hand is on the row")
            mouseMove(root, 130, 20)
            tryVerify(() => !row.containsMouse, undefined, "on the box it is off the row")
            mouseMove(root, 50, 20)
            tryVerify(() => row.containsMouse, undefined, "and left of the box on the row again")
        }
    }
}
