import QtQuick
import QtTest
import platitude.ui

// What a file row asks the line-ending card for, and how every ask ends (`NavItemDelegate.eolAskFor`): the hand
// walking off, the row's path changing under the hand, the row being pooled — each sends the leave for the path that
// asked, so the pane's seat is never left waiting on a row that let go (`WipPane.pointEolIn` / `unpointEolIn`). A mark
// that goes comes with a status read, which the pane weighs (`wip-reread fixed`). The real row, under a real pointer,
// on a panel that keeps the hand as the WIP pane does.
Item {
    id: root
    width: 320
    height: 120

    /// What the row said, in order: `ask:<path>` / `leave:<path>`.
    property var said: []

    Item {
        id: panel
        anchors.fill: parent
        HandWatch {
            id: hand
        }
    }

    Component {
        id: rowMaker
        NavItemDelegate {
            id: row
            index: 0
            name: "flipped.kt"
            full: "src/flipped.kt"
            oid_hex: ""
            change: "M"
            bucket: "unstaged"
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            folder: false
            eol_mark: true
            ahead: 0
            behind: 0
            kindHint: "file"
            rowKey: "file:src/flipped.kt"
            listWidth: panel.width
            paneHand: hand
            // The list the row stands in names the bucket (`WipBucketPane.section`); the row says the path.
            onEolPointed: (path, on) => root.said.push((on ? "ask:" : "leave:") + "unstaged:" + path)
        }
    }

    TestCase {
        id: testCase
        name: "EolAsk"
        when: windowShown

        property Item row: null
        function init() {
            mouseMove(root, root.width - 1, root.height - 1)
            root.said = []
            testCase.row = rowMaker.createObject(panel)
        }
        function cleanup() {
            testCase.row.destroy()
            testCase.row = null
        }
        function restOnRow() {
            mouseMove(panel, panel.width / 2, Theme.rowHeight / 2)
            tryVerify(() => testCase.row.handOn, undefined, "the hand is on the row")
            compare(root.said, ["ask:unstaged:src/flipped.kt"], "the row asks for its path")
        }

        function test_the_hand_walking_off_sends_the_leave() {
            restOnRow()
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !testCase.row.handOn)
            compare(root.said, ["ask:unstaged:src/flipped.kt", "leave:unstaged:src/flipped.kt"])
        }

        /// A recycled delegate is handed another path with the hand where it was: the leave is the path that asked,
        /// the ask the new one.
        function test_the_path_changing_under_the_hand_leaves_the_old_and_asks_the_new() {
            restOnRow()
            testCase.row.full = "src/mixed.kt"
            compare(root.said,
                    ["ask:unstaged:src/flipped.kt", "leave:unstaged:src/flipped.kt", "ask:unstaged:src/mixed.kt"])
        }

        /// A delegate the view pools forgets its hand (`forgetHand`), and lets go of what it asked for.
        function test_a_pooled_row_lets_go() {
            restOnRow()
            testCase.row.forgetHand()
            compare(root.said, ["ask:unstaged:src/flipped.kt", "leave:unstaged:src/flipped.kt"])
        }
    }
}
