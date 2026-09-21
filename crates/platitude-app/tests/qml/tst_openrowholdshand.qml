import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A row of the left panel with its lines open under it is one row, lines included: the hand walking down from the
// row's own line into what it opened has not left the row, so the row stays lit and the lines stay out under the
// hand reaching for them (デザイン規約 §左メニューの所作, 規約 §hover のツールチップ「出したものは持ち帰れる」).
//
// **Measured with the real row, in a panel that counts the hand the way the sidebar does** (`SidebarPane.handWatch`
// → `SidebarRowGestures.handMoves` → `NavItemDelegate.syncHover`). The lines take hover of their own — the block's,
// for a working copy's path; a noted line's; the words' (`CardText`) — and a hover-taking subtree stacked as a
// sibling over the row's own handler takes the pointer off that handler (rules-refs/app-ui.md 「奪うのは覆う
// `MouseArea` の子孫でない hover 持ちだけ」). So the row reads its lines' hover as its own, and this is what holds
// it to that.
Item {
    id: root
    width: 320
    height: 240

    // The row's section, answering with the one reading a branch row opens a line for. The table and the ink are
    // read in `tst_navfacts.qml`; here the stand-in only has to give the row a line to walk into.
    QtObject {
        id: sections
        function upstreamOf(name) { return "origin/feature/topic-a" }
        function trackedBy(full) { return "" }
        function tagRemotes(name, against) { return [] }
    }
    QtObject {
        id: branches
        function upstreamOf(name) { return "" }
        function upstreamGoneOf(name) { return "" }
        function aheadOf(name) { return 0 }
        function behindOf(name) { return 0 }
    }
    QtObject {
        id: copies
        function worktreeHolding(name) { return "" }
    }

    /// The panel the row stands in, counting the hand by the place it was last seen in — the sidebar's own reading
    /// (`SidebarPane`), so the row reads its hover exactly when the product's rows do.
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
            // The one gesture the sidebar keeps for these rows, cut to what the row asks of it: the key the row
            // hands up is the open one (`SidebarRowGestures.openFacts` / `closeFacts`).
            onFactsAsked: (open, at) => row.openKey = open ? row.rowKey : ""
        }
    }

    TestCase {
        id: testCase
        name: "OpenRowHoldsTheHand"
        when: windowShown

        /// The row under test, built fresh for each case and taken down however the case ended — one left standing
        /// is what the next case's hand would land on.
        property Item row: null
        function init() {
            testCase.row = rowMaker.createObject(panel)
        }
        function cleanup() {
            testCase.row.destroy()
            testCase.row = null
        }

        /// The turn the row reads its hover in has passed. The row reads a turn after the hand moved
        /// (`NavItemDelegate.onHandMovesChanged` → `Qt.callLater`), and a call queued after the move runs after the
        /// row's own, so waiting on it is waiting on that read. **Not a frame**: a still offscreen scene need not
        /// draw one (`waitForRendering` came back false in the container with the read long done).
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
            // The hand comes to rest on the row's own line, and the row opens after the rest every supplement
            // waits (`Metrics.tipDelayMs`).
            mouseMove(panel, panel.width / 2, Theme.rowHeight / 2)
            tryCompare(row, "pointed", true, undefined, "the row is under the hand")
            tryCompare(row, "factsOpen", true, undefined, "and opens its lines under itself")
            tryVerify(() => row.factsItem !== null && row.factsItem.height > 0, undefined,
                      "the lines are built and stand under the row's own line")

            // The hand walks down into the lines — onto the words, which is where a reader goes. **Two witnesses
            // outside the row's own answer**: the panel heard the hand move (so the row was asked to read its hover
            // again), and the lines saw it arrive (so the hand is on them, not still on the row's line).
            const heard = panel.handMoves
            mouseMove(panel, panel.width / 2, Theme.rowHeight + row.factsItem.height / 2)
            verify(panel.handMoves > heard, "the panel hears the hand move into the lines")
            verify(row.factsItem.pointed, "and the lines see the hand on them")
            // The row reads its hover a turn after the hand moved (`syncHover`): let that turn pass.
            testCase.turnPassed()
            compare(row.pointed, true, "the hand on the lines is the hand on the row")
            compare(row.factsOpen, true, "so the lines stay out under it")

            // Out of the row altogether, and it lets go: what holds it is the hand, not the test.
            mouseMove(panel, panel.width / 2, row.height + Theme.rowHeight)
            tryCompare(row, "factsOpen", false, undefined, "the row closes once the hand is off it")
        }
    }
}
