import QtQuick
import QtTest

// What Qt hands a list row for the two presses of a double-click, and what a Flickable takes away from a row that
// lets go of the drag — the three facts the graph's rows answer their presses on
// (デザイン規約 §グラフ行のダブルクリック).
//
// **The second click of a double-click arrives as a press and never as a click**, so a surface that answers on the
// press sees both halves and a surface that answers on the release sees only the first. That is why the gesture's
// guard sits on the press path (`ReclickGesture.click`): without it the second press would select again and arm the
// name box under a reader who is going somewhere.
Item {
    id: root
    width: 300
    height: 300

    property var log: []
    /// Whether the row keeps the drag it was handed. False is what every row in this window used to be.
    property bool rowHolds: false

    Flickable {
        id: flick
        anchors.fill: parent
        contentHeight: 3000
        Item {
            width: 300
            height: 3000
            MouseArea {
                id: row
                anchors.fill: parent
                preventStealing: root.rowHolds
                onPressed: root.log.push("press")
                onClicked: root.log.push("click")
                onDoubleClicked: root.log.push("double")
                onCanceled: root.log.push("cancel")
            }
        }
    }

    TestCase {
        name: "RowPressOrder"
        when: windowShown

        function init() {
            root.log = []
            flick.contentY = 0
        }

        function test_the_second_click_arrives_as_a_press_and_never_as_a_click() {
            mouseDoubleClickSequence(row, 40, 40, Qt.LeftButton)
            compare(JSON.stringify(root.log), JSON.stringify(["press", "click", "press", "double"]),
                    "Qt raises the second press before the double, and withholds the second click")
        }

        function test_a_drag_past_the_threshold_takes_the_click_off_a_row_that_lets_go() {
            root.rowHolds = false
            mousePress(row, 40, 40, Qt.LeftButton)
            for (let i = 1; i <= 12; ++i)
                mouseMove(row, 40, 40 + i * 3)
            mouseRelease(row, 40, 76, Qt.LeftButton)
            compare(JSON.stringify(root.log), JSON.stringify(["press", "cancel"]),
                    "the view takes the grab at the platform's drag distance and the row is told it lost it")
            verify(flick.contentY !== 0, "and the list moved under the hand instead")
        }

        function test_a_row_that_holds_keeps_both_its_click_and_its_ground() {
            root.rowHolds = true
            mousePress(row, 40, 40, Qt.LeftButton)
            for (let i = 1; i <= 12; ++i)
                mouseMove(row, 40, 40 + i * 3)
            mouseRelease(row, 40, 76, Qt.LeftButton)
            compare(JSON.stringify(root.log), JSON.stringify(["press", "click"]),
                    "the row keeps the drag it was handed")
            compare(flick.contentY, 0, "and the list stays where the reader left it")
        }
    }
}
