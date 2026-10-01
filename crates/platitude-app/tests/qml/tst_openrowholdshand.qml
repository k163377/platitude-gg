import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A row with its lines open is one row, lines included: a hand walking down into them has not left it, so the row
// stays lit and the lines stay out (デザイン規約 §左メニューの所作, 規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」).
//
// The real row, in a panel that counts the hand the way the sidebar does (`SidebarPane.handWatch` →
// `SidebarRowGestures.handMoves` → `NavItemDelegate.syncHover`). The lines take hover of their own (`CardText` among
// them), and stacked as a sibling over the row's handler they take the pointer off it (rules-refs/app-ui.md
// 「奪うのは覆う `MouseArea` の子孫でない hover 持ちだけ」) — so the row has to read its lines' hover as its own.
Item {
    id: root
    width: 320
    height: 240
    // What `Main.qml` and the popup bases declare (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    /// Empty leaves the line going nowhere; a commit makes it a target (`NavFacts.place`), which lays a band over it.
    property string readingAt: ""

    // Stand-ins that give the row one line to walk into; the table itself is read in `tst_navfacts.qml`.
    QtObject {
        id: sections
        function upstreamOf(name) { return "origin/feature/topic-a" }
        function upstreamOidOf(name) { return root.readingAt }
        function trackedBy(full) { return "" }
        function tagRemotes(name, against) { return [] }
    }
    QtObject {
        id: branches
        function upstreamOf(name) { return "" }
        function upstreamOidOf(name) { return "" }
        function upstreamGoneOf(name) { return "" }
        function aheadOf(name) { return 0 }
        function behindOf(name) { return 0 }
        function oidOfName(name) { return "" }
    }
    QtObject {
        id: copies
        function worktreeHolding(name) { return "" }
        function headOfCopy(path) { return "" }
    }

    Item {
        id: panel
        anchors.fill: parent
        property int handMoves: 0
        property point handAt: Qt.point(-1, -1)
        HoverHandler {
            id: handWatch
            onPointChanged: {
                if (handWatch.point.position.x === panel.handAt.x
                        && handWatch.point.position.y === panel.handAt.y)
                    return
                panel.handAt = handWatch.point.position
                panel.handMoves++
            }
            onHoveredChanged: {
                panel.handAt = Qt.point(-1, -1)
                panel.handMoves++
            }
        }
    }

    Component {
        id: rowMaker
        NavItemDelegate {
            id: row
            index: 0
            name: "feature/topic-a"
            full: "feature/topic-a"
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
            kindHint: "branch"
            rowKey: "branch:feature/topic-a"
            listWidth: panel.width
            opensFacts: true
            sectionModel: sections
            branchesModel: branches
            worktreesModel: copies
            // The panel counts the hand for this row, as the sidebar's lists do (`NavList`).
            handCounted: true
            handMoves: panel.handMoves
            // `SidebarRowGestures.openFacts` / `closeFacts`, cut down: the key the row hands up becomes the open one.
            onFactsAsked: (open, at) => row.openKey = open ? row.rowKey : ""
        }
    }

    TestCase {
        id: testCase
        name: "OpenRowHoldsTheHand"
        when: windowShown

        /// Built per case and taken down however the case ended — a leftover row is what the next case's hand would
        /// land on.
        property Item row: null
        function init() {
            root.readingAt = ""
            testCase.row = rowMaker.createObject(panel)
        }
        function cleanup() {
            testCase.row.destroy()
            testCase.row = null
        }

        /// The row reads its hover a turn after the hand moved (`NavItemDelegate.onHandMovesChanged` →
        /// `Qt.callLater`); a call queued after the move runs after the row's, so waiting on it waits on that read.
        /// Not a frame: a still offscreen scene need not draw one, and `waitForRendering` then returns false.
        property int turns: 0
        function turnDone() {
            testCase.turns++
        }
        function turnPassed() {
            const before = testCase.turns
            Qt.callLater(testCase.turnDone)
            tryCompare(testCase, "turns", before + 1, undefined, "the turn after the hand moved has passed")
        }

        function test_the_hand_walking_into_the_lines_is_still_on_the_row() {
            const row = testCase.row
            // The row opens after the usual rest (`Metrics.tipDelayMs`).
            mouseMove(panel, panel.width / 2, Theme.rowHeight / 2)
            tryCompare(row, "pointed", true, undefined, "the row is under the hand")
            tryCompare(row, "factsOpen", true, undefined, "and opens its lines under itself")
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined,
                      "the lines are built and stand under the row's own line")

            // Down onto the words. Two witnesses outside the row's own answer: the panel heard the move (so the row
            // was asked to read again), and the lines saw the hand arrive.
            const heard = panel.handMoves
            mouseMove(panel, panel.width / 2, Theme.rowHeight + row.factsItem.height / 2)
            verify(panel.handMoves > heard, "the panel hears the hand move into the lines")
            verify(row.factsItem.pointed, "and the lines see the hand on them")
            testCase.turnPassed()
            compare(row.pointed, true, "the hand on the lines is the hand on the row")
            compare(row.factsOpen, true, "so the lines stay out under it")

            // What holds the row open is the hand, not the test.
            mouseMove(panel, panel.width / 2, row.height + Theme.rowHeight)
            tryCompare(row, "factsOpen", false, undefined, "the row closes once the hand is off it")
        }

        /// Onto a line that goes somewhere (`NavRowFacts.aimRow`): its band lights from the line alone, never from the
        /// row's own line above, and the hand on it is still on the row.
        function test_the_hand_walking_onto_a_line_that_goes_lights_that_line_alone() {
            root.readingAt = "abc"
            const row = testCase.row
            mouseMove(panel, panel.width / 2, Theme.rowHeight / 2)
            tryCompare(row, "factsOpen", true, undefined, "the row opens under the hand")
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined, "and its lines stand")
            compare(row.factsLines[0].to.oid, "abc", "the line goes somewhere")
            // Every height of the row's own line, its foot included — the band must not reach up into it.
            for (let y = 1; y < Theme.rowHeight; y += 2) {
                mouseMove(panel, panel.width / 2, y)
                testCase.turnPassed()
                verify(row.factsItem !== null, "the row is still open with the hand on its own line (y " + y + ")")
                verify(!row.factsItem.lineAimed(0), "the line is not lit from the row's own line (y " + y + ")")
            }
            mouseMove(panel, panel.width / 2, Theme.rowHeight + row.factsItem.height / 2)
            testCase.turnPassed()
            verify(row.factsItem.lineAimed(0), "on the line, the line is lit")
            compare(row.pointed, true, "and the hand on it is the hand on the row")
            compare(row.factsOpen, true, "so the lines stay out under it")
        }
    }
}
