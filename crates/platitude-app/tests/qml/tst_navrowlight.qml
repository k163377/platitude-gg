import QtQuick
// For the attached ToolTip alone.
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Which row of the left panel wears the light, with a real pointer over the real `NavItemDelegate`: the row the hand
// walked onto, kept while the list moves under a still hand (デザイン規約 §左メニューの所作).
//
// Rows read their hover only when the hand moves: a row arriving under a still pointer is handed hover
// (`tst_hoverunderstillhand.qml`), so the panel counts the place it last saw the hand and tells the rows to read again
// only when that changed.
Item {
    id: root
    width: 300
    height: 200

    /// The panel's count, as the rows are handed it (`SidebarRowGestures.handMoves`).
    property int handMoves: 0

    Column {
        id: rows
        width: 260
        // Stands in for the first row opening: it grows and pushes the second down.
        property bool firstOpen: false

        NavItemDelegate {
            id: top
            index: 0
            name: "topic"
            full: "C:/somewhere/topic"
            oid_hex: ""
            change: ""
            bucket: ""
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            folder: false
            eol_mark: false
            ahead: 0
            behind: 0
            listWidth: rows.width
            height: Theme.rowHeight + (rows.firstOpen ? Theme.rowHeight * 3 : 0)
            handMoves: root.handMoves
            handCounted: true
            kindHint: "worktree"
        }
        NavItemDelegate {
            id: below
            index: 1
            name: "spike"
            full: "spike"
            oid_hex: ""
            change: ""
            bucket: ""
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            folder: false
            eol_mark: false
            ahead: 0
            behind: 0
            listWidth: rows.width
            height: Theme.rowHeight
            handMoves: root.handMoves
        }
    }

    /// Another target for the shared box, as the next section the hand walks onto is (`SharedToolTip.handOver`).
    Item {
        id: elsewhere
        width: 40
        height: 20
        property bool asking: false
        ToolTip.visible: elsewhere.asking
        ToolTip.text: "somewhere else"
    }

    TestCase {
        name: "NavRowLight"
        when: windowShown

        /// Rows read the hand a turn after the panel counts it (`NavItemDelegate.syncHover` on `Qt.callLater`). The
        /// queue runs in order and a re-queued call moves to the back (qqmldelayedcallqueue.cpp
        /// `addUniquelyAndExecuteLater` erases the entry and appends it), so this call landing means the rows have
        /// read.
        function rowsRead() {
            let read = false
            Qt.callLater(() => { read = true })
            tryVerify(() => read, undefined, "the rows have read where the hand is")
        }

        function test_the_row_the_hand_walked_onto_keeps_the_light() {
            rows.firstOpen = false
            root.handMoves = 0

            mouseMove(top, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.pointed, undefined, "the row the hand walked onto is lit")
            verify(!below.pointed, "and the one below it is not")

            // Qt re-hands hover as the layout moves, but nothing tells the rows to read again: the hand did not move.
            rows.firstOpen = true
            verify(waitForRendering(rows), "the row is taller on screen")
            verify(top.pointed, "the row the hand is in keeps the light")
            verify(!below.pointed, "and nothing else takes it")
        }

        /// A working copy's path is the one answer its row cannot show, so the rest on the row's own name puts it
        /// out — walking down to the lines would be a second rest for it (デザイン規約 §左メニューの所作).
        function test_the_rest_on_the_rows_own_line_asks_for_the_supplement() {
            rows.firstOpen = false
            root.handMoves = 0
            top.opensFacts = true
            top.rowKey = "worktree:topic"
            top.openKey = "worktree:topic"
            verify(top.factsItem !== null, "the row has its lines open")

            // Start off the row, so the answer below is this rest's and not the previous case's pointer.
            mouseMove(below, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.factsItem.says === "", undefined, "nothing is asked for off a row nobody is on")

            mouseMove(top, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.factsItem.says === top.factsPath, undefined,
                      "the hand on the row's own line asks for where the copy stands")
            tryVerify(() => top.factsTipOut, undefined, "and the box comes out on the rest already taken")
        }

        /// The tip is a popup that takes the pointer as the hand walks into it, so a row reading only its own hover
        /// would close under the words the reader was reaching for
        /// (デザイン規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」).
        function test_a_row_holds_itself_open_under_its_own_supplement() {
            rows.firstOpen = false
            root.handMoves = 0

            // The row opens and puts the path out; `pointedTipRow` stands in for the hand resting on its name.
            top.opensFacts = true
            top.rowKey = "worktree:topic"
            top.openKey = "worktree:topic"
            verify(top.factsItem !== null, "the row has its lines open")
            top.pointedTipRow = top.index
            tryVerify(() => top.factsTipOut, undefined, "and the supplement is standing on it")
            // The row takes the light a turn after the box comes out (`NavItemDelegate.onFactsTipOutChanged`).
            tryVerify(() => top.pointed, undefined, "and the row wears the light its supplement holds")

            // The hand walks off the row into the tip and the panel counts the move; the claim is what the row reads
            // a turn later.
            mouseMove(below, 100, Theme.rowHeight / 2)
            root.handMoves++
            rowsRead()
            verify(top.pointed, "the row stays the row the reader is on while its own supplement stands")

            top.pointedTipRow = -1
            tryVerify(() => !top.pointed, undefined, "the row gives up the light with the words it put out")
        }

        /// The shared box is handed to another target without this row being told (`SharedToolTip.handOver`), so a
        /// row reading its own attached property would stay open behind a hand long gone.
        function test_a_row_lets_go_when_the_box_is_handed_to_another_target() {
            rows.firstOpen = false
            root.handMoves = 0
            elsewhere.asking = false

            top.opensFacts = true
            top.rowKey = "worktree:topic"
            top.openKey = "worktree:topic"
            verify(top.factsItem !== null, "the row has its lines open")
            top.pointedTipRow = top.index
            tryVerify(() => top.factsTipOut, undefined, "the supplement is standing on this row")

            // The one instance moves, words and target together.
            elsewhere.asking = true
            tryVerify(() => !top.factsTipOut, undefined, "the box standing is no longer this row's")
            tryVerify(() => !top.pointed, undefined, "so the row gives the light up")
            // Reset the stand-in pointer, or the next case opens on it.
            top.pointedTipRow = -1
        }

        function test_a_hand_that_moves_takes_the_light_with_it() {
            rows.firstOpen = false
            root.handMoves = 0

            mouseMove(top, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => top.pointed, undefined, "the hand is on the first row")

            mouseMove(below, 100, Theme.rowHeight / 2)
            root.handMoves++
            tryVerify(() => below.pointed, undefined, "the row it walked to takes the light")
            // Waited for: the count here is raised by hand with the move, so the rows can read before Qt hands the
            // hover on (the panel's own count is raised by the delivery itself).
            tryVerify(() => !top.pointed, undefined,
                      "and the one it left gives it up — the light stands on one row")
        }
    }
}
